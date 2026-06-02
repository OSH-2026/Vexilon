#!/usr/bin/env python3
import argparse
import csv
import json
import re
import shutil
import subprocess
import sys
import time
from datetime import datetime
from pathlib import Path

TOKENS_PER_SECOND_PATTERNS = [
    re.compile(r"([\d.]+)\s*tokens per second", re.IGNORECASE),
    re.compile(r"([\d.]+)\s*tok/s", re.IGNORECASE),
    re.compile(r"Generation:\s*([\d.]+)\s*t/s", re.IGNORECASE),
]

RSS_PATTERN = re.compile(r"Maximum resident set size.*?:\s*(\d+)", re.IGNORECASE)


def load_prompts(path: Path):
    prompts = []
    with path.open("r", encoding="utf-8") as f:
        for line_no, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue
            try:
                item = json.loads(line)
            except json.JSONDecodeError as e:
                raise ValueError(f"Invalid JSONL at {path}:{line_no}: {e}") from e
            if "id" not in item or "prompt" not in item:
                raise ValueError(f"Missing id or prompt at {path}:{line_no}")
            item.setdefault("category", "")
            prompts.append(item)
    return prompts


def parse_tokens_per_second(text: str):
    matches = []
    for pattern in TOKENS_PER_SECOND_PATTERNS:
        matches.extend(float(x) for x in pattern.findall(text))
    return matches[-1] if matches else ""


def parse_max_rss(text: str):
    match = RSS_PATTERN.search(text)
    return match.group(1) if match else ""


def build_command(args, prompt_file: Path):
    cmd = [
        str(args.llama_cli),
        "-m", str(args.model_path),
        "-f", str(prompt_file),
        "-n", str(args.n_predict),
        "--threads", str(args.threads),
        "--ctx-size", str(args.ctx_size),
        "--batch-size", str(args.batch_size),
        "--single-turn",
        "--simple-io",
    ]
    if args.no_mmap:
        cmd.append("--no-mmap")
    cmd.extend(args.extra_args)
    return cmd


def maybe_wrap_with_time(cmd):
    time_bin = shutil.which("time")
    if time_bin and Path(time_bin).name == "time":
        return [time_bin, "-v"] + cmd, True
    usr_time = Path("/usr/bin/time")
    if usr_time.exists():
        return [str(usr_time), "-v"] + cmd, True
    return cmd, False


def run_once(args, item, run_id, raw_dir: Path):
    raw_dir.mkdir(parents=True, exist_ok=True)
    raw_output_path = raw_dir / f"{run_id}_{item['id']}.txt"
    prompt_file = raw_dir / f"{run_id}_{item['id']}_prompt.txt"
    prompt_file.write_text(item["prompt"], encoding="utf-8")

    cmd = build_command(args, prompt_file)
    cmd, has_time = maybe_wrap_with_time(cmd)
    start_time = datetime.now().isoformat(timespec="seconds")
    start = time.perf_counter()
    success = True
    error_message = ""
    combined = ""

    try:
        proc = subprocess.run(
            cmd,
            text=True,
            encoding="utf-8",
            errors="replace",
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=args.timeout,
            check=False,
        )
        combined = proc.stdout or ""
        if proc.returncode != 0:
            success = False
            error_message = f"returncode={proc.returncode}"
    except subprocess.TimeoutExpired as e:
        success = False
        error_message = f"timeout after {args.timeout}s"
        combined = e.stdout or ""
        if isinstance(combined, bytes):
            combined = combined.decode("utf-8", errors="replace")
    except Exception as e:
        success = False
        error_message = repr(e)

    end = time.perf_counter()
    end_time = datetime.now().isoformat(timespec="seconds")

    raw_output_path.write_text(
        "COMMAND:\n"
        + " ".join(str(x) for x in cmd)
        + "\n\n"
        + ("TIME_WRAPPER: /usr/bin/time -v compatible\n\n" if has_time else "TIME_WRAPPER: NOT_AVAILABLE\n\n")
        + combined,
        encoding="utf-8",
    )

    return {
        "run_id": run_id,
        "prompt_id": item["id"],
        "category": item.get("category", ""),
        "threads": args.threads,
        "ctx_size": args.ctx_size,
        "batch_size": args.batch_size,
        "n_predict": args.n_predict,
        "no_mmap": str(bool(args.no_mmap)).lower(),
        "start_time": start_time,
        "end_time": end_time,
        "total_latency_s": f"{end - start:.6f}",
        "tokens_per_second": parse_tokens_per_second(combined),
        "max_rss_kb": parse_max_rss(combined),
        "output_chars": len(combined),
        "success": str(success).lower(),
        "error_message": error_message,
        "raw_output_path": str(raw_output_path),
    }


def main():
    parser = argparse.ArgumentParser(description="Single-machine llama.cpp benchmark for OSH Lab4 role A.")
    parser.add_argument("--model-path", required=True)
    parser.add_argument("--llama-cli", required=True)
    parser.add_argument("--prompts", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--n-predict", type=int, default=128)
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--ctx-size", type=int, default=2048)
    parser.add_argument("--batch-size", type=int, default=256)
    parser.add_argument("--repeat", type=int, default=3)
    parser.add_argument("--timeout", type=int, default=300)
    parser.add_argument("--no-mmap", action="store_true")
    parser.add_argument("--extra-args", nargs="*", default=[])
    args = parser.parse_args()

    args.model_path = Path(args.model_path)
    args.llama_cli = Path(args.llama_cli)
    prompts_path = Path(args.prompts)
    output_path = Path(args.output)

    if not args.model_path.is_file():
        print(f"ERROR: model not found: {args.model_path}", file=sys.stderr)
        return 1
    if not args.llama_cli.is_file():
        print(f"ERROR: llama-cli not found: {args.llama_cli}", file=sys.stderr)
        return 1
    if not prompts_path.is_file():
        print(f"ERROR: prompts not found: {prompts_path}", file=sys.stderr)
        return 1

    output_path.parent.mkdir(parents=True, exist_ok=True)
    raw_dir = output_path.parent / "raw_single_outputs" / output_path.stem

    prompts = load_prompts(prompts_path)
    fieldnames = [
        "run_id", "prompt_id", "category",
        "threads", "ctx_size", "batch_size", "n_predict", "no_mmap",
        "start_time", "end_time", "total_latency_s",
        "tokens_per_second", "max_rss_kb", "output_chars",
        "success", "error_message", "raw_output_path",
    ]

    rows = []
    counter = 0
    for r in range(args.repeat):
        for item in prompts:
            counter += 1
            run_id = f"run{counter:04d}"
            print(f"[{run_id}] prompt={item['id']} repeat={r+1}/{args.repeat}")
            rows.append(run_once(args, item, run_id, raw_dir))

    with output_path.open("w", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        writer.writeheader()
        writer.writerows(rows)

    print(f"Benchmark CSV written to {output_path}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
