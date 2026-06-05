#!/usr/bin/env python3
"""llama-server concurrency test script for OSH Lab4 role B.

Uses ThreadPoolExecutor to send concurrent requests to llama-server's
OpenAI-compatible /v1/chat/completions endpoint, measuring latency,
throughput, and failure rates at different concurrency levels.
"""

import argparse
import csv
import json
import sys
import time
import os
from concurrent.futures import ThreadPoolExecutor, as_completed
from datetime import datetime, timezone
from pathlib import Path

import requests

# Paths are resolved relative to the project root
PROJECT_ROOT = Path(__file__).resolve().parent.parent


def load_prompts(path: Path) -> list[dict]:
    """Load prompts from a JSONL file."""
    prompts = []
    with path.open("r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            item = json.loads(line)
            prompts.append(item)
    return prompts


def send_request(
    server_url: str,
    prompt_text: str,
    max_tokens: int,
    timeout: int,
) -> dict:
    """Send a single chat completion request. Returns a result dict."""
    t0 = time.perf_counter()
    result = {
        "success": False,
        "status_code": None,
        "output_chars": 0,
        "error_message": "",
        "raw_response": "",
    }

    payload = {
        "messages": [{"role": "user", "content": prompt_text}],
        "max_tokens": max_tokens,
    }

    try:
        r = requests.post(
            f"{server_url}/v1/chat/completions",
            json=payload,
            timeout=timeout,
        )
        t1 = time.perf_counter()
        result["status_code"] = r.status_code

        if r.status_code == 200:
            data = r.json()
            raw = json.dumps(data, ensure_ascii=False, indent=2)
            result["raw_response"] = raw
            content = (
                data.get("choices", [{}])[0]
                .get("message", {})
                .get("content", "")
            )
            result["output_chars"] = len(content)
            result["success"] = True
        else:
            result["raw_response"] = r.text[:2000]
            result["error_message"] = f"HTTP {r.status_code}: {r.text[:200]}"
    except requests.exceptions.Timeout:
        t1 = time.perf_counter()
        result["error_message"] = f"timeout after {timeout}s"
    except requests.exceptions.ConnectionError as e:
        t1 = time.perf_counter()
        result["error_message"] = f"connection error: {e}"
    except Exception as e:
        t1 = time.perf_counter()
        result["error_message"] = repr(e)

    result["latency_s"] = round(t1 - t0, 3)
    return result


def ensure_dirs(*paths: Path):
    """Create parent directories for each path if they don't exist."""
    for p in paths:
        p.parent.mkdir(parents=True, exist_ok=True)


def run_concurrency_level(
    level: int,
    num_requests: int,
    prompts: list[dict],
    args,
    raw_dir: Path,
) -> list[dict]:
    """Run all requests for a single concurrency level."""
    raw_dir.mkdir(parents=True, exist_ok=True)
    rows = []

    # Build list of (request_index, prompt) pairs, cycling through prompts
    tasks = []
    for i in range(num_requests):
        prompt = prompts[i % len(prompts)]
        tasks.append((i, prompt))

    level_start = time.perf_counter()

    with ThreadPoolExecutor(max_workers=level) as executor:
        future_map = {}
        for idx, prompt in tasks:
            request_id = f"c{level}_r{idx + 1:03d}"
            future = executor.submit(
                send_request,
                args.server_url,
                prompt["prompt"],
                args.n_predict,
                args.timeout,
            )
            future_map[future] = (request_id, prompt, idx)

        for future in as_completed(future_map):
            request_id, prompt, idx = future_map[future]
            start = datetime.now(timezone.utc)
            resp = future.result()

            # start_time is approximate (when we schedule); record end based on latency
            end = datetime.now(timezone.utc)

            # Save raw response
            raw_file = raw_dir / f"{request_id}.txt"
            raw_file.write_text(
                f"REQUEST_ID: {request_id}\n"
                f"PROMPT_ID: {prompt['id']}\n"
                f"CONCURRENCY: {level}\n"
                f"LATENCY_S: {resp['latency_s']}\n"
                f"SUCCESS: {resp['success']}\n"
                f"STATUS_CODE: {resp['status_code']}\n"
                f"ERROR: {resp['error_message']}\n\n"
                + resp.get("raw_response", ""),
                encoding="utf-8",
            )

            try:
                rel_path = str(raw_file.relative_to(PROJECT_ROOT))
            except ValueError:
                rel_path = str(raw_file)

            rows.append({
                "request_id": request_id,
                "concurrency": level,
                "prompt_id": prompt["id"],
                "start_time": start.isoformat(),
                "end_time": end.isoformat(),
                "latency_s": resp["latency_s"],
                "success": str(resp["success"]).lower(),
                "status_code": resp["status_code"] or "",
                "output_chars": resp["output_chars"],
                "error_message": resp["error_message"],
            })

    level_end = time.perf_counter()
    total_time = level_end - level_start

    # Print summary
    success_count = sum(1 for r in rows if r["success"] == "true")
    failure_count = len(rows) - success_count
    latencies = sorted(
        [r["latency_s"] for r in rows if r["success"] == "true"]
    )
    avg_lat = sum(latencies) / len(latencies) if latencies else 0
    p95_idx = int(len(latencies) * 0.95)
    p95_lat = latencies[p95_idx] if latencies and p95_idx < len(latencies) else (
        latencies[-1] if latencies else 0
    )
    throughput = success_count / total_time if total_time > 0 else 0

    print(f"\n---- Concurrency {level} ----")
    print(f"  Total requests:    {len(rows)}")
    print(f"  Success:           {success_count}")
    print(f"  Failure:           {failure_count}")
    print(f"  Avg latency (s):   {avg_lat:.3f}")
    print(f"  P95 latency (s):   {p95_lat:.3f}")
    print(f"  Throughput (req/s): {throughput:.3f}")
    print(f"  Wall time (s):     {total_time:.2f}")

    return rows


def main():
    default_prompts = PROJECT_ROOT / "prompts" / "quality_prompts.jsonl"
    default_output = PROJECT_ROOT / "results" / "server_concurrency.csv"
    default_raw_dir = PROJECT_ROOT / "results" / "server_concurrency_raw"

    parser = argparse.ArgumentParser(
        description="llama-server concurrency test for OSH Lab4 role B."
    )
    parser.add_argument(
        "--server-url", type=str, default="http://127.0.0.1:8080",
        help="Base URL of llama-server",
    )
    parser.add_argument(
        "--prompts", type=Path, default=default_prompts,
        help="Path to JSONL prompt file",
    )
    parser.add_argument(
        "--output", type=Path, default=default_output,
        help="Path to output CSV",
    )
    parser.add_argument(
        "--concurrency-levels", type=str, default="1,2,4",
        help="Comma-separated concurrency levels to test",
    )
    parser.add_argument(
        "--requests-per-level", type=int, default=10,
        help="Number of requests per concurrency level",
    )
    parser.add_argument(
        "--timeout", type=int, default=120,
        help="Timeout per request in seconds",
    )
    parser.add_argument(
        "--n-predict", type=int, default=128,
        help="max_tokens for each request",
    )

    args = parser.parse_args()

    # Parse concurrency levels
    try:
        concurrency_levels = [
            int(x.strip()) for x in args.concurrency_levels.split(",")
        ]
    except ValueError:
        print("ERROR: --concurrency-levels must be comma-separated integers",
              file=sys.stderr)
        return 1

    # Pre-flight checks
    if not args.prompts.is_file():
        print(f"ERROR: prompts file not found: {args.prompts}", file=sys.stderr)
        return 1

    prompts = load_prompts(args.prompts)
    if len(prompts) == 0:
        print("ERROR: no prompts loaded", file=sys.stderr)
        return 1
    print(f"Loaded {len(prompts)} prompts")

    # Quick connectivity check
    print(f"Checking connectivity to {args.server_url} ...", end=" ", flush=True)
    try:
        r = requests.get(f"{args.server_url}/health", timeout=10)
        print(f"OK (health: {r.status_code})")
    except Exception:
        # /health may not exist; try /v1/models
        try:
            r = requests.get(f"{args.server_url}/v1/models", timeout=10)
            print(f"OK (models: {r.status_code})")
        except Exception as e:
            print(f"WARNING: could not reach server: {e}")
            print("Continuing anyway — requests will fail if server is down.")

    # CSV fields
    fieldnames = [
        "request_id", "concurrency", "prompt_id",
        "start_time", "end_time", "latency_s",
        "success", "status_code", "output_chars", "error_message",
    ]

    ensure_dirs(args.output)
    raw_dir = default_raw_dir

    # Run each concurrency level
    all_rows = []
    for level in concurrency_levels:
        print(f"\n{'='*50}")
        print(f"Testing concurrency={level} ({args.requests_per_level} requests)")
        print(f"{'='*50}")
        rows = run_concurrency_level(
            level, args.requests_per_level, prompts, args, raw_dir
        )
        all_rows.extend(rows)

    # Write CSV (overwrite, not append — each run is a complete test)
    with args.output.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(all_rows)

    print(f"\n{'='*50}")
    print(f"Wrote {len(all_rows)} rows to {args.output}")
    print(f"Raw outputs saved to {raw_dir}/")

    total_ok = sum(1 for r in all_rows if r["success"] == "true")
    total_fail = len(all_rows) - total_ok
    print(f"Overall: {total_ok} success, {total_fail} failure")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
