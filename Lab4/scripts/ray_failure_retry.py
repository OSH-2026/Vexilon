#!/usr/bin/env python3
"""
Ray Failure-Retry Inference — OSH 2026 Lab4 Role C (Bonus)
============================================================
Distributes prompts across multiple llama.cpp servers with automatic
failure detection and retry on alternative servers.

Failure types handled:
  - Connection refused (server down)
  - Timeout (server hung)
  - HTTP 5xx (server error)
  - Response parse failure (malformed JSON)

Retry strategy:
  - Initial assignment via round-robin.
  - On failure: mark the failing server as suspected, try the next
    available server in the pool.
  - Up to --max-retries total attempts (including the initial try).
  - If all servers exhausted or retries exhausted, record failure.

Usage:
  python3 ray_failure_retry.py \
    --prompts Lab4/prompts/ray_prompts_30.jsonl \
    --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
    --output Lab4/results/ray_failure_retry.csv \
    --log Lab4/results/ray_failure_retry.log \
    --timeout 60 --max-retries 2
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import sys
import time
import traceback
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional, Tuple

import requests

# ---------------------------------------------------------------------------
# Ray check
# ---------------------------------------------------------------------------
_RAY_AVAILABLE = False
try:
    import ray  # type: ignore
    _RAY_AVAILABLE = True
except ImportError:
    pass

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------
DEFAULT_TIMEOUT = 60
DEFAULT_MAX_RETRIES = 2

DETAIL_COLUMNS = [
    "request_id",
    "prompt_id",
    "original_worker",
    "final_worker",
    "original_server_url",
    "final_server_url",
    "start_time_iso",
    "end_time_iso",
    "latency_s",
    "success",
    "retry_count",
    "error_message",
    "output_chars",
]


# ===================================================================
# Structured logger
# ===================================================================


class RetryLogger:
    """Thread-safe-ish structured logger for retry events."""

    def __init__(self, log_path: str):
        out_dir = os.path.dirname(os.path.abspath(log_path))
        os.makedirs(out_dir, exist_ok=True)
        self._fh = open(log_path, "w", encoding="utf-8")
        self._fh.write(
            "# Ray Failure-Retry Log\n"
            f"# Started: {datetime.now(timezone.utc).isoformat()}\n"
            "# Format: TIMESTAMP | LEVEL | prompt_id | event | worker | server_url | detail\n"
            "# Fields: timestamp, level, prompt_id, event_type, worker_id, server_url, detail\n"
            "---\n"
        )
        self._fh.flush()

    def log(
        self,
        level: str,
        prompt_id: str,
        event: str,
        worker_id: str = "-",
        server_url: str = "-",
        detail: str = "",
    ) -> None:
        ts = datetime.now(timezone.utc).isoformat()
        line = f"{ts} | {level:<5} | {prompt_id:<6} | {event:<20} | {worker_id:<10} | {server_url:<30} | {detail}"
        self._fh.write(line + "\n")
        self._fh.flush()
        # Also print to stdout for visibility
        print(f"  [LOG] {level} {prompt_id} {event} {worker_id} {detail[:80]}")

    def close(self) -> None:
        self._fh.write(f"\n# Finished: {datetime.now(timezone.utc).isoformat()}\n")
        self._fh.close()


# ===================================================================
# Prompt loading
# ===================================================================


def load_prompts(path: str) -> List[Dict[str, str]]:
    items: List[Dict[str, str]] = []
    with open(path, "r", encoding="utf-8") as fh:
        for lineno, line in enumerate(fh, start=1):
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError as exc:
                print(f"[WARN] Skipping line {lineno}: {exc}", file=sys.stderr)
                continue
            items.append({
                "id": obj.get("id", f"L{lineno:04d}"),
                "category": obj.get("category", "unknown"),
                "prompt": obj.get("prompt", ""),
            })
    return items


# ===================================================================
# llama-server HTTP client (with failure classification)
# ===================================================================


class InferenceError(Exception):
    """Classified inference error."""

    def __init__(self, error_type: str, message: str, status_code: Optional[int] = None):
        self.error_type = error_type  # connection_refused, timeout, http_5xx, parse_error, unknown
        self.message = message
        self.status_code = status_code
        super().__init__(message)


def _build_chat_payload(prompt_text: str) -> Dict[str, Any]:
    return {
        "model": "local-model",
        "messages": [{"role": "user", "content": prompt_text}],
        "max_tokens": 256,
        "temperature": 0.0,
    }


def _build_completion_payload(prompt_text: str) -> Dict[str, Any]:
    return {"prompt": prompt_text, "n_predict": 256, "temperature": 0.0}


def _extract_text_chat(resp_json: Dict[str, Any]) -> Optional[str]:
    try:
        return resp_json["choices"][0]["message"]["content"]
    except (KeyError, IndexError, TypeError):
        return None


def _extract_text_completion(resp_json: Dict[str, Any]) -> Optional[str]:
    try:
        return resp_json.get("content", "")
    except (KeyError, TypeError):
        return None


def call_llama_server(
    server_url: str,
    prompt_text: str,
    timeout_s: int = DEFAULT_TIMEOUT,
) -> Dict[str, Any]:
    """Send a prompt to a llama-server.  Raises InferenceError on failure.

    Returns dict with: start_time_iso, end_time_iso, latency_s,
    output_chars, output_text, success, status_code, error_message.
    """
    chat_url = f"{server_url.rstrip('/')}/v1/chat/completions"
    completion_url = f"{server_url.rstrip('/')}/completion"

    start_dt = datetime.now(timezone.utc)
    start_ts = time.monotonic()

    result: Dict[str, Any] = {
        "start_time_iso": start_dt.isoformat(),
        "end_time_iso": "",
        "latency_s": 0.0,
        "output_chars": 0,
        "output_text": "",
        "success": False,
        "status_code": None,
        "error_message": "",
    }

    def _finish(success: bool, status_code: Optional[int], error: str, text: str) -> Dict[str, Any]:
        result["end_time_iso"] = datetime.now(timezone.utc).isoformat()
        result["latency_s"] = round(time.monotonic() - start_ts, 4)
        result["success"] = success
        result["status_code"] = status_code
        result["error_message"] = error
        result["output_text"] = text
        result["output_chars"] = len(text)
        return result

    # --- Attempt 1: /v1/chat/completions ---
    chat_err = ""
    try:
        payload = _build_chat_payload(prompt_text)
        resp = requests.post(
            chat_url, json=payload, timeout=timeout_s,
            headers={"Content-Type": "application/json"},
        )
        if resp.status_code == 200:
            try:
                resp_json = resp.json()
            except json.JSONDecodeError:
                raise InferenceError("parse_error", f"Invalid JSON from {server_url}/v1/chat/completions")
            text = _extract_text_chat(resp_json) or ""
            return _finish(True, 200, "", text)

        if resp.status_code >= 500:
            raise InferenceError(
                "http_5xx",
                f"Server error {resp.status_code} from {server_url}",
                status_code=resp.status_code,
            )
        # 4xx — also treated as error
        chat_err = f"HTTP {resp.status_code}: {resp.text[:200]}"
    except requests.exceptions.ConnectionError as exc:
        raise InferenceError(
            "connection_refused",
            f"Connection refused on {server_url}: {exc}",
        )
    except requests.exceptions.Timeout:
        raise InferenceError(
            "timeout",
            f"Request timeout ({timeout_s}s) on {server_url}",
        )
    except InferenceError:
        raise
    except Exception as exc:
        chat_err = f"Unexpected: {exc}"

    # --- Attempt 2: /completion (legacy) ---
    try:
        payload = _build_completion_payload(prompt_text)
        resp = requests.post(
            completion_url, json=payload, timeout=timeout_s,
            headers={"Content-Type": "application/json"},
        )
        if resp.status_code == 200:
            try:
                resp_json = resp.json()
            except json.JSONDecodeError:
                raise InferenceError("parse_error", f"Invalid JSON from {server_url}/completion")
            text = _extract_text_completion(resp_json) or ""
            return _finish(True, 200, "", text)

        if resp.status_code >= 500:
            raise InferenceError(
                "http_5xx",
                f"Server error {resp.status_code} from {server_url}/completion",
                status_code=resp.status_code,
            )
        raise InferenceError(
            "http_error",
            f"Both endpoints failed. Chat: {chat_err}; Completion: HTTP {resp.status_code}",
            status_code=resp.status_code,
        )
    except requests.exceptions.ConnectionError as exc:
        raise InferenceError("connection_refused", f"Connection refused on {server_url}: {exc}")
    except requests.exceptions.Timeout:
        raise InferenceError("timeout", f"Request timeout ({timeout_s}s) on {server_url}/completion")
    except InferenceError:
        raise
    except Exception as exc:
        raise InferenceError("unknown", f"Both endpoints failed. Chat: {chat_err}; Completion: {exc}")


# ===================================================================
# Ray Actor
# ===================================================================


def _ensure_ray() -> None:
    if not _RAY_AVAILABLE:
        print("ERROR: Ray is not installed. Install: pip install ray", file=sys.stderr)
        sys.exit(1)
    if not ray.is_initialized():
        # Monkey-patch Ray's IP detection to force 127.0.0.1
        import ray.util as _ray_util
        _ray_util.get_node_ip_address = lambda address=None: "127.0.0.1"

        ray.init(address="local", ignore_reinit_error=True)
        print("[Ray] Initialised (local mode)")


def _shutdown_ray() -> None:
    if _RAY_AVAILABLE and ray.is_initialized():
        ray.shutdown()
        print("[Ray] Shut down")


if _RAY_AVAILABLE:

    @ray.remote
    class LlamaServerActor:
        """Ray Actor wrapping one llama-server endpoint."""

        def __init__(self, worker_id: str, server_url: str, timeout_s: int = DEFAULT_TIMEOUT):
            self.worker_id = worker_id
            self.server_url = server_url
            self.timeout_s = timeout_s
            self._req_count = 0
            print(f"[Actor:{worker_id}] Init server={server_url}")

        def infer(self, prompt_item: Dict[str, Any], request_id: str) -> Dict[str, Any]:
            """Attempt inference.  Catches all errors and always returns a result dict.

            The caller checks the 'success' and 'error_type' fields to decide
            whether to retry on another server.
            """
            self._req_count += 1
            prompt_text = prompt_item.get("prompt", "")

            try:
                result = call_llama_server(self.server_url, prompt_text, timeout_s=self.timeout_s)
                error_type = "" if result["success"] else "unknown"
                error_msg = result.get("error_message", "")
            except InferenceError as exc:
                result = {
                    "start_time_iso": datetime.now(timezone.utc).isoformat(),
                    "end_time_iso": datetime.now(timezone.utc).isoformat(),
                    "latency_s": 0.0,
                    "output_chars": 0,
                    "success": False,
                    "status_code": exc.status_code,
                    "error_message": exc.message,
                }
                error_type = exc.error_type
                error_msg = exc.message
            except Exception as exc:
                result = {
                    "start_time_iso": datetime.now(timezone.utc).isoformat(),
                    "end_time_iso": datetime.now(timezone.utc).isoformat(),
                    "latency_s": 0.0,
                    "output_chars": 0,
                    "success": False,
                    "status_code": None,
                    "error_message": str(exc)[:500],
                }
                error_type = "actor_exception"
                error_msg = str(exc)

            row = {
                "request_id": request_id,
                "prompt_id": prompt_item.get("id", "?"),
                "original_worker": prompt_item.get("_original_worker", self.worker_id),
                "final_worker": self.worker_id,
                "original_server_url": prompt_item.get("_original_server_url", self.server_url),
                "final_server_url": self.server_url,
                **{k: result[k] for k in
                   ["start_time_iso", "end_time_iso", "latency_s",
                    "output_chars", "success", "error_message"]},
                "retry_count": prompt_item.get("_retry_count", 0),
                "_error_type": error_type,  # internal: used by caller for logging
            }
            status = "OK" if result["success"] else f"FAIL({error_type})"
            print(f"[Actor:{self.worker_id}] {request_id} → {prompt_item.get('id')} "
                  f"({result['latency_s']:.2f}s {status})")
            return row


# ===================================================================
# Core retry logic
# ===================================================================


def _pick_next_server(
    failed_url: str,
    all_urls: List[str],
    attempted_urls: List[str],
) -> Optional[Tuple[str, str]]:
    """Pick the next server URL that hasn't been tried yet.

    Returns (worker_id, server_url) or None if all exhausted.
    """
    for i, url in enumerate(all_urls):
        if url not in attempted_urls:
            return (f"worker_{i}", url)
    return None


def run_with_retry(
    prompts: List[Dict[str, str]],
    server_urls: List[str],
    timeout_s: int,
    max_retries: int,
    logger: RetryLogger,
) -> Tuple[List[Dict[str, Any]], Dict[str, int]]:
    """Main execution loop with retry-on-failure.

    Strategy:
      1. Initial assignment: round-robin across server_urls.
      2. Submit to the assigned actor.
      3. If InferenceError is raised, log the failure, pick the next
         available server, and retry.
      4. Up to max_retries total attempts.
      5. If all servers exhausted or retries exhausted → final failure.

    Returns (rows, stats).
    """
    _ensure_ray()

    # Create one actor per server
    actors: Dict[str, Any] = {}
    for i, url in enumerate(server_urls):
        wid = f"worker_{i}"
        actor = LlamaServerActor.options(name=f"fr_{wid}").remote(  # type: ignore[attr-defined]
            worker_id=wid, server_url=url, timeout_s=timeout_s,
        )
        actors[wid] = actor

    print(f"[retry] {len(actors)} actor(s) for {len(server_urls)} server(s)")
    logger.log("INFO", "SYSTEM", "init", detail=f"{len(server_urls)} servers: {server_urls}")

    stats = {
        "total": len(prompts),
        "first_try_success": 0,
        "retry_success": 0,
        "final_failure": 0,
    }

    rows: List[Dict[str, Any]] = []
    t0 = time.monotonic()

    for idx, item in enumerate(prompts):
        prompt_id = item["id"]
        # Initial round-robin assignment
        initial_idx = idx % len(server_urls)
        initial_wid = f"worker_{initial_idx}"
        initial_url = server_urls[initial_idx]

        print(f"\n[retry] ── Prompt {prompt_id} (idx={idx}) ──")
        print(f"[retry] Initial → {initial_wid} ({initial_url})")
        logger.log("INFO", prompt_id, "initial_assign", initial_wid, initial_url,
                    f"idx={idx}, round_robin")

        attempted_urls: List[str] = []
        final_row: Optional[Dict[str, Any]] = None
        retry_count = 0

        current_url = initial_url
        current_wid = initial_wid

        for attempt in range(1 + max_retries):  # 1 initial + N retries
            retry_count = attempt
            actor = actors[current_wid]

            # Annotate item with tracking metadata
            annotated = {
                **item,
                "_original_worker": initial_wid,
                "_original_server_url": initial_url,
                "_retry_count": retry_count,
            }

            req_id = f"{prompt_id}-att{attempt}"

            row = ray.get(actor.infer.remote(annotated, req_id), timeout=timeout_s + 30)

            if row.get("success"):
                # Success!
                row["retry_count"] = retry_count
                row["original_worker"] = initial_wid
                row["original_server_url"] = initial_url
                row["final_worker"] = current_wid
                row["final_server_url"] = current_url

                if retry_count == 0:
                    stats["first_try_success"] += 1
                    logger.log("OK", prompt_id, "first_try_ok", current_wid, current_url,
                                f"latency={row['latency_s']:.2f}s")
                else:
                    stats["retry_success"] += 1
                    logger.log("OK", prompt_id, "retry_ok", current_wid, current_url,
                                f"retry={retry_count}, latency={row['latency_s']:.2f}s")

                final_row = row
                break

            else:
                # Failure — classify and log
                error_type = row.get("_error_type", "unknown")
                error_msg = row.get("error_message", "")[:200]
                print(f"[retry] {prompt_id} ATTEMPT {attempt} FAILED ({error_type}): {error_msg[:100]}")
                logger.log("WARN", prompt_id, f"fail_{error_type}", current_wid, current_url, error_msg)
                attempted_urls.append(current_url)

            # ---- Pick next server for retry ----
            if attempt < max_retries:
                next_choice = _pick_next_server(current_url, server_urls, attempted_urls)
                if next_choice is None:
                    # All servers exhausted
                    print(f"[retry] {prompt_id} ALL SERVERS EXHAUSTED after {len(attempted_urls)} attempts")
                    logger.log("ERROR", prompt_id, "all_exhausted", "-", "-",
                                f"attempted: {attempted_urls}")
                    break
                next_wid, next_url = next_choice
                print(f"[retry] {prompt_id} RETRY → {next_wid} ({next_url})")
                logger.log("INFO", prompt_id, "retry_switch", next_wid, next_url,
                            f"attempt={attempt+1}, from={current_url}")
                current_wid = next_wid
                current_url = next_url

        # ---- After all attempts ----
        if final_row is None:
            # All retries exhausted
            stats["final_failure"] += 1
            final_row = {
                "request_id": f"{prompt_id}-FAIL",
                "prompt_id": prompt_id,
                "original_worker": initial_wid,
                "final_worker": current_wid if 'current_wid' in dir() else "?",
                "original_server_url": initial_url,
                "final_server_url": current_url if 'current_url' in dir() else "?",
                "start_time_iso": "",
                "end_time_iso": "",
                "latency_s": -1.0,
                "success": False,
                "retry_count": retry_count,
                "error_message": f"All {len(server_urls)} server(s) exhausted after {retry_count} retries. "
                                 f"Attempted: {attempted_urls}",
                "output_chars": 0,
            }
            logger.log("ERROR", prompt_id, "final_failure", "-", "-",
                        f"attempted={attempted_urls}, retries={retry_count}")

        rows.append(final_row)

    elapsed = time.monotonic() - t0
    print(f"\n[retry] Done. {len(rows)} requests in {elapsed:.1f}s")

    # Summary
    print(f"\n{'='*60}")
    print(f"  Failure Retry Summary")
    print(f"  {'='*60}")
    print(f"  Total requests:       {stats['total']}")
    print(f"  First-try success:    {stats['first_try_success']}")
    print(f"  Retry success:        {stats['retry_success']}")
    print(f"  Final failure:        {stats['final_failure']}")
    final_success_rate = (stats['total'] - stats['final_failure']) / stats['total'] * 100 if stats['total'] > 0 else 0
    print(f"  Final success rate:   {final_success_rate:.1f}%")
    print(f"  Total wall time:      {elapsed:.1f}s")
    print(f"{'='*60}\n")

    logger.log("INFO", "SYSTEM", "summary", detail=json.dumps(stats))
    logger.log("INFO", "SYSTEM", "done", detail=f"elapsed={elapsed:.1f}s, "
               f"success_rate={final_success_rate:.1f}%")

    return rows, stats


# ===================================================================
# CSV output
# ===================================================================


def write_csv(rows: List[Dict[str, Any]], path: str) -> None:
    out_dir = os.path.dirname(os.path.abspath(path))
    os.makedirs(out_dir, exist_ok=True)
    with open(path, "w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=DETAIL_COLUMNS, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)
    print(f"[CSV] {len(rows)} rows → {path}")


# ===================================================================
# CLI
# ===================================================================


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Ray Failure-Retry Inference — OSH 2026 Lab4 Role C (Bonus)",
    )
    parser.add_argument("--prompts", required=True, help="JSONL prompts file")
    parser.add_argument("--server-urls", required=True,
                        help="Comma-separated server URLs")
    parser.add_argument("--output", required=True, help="Output CSV path")
    parser.add_argument("--log", required=True, help="Retry log file path")
    parser.add_argument("--timeout", type=int, default=DEFAULT_TIMEOUT,
                        help=f"Request timeout (s), default={DEFAULT_TIMEOUT}")
    parser.add_argument("--max-retries", type=int, default=DEFAULT_MAX_RETRIES,
                        help=f"Max retries per request, default={DEFAULT_MAX_RETRIES}")
    return parser.parse_args(argv)


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)

    server_urls = [u.strip() for u in args.server_urls.split(",") if u.strip()]
    if len(server_urls) < 2:
        print("ERROR: At least 2 --server-urls required for retry to be meaningful.",
              file=sys.stderr)
        return 1

    print(f"Server URLs: {server_urls}")
    print(f"Timeout:     {args.timeout}s")
    print(f"Max retries: {args.max_retries}")

    prompts = load_prompts(args.prompts)
    if len(prompts) < 1:
        print("ERROR: No prompts loaded.", file=sys.stderr)
        return 1
    print(f"Prompts:     {len(prompts)} loaded")

    # Open logger
    logger = RetryLogger(args.log)

    try:
        rows, stats = run_with_retry(
            prompts, server_urls, args.timeout, args.max_retries, logger,
        )
    except KeyboardInterrupt:
        print("\n[Interrupted]", file=sys.stderr)
        logger.log("WARN", "SYSTEM", "interrupted")
        logger.close()
        _shutdown_ray()
        return 130
    finally:
        _shutdown_ray()

    write_csv(rows, args.output)
    logger.close()

    print(f"\n[retry] Output: {args.output}")
    print(f"[retry] Log:    {args.log}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
