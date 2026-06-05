# OSH 2026 Lab4 Role A README

## 角色 A 范围

角色 A 已完成以下内容：

1. 本地 llama.cpp 部署；
2. 性能指标定义；
3. 单机 baseline benchmark；
4. 参数调优；
5. 供角色 B 和角色 C 继续使用的交接材料。

角色 A 已完成本地单机 baseline。角色 B 和角色 C 后续应以本目录中的模型信息、prompt、CSV 和汇总文档作为共同基准；如需重跑或替换数据，需在各自文档中说明原因。

## 整体分工

| 模块 | 目标 | 负责人 | 当前状态 |
|---|---|---|---|
| llama.cpp 主线本地环境 | 创建本地 Lab4 目录、收集环境、构建 llama.cpp | 角色 A | 已完成 |
| 性能指标 | 定义至少 5 个 LLM 部署性能指标 | 角色 A | 已完成，见 `docs/performance_metrics.md` |
| GGUF 单机部署 | 准备 GGUF 模型并运行 `llama-cli` 推理 | 角色 A | 已完成 |
| 单机 benchmark | 使用 benchmark prompts 测量至少 3 个指标 | 角色 A | 已完成，见 `results/single_benchmark.csv` |
| 参数调优 | 比较 `threads`、`batch-size`、`ctx-size`、`no-mmap` | 角色 A | 已完成，见 `results/param_tuning.csv` |
| 输出质量评估 | 使用共享 prompt 评估生成质量 | 角色 B | 待角色 B 完成；使用 A 的模型和 prompt |
| RPC 分布式推理 | 构建并测试 llama.cpp RPC 流程 | 角色 B | 待角色 B 完成；使用 A 的模型、构建和 baseline |
| 单机与 RPC 对比 | 将 RPC 结果与 A 的 baseline 对比 | 角色 B | 待角色 B 完成；引用 A 的 CSV 汇总 |
| Ray 批量推理 | 实现 Ray 批量任务调度 | 角色 C | 待角色 C 完成；使用 A 的 prompt 和 baseline |
| Ray 负载均衡与重试 | 选做加分项 | 角色 C | 待角色 C 按实际完成情况补充 |

## 角色 A 已完成工作

角色 A 已在 Windows 上使用 MSYS2 UCRT64 GCC 14.1.0 完成 llama.cpp CPU backend 构建。可运行构建目录为：

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10
```

本次使用模型：

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf
```

baseline benchmark 共 15 次真实运行：5 条 prompt，每条重复 3 次，全部成功。参数调优 benchmark 共 24 次真实运行，全部成功。

已采集字段：

- `total_latency_s`
- `tokens_per_second`
- `output_chars`
- `success`
- 参数配置：`threads`、`ctx_size`、`batch_size`、`no_mmap`

`max_rss_kb` 本次未在 Windows / Git Bash 环境稳定采集；CSV 原始列为空值，汇总文档统一标注 `not measured on Windows`。原因和限制见 `docs/performance_metrics.md` 和 `docs/performance_analysis.md`。

## 本地目录

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4
```

## 关键文件

| 文件 | 用途 |
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

## 构建与模型

| Item | Value |
|---|---|
| llama.cpp commit | 60130d18f9ac7f42cb4d7f6060b088a45d8f242e |
| CPU build | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10 |
| MODEL_PATH | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf |
| LLAMA_CLI | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe |
| LLAMA_BENCH | C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10\bin\llama-bench.exe |
| 推荐配置 | `--threads 4 --ctx-size 1024 --batch-size 256` |
| baseline 配置 | `--threads 4 --ctx-size 2048 --batch-size 256` |

## 完成度检查

| 检查项 | 结果 |
|---|---|
| 角色 A 指南要求文件 | 已补齐 |
| 环境收集 | 已完成：`results/env_info.txt` |
| llama.cpp CPU build | 已完成 |
| 单次推理 | 已完成：`results/single_inference_output.txt` |
| baseline benchmark | 15 行，15 次成功 |
| 参数调优 | 24 行，24 次成功 |
| tokens/s 解析 | benchmark 记录均已解析 |
| max RSS | Windows 环境未稳定采集，CSV 原始列为空值，汇总表标注 `not measured on Windows` |
| GPU backend | 未构建 CUDA backend，本文只报告 CPU backend 结果 |

## 角色 B 对接

角色 B 继续工作时优先使用以下文件：

- `configs/model_info.md`
- `results/single_benchmark.csv`
- `results/param_tuning.csv`
- `results/single_benchmark_summary.md`
- `docs/performance_analysis.md`

除非确有必要，不要覆盖 A 的 baseline 文件；如需重跑，需在 B 的文档中说明原因。

角色 B 后续任务：

1. 使用同一 GGUF 模型进行输出质量评估；
2. 启动和测试 `llama-server`；
3. 完成 llama.cpp RPC 多机分布式推理；
4. 对比单机 baseline 与 RPC 结果；
5. 如进行 llama-server 并发测试，明确记录并发数、请求数、延迟、吞吐和失败率。

角色 B 文件和数据约定：

1. 不替换 A 的模型；如替换，必须在报告中说明原因。
2. 不覆盖 `results/single_benchmark.csv` 或 `results/param_tuning.csv`。
3. B 的结果单独放文件，例如 `results/rpc_benchmark.csv` 或 `results/server_concurrency.csv`。
4. 对比单机和 RPC 时，引用 A 的 baseline 配置和 prompt 文件。

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

### baseline 复用命令

```powershell
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
& 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe' `
  -m 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf' `
  -f 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\results\single_prompt.txt' `
  -n 128 --threads 4 --ctx-size 2048 --batch-size 256 --single-turn --simple-io
```

### 质量评估起始命令

角色 B 可继续使用同一个 `llama-cli` 和 UTF-8 prompt 文件。Windows 下优先使用 `-f`，不要直接通过 `-p` 传中文 prompt。

```powershell
$lab4 = 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4'
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
$model = "$lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf"
$cli = "$lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe"
$promptFile = "$lab4\prompts\quality_prompt_one.txt"
[System.IO.File]::WriteAllText($promptFile, '请解释页表、TLB 和缺页中断之间的关系。', [System.Text.UTF8Encoding]::new($false))
& $cli -m $model -f $promptFile -n 128 --threads 4 --ctx-size 2048 --batch-size 256 --single-turn --simple-io
```

### llama-server 起始命令

角色 B 可从同一构建目录启动 `llama-server.exe`。如增加网络或 RPC 参数，需要在 B 的命令日志中记录完整命令。

```powershell
$lab4 = 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4'
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
& "$lab4\llama.cpp\build-ucrt-win10\bin\llama-server.exe" `
  -m "$lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf" `
  --threads 4 `
  --ctx-size 2048 `
  --batch-size 256
```

B 建议新增文件：

| 文件 | 用途 |
|---|---|
| `docs/quality_evaluation.md` | 输出质量评估说明 |
| `docs/rpc_deploy.md` | RPC 部署步骤 |
| `results/quality_eval.csv` | 质量评估记录 |
| `results/rpc_benchmark.csv` | RPC 性能测试结果 |
| `results/single_vs_rpc_summary.md` | 单机和 RPC 对比 |
| `command_logs/B_rpc_commands.md` | B 的真实命令记录 |

### 角色 B 下一步

1. 使用同一模型做输出质量评估；
2. 使用同一模型启动 llama-server；
3. 使用 A 的 baseline 作为单机性能参考；
4. RPC 对比时不要更换模型和 prompt，除非文档说明原因。

## 角色 C 对接

角色 C 继续工作时优先使用以下文件和配置：

- `prompts/role_a_benchmark_prompts.jsonl`
- `results/single_benchmark_summary.md`
- A 的推荐单机配置

角色 C 后续任务：

1. 完成 Ray 批量推理任务；
2. 设计 Ray 任务调度、批量输入、结果收集和错误记录；
3. 将 Ray 批量吞吐与 A 的单机 baseline 谨慎对比；
4. 如做选做加分，补充负载均衡和失败重试；
5. 明确区分“单条请求延迟”和“批量任务吞吐”。

角色 C 文件和数据约定：

1. 尽量复用 `prompts/role_a_benchmark_prompts.jsonl`。
2. 除非直接测量了单请求延迟，否则不要写 Ray 加速了单条 prompt。
3. 批量测试需记录请求数、总 wall-clock 时间、吞吐、成功数、失败数和重试次数。
4. C 的结果单独放文件，例如 `results/ray_batch_results.csv`。

### 共享 prompt 文件

| 文件 | 用途 |
|---|---|
| prompts/role_a_benchmark_prompts.jsonl | A 的性能测试 prompt，可作为 Ray prompt 基础 |
| prompts/quality_prompts.jsonl | B 的质量评估 prompt，也可被 C 复用 |

### Ray 对比基准

Role C 对比 Ray 批量推理和 A 的单机 baseline 时，应区分单请求延迟和批量吞吐。

注意：
A 的单机 benchmark 测的是单机 llama-cli 一次性推理；
C 的 Ray 测的是多个请求的任务级调度。
两者不是完全同一指标，不能简单说 Ray 加速了单条 prompt。
更准确的说法是：
Ray 可能提高批量任务吞吐，但单条请求延迟可能由于调度和网络开销变大。

### Ray 输入格式

角色 C 可直接读取 JSONL prompt 记录：

```jsonl
{"id":"A001","category":"short_qa","prompt":"请用三句话解释什么是虚拟内存。"}
```

C 建议新增文件：

| 文件 | 用途 |
|---|---|
| `docs/ray_batch_inference.md` | Ray 批量推理说明 |
| `docs/ray_analysis.md` | Ray 与单机 baseline 分析 |
| `results/ray_batch_results.csv` | Ray 每条任务结果 |
| `results/ray_batch_summary.md` | Ray 汇总结果 |
| `command_logs/C_ray_commands.md` | C 的真实命令记录 |

### Ray 对比字段

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

## 最终报告整合说明

最终报告中，A 的材料应作为单机 baseline 引用：

| 最终报告部分 | 角色 A 材料 |
|---|---|
| 单机部署 | `docs/deploy_llama_single.md` |
| 性能指标 | `docs/performance_metrics.md` |
| 单机 benchmark | `results/single_benchmark.csv` 和 `results/single_benchmark_summary.md` |
| 参数优化 | `results/param_tuning.csv` 和 `results/param_tuning_summary.md` |
| 系统分析 | `docs/performance_analysis.md` |
| 复现命令 | `command_logs/A_single_benchmark_commands.md` |

Git 提交不包含大模型文件、`llama.cpp/` 源码目录和构建目录；这些内容已由 `.gitignore` 排除。

## 未纳入 Git 的内容

Git 提交不包含 GGUF 模型文件、`llama.cpp/` 源码目录和构建产物。后续成员需要将同名模型文件放到 `Lab4/models/`，或在自己的文档中说明替代模型。
