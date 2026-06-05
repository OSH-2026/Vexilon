#!/usr/bin/env python3
"""Quality evaluation script for OSH Lab4 role B.

Runs llama-cli with multiple prompts under different sampling configurations,
collects raw outputs, and writes a CSV suitable for manual quality scoring.
"""

import argparse
import csv
import json
import subprocess
import sys
import time
import os
from datetime import datetime
from pathlib import Path

# Paths are resolved relative to the project root (where this script lives)
PROJECT_ROOT = Path(__file__).resolve().parent.parent

# MinGW bin directory — needed so Windows can find libstdc++-6.dll etc.
_MINGW_BIN = Path("C:/Program Files/MinGW/mingw64/bin")
_MINGW_BIN_STR = str(_MINGW_BIN.resolve())


def load_prompts(path: Path):
    """Load prompts from a JSONL file. Returns list of dicts with id, category, prompt."""
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


def ensure_dirs(*paths: Path):
    """Create parent directories for each path if they don't exist."""
    for p in paths:
        p.parent.mkdir(parents=True, exist_ok=True)


def build_command(args, prompt_file: Path) -> list[str]:
    """Build the llama-cli command as a list of strings (no shell escaping needed).

    Uses file-based prompt (-f) instead of command-line prompt (-p) to avoid
    Windows encoding issues with Chinese text.  Also enables --single-turn to
    prevent interactive mode from hanging the subprocess.
    """
    cmd = [
        str(args.llama_cli),
        "-m", str(args.model_path),
        "-f", str(prompt_file),
        "-n", str(args.n_predict),
        "--threads", str(args.threads),
        "--ctx-size", str(args.ctx_size),
        "--temp", str(args.temp),
        "--top-p", str(args.top_p),
        "--repeat-penalty", str(args.repeat_penalty),
        "--no-display-prompt",
        "--single-turn",
        "--simple-io",
    ]
    return cmd


def run_one_prompt(args, item: dict, raw_dir: Path) -> dict:
    """Run llama-cli for a single prompt and return a result row dict."""
    raw_dir.mkdir(parents=True, exist_ok=True)
    raw_filename = f"{args.config_name}_{item['id']}.txt"
    raw_output_path = raw_dir / raw_filename

    # Write prompt to a temp file so we can use -f (file-based prompt).
    # This avoids Windows encoding issues when passing Chinese text via -p.
    prompt_file = raw_dir / f"_tmp_{args.config_name}_{item['id']}.txt"
    prompt_file.write_text(item["prompt"], encoding="utf-8")

    cmd = build_command(args, prompt_file)
    start = time.perf_counter()
    success = True
    error_message = ""
    combined = ""

    try:
        # Ensure MinGW DLLs are findable by prepending to PATH
        env = os.environ.copy()
        env["PATH"] = _MINGW_BIN_STR + os.pathsep + env.get("PATH", "")

        proc = subprocess.run(
            cmd,
            text=True,
            encoding="utf-8",
            errors="replace",
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=args.timeout,
            check=False,
            env=env,
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

    elapsed = time.perf_counter() - start

    # Clean up temp prompt file
    try:
        prompt_file.unlink()
    except OSError:
        pass

    # Write raw output file with command header
    raw_output_path.write_text(
        "COMMAND:\n"
        + " ".join(str(x) for x in cmd)
        + f"\n\nELAPSED: {elapsed:.2f}s\n\n"
        + combined,
        encoding="utf-8",
    )

    # Compute relative path from project root for portability
    try:
        rel_output_path = str(raw_output_path.relative_to(PROJECT_ROOT))
    except ValueError:
        rel_output_path = str(raw_output_path)

    return {
        "prompt_id": item["id"],
        "category": item.get("category", ""),
        "config_name": args.config_name,
        "threads": args.threads,
        "ctx_size": args.ctx_size,
        "temp": args.temp,
        "top_p": args.top_p,
        "repeat_penalty": args.repeat_penalty,
        "n_predict": args.n_predict,
        "output_path": rel_output_path,
        "output_chars": len(combined),
        "success": str(success).lower(),
        "error_message": error_message,
        "manual_correctness_score": "",
        "manual_completeness_score": "",
        "manual_clarity_score": "",
        "hallucination_level": "",
        "notes": "",
    }


def write_csv(output_path: Path, fieldnames: list[str], rows: list[dict]):
    """Write rows to CSV. Append if file exists, otherwise create with header."""
    file_exists = output_path.is_file()
    with output_path.open("a", newline="", encoding="utf-8") as f:
        writer = csv.DictWriter(f, fieldnames=fieldnames)
        if not file_exists:
            writer.writeheader()
        writer.writerows(rows)


def main():
    # -- Resolve default paths relative to project root --
    default_model = PROJECT_ROOT / "models" / "qwen2.5-0.5b-instruct-q4_k_m.gguf"
    default_cli = PROJECT_ROOT / "llama.cpp" / "build" / "bin" / "llama-cli.exe"
    default_prompts = PROJECT_ROOT / "prompts" / "quality_prompts.jsonl"
    default_output = PROJECT_ROOT / "results" / "quality_eval.csv"
    default_raw_dir = PROJECT_ROOT / "results" / "raw_quality_outputs"

    parser = argparse.ArgumentParser(
        description="Quality evaluation script for OSH Lab4 role B."
    )
    parser.add_argument(
        "--model-path", type=Path, default=default_model,
        help=f"Path to GGUF model (default: {default_model})",
    )
    parser.add_argument(
        "--llama-cli", type=Path, default=default_cli,
        help=f"Path to llama-cli executable (default: {default_cli})",
    )
    parser.add_argument(
        "--prompts", type=Path, default=default_prompts,
        help=f"Path to JSONL prompt file (default: {default_prompts})",
    )
    parser.add_argument(
        "--output", type=Path, default=default_output,
        help=f"Path to output CSV (default: {default_output})",
    )
    parser.add_argument(
        "--config-name", type=str, required=True,
        help="Label for this configuration run (e.g. configA, configB)",
    )
    parser.add_argument("--threads", type=int, default=2)
    parser.add_argument("--ctx-size", type=int, default=2048)
    parser.add_argument("--temp", type=float, default=0.7)
    parser.add_argument("--top-p", type=float, default=0.9)
    parser.add_argument("--repeat-penalty", type=float, default=1.1)
    parser.add_argument("--n-predict", type=int, default=256)
    parser.add_argument("--timeout", type=int, default=120)

    args = parser.parse_args()

    # -- Pre-flight checks --
    if not args.model_path.is_file():
        print(f"ERROR: model not found: {args.model_path}", file=sys.stderr)
        return 1
    if not args.llama_cli.is_file():
        print(f"ERROR: llama-cli not found: {args.llama_cli}", file=sys.stderr)
        return 1
    if not args.prompts.is_file():
        print(f"ERROR: prompts file not found: {args.prompts}", file=sys.stderr)
        return 1

    # -- Load prompts --
    prompts = load_prompts(args.prompts)
    print(f"Loaded {len(prompts)} prompts from {args.prompts}")

    # -- CSV field names --
    fieldnames = [
        "prompt_id", "category", "config_name",
        "threads", "ctx_size", "temp", "top_p", "repeat_penalty", "n_predict",
        "output_path", "output_chars",
        "success", "error_message",
        "manual_correctness_score", "manual_completeness_score",
        "manual_clarity_score", "hallucination_level", "notes",
    ]

    # -- Ensure output directories exist --
    ensure_dirs(args.output)
    raw_dir = default_raw_dir

    # -- Run each prompt --
    rows = []
    for item in prompts:
        label = f"[{args.config_name}] prompt={item['id']} category={item['category']}"
        print(f"{label} running ...", end=" ", flush=True)
        row = run_one_prompt(args, item, raw_dir)
        status = "OK" if row["success"] == "true" else f"FAIL ({row['error_message']})"
        print(f"{status}  ({row['output_chars']} chars)")
        rows.append(row)

    # -- Write CSV (append mode) --
    write_csv(args.output, fieldnames, rows)
    print(f"\nAppended {len(rows)} rows to {args.output}")
    print(f"Raw outputs saved to {raw_dir}/")

    # -- Summary --
    ok = sum(1 for r in rows if r["success"] == "true")
    fail = len(rows) - ok
    print(f"Summary: {ok} success, {fail} failure")
    if fail:
        for r in rows:
            if r["success"] != "true":
                print(f"  FAIL [{r['prompt_id']}]: {r['error_message']}")

    return 0 if fail == 0 else 0  # soft fail — don't block remaining configs


if __name__ == "__main__":
    raise SystemExit(main())
