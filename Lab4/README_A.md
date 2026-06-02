# OSH 2026 Lab4 Role A README

## Role A Scope

Role A covers:

1. Local llama.cpp deployment;
2. Performance metrics;
3. Single-machine benchmark;
4. Parameter tuning;
5. Handoff artifacts for Role B and Role C.

Role A has completed the local baseline work. Role B and Role C should treat the files in this directory as the shared reference unless they explicitly document a reason to rerun or replace an artifact.

## Overall Task Split

| Module | Goal | Owner | Current status |
|---|---|---|---|
| llama.cpp main task, local setup | Create local Lab4 directory, collect environment, build llama.cpp | Role A | Completed |
| Performance metrics | Define at least 5 LLM deployment metrics | Role A | Completed in `docs/performance_metrics.md` |
| GGUF single-machine deployment | Prepare GGUF model and run `llama-cli` inference | Role A | Completed |
| Single-machine benchmark | Run benchmark prompts and measure at least 3 metrics | Role A | Completed in `results/single_benchmark.csv` |
| Parameter tuning | Compare `threads`, `batch-size`, `ctx-size`, `no-mmap` | Role A | Completed in `results/param_tuning.csv` |
| Output quality evaluation | Evaluate generated quality with shared prompts | Role B | Not part of Role A; use A's model and prompts |
| RPC distributed inference | Build and test llama.cpp RPC flow | Role B | Not part of Role A; use A's model/build/baseline |
| Single-machine vs RPC comparison | Compare RPC results against A baseline | Role B | Not part of Role A; use A's CSV summaries |
| Ray batch inference | Implement Ray batch task scheduling | Role C | Not part of Role A; use A's prompts and baseline |
| Ray load balancing and retry | Optional Ray extra credit | Role C | Not part of Role A |

## Role A Completed Work

Role A produced a reproducible local llama.cpp CPU deployment on Windows using MSYS2 UCRT64 GCC 14.1.0. The final working build is:

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10
```

Role A used this model:

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf
```

The baseline benchmark contains 15 real runs: 5 prompts, 3 repeats each, all successful. The parameter tuning benchmark contains 24 real runs, all successful.

Measured columns include:

- `total_latency_s`
- `tokens_per_second`
- `output_chars`
- `success`
- parameter settings: `threads`, `ctx_size`, `batch_size`, `no_mmap`

`max_rss_kb` is intentionally blank because the current Windows / Git Bash environment did not provide `/usr/bin/time -v`. This is documented in `docs/performance_metrics.md` and `docs/performance_analysis.md`.

## Local Directory

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4
```

## Important Files

| File | Purpose |
|---|---|
| docs/deploy_llama_single.md | 单机部署说明 |
| docs/performance_metrics.md | 性能指标说明 |
| docs/performance_analysis.md | 性能测试与系统分析 |
| scripts/env_collect.sh | 环境收集 |
| scripts/run_llama_cli.sh | 单次推理 |
| scripts/bench_single.py | 性能测试 |
| scripts/summarize_csv.py | CSV 汇总 |
| results/single_benchmark.csv | 单机 baseline |
| results/param_tuning.csv | 参数优化结果 |

## Build and Model

| Item | Value |
|---|---|
| llama.cpp commit | 60130d18f9ac7f42cb4d7f6060b088a45d8f242e |
| CPU build | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10 |
| MODEL_PATH | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf |
| LLAMA_CLI | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe |
| LLAMA_BENCH | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10\bin\llama-bench.exe |
| Recommended config | `--threads 4 --ctx-size 1024 --batch-size 256` |
| Baseline config | `--threads 4 --ctx-size 2048 --batch-size 256` |

## Validation Summary

| Check | Result |
|---|---|
| Required files from Role A guide | All present |
| Environment collection | Completed: `results/env_info.txt` |
| llama.cpp CPU build | Completed |
| Single inference | Completed: `results/single_inference_output.txt` |
| Baseline benchmark | 15 rows, 15 successful |
| Parameter tuning | 24 rows, 24 successful |
| tokens/s parsing | Completed for all benchmark rows |
| max RSS | Not available on this Windows setup; left blank honestly |
| GPU backend | Skipped; not required for Role A main task |

## Handoff to Role B

Role B should use:

- `configs/model_info.md`
- `results/single_benchmark.csv`
- `results/param_tuning.csv`
- `results/single_benchmark_summary.md`
- `docs/performance_analysis.md`

Role B should not rerun or overwrite A's baseline unless necessary.

Role B's main responsibilities are:

1. 使用同一 GGUF 模型进行输出质量评估；
2. 启动和测试 `llama-server`；
3. 完成 llama.cpp RPC 多机分布式推理；
4. 对比单机 baseline 与 RPC 结果；
5. 如进行 llama-server 并发测试，明确记录并发数、请求数、延迟、吞吐和失败率。

Role B should keep the following rules:

1. Do not replace A's model unless the report explains why.
2. Do not overwrite `results/single_benchmark.csv` or `results/param_tuning.csv`.
3. Put B-specific results in separate files, for example `results/rpc_benchmark.csv` or `results/server_concurrency.csv`.
4. When comparing single-machine and RPC results, cite A's baseline config and prompt file.

### Paths

| Item | Path |
|---|---|
| LAB4_DIR | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4 |
| MODEL_PATH | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf |
| LLAMA_CLI | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe |
| LLAMA_BENCH | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10\bin\llama-bench.exe |
| baseline CSV | Lab4/results/single_benchmark.csv |
| parameter tuning CSV | Lab4/results/param_tuning.csv |
| single inference output | Lab4/results/single_inference_output.txt |

### Recommended baseline command

```powershell
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
& 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe' `
  -m 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf' `
  -f 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\results\single_prompt.txt' `
  -n 128 --threads 4 --ctx-size 2048 --batch-size 256 --single-turn --simple-io
```

### Suggested quality evaluation command

Role B can use the same `llama-cli` and a UTF-8 prompt file. On Windows, prefer `-f` instead of passing Chinese prompt text through `-p`.

```powershell
$lab4 = 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4'
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
$model = "$lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf"
$cli = "$lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe"
$promptFile = "$lab4\prompts\quality_prompt_one.txt"
[System.IO.File]::WriteAllText($promptFile, '请解释页表、TLB 和缺页中断之间的关系。', [System.Text.UTF8Encoding]::new($false))
& $cli -m $model -f $promptFile -n 128 --threads 4 --ctx-size 2048 --batch-size 256 --single-turn --simple-io
```

### Suggested llama-server starting point

Role B may start from the same build directory. If `llama-server.exe` needs network/RPC-specific options, record the exact command in B's command log.

```powershell
$lab4 = 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4'
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
& "$lab4\llama.cpp\build-ucrt-win10\bin\llama-server.exe" `
  -m "$lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf" `
  --threads 4 `
  --ctx-size 2048 `
  --batch-size 256
```

Suggested B output files:

| File | Purpose |
|---|---|
| `docs/quality_evaluation.md` | 输出质量评估说明 |
| `docs/rpc_deploy.md` | RPC 部署步骤 |
| `results/quality_eval.csv` | 质量评估记录 |
| `results/rpc_benchmark.csv` | RPC 性能测试结果 |
| `results/single_vs_rpc_summary.md` | 单机和 RPC 对比 |
| `command_logs/B_rpc_commands.md` | B 的真实命令记录 |

### What Role B should do next

1. 使用同一模型做输出质量评估；
2. 使用同一模型启动 llama-server；
3. 使用 A 的 baseline 作为单机性能参考；
4. RPC 对比时不要更换模型和 prompt，除非文档说明原因。

## Handoff to Role C

Role C should use:

- `prompts/role_a_benchmark_prompts.jsonl`
- `results/single_benchmark_summary.md`
- A's recommended single-machine configuration

Role C's main responsibilities are:

1. 完成 Ray 批量推理任务；
2. 设计 Ray 任务调度、批量输入、结果收集和错误记录；
3. 将 Ray 批量吞吐与 A 的单机 baseline 谨慎对比；
4. 如做选做加分，补充负载均衡和失败重试；
5. 明确区分“单条请求延迟”和“批量任务吞吐”。

Role C should keep the following rules:

1. Reuse `prompts/role_a_benchmark_prompts.jsonl` when possible.
2. Do not claim Ray speeds up a single prompt unless the data directly measures single-request latency.
3. For batch tests, report request count, total wall-clock time, throughput, success count, fail count, and retry count.
4. Put C-specific results in separate files, for example `results/ray_batch_results.csv`.

### Shared prompt files

| File | Purpose |
|---|---|
| prompts/role_a_benchmark_prompts.jsonl | A 的性能测试 prompt，可作为 Ray prompt 基础 |
| prompts/quality_prompts.jsonl | B 的质量评估 prompt，也可被 C 复用 |

### Baseline for Ray analysis

Role C should compare Ray batch inference against A's single-machine baseline carefully.

注意：
A 的单机 benchmark 测的是单机 llama-cli 一次性推理；
C 的 Ray 测的是多个请求的任务级调度。
两者不是完全同一指标，不能简单说 Ray 加速了单条 prompt。
更准确的说法是：
Ray 可能提高批量任务吞吐，但单条请求延迟可能由于调度和网络开销变大。

### Suggested Ray input format

Role C can read JSONL prompt records directly:

```jsonl
{"id":"A001","category":"short_qa","prompt":"请用三句话解释什么是虚拟内存。"}
```

Suggested C output files:

| File | Purpose |
|---|---|
| `docs/ray_batch_inference.md` | Ray 批量推理说明 |
| `docs/ray_analysis.md` | Ray 与单机 baseline 分析 |
| `results/ray_batch_results.csv` | Ray 每条任务结果 |
| `results/ray_batch_summary.md` | Ray 汇总结果 |
| `command_logs/C_ray_commands.md` | C 的真实命令记录 |

### Suggested Ray comparison fields

| Field | Meaning |
|---|---|
| `prompt_id` | prompt 编号，建议沿用 A001-A005 |
| `category` | prompt 类型 |
| `worker_id` | Ray worker 标识 |
| `start_time` | 任务开始时间 |
| `end_time` | 任务结束时间 |
| `latency_s` | 单任务耗时 |
| `success` | 是否成功 |
| `retry_count` | 重试次数 |
| `output_chars` | 输出长度 |
| `error_message` | 失败原因 |

## Integration Notes for Final Report

Final integration should cite Role A artifacts as the single-machine baseline:

| Final report section | Role A artifact |
|---|---|
| 单机部署 | `docs/deploy_llama_single.md` |
| 性能指标 | `docs/performance_metrics.md` |
| 单机 benchmark | `results/single_benchmark.csv` and `results/single_benchmark_summary.md` |
| 参数优化 | `results/param_tuning.csv` and `results/param_tuning_summary.md` |
| 系统分析 | `docs/performance_analysis.md` |
| 复现命令 | `command_logs/A_single_benchmark_commands.md` |

Do not include large model files, `llama.cpp/`, or build directories in Git submission. `.gitignore` already excludes them.

## Not Included

This local directory does not include model files in Git submission. Models must be downloaded separately or placed manually under `Lab4/models/`.
