#!/usr/bin/env python3
"""
Ray Load-Balancing Inference — OSH 2026 Lab4 Role C (Bonus)
=============================================================
Distributes prompts across multiple llama.cpp servers using two
load-balancing strategies built on Ray Actors.

Strategies:
  - round_robin:    Static round-robin assignment across workers.
  - latency_aware:  Dynamically assigns each request to the worker with
                    the lowest historical average latency (greedy online).

Outputs:
  - Detail CSV: one row per request.
  - Summary CSV: one row per worker, with aggregated statistics.

Usage:
  python3 ray_load_balance.py \
    --prompts Lab4/prompts/ray_prompts_30.jsonl \
    --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
    --strategy round_robin \
    --output Lab4/results/ray_load_balance_round_robin.csv \
    --summary-output Lab4/results/ray_load_balance_round_robin_summary.csv
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import os
import sys
import time
from datetime import datetime, timezone
from typing import Any, Dict, List, Optional, Tuple

import requests

# ---------------------------------------------------------------------------
# Ray availability check
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
DEFAULT_TIMEOUT = 120  # seconds per request
DEFAULT_MAX_RETRIES = 2

# Detail CSV columns
DETAIL_COLUMNS = [
    "request_id",
    "prompt_id",
    "strategy",
    "assigned_worker",
    "server_url",
    "start_time_iso",
    "end_time_iso",
    "latency_s",
    "output_chars",
    "success",
    "error_message",
]

# Summary CSV columns
SUMMARY_COLUMNS = [
    "strategy",
    "worker_id",
    "server_url",
    "request_count",
    "success_count",
    "failure_count",
    "avg_latency_s",
    "p95_latency_s",
    "total_latency_s",
    "throughput_req_per_s",
]


# ===================================================================
# Prompt loading
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
    return {
        "model": "local-model",
        "messages": [{"role": "user", "content": prompt_text}],
        "max_tokens": 256,
        "temperature": 0.0,
    }


def _build_completion_payload(prompt_text: str) -> Dict[str, Any]:
    return {
        "prompt": prompt_text,
        "n_predict": 256,
        "temperature": 0.0,
    }


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
    """Send a single prompt to a llama-server.

    Tries /v1/chat/completions first; falls back to /completion.

    Returns dict with keys: start_time_iso, end_time_iso, latency_s,
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

    def _finish(success: bool, status_code: Optional[int], error: str, text: str) -> None:
        end_dt = datetime.now(timezone.utc)
        result["end_time_iso"] = end_dt.isoformat()
        result["latency_s"] = round(time.monotonic() - start_ts, 4)
        result["success"] = success
        result["status_code"] = status_code
        result["error_message"] = error
        result["output_text"] = text
        result["output_chars"] = len(text)

    # Attempt 1: /v1/chat/completions
    try:
        payload = _build_chat_payload(prompt_text)
        resp = requests.post(
            chat_url, json=payload, timeout=timeout_s,
            headers={"Content-Type": "application/json"},
        )
        if resp.status_code == 200:
            text = _extract_text_chat(resp.json()) or ""
            _finish(True, resp.status_code, "", text)
            return result
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

    # Attempt 2: /completion (legacy)
    try:
        payload = _build_completion_payload(prompt_text)
        resp = requests.post(
            completion_url, json=payload, timeout=timeout_s,
            headers={"Content-Type": "application/json"},
        )
        if resp.status_code == 200:
            text = _extract_text_completion(resp.json()) or ""
            _finish(True, resp.status_code, "", text)
            return result
        _finish(False, resp.status_code,
                f"Both endpoints failed. Chat: {chat_err}; Completion: {resp.status_code}: {resp.text[:200]}", "")
    except requests.exceptions.Timeout:
        _finish(False, None, f"Timeout ({timeout_s}s) on /completion", "")
    except requests.exceptions.ConnectionError as exc:
        _finish(False, None, f"Connection refused on {server_url}: {exc}", "")
    except Exception as exc:
        _finish(False, None, f"Both endpoints failed. Chat: {chat_err}; Completion: {exc}", "")

    return result


# ===================================================================
# Worker latency tracker (used by latency_aware scheduler)
# ===================================================================


class WorkerStats:
    """Tracks cumulative latency statistics for a single worker."""

    def __init__(self, worker_id: str, server_url: str):
        self.worker_id = worker_id
        self.server_url = server_url
        self._latencies: List[float] = []
        self._sum_latency = 0.0

    def record(self, latency_s: float, success: bool) -> None:
        """Record a completed request's latency."""
        if success and latency_s > 0:
            self._latencies.append(latency_s)
            self._sum_latency += latency_s

    @property
    def avg_latency(self) -> float:
        """Average latency of successful requests; INF if no data yet."""
        if not self._latencies:
            return float("inf")
        return self._sum_latency / len(self._latencies)

    @property
    def request_count(self) -> int:
        return len(self._latencies)

    @property
    def p95_latency(self) -> float:
        if not self._latencies:
            return float("inf")
        sorted_lats = sorted(self._latencies)
        idx = int(len(sorted_lats) * 0.95)
        # clamp to valid range
        idx = min(idx, len(sorted_lats) - 1)
        return sorted_lats[idx]

    def snapshot(self, strategy: str, total_wall_s: float) -> Dict[str, Any]:
        """Build a summary row for CSV output."""
        n = self.request_count
        if n > 0:
            avg = self.avg_latency
            p95 = self.p95_latency
            total_lat = self._sum_latency
            throughput = n / total_wall_s if total_wall_s > 0 else 0.0
        else:
            avg = -1.0
            p95 = -1.0
            total_lat = 0.0
            throughput = 0.0
        return {
            "strategy": strategy,
            "worker_id": self.worker_id,
            "server_url": self.server_url,
            "request_count": n,
            "success_count": n,
            "failure_count": 0,
            "avg_latency_s": round(avg, 4),
            "p95_latency_s": round(p95, 4),
            "total_latency_s": round(total_lat, 4),
            "throughput_req_per_s": round(throughput, 4),
        }


# ===================================================================
# Ray Actor: wraps one llama-server
# ===================================================================


def _ensure_ray() -> None:
    if not _RAY_AVAILABLE:
        print(
            "ERROR: Ray is not installed. Install it with:\n"
            "  pip install ray\n"
            "or use the --strategy serial option (see ray_batch_infer.py).",
            file=sys.stderr,
        )
        sys.exit(1)
    if not ray.is_initialized():
        # Monkey-patch Ray's IP detection to force 127.0.0.1.
        # See ray_batch_infer.py for detailed explanation.
        import ray.util as _ray_util
        _ray_util.get_node_ip_address = lambda address=None: "127.0.0.1"

        ray.init(address="local", ignore_reinit_error=True)
        print("[Ray] Initialised (local mode)")
    else:
        print("[Ray] Already initialised")


def _shutdown_ray() -> None:
    if _RAY_AVAILABLE and ray.is_initialized():
        ray.shutdown()
        print("[Ray] Shut down")


if _RAY_AVAILABLE:

    @ray.remote
    class LlamaServerActor:
        """Ray Actor that wraps one llama.cpp server endpoint."""

        def __init__(self, worker_id: str, server_url: str, timeout_s: int = DEFAULT_TIMEOUT):
            self.worker_id = worker_id
            self.server_url = server_url
            self.timeout_s = timeout_s
            self._req_count = 0
            print(f"[Actor:{worker_id}] Initialised, server={server_url}")

        def infer(self, prompt_item: Dict[str, Any], strategy: str) -> Dict[str, Any]:
            """Run inference on this actor's server."""
            self._req_count += 1
            req_id = f"{self.worker_id}-{self._req_count:04d}"
            infer_result = call_llama_server(
                self.server_url, prompt_item.get("prompt", ""),
                timeout_s=self.timeout_s,
            )
            row = {
                "request_id": req_id,
                "prompt_id": prompt_item.get("id", "?"),
                "strategy": strategy,
                "assigned_worker": self.worker_id,
                "server_url": self.server_url,
                **{k: infer_result[k] for k in
                   ["start_time_iso", "end_time_iso", "latency_s",
                    "output_chars", "success", "error_message"]},
            }
            status = "OK" if infer_result["success"] else "FAIL"
            print(
                f"[Actor:{self.worker_id}] {req_id} → {prompt_item.get('id')} "
                f"({infer_result['latency_s']:.2f}s {status})"
            )
            return row

        def get_worker_id(self) -> str:
            return self.worker_id


# ===================================================================
# Strategy: round_robin
# ===================================================================


def run_round_robin(
    prompts: List[Dict[str, str]],
    server_urls: List[str],
    timeout_s: int,
    max_retries: int,
) -> Tuple[List[Dict[str, Any]], List[Dict[str, Any]], float]:
    """Round-robin: assign prompt[i] to server_urls[i % N].

    Uses Ray Actors, but submits and waits sequentially within each actor
    (actors process one request at a time naturally).
    """
    _ensure_ray()

    # Create actors
    actors = []
    for i, url in enumerate(server_urls):
        worker_id = f"worker_{i}"
        actor = LlamaServerActor.options(name=f"lb_rr_{worker_id}").remote(  # type: ignore[attr-defined]
            worker_id=worker_id, server_url=url, timeout_s=timeout_s,
        )
        actors.append(actor)

    print(f"[round_robin] {len(actors)} actor(s) for {len(server_urls)} server(s)")

    # Submit: round-robin assignment
    futures: List[Tuple[int, Any]] = []  # (prompt_index, future)
    for idx, item in enumerate(prompts):
        actor = actors[idx % len(actors)]
        fut = actor.infer.remote(item, "round_robin")
        futures.append((idx, fut))

    # Collect results
    rows: List[Dict[str, Any]] = []
    t0 = time.monotonic()
    for idx, fut in futures:
        for attempt in range(1 + max_retries):
            try:
                row = ray.get(fut, timeout=timeout_s + 30)
                rows.append(row)
                break
            except ray.exceptions.GetTimeoutError:
                if attempt < max_retries:
                    print(f"[round_robin] Retry {attempt+1} for prompt {prompts[idx]['id']}")
                    actor = actors[idx % len(actors)]
                    fut = actor.infer.remote(prompts[idx], "round_robin")
                else:
                    rows.append(_error_row(prompts[idx], "round_robin", "ray.get() timeout after retries"))
            except Exception as exc:
                rows.append(_error_row(prompts[idx], "round_robin", str(exc)[:500]))
                break

    elapsed = time.monotonic() - t0
    print(f"[round_robin] Done. {len(rows)} requests in {elapsed:.1f}s")
    summaries = _build_summaries(rows, server_urls, "round_robin", elapsed)
    return rows, summaries, elapsed


# ===================================================================
# Strategy: latency_aware
# ===================================================================


def run_latency_aware(
    prompts: List[Dict[str, str]],
    server_urls: List[str],
    timeout_s: int,
    max_retries: int,
) -> Tuple[List[Dict[str, Any]], List[Dict[str, Any]], float]:
    """Latency-aware scheduling.

    Phase 1 (warmup): Send one request to each worker (round-robin) to
    collect initial latency data.
    Phase 2: For each remaining prompt, pick the worker with the lowest
    historical average latency.  Update stats after each response.

    Because Ray Actors process requests sequentially, we submit one
    request at a time — wait for it to complete, update stats, then
    pick the best worker for the next request.
    """
    _ensure_ray()

    # Create actors
    actors: Dict[str, Any] = {}
    for i, url in enumerate(server_urls):
        worker_id = f"worker_{i}"
        actor = LlamaServerActor.options(name=f"lb_la_{worker_id}").remote(  # type: ignore[attr-defined]
            worker_id=worker_id, server_url=url, timeout_s=timeout_s,
        )
        actors[worker_id] = actor

    print(f"[latency_aware] {len(actors)} actor(s) for {len(server_urls)} server(s)")

    # Per-worker stats tracker
    stats: Dict[str, WorkerStats] = {
        wid: WorkerStats(wid, url) for wid, url in
        zip([f"worker_{i}" for i in range(len(server_urls))], server_urls)
    }

    N = len(server_urls)
    rows: List[Dict[str, Any]] = []
    t0 = time.monotonic()

    # ---- Phase 1: Warmup (one request per worker) ----
    warmup_count = min(N, len(prompts))
    print(f"[latency_aware] Phase 1: warmup — {warmup_count} request(s)")

    for i in range(warmup_count):
        worker_id = f"worker_{i}"
        actor = actors[worker_id]
        item = prompts[i]
        row = _submit_and_wait(actor, item, "latency_aware", timeout_s, max_retries)
        rows.append(row)
        # Record latency
        if row.get("success"):
            stats[worker_id].record(row["latency_s"], True)
            print(f"[latency_aware] warmup {worker_id}: latency={row['latency_s']:.2f}s, "
                  f"avg={stats[worker_id].avg_latency:.2f}s")
        else:
            # Failed request: record a pessimistic latency so this worker
            # is deprioritised but not excluded entirely.
            stats[worker_id].record(timeout_s, True)
            print(f"[latency_aware] warmup {worker_id}: FAILED, penalised with {timeout_s}s")

    # ---- Phase 2: Greedy latency-aware ----
    for i in range(warmup_count, len(prompts)):
        # Pick worker with lowest average latency
        best_worker = min(stats.keys(), key=lambda wid: stats[wid].avg_latency)
        best_actor = actors[best_worker]

        item = prompts[i]
        print(f"[latency_aware] Prompt {item['id']} → {best_worker} "
              f"(avg={stats[best_worker].avg_latency:.2f}s)")

        row = _submit_and_wait(best_actor, item, "latency_aware", timeout_s, max_retries)
        rows.append(row)

        if row.get("success"):
            stats[best_worker].record(row["latency_s"], True)
        else:
            stats[best_worker].record(timeout_s, True)  # penalise

    elapsed = time.monotonic() - t0
    print(f"[latency_aware] Done. {len(rows)} requests in {elapsed:.1f}s")

    # Print final stats
    print(f"\n[latency_aware] Final per-worker stats:")
    for wid in sorted(stats.keys()):
        s = stats[wid]
        print(f"  {wid}: count={s.request_count}, avg_lat={s.avg_latency:.2f}s, "
              f"p95_lat={s.p95_latency:.2f}s")

    summaries = _build_summaries(rows, server_urls, "latency_aware", elapsed)
    return rows, summaries, elapsed


# ===================================================================
# Helpers
# ===================================================================


def _submit_and_wait(
    actor: Any,
    prompt_item: Dict[str, str],
    strategy: str,
    timeout_s: int,
    max_retries: int,
) -> Dict[str, Any]:
    """Submit a single request to an actor and wait for the result, with retries."""
    for attempt in range(1 + max_retries):
        try:
            fut = actor.infer.remote(prompt_item, strategy)
            return ray.get(fut, timeout=timeout_s + 30)
        except ray.exceptions.GetTimeoutError:
            if attempt < max_retries:
                print(f"  [retry {attempt+1}] {prompt_item.get('id')}")
                continue
            return _error_row(prompt_item, strategy, "ray.get() timeout after retries")
        except Exception as exc:
            return _error_row(prompt_item, strategy, str(exc)[:500])
    return _error_row(prompt_item, strategy, "unreachable")


def _error_row(item: Dict[str, str], strategy: str, error: str) -> Dict[str, Any]:
    return {
        "request_id": f"err-{item.get('id', '?')}",
        "prompt_id": item.get("id", "?"),
        "strategy": strategy,
        "assigned_worker": "?",
        "server_url": "?",
        "start_time_iso": "",
        "end_time_iso": "",
        "latency_s": -1.0,
        "output_chars": 0,
        "success": False,
        "error_message": error,
    }


def _build_summaries(
    rows: List[Dict[str, Any]],
    server_urls: List[str],
    strategy: str,
    total_wall_s: float,
) -> List[Dict[str, Any]]:
    """Build per-worker and overall summary rows."""
    # Group rows by worker
    workers: Dict[str, Dict[str, Any]] = {}
    for row in rows:
        wid = row.get("assigned_worker", "?")
        if wid not in workers:
            workers[wid] = {
                "strategy": strategy,
                "worker_id": wid,
                "server_url": row.get("server_url", "?"),
                "request_count": 0,
                "success_count": 0,
                "failure_count": 0,
                "latencies": [],
            }
        w = workers[wid]
        w["request_count"] += 1
        if row.get("success"):
            w["success_count"] += 1
            lat = row.get("latency_s", 0)
            if lat and lat > 0:
                w["latencies"].append(lat)
        else:
            w["failure_count"] += 1

    summaries: List[Dict[str, Any]] = []
    for wid in sorted(workers.keys()):
        w = workers[wid]
        lats = w["latencies"]
        n = w["request_count"]
        n_succ = w["success_count"]
        if lats:
            avg_lat = sum(lats) / len(lats)
            sorted_lats = sorted(lats)
            p95_idx = min(int(len(sorted_lats) * 0.95), len(sorted_lats) - 1)
            p95_lat = sorted_lats[p95_idx]
            total_lat = sum(lats)
        else:
            avg_lat = -1.0
            p95_lat = -1.0
            total_lat = 0.0
        throughput = n / total_wall_s if total_wall_s > 0 else 0.0
        summaries.append({
            "strategy": strategy,
            "worker_id": wid,
            "server_url": w["server_url"],
            "request_count": n,
            "success_count": n_succ,
            "failure_count": w["failure_count"],
            "avg_latency_s": round(avg_lat, 4),
            "p95_latency_s": round(p95_lat, 4),
            "total_latency_s": round(total_lat, 4),
            "throughput_req_per_s": round(throughput, 4),
        })

    # Add an "overall" row
    all_lats = []
    for w in workers.values():
        all_lats.extend(w["latencies"])
    total_n = sum(w["request_count"] for w in workers.values())
    total_succ = sum(w["success_count"] for w in workers.values())
    total_fail = sum(w["failure_count"] for w in workers.values())
    if all_lats:
        avg_all = sum(all_lats) / len(all_lats)
        sorted_all = sorted(all_lats)
        p95_all = sorted_all[min(int(len(sorted_all) * 0.95), len(sorted_all) - 1)]
        total_lat_all = sum(all_lats)
    else:
        avg_all = -1.0
        p95_all = -1.0
        total_lat_all = 0.0
    throughput_all = total_n / total_wall_s if total_wall_s > 0 else 0.0

    summaries.append({
        "strategy": strategy,
        "worker_id": "overall",
        "server_url": "all",
        "request_count": total_n,
        "success_count": total_succ,
        "failure_count": total_fail,
        "avg_latency_s": round(avg_all, 4),
        "p95_latency_s": round(p95_all, 4),
        "total_latency_s": round(total_lat_all, 4),
        "throughput_req_per_s": round(throughput_all, 4),
    })

    return summaries


# ===================================================================
# CSV output
# ===================================================================


def write_detail_csv(rows: List[Dict[str, Any]], path: str) -> None:
    out_dir = os.path.dirname(os.path.abspath(path))
    os.makedirs(out_dir, exist_ok=True)
    with open(path, "w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=DETAIL_COLUMNS, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(rows)
    print(f"[CSV] Detail: {len(rows)} rows → {path}")


def write_summary_csv(summaries: List[Dict[str, Any]], path: str) -> None:
    out_dir = os.path.dirname(os.path.abspath(path))
    os.makedirs(out_dir, exist_ok=True)
    with open(path, "w", newline="", encoding="utf-8") as fh:
        writer = csv.DictWriter(fh, fieldnames=SUMMARY_COLUMNS, extrasaction="ignore")
        writer.writeheader()
        writer.writerows(summaries)
    print(f"[CSV] Summary: {len(summaries)} rows → {path}")


# ===================================================================
# Print summary table
# ===================================================================


def print_summary_table(summaries: List[Dict[str, Any]], strategy: str, total_wall_s: float) -> None:
    print(f"\n{'='*80}")
    print(f"  Strategy: {strategy}  |  Total wall time: {total_wall_s:.1f}s")
    print(f"{'='*80}")
    header = f"{'Worker':<14} {'Reqs':>5} {'Succ':>5} {'Fail':>5} {'Avg(s)':>8} {'P95(s)':>8} {'Thru(req/s)':>12}"
    print(header)
    print("-" * len(header))
    for s in summaries:
        if s["worker_id"] == "overall":
            print("-" * len(header))
        print(
            f"{s['worker_id']:<14} {s['request_count']:>5} {s['success_count']:>5} "
            f"{s['failure_count']:>5} {s['avg_latency_s']:>8.2f} {s['p95_latency_s']:>8.2f} "
            f"{s['throughput_req_per_s']:>12.4f}"
        )
    print(f"{'='*80}\n")


# ===================================================================
# CLI
# ===================================================================


def parse_args(argv: Optional[List[str]] = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Ray Load-Balancing Inference — OSH 2026 Lab4 Role C (Bonus)",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument("--prompts", required=True,
                        help="Path to JSONL prompts file (>=30 prompts)")
    parser.add_argument("--server-urls", required=True,
                        help="Comma-separated llama-server URLs")
    parser.add_argument("--strategy", required=True,
                        choices=["round_robin", "latency_aware"],
                        help="Load-balancing strategy")
    parser.add_argument("--output", required=True,
                        help="Detail CSV output path")
    parser.add_argument("--summary-output", required=True,
                        help="Summary CSV output path")
    parser.add_argument("--timeout", type=int, default=DEFAULT_TIMEOUT,
                        help=f"Request timeout in seconds (default: {DEFAULT_TIMEOUT})")
    parser.add_argument("--max-retries", type=int, default=DEFAULT_MAX_RETRIES,
                        help=f"Max retries per request (default: {DEFAULT_MAX_RETRIES})")
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
    print(f"Max retries: {args.max_retries}")

    # Load prompts
    prompts = load_prompts(args.prompts)
    if len(prompts) < 2:
        print("ERROR: Need at least 2 prompts.", file=sys.stderr)
        return 1
    print(f"Prompts:     {len(prompts)} loaded from {args.prompts}")

    # Validate: latency_aware needs at least as many prompts as servers for warmup
    if args.strategy == "latency_aware" and len(prompts) < len(server_urls):
        print(
            f"WARNING: latency_aware needs at least {len(server_urls)} prompts "
            f"for warmup, but only {len(prompts)} loaded.",
            file=sys.stderr,
        )
        # Not a fatal error — will just do as many warmup requests as possible.

    # Run
    strategy = args.strategy
    try:
        if strategy == "round_robin":
            rows, summaries, elapsed = run_round_robin(
                prompts, server_urls, args.timeout, args.max_retries,
            )
        elif strategy == "latency_aware":
            rows, summaries, elapsed = run_latency_aware(
                prompts, server_urls, args.timeout, args.max_retries,
            )
        else:
            print(f"ERROR: Unknown strategy '{strategy}'", file=sys.stderr)
            return 1
    except KeyboardInterrupt:
        print("\n[Interrupted] Shutting down...", file=sys.stderr)
        _shutdown_ray()
        return 130
    finally:
        _shutdown_ray()

    # Write outputs
    write_detail_csv(rows, args.output)
    write_summary_csv(summaries, args.summary_output)

    # Print summary
    print_summary_table(summaries, strategy, elapsed)

    return 0


if __name__ == "__main__":
    sys.exit(main())
