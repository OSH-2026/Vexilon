# Performance Test and System Analysis

## 1. 实验目标

本部分完成角色 A 负责的 llama.cpp 单机性能测试与参数优化。

## 2. 实验环境

引用 `results/env_info.txt` 中的关键信息：

| Item | Value |
|---|---|
| CPU | Intel Core i9-14900HX, 24 cores / 32 logical processors |
| Memory | 16,779,841,536 bytes, about 16 GB |
| OS | Windows 11 / Git Bash MINGW64_NT-10.0-26200 |
| GPU | NVIDIA driver detected, CUDA 12.6 reported by nvidia-smi |
| llama.cpp commit | 60130d18f9ac7f42cb4d7f6060b088a45d8f242e |
| Model | Qwen2.5-0.5B-Instruct-GGUF |
| Quantization | Q4_K_M |

## 3. 测试任务设计

| ID | Category | Purpose |
|---|---|---|
| A001 | short_qa | 短问答 |
| A002 | os_course | 操作系统课程问答 |
| A003 | code_explanation | 代码解释 |
| A004 | summary | 摘要 |
| A005 | reasoning | 推理 |

## 4. 测量指标

本次实际测量：

1. 总生成延迟：`total_latency_s`；
2. 输出速度：`tokens_per_second`，解析 llama.cpp 输出中的 `Generation: x t/s`；
3. 输出长度：`output_chars`；
4. 成功率：`success`；
5. 参数配置：threads、ctx-size、batch-size、no-mmap。

内存占用 `max_rss_kb` 本次未在 Windows 环境稳定采集。原因：当前 Windows / Git Bash 环境没有可用的 `/usr/bin/time -v`。CSV 原始列保留为空值，Markdown 汇总表统一标注为 `not measured on Windows`，不填估算值。

## 5. Baseline 结果

Baseline 配置：

```bash
--threads 4 --ctx-size 2048 --batch-size 256
```

`results/single_benchmark_summary.md`：

| prompt_id | threads | ctx_size | batch_size | no_mmap | count | success | fail | avg_latency_s | avg_tokens_per_second | avg_max_rss_kb |
|---|---|---|---|---|---|---|---|---|---|---|
| A001 | 4 | 2048 | 256 | false | 3 | 3 | 0 | 13.2022 | 10.2000 | not measured on Windows |
| A002 | 4 | 2048 | 256 | false | 3 | 3 | 0 | 16.5736 | 10.0000 | not measured on Windows |
| A003 | 4 | 2048 | 256 | false | 3 | 3 | 0 | 17.8684 | 10.2667 | not measured on Windows |
| A004 | 4 | 2048 | 256 | false | 3 | 3 | 0 | 12.9490 | 10.0333 | not measured on Windows |
| A005 | 4 | 2048 | 256 | false | 3 | 3 | 0 | 17.9191 | 10.1667 | not measured on Windows |

Baseline 共 15 次推理，15 次成功，成功率 100%。

## 6. 参数优化结果

`results/param_tuning_summary.md`：

| threads | ctx_size | batch_size | no_mmap | count | success | fail | avg_latency_s | avg_tokens_per_second | avg_max_rss_kb |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 2048 | 256 | false | 2 | 2 | 0 | 34.4305 | 4.5000 | not measured on Windows |
| 2 | 2048 | 256 | false | 2 | 2 | 0 | 17.4359 | 7.6500 | not measured on Windows |
| 4 | 1024 | 256 | false | 2 | 2 | 0 | 11.0944 | 10.6000 | not measured on Windows |
| 4 | 2048 | 128 | false | 2 | 2 | 0 | 13.5222 | 10.3500 | not measured on Windows |
| 4 | 2048 | 256 | false | 8 | 8 | 0 | 13.6124 | 10.4250 | not measured on Windows |
| 4 | 2048 | 256 | true | 2 | 2 | 0 | 11.9652 | 10.2500 | not measured on Windows |
| 4 | 2048 | 512 | false | 2 | 2 | 0 | 16.0380 | 10.2500 | not measured on Windows |
| 4 | 512 | 256 | false | 2 | 2 | 0 | 12.0538 | 10.1000 | not measured on Windows |
| 8 | 2048 | 256 | false | 2 | 2 | 0 | 12.0824 | 10.4500 | not measured on Windows |

参数优化共 24 次推理，24 次成功，成功率 100%。

## 7. 参数影响分析

### 7.1 threads

线程数对 CPU 推理影响明显。1 线程平均 4.5 t/s，2 线程提升到 7.65 t/s，4 线程和 8 线程都在约 10.4 t/s 左右。继续增加到 8 线程提升很小，说明本机对该 0.5B Q4 模型已经接近内存带宽、调度开销或小模型计算规模限制。

### 7.2 batch-size

batch-size 为 128、256、512 时，平均输出速度分别约为 10.35、10.43、10.25 t/s。差异不大，batch-size=256 稍好。batch-size 增大不一定总是更快，过大时可能增加缓存和调度压力。

### 7.3 ctx-size

ctx-size 为 512、1024、2048 时，平均输出速度分别约为 10.10、10.60、10.43 t/s。本次短 prompt 下，ctx-size 对速度影响不大；1024 在测试中略好，但结果可能受短 prompt 和重复次数有限影响。

### 7.4 no-mmap

默认 mmap 的 4/2048/256 平均约 10.43 t/s，`--no-mmap` 平均约 10.25 t/s。生成速度差异不大；mmap 更多影响模型加载和页面换入方式。本次未稳定单独测量 load time 和 peak RSS。

### 7.5 GPU offload

本机检测到 NVIDIA 驱动，但本次没有构建 CUDA backend，也没有测试 `--n-gpu-layers`。因此本文只给出 CPU backend 结论，不对 GPU offload 做性能判断。

## 8. 最优配置

根据真实 CSV 数据，本机推荐配置：

```bash
--threads 4 --ctx-size 1024 --batch-size 256
```

说明：该配置在 A001/A002 调优 prompt 上平均 10.60 t/s，延迟也较低。若角色 B/C 需要与 baseline 保持完全一致，可继续使用 baseline 配置：

```bash
--threads 4 --ctx-size 2048 --batch-size 256
```

## 9. 系统原因分析

1. CPU 多线程：从 1 到 4 线程提升明显，8 线程收益变小，说明线程过多后出现边际收益下降。
2. 内存带宽：小量化模型生成阶段可能受内存访问、缓存命中和调度影响。
3. 模型量化：Q4_K_M 降低模型体积和内存压力，适合 16GB 内存机器做 baseline。
4. 上下文长度：短 prompt 下 ctx-size 差异不显著，长上下文任务可能更明显。
5. mmap 和页面换入：默认 mmap 便于系统按需映射模型文件；no-mmap 可能增加加载开销。
6. 小模型和短 prompt 的测试偏差：0.5B 模型和 128 token 输出较短，结果适合课程 baseline，不代表大模型服务场景。

## 10. 与角色 B/C 的对接

## Handoff to Role B

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

### 角色 B 下一步

1. 使用同一模型做输出质量评估；
2. 使用同一模型启动 llama-server；
3. 使用 A 的 baseline 作为单机性能参考；
4. RPC 对比时不要更换模型和 prompt，除非文档说明原因。

## Handoff to Role C

### Shared prompt files

| File | Purpose |
|---|---|
| prompts/role_a_benchmark_prompts.jsonl | A 的性能测试 prompt，可作为 Ray prompt 基础 |
| prompts/quality_prompts.jsonl | B 的质量评估 prompt，也可被 C 复用 |

### Baseline for Ray analysis

角色 C 对比 Ray 批量推理和 A 的单机 baseline 时，应区分单请求延迟和批量吞吐。

注意：
A 的单机 benchmark 测的是单机 llama-cli 一次性推理；
C 的 Ray 测的是多个请求的任务级调度。
两者不是完全同一指标，不能简单说 Ray 加速了单条 prompt。
更准确的说法是：
Ray 可能提高批量任务吞吐，但单条请求延迟可能由于调度和网络开销变大。

## 11. 局限性

1. 机器资源有限，内存约 16GB；
2. 测试 prompt 数量有限；
3. 参数调优每组重复次数为 1，baseline 每条 prompt 重复 3 次；
4. tokens/s 解析依赖 llama.cpp 输出格式；
5. Windows 环境未稳定采集 max RSS，汇总表已标注 `not measured on Windows`；
6. 未构建 CUDA backend，未评估 GPU offload。

## 12. 复现命令

完整命令见 `command_logs/A_single_benchmark_commands.md`。
