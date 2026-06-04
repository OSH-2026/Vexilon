#!/usr/bin/env python3
"""
Ray Batch Inference Script — OSH 2026 Lab4 Role C
===================================================
Distributes a set of prompts across one or more llama.cpp servers using Ray.

Strategies:
  - serial:          No Ray; sequential requests to a single server.
  - ray_round_robin: Ray Actors; prompts assigned round-robin across workers.
  - ray_parallel:    Ray remote tasks; all prompts submitted concurrently.

Usage:
  python3 ray_batch_infer.py \
    --prompts Lab4/prompts/ray_prompts_20.jsonl \
    --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
    --strategy ray_round_robin \
    --output Lab4/results/ray_round_robin.csv \
    --timeout 120
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
from typing import Any, Dict, List, Optional

import requests

# ---------------------------------------------------------------------------
# Helper: check for Ray at import time so we can give a clear message
# ---------------------------------------------------------------------------

_RAY_AVAILABLE = False
try:
    import ray  # type: ignore

    _RAY_AVAILABLE = True
except ImportError:
    pass


# ===================================================================
# Constants
# ===================================================================

DEFAULT_TIMEOUT = 120  # seconds per request
DEFAULT_MAX_CONCURRENCY = 8

# CSV columns shared by all strategies
CSV_COLUMNS: List[str] = [
    "request_id",
    "prompt_id",
    "category",
    "strategy",
    "assigned_worker",
    "server_url",
    "start_time_iso",
    "end_time_iso",
    "latency_s",
    "output_chars",
    "success",
    "status_code",
    "error_message",
]


# ===================================================================
# Loading prompts
# ===================================================================


def load_prompts(path: str) -> List[Dict[str, str]]:
    """Load JSONL prompts.  Each line: {"id":"R001","category":"os","prompt":"..."}"""
    items: List[Dict[str, str]] = []
    with open(path, "r", encoding="utf-8") as fh:
        for lineno, line in enumerate(fh, start=1):
            line = line.strip()
            if not line:
                continue
            try:
                obj = json.loads(line)
            except json.JSONDecodeError as exc:
                print(f"[WARN] Skipping line {lineno} — invalid JSON: {exc}", file=sys.stderr)
                continue
            # Normalise keys
            items.append(
                {
                    "id": obj.get("id", f"L{lineno:04d}"),
                    "category": obj.get("category", "unknown"),
                    "prompt": obj.get("prompt", ""),
                }
            )
    return items


# ===================================================================
# llama-server HTTP client
# ===================================================================


def _build_chat_payload(prompt_text: str) -> Dict[str, Any]:
    """Build a /v1/chat/completions request body (OpenAI-compatible)."""
    return {
        "model": "local-model",
        "messages": [
            {"role": "user", "content": prompt_text},
        ],
        "max_tokens": 256,
        "temperature": 0.0,
    }


def _build_completion_payload(prompt_text: str) -> Dict[str, Any]:
    """Build a legacy /completion request body."""
    return {
        "prompt": prompt_text,
        "n_predict": 256,
        "temperature": 0.0,
    }


def _extract_text_chat(resp_json: Dict[str, Any]) -> Optional[str]:
    """Extract generated text from a /v1/chat/completions response."""
    try:
        return resp_json["choices"][0]["message"]["content"]
    except (KeyError, IndexError, TypeError):
        return None


def _extract_text_completion(resp_json: Dict[str, Any]) -> Optional[str]:
    """Extract generated text from a /completion response."""
    try:
        return resp_json.get("content", "")
    except (KeyError, TypeError):
        return None


def call_llama_server(
    server_url: str,
    prompt_text: str,
    timeout_s: int = DEFAULT_TIMEOUT,
) -> Dict[str, Any]:
    """Send a single prompt to a llama-server and return structured result.

    Tries /v1/chat/completions first; falls back to /completion.

    Returns a dict with keys:
      start_time_iso, end_time_iso, latency_s, output_chars, output_text,
      success, status_code, error_message
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

    def _finish(success: bool, status_code: Optional[int], error: str, text: str) -> None:
        end_dt = datetime.now(timezone.utc)
        result["end_time_iso"] = end_dt.isoformat()
        result["latency_s"] = round(time.monotonic() - start_ts, 4)
        result["success"] = success
        result["status_code"] = status_code
        result["error_message"] = error
        result["output_text"] = text
        result["output_chars"] = len(text)

    # --- Attempt 1: /v1/chat/completions ---
    try:
        payload = _build_chat_payload(prompt_text)
        resp = requests.post(
            chat_url,
            json=payload,
            timeout=timeout_s,
            headers={"Content-Type": "application/json"},
        )
        if resp.status_code == 200:
            text = _extract_text_chat(resp.json()) or ""
            _finish(True, resp.status_code, "", text)
            return result
        # Non-200: record but fall through to legacy endpoint
        chat_status = resp.status_code
        chat_err = f"/v1/chat/completions returned {resp.status_code}: {resp.text[:200]}"
    except requests.exceptions.Timeout:
        _finish(False, None, f"Timeout ({timeout_s}s) on /v1/chat/completions", "")
        return result
    except requests.exceptions.ConnectionError as exc:
        _finish(False, None, f"Connection refused on {server_url}: {exc}", "")
        return result
    except Exception as exc:
        chat_status = None
        chat_err = f"Unexpected error on /v1/chat/completions: {exc}"

    # --- Attempt 2: /completion (legacy) ---
    try:
        payload = _build_completion_payload(prompt_text)
        resp = requests.post(
            completion_url,
            json=payload,
            timeout=timeout_s,
            headers={"Content-Type": "application/json"},
        )
        if resp.status_code == 200:
            text = _extract_text_completion(resp.json()) or ""
            _finish(True, resp.status_code, "", text)
            return result
        _finish(False, resp.status_code, f"Both endpoints failed. Chat: {chat_err}; Completion: {resp.status_code}: {resp.text[:200]}", "")
    except requests.exceptions.Timeout:
        _finish(False, None, f"Timeout ({timeout_s}s) on /completion", "")
    except requests.exceptions.ConnectionError as exc:
        _finish(False, None, f"Connection refused on {server_url}: {exc}", "")
    except Exception as exc:
        _finish(False, None, f"Both endpoints failed. Chat: {chat_err}; Completion: {exc}", "")

    return result


# ===================================================================
# Strategy 1: Serial (no Ray)
# ===================================================================


def run_serial(
    prompts: List[Dict[str, str]],
    server_urls: List[str],
    timeout_s: int,
) -> List[Dict[str, Any]]:
    """Send every prompt to the first server, one after another."""
    server = server_urls[0]
    print(f"[serial] Using server: {server}")
    rows: List[Dict[str, Any]] = []
    t0 = time.monotonic()

    for idx, item in enumerate(prompts):
        req_id = f"serial-{idx:04d}"
        print(f"[serial] {req_id} → {item['id']} ({item['category']})", end=" ", flush=True)
        infer = call_llama_server(server, item["prompt"], timeout_s=timeout_s)
        row = {
            "request_id": req_id,
            "prompt_id": item["id"],
            "category": item["category"],
            "strategy": "serial",
            "assigned_worker": "worker_0",
            "server_url": server,
            **infer,
        }
        rows.append(row)
        status = "OK" if infer["success"] else f"FAIL({infer['error_message'][:60]})"
        print(f"→ {infer['latency_s']:.2f}s {status}")

    elapsed = time.monotonic() - t0
    print(f"[serial] Done. {len(rows)} requests in {elapsed:.1f}s")
    return rows


# ===================================================================
# Strategy 2 & 3: Ray-based strategies (Actor + Task)
# ===================================================================

if _RAY_AVAILABLE:

    @ray.remote
    class LlamaServerActor:
        """Ray Actor that wraps one llama.cpp server endpoint.

        Each actor instance is pinned to one server URL.
        Multiple actors may live on the same physical machine if
        the Ray cluster only has a single node — resource contention
        is expected in that case.
        """

        def __init__(self, worker_id: str, server_url: str, timeout_s: int = DEFAULT_TIMEOUT):
            self.worker_id = worker_id
            self.server_url = server_url
            self.timeout_s = timeout_s
            self._request_count = 0
            print(f"[Actor:{worker_id}] Initialised, server={server_url}")

        def infer(self, prompt_item: Dict[str, Any]) -> Dict[str, Any]:
            """Run inference on this actor's server and return a result row."""
            self._request_count += 1
            req_id = f"{self.worker_id}-{self._request_count:04d}"
            prompt_text = prompt_item.get("prompt", "")
            infer_result = call_llama_server(
                self.server_url, prompt_text, timeout_s=self.timeout_s
            )
            row = {
                "request_id": req_id,
                "prompt_id": prompt_item.get("id", "?"),
                "category": prompt_item.get("category", "unknown"),
                "strategy": prompt_item.get("_strategy", "ray"),
                "assigned_worker": self.worker_id,
                "server_url": self.server_url,
                **infer_result,
            }
            status = "OK" if infer_result["success"] else f"FAIL"
            print(
                f"[Actor:{self.worker_id}] {req_id} → {prompt_item.get('id')} "
                f"({infer_result['latency_s']:.2f}s {status})"
            )
            return row

        def get_worker_id(self) -> str:
            return self.worker_id

        def get_request_count(self) -> int:
            return self._request_count


def _start_ray_local() -> None:
    """Initialise Ray in local mode (single-node)."""
    if not _RAY_AVAILABLE:
        print(
            "ERROR: Ray is not installed. Install it with:\n"
            "  pip install ray\n"
            "or use the --strategy serial option which does not require Ray.",
            file=sys.stderr,
        )
        sys.exit(1)
    if not ray.is_initialized():
        ray.init(ignore_reinit_error=True)
        print("[Ray] Initialised (local mode)")
    else:
        print("[Ray] Already initialised")


def _shutdown_ray() -> None:
    if _RAY_AVAILABLE and ray.is_initialized():
        ray.shutdown()
        print("[Ray] Shut down")


def run_ray_round_robin(
    prompts: List[Dict[str, str]],
    server_urls: List[str],
    timeout_s: int,
) -> List[Dict[str, Any]]:
    """Create one Ray Actor per server; assign prompts round-robin."""
    _start_ray_local()

    # Create actors
    actors = []
    for i, url in enumerate(server_urls):
        worker_id = f"worker_{i}"
        # Use a placement group or node affinity hint?  For a local cluster
        # all actors land on the same node anyway, so we skip that.
        actor = LlamaServerActor.options(name=f"llama_{worker_id}").remote(  # type: ignore[attr-defined]
            worker_id=worker_id,
            server_url=url,
            timeout_s=timeout_s,
        )
        actors.append(actor)

    print(f"[round_robin] {len(actors)} actor(s) created for {len(server_urls)} server(s)")

    # Submit round-robin
    futures = []
    for idx, item in enumerate(prompts):
        annotated = {**item, "_strategy": "ray_round_robin"}
        actor = actors[idx % len(actors)]
        fut = actor.infer.remote(annotated)
        futures.append(fut)

    # Collect results in submission order
    rows: List[Dict[str, Any]] = []
    t0 = time.monotonic()
    for idx, fut in enumerate(futures):
        try:
            row = ray.get(fut, timeout=timeout_s + 30)
        except ray.exceptions.GetTimeoutError:
            row = {
                "request_id": f"rr-timeout-{idx:04d}",
                "prompt_id": prompts[idx]["id"],
                "category": prompts[idx]["category"],
                "strategy": "ray_round_robin",
                "assigned_worker": "?",
                "server_url": "?",
                "start_time_iso": "",
                "end_time_iso": "",
                "latency_s": -1,
                "output_chars": 0,
                "success": False,
                "status_code": None,
                "error_message": "ray.get() timeout",
            }
        except Exception as exc:
            row = {
                "request_id": f"rr-error-{idx:04d}",
                "prompt_id": prompts[idx]["id"],
                "category": prompts[idx]["category"],
                "strategy": "ray_round_robin",
                "assigned_worker": "?",
                "server_url": "?",
                "start_time_iso": "",
                "end_time_iso": "",
                "latency_s": -1,
                "output_chars": 0,
                "success": False,
                "status_code": None,
                "error_message": str(exc)[:500],
            }
        rows.append(row)

    elapsed = time.monotonic() - t0
    print(f"[round_robin] Done. {len(rows)} requests collected in {elapsed:.1f}s")
    return rows


def run_ray_parallel(
    prompts: List[Dict[str, str]],
    server_urls: List[str],
    timeout_s: int,
    max_concurrency: int = DEFAULT_MAX_CONCURRENCY,
) -> List[Dict[str, Any]]:
    """Submit all prompts as Ray remote Tasks concurrently.

    Uses a semaphore-like pattern to bound concurrency so we don't
    overwhelm the servers.
    """
    _start_ray_local()

    @ray.remote
    def infer_task(
        prompt_item: Dict[str, Any],
        server_url: str,
        worker_id: str,
        request_id: str,
        timeout_s: int,
    ) -> Dict[str, Any]:
        """Stateless Ray task: send one prompt to one server."""
        infer_result = call_llama_server(server_url, prompt_item["prompt"], timeout_s=timeout_s)
        row = {
            "request_id": request_id,
            "prompt_id": prompt_item.get("id", "?"),
            "category": prompt_item.get("category", "unknown"),
            "strategy": prompt_item.get("_strategy", "ray_parallel"),
            "assigned_worker": worker_id,
            "server_url": server_url,
            **infer_result,
        }
        status = "OK" if infer_result["success"] else "FAIL"
        print(
            f"[Task:{worker_id}] {request_id} → {prompt_item.get('id')} "
            f"({infer_result['latency_s']:.2f}s {status})"
        )
        return row

    # Build task list
    tasks = []
    for idx, item in enumerate(prompts):
        worker_idx = idx % len(server_urls)
        worker_id = f"worker_{worker_idx}"
        server_url = server_urls[worker_idx]
        req_id = f"par-{idx:04d}"
        annotated = {**item, "_strategy": "ray_parallel"}
        tasks.append((annotated, server_url, worker_id, req_id))

    print(
        f"[parallel] {len(tasks)} tasks across {len(server_urls)} server(s), "
        f"max_concurrency={max_concurrency}"
    )

    # Submit with bounded concurrency
    rows: List[Dict[str, Any]] = []
    t0 = time.monotonic()

    pending: List[Any] = []
    task_iter = iter(tasks)
    task_idx = 0

    while True:
        # Fill the pipeline up to max_concurrency
        while len(pending) < max_concurrency:
            try:
                t = next(task_iter)
            except StopIteration:
                break
            annotated, srv, wid, rid = t
            fut = infer_task.remote(annotated, srv, wid, rid, timeout_s)
            pending.append((task_idx, fut))
            task_idx += 1

        if not pending:
            break

        # Wait for at least one to finish
        ready_futs, _ = ray.wait([f for _, f in pending], num_returns=1, timeout=None)

        new_pending = []
        for idx_pending, fut in pending:
            if fut in ready_futs:
                try:
                    row = ray.get(fut, timeout=10)
                except Exception as exc:
                    # We don't have the original prompt item here, so build a
                    # minimal error row from idx_pending.
                    orig_item = prompts[idx_pending] if idx_pending < len(prompts) else {}
                    row = {
                        "request_id": f"par-err-{idx_pending:04d}",
                        "prompt_id": orig_item.get("id", "?"),
                        "category": orig_item.get("category", "unknown"),
                        "strategy": "ray_parallel",
                        "assigned_worker": "?",
                        "server_url": "?",
                        "start_time_iso": "",
                        "end_time_iso": "",
                        "latency_s": -1,
                        "output_chars": 0,
                        "success": False,
                        "status_code": None,
                        "error_message": str(exc)[:500],
                    }
                rows.append(row)
            else:
                new_pending.append((idx_pending, fut))

        pending = new_pending

    elapsed = time.monotonic() - t0
    print(f"[parallel] Done. {len(rows)} requests collected in {elapsed:.1f}s")
    return rows


# ===================================================================
# CSV output
# ===================================================================


def write_csv(rows: List[Dict[str, Any]], output_path: str) -> None:
    """Write results to CSV, ensuring the output directory exists."""
    out_dir = os.path.dirname(os.path.abspath(output_path))
    os.makedirs(out_dir, exist_ok=True)
    with open(output_path, "w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=CSV_COLUMNS, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)
    print(f"[CSV] Wrote {len(rows)} rows → {output_path}")


# ===================================================================
# Summary statistics
# ===================================================================


def print_summary(rows: List[Dict[str, Any]], strategy: str) -> None:
    """Print a quick summary table to stdout."""
    latencies = [r["latency_s"] for r in rows if r.get("success") and r.get("latency_s", -1) > 0]
    failures = [r for r in rows if not r.get("success")]
    total_chars = sum(r.get("output_chars", 0) for r in rows)

    if not latencies:
        print(f"\n[Summary:{strategy}] No successful requests to report.")
        return

    avg_lat = sum(latencies) / len(latencies)
    sorted_lats = sorted(latencies)
    p95_lat = sorted_lats[int(len(sorted_lats) * 0.95)] if len(sorted_lats) >= 20 else sorted_lats[-1]
    # Total wall time is approximated from the earliest start to latest end
    starts = [r["start_time_iso"] for r in rows if r.get("start_time_iso")]
    ends = [r["end_time_iso"] for r in rows if r.get("end_time_iso")]
    total_wall = "N/A"
    if starts and ends:
        try:
            t0 = min(starts)
            t1 = max(ends)
            # ISO-format strings are sortable alphabetically
            t0_dt = datetime.fromisoformat(t0)
            t1_dt = datetime.fromisoformat(t1)
            total_wall = f"{(t1_dt - t0_dt).total_seconds():.1f}s"
        except Exception:
            pass

    print(f"\n{'='*60}")
    print(f"  Strategy:  {strategy}")
    print(f"  {'='*60}")
    print(f"  Total requests:       {len(rows)}")
    print(f"  Successful:           {len(rows) - len(failures)}")
    print(f"  Failed:               {len(failures)}")
    print(f"  Total output chars:   {total_chars}")
    print(f"  Total wall time:      {total_wall}")
    print(f"  Mean latency (s):     {avg_lat:.2f}")
    print(f"  P95 latency (s):      {p95_lat:.2f}")
    print(f"  Min / Max latency (s):{min(latencies):.2f} / {max(latencies):.2f}")
    if len(rows) > 0 and total_wall != "N/A":
        try:
            tw = float(total_wall.replace("s", ""))
            if tw > 0:
                print(f"  Throughput (req/s):   {len(rows)/tw:.2f}")
        except Exception:
            pass
    print(f"{'='*60}\n")


# ===================================================================
# CLI
# ===================================================================


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Ray Batch Inference — OSH 2026 Lab4 Role C",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--prompts",
        required=True,
        help="Path to JSONL prompts file",
    )
    parser.add_argument(
        "--server-urls",
        required=True,
        help="Comma-separated llama-server URLs, e.g. http://127.0.0.1:8080,http://127.0.0.1:8081",
    )
    parser.add_argument(
        "--strategy",
        required=True,
        choices=["serial", "ray_round_robin", "ray_parallel"],
        help="Scheduling strategy",
    )
    parser.add_argument(
        "--output",
        required=True,
        help="Output CSV path",
    )
    parser.add_argument(
        "--timeout",
        type=int,
        default=DEFAULT_TIMEOUT,
        help=f"Request timeout in seconds (default: {DEFAULT_TIMEOUT})",
    )
    parser.add_argument(
        "--max-concurrency",
        type=int,
        default=DEFAULT_MAX_CONCURRENCY,
        help=f"Max concurrent requests for ray_parallel (default: {DEFAULT_MAX_CONCURRENCY})",
    )
    return parser.parse_args(argv)


def main(argv: Optional[List[str]] = None) -> int:
    args = parse_args(argv)

    # Parse server URLs
    server_urls = [u.strip() for u in args.server_urls.split(",") if u.strip()]
    if not server_urls:
        print("ERROR: At least one --server-urls is required.", file=sys.stderr)
        return 1

    print(f"Server URLs: {server_urls}")
    print(f"Strategy:    {args.strategy}")
    print(f"Timeout:     {args.timeout}s")
    if args.strategy == "ray_parallel":
        print(f"Max concur:  {args.max_concurrency}")

    # Load prompts
    prompts = load_prompts(args.prompts)
    if not prompts:
        print("ERROR: No prompts loaded.", file=sys.stderr)
        return 1
    print(f"Prompts:     {len(prompts)} loaded from {args.prompts}")

    # Run strategy
    strategy = args.strategy
    try:
        if strategy == "serial":
            rows = run_serial(prompts, server_urls, args.timeout)
        elif strategy == "ray_round_robin":
            rows = run_ray_round_robin(prompts, server_urls, args.timeout)
        elif strategy == "ray_parallel":
            rows = run_ray_parallel(
                prompts, server_urls, args.timeout, max_concurrency=args.max_concurrency
            )
        else:
            print(f"ERROR: Unknown strategy '{strategy}'", file=sys.stderr)
            return 1
    except KeyboardInterrupt:
        print("\n[Interrupted] Shutting down...", file=sys.stderr)
        _shutdown_ray()
        return 130
    finally:
        if strategy != "serial":
            _shutdown_ray()

    # Write CSV
    write_csv(rows, args.output)

    # Print summary
    print_summary(rows, strategy)

    return 0


if __name__ == "__main__":
    sys.exit(main())
