#!/usr/bin/env python3
import argparse
import csv
from collections import defaultdict
from pathlib import Path
from statistics import mean


def to_float(x):
    try:
        if x == "" or x is None:
            return None
        return float(x)
    except ValueError:
        return None


def summarize(path: Path, group_keys):
    rows = list(csv.DictReader(path.open("r", encoding="utf-8")))
    groups = defaultdict(list)
    for row in rows:
        key = tuple(row.get(k, "") for k in group_keys)
        groups[key].append(row)

    lines = []
    lines.append(f"# Summary for `{path}`")
    lines.append("")
    lines.append(f"Group keys: {', '.join(group_keys)}")
    lines.append("")
    header = list(group_keys) + [
        "count", "success", "fail",
        "avg_latency_s", "avg_tokens_per_second", "avg_max_rss_kb",
    ]
    lines.append("| " + " | ".join(header) + " |")
    lines.append("|" + "|".join(["---"] * len(header)) + "|")

    for key, items in sorted(groups.items()):
        lat = [to_float(r.get("total_latency_s")) for r in items]
        lat = [x for x in lat if x is not None]

        tps = [to_float(r.get("tokens_per_second")) for r in items]
        tps = [x for x in tps if x is not None]

        rss = [to_float(r.get("max_rss_kb")) for r in items]
        rss = [x for x in rss if x is not None]

        success = sum(1 for r in items if str(r.get("success", "")).lower() == "true")
        fail = len(items) - success

        values = list(key) + [
            str(len(items)),
            str(success),
            str(fail),
            f"{mean(lat):.4f}" if lat else "",
            f"{mean(tps):.4f}" if tps else "",
            f"{mean(rss):.2f}" if rss else "",
        ]
        lines.append("| " + " | ".join(values) + " |")

    return "\n".join(lines) + "\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--output", required=True)
    parser.add_argument("--group-by", default="threads,ctx_size,batch_size,no_mmap")
    args = parser.parse_args()

    group_keys = [x.strip() for x in args.group_by.split(",") if x.strip()]
    text = summarize(Path(args.input), group_keys)
    Path(args.output).write_text(text, encoding="utf-8")
    print(f"Summary written to {args.output}")


if __name__ == "__main__":
    main()
