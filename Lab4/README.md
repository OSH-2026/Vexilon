# OSH 2026 Lab4 — LLM 部署、性能测试与分布式推理

> **最终整合版本** | 整合时间：2026-06-06
>
> 本项目完成 llama.cpp 单机部署、性能测试、参数优化、输出质量评估、RPC 分布式推理、
> llama-server 并发测试，以及基于 Ray 的批量推理调度、负载均衡与失败重试。
>
> 详细检查清单见 [`docs/final_checklist.md`](docs/final_checklist.md)。

---

## 目录

- [1. 项目简介](#1-项目简介)
- [2. 为什么选择 Ray](#2-为什么选择-ray)
- [3. 目录结构](#3-目录结构)
- [4. 依赖环境](#4-依赖环境)
- [5. 模型准备](#5-模型准备)
- [6. 单机部署复现](#6-单机部署复现)
- [7. 性能测试复现](#7-性能测试复现)
- [8. 输出质量评估复现](#8-输出质量评估复现)
- [9. RPC 分布式推理复现](#9-rpc-分布式推理复现)
- [10. llama-server 并发测试复现](#10-llama-server-并发测试复现)
- [11. Ray 批量推理复现](#11-ray-批量推理复现)
- [12. Ray 负载均衡复现](#12-ray-负载均衡复现)
- [13. Ray 失败重试复现](#13-ray-失败重试复现)
- [14. 结果文件说明](#14-结果文件说明)
- [15. 截图位置说明](#15-截图位置说明)
- [16. 注意事项](#16-注意事项)

---

## 1. 项目简介

本项目围绕 **Qwen2.5-0.5B-Instruct（Q4_K_M 量化，GGUF 格式）** 模型，
使用 llama.cpp 框架，完成以下实验：

| 模块 | 内容 | 负责人 | 状态 |
|---|---|---|---|
| llama.cpp 主线 | 单机部署、性能指标、benchmark、参数优化、质量评估、RPC 分布式推理、单机 vs RPC 对比 | A + B | 完成 |
| llama.cpp 选做 | llama-server 服务化 + 轻量并发测试 | B | 完成 |
| Ray 必做 | Ray 环境部署、批量推理（serial / round_robin / parallel）、系统分析 | C | 完成 |
| Ray 选做 | 负载均衡调度（round_robin / latency_aware）、失败重试 | C | 完成 |

### 实验机器

| 角色 | 机器 | CPU | OS | 用途 |
|---|---|---|---|---|
| A | LAPTOP-G44Q460K | i9-14900HX (24C/32T) | Windows 11 | 单机部署、性能测试、参数调优 |
| B | LAPTOP-CNRQSONN | Ultra 7 255HX (20C/20T) | Windows 11 | 质量评估、llama-server、RPC host |
| B (RPC worker) | ljyUSTC | i7-10700 (8C/16T) | Ubuntu 24.04 | RPC server |
| C | ljyUSTC | i7-10700 (8C/16T) | Ubuntu 24.04 | Ray 实验（2 个 llama-server 本地模拟多节点） |

> **注意**：三台机器硬件不同，A 的 baseline 数字不能直接与 B/C 的结果比较绝对值。
> B 的 RPC 通信通过移动热点（192.168.137.0/24）完成。

---

## 2. 为什么选择 Ray

Ray 选型理由详见 [`docs/ray_task.md`](docs/ray_task.md) 第 2 节。核心原因：

| 特性 | 说明 |
|---|---|
| **Python-native API** | `@ray.remote` 装饰器将普通 Python 函数/类变为分布式任务/Actor |
| **Actor 模型** | 每个 `LlamaServerActor` 绑定一个 llama-server，保持长连接 |
| **自动容错** | Task 失败可自动重试；Actor 崩溃可重建 |
| **统一调度** | 单机 `ray.init()` 与多机集群使用相同 API |
| **零序列化负担** | Ray 自动处理对象序列化和传输 |
| **vs 手动 RPC** | Role B 的 gRPC 适合直连场景；Ray 更适合多 worker 协调、负载均衡、状态管理 |

---

## 3. 目录结构

```
Lab4/
├── README.md                          ← 本文件
├── .gitignore                         ← 排除 models/、llama.cpp/、build/
├── configs/
│   ├── experiment_config.example.json ← 实验配置模板
│   ├── machine_info_template.md       ← 机器信息模板
│   ├── model_info.md                  ← 模型信息
│   ├── role_a_paths.env.example       ← A 的路径环境变量模板
│   └── server_ports.md                ← Ray 实验 server 端口说明
├── docs/
│   ├── deploy_llama_single.md         ← 单机部署说明（A）
│   ├── performance_metrics.md         ← 性能指标定义（A，8 个指标）
│   ├── performance_analysis.md        ← 性能测试与系统分析（A）
│   ├── quality_eval.md                ← 输出质量评估（B）
│   ├── rpc_deploy.md                  ← RPC 分布式推理部署（B）
│   ├── llama_server_concurrency.md    ← llama-server 并发测试（B）
│   └── ray_task.md                    ← Ray 批量推理调度实验（C）
├── scripts/
│   ├── bench_single.py                ← 单机 benchmark 脚本（A）
│   ├── run_quality_eval.py            ← 质量评估脚本（B）
│   ├── test_llama_server_concurrency.py ← 并发测试脚本（B）
│   ├── ray_batch_infer.py             ← Ray 批量推理脚本（C）
│   ├── ray_load_balance.py            ← Ray 负载均衡脚本（C）
│   ├── ray_failure_retry.py           ← Ray 失败重试脚本（C）
│   ├── env_collect.sh                 ← 环境信息收集（A）
│   ├── run_llama_cli.sh               ← 单次推理快捷脚本（A）
│   └── summarize_csv.py               ← CSV 汇总工具
├── prompts/
│   ├── quality_prompts.jsonl          ← 质量评估 prompt（5 条）
│   ├── ray_prompts_20.jsonl           ← Ray 基础 prompt（20 条）
│   ├── ray_prompts_30.jsonl           ← Ray 扩展 prompt（30 条）
│   ├── role_a_benchmark_prompts.jsonl ← A 的 benchmark prompt
│   └── role_a_tuning_prompts.jsonl    ← A 的参数调优 prompt
├── results/
│   ├── README.md                      ← 结果目录说明（A）
│   ├── env_info.txt                   ← 环境信息（A 的机器）
│   ├── rpc_network_info.txt           ← RPC 网络信息（B 的机器）
│   ├── single_benchmark.csv           ← 单机 baseline（15 行）
│   ├── single_benchmark_summary.md    ← baseline 汇总
│   ├── param_tuning.csv               ← 参数调优（24 行）
│   ├── param_tuning_summary.md        ← 参数调优汇总
│   ├── quality_eval.csv               ← 质量评估（10 行，5 prompt × 2 config）
│   ├── server_concurrency.csv         ← 并发测试（30 行，3 档 × 10 请求）
│   ├── single_vs_rpc.csv              ← 单机 vs RPC 对比（12 行）
│   ├── single_vs_rpc_summary.md       ← 对比分析
│   ├── ray_serial.csv                 ← Ray 串行（30 行）
│   ├── ray_round_robin.csv            ← Ray 轮询（30 行）
│   ├── ray_parallel.csv               ← Ray 并行（30 行）
│   ├── ray_load_balance_round_robin.csv        ← 负载均衡-轮询（30 行）
│   ├── ray_load_balance_round_robin_summary.csv
│   ├── ray_load_balance_latency_aware.csv      ← 负载均衡-延迟感知（30 行）
│   ├── ray_load_balance_latency_aware_summary.csv
│   ├── ray_failure_retry.csv           ← 失败重试（30 行）
│   ├── ray_failure_retry.log           ← 失败重试日志
│   ├── raw_single_outputs/             ← A 的原始推理输出
│   ├── raw_quality_outputs/            ← B 的质量评估原始输出
│   ├── server_concurrency_raw/         ← B 的并发测试原始响应
│   └── logs/                           ← A 的构建日志 & 参数调优分解
├── command_logs/
│   ├── A_single_benchmark_commands.md  ← A 的真实命令记录
│   ├── B_rpc_server_commands.md        ← B 的真实命令记录
│   └── C_ray_commands.md               ← C 的真实命令记录
└── screenshots/
    ├── single_deploy/                  ← A 的单机部署截图（5 张）
    ├── rpc_deploy/                     ← 待补充
    ├── llama_server_concurrency/       ← 待补充
    ├── ray_task/                       ← 待补充
    ├── ray_load_balance/               ← 待补充
    └── ray_failure_retry/              ← 待补充
```

---

## 4. 依赖环境

### 4.1 Windows（Role A / B — llama.cpp 编译与推理）

- **OS**: Windows 11（或 Windows 10）
- **编译器**: MSYS2 UCRT64 GCC 14.1.0+ 或 MinGW GCC
- **CMake**: ≥ 3.20（可通过 `pip install cmake` 安装）
- **Python**: ≥ 3.9（Anaconda 或系统安装均可）
- **Python 包**: 标准库即可运行 `bench_single.py`；质量评估需 `requests`
- **Git Bash**: 用于运行 shell 脚本

### 4.2 Linux（Role C — Ray 实验）

- **OS**: Ubuntu 24.04（或其他 Linux 发行版）
- **Python**: ≥ 3.10
- **关键包**:
  ```bash
  pip install ray requests pandas
  ```

### 4.3 llama.cpp

- 源码：<https://github.com/ggml-org/llama.cpp>
- 本项目使用的 commit：`60130d18f9ac7f42cb4d7f6060b088a45d8f242e`（A）
- B 使用的 build：`b9502-6ddc9430b`
- 编译选项：
  - **Windows CPU（A）**: `-DCMAKE_C_FLAGS='-D_WIN32_WINNT=0x0A00' -DCMAKE_CXX_FLAGS='-D_WIN32_WINNT=0x0A00' -DLLAMA_BUILD_UI=OFF`
  - **Windows CPU + RPC（B）**: `-DGGML_RPC=ON -DGGML_CUDA=OFF -DGGML_VULKAN=OFF`
  - **Linux CPU（C）**: 标准 CMake 编译，无需特殊选项

---

## 5. 模型准备

本项目统一使用以下模型：

| 项目 | 值 |
|---|---|
| 模型 | Qwen2.5-0.5B-Instruct |
| 量化格式 | Q4_K_M（4-bit） |
| 文件大小 | ~469 MB |
| 文件名 | `qwen2.5-0.5b-instruct-q4_k_m.gguf` |

**下载地址**：
```
https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf
```

**放置位置**：将下载的 `.gguf` 文件放到 `Lab4/models/` 目录下。

```bash
# Windows PowerShell
curl.exe -L --fail --retry 3 \
  -o 'Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf' \
  'https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf?download=true'

# Linux
wget -O Lab4/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  'https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf'
```

> **注意**：模型文件已被 `.gitignore` 排除，不会提交到 Git。

---

## 6. 单机部署复现

> 详细文档：[`docs/deploy_llama_single.md`](docs/deploy_llama_single.md)
> 命令日志：[`command_logs/A_single_benchmark_commands.md`](command_logs/A_single_benchmark_commands.md)

### 6.1 编译 llama.cpp（Windows + MSYS2 UCRT64）

```powershell
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
cmake -S llama.cpp -B llama.cpp/build-ucrt-win10 -G 'MinGW Makefiles' `
  -DCMAKE_C_COMPILER='C:\msys64\ucrt64\bin\gcc.exe' `
  -DCMAKE_CXX_COMPILER='C:\msys64\ucrt64\bin\g++.exe' `
  -DCMAKE_C_FLAGS='-D_WIN32_WINNT=0x0A00' `
  -DCMAKE_CXX_FLAGS='-D_WIN32_WINNT=0x0A00' `
  -DLLAMA_BUILD_UI=OFF

cmake --build llama.cpp/build-ucrt-win10 --config Release -j 8
```

> **注意**：旧的 MinGW 8.1 无法编译 `cpp-httplib`；缺少 `-D_WIN32_WINNT=0x0A00` 也会导致编译失败。
> 详见 A 的命令日志中记录的两次失败尝试。

编译成功后，`llama-cli.exe` 位于 `llama.cpp/build-ucrt-win10/bin/`。

### 6.2 单次推理验证

```powershell
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
$model = 'Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf'
$cli = 'Lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe'

# 准备 UTF-8 prompt 文件（Windows 下推荐 -f 而非 -p）
[System.IO.File]::WriteAllText(
  'Lab4\results\single_prompt.txt',
  '请用三句话解释什么是虚拟内存，并说明页表和 TLB 的关系。',
  [System.Text.UTF8Encoding]::new($false)
)

& $cli -m $model -f Lab4\results\single_prompt.txt `
  -n 128 --threads 4 --ctx-size 2048 --batch-size 256 --single-turn --simple-io
```

预期：输出保存到 `results/single_inference_output.txt`。

---

## 7. 性能测试复现

> 详细文档：[`docs/performance_metrics.md`](docs/performance_metrics.md)、[`docs/performance_analysis.md`](docs/performance_analysis.md)

### 7.1 Baseline Benchmark

```powershell
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
python Lab4\scripts\bench_single.py `
  --model-path Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf `
  --llama-cli Lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe `
  --prompts Lab4\prompts\role_a_benchmark_prompts.jsonl `
  --output Lab4\results\single_benchmark.csv `
  --n-predict 128 --threads 4 --ctx-size 2048 --batch-size 256 `
  --repeat 3 --timeout 300
```

输出：`results/single_benchmark.csv`（15 行，5 prompt × 3 次重复）。

### 7.2 参数优化

Benchmark 脚本支持传入 `--threads`、`--ctx-size`、`--batch-size`、`--no-mmap` 参数。
按照 [`command_logs/A_single_benchmark_commands.md`](command_logs/A_single_benchmark_commands.md) 第 7 节中记录的参数组合运行，结果汇总为 `results/param_tuning.csv`（24 行）。

A 的推荐配置：`--threads 4 --ctx-size 1024 --batch-size 256`。

### 7.3 实测指标

| 指标 | 字段 | 测量方式 |
|---|---|---|
| 总生成延迟 | `total_latency_s` | 子进程启动到退出的 wall-clock 时间 |
| 输出速度 | `tokens_per_second` | 解析 llama.cpp 输出中的 `Generation: x t/s` |
| 输出长度 | `output_chars` | 统计生成文本字符数 |
| 成功率 | `success` | 进程正常退出且输出非空 |
| CPU 线程影响 | `threads` 1/2/4/8 | 对比不同线程数的延迟和 tokens/s |

> **内存占用说明**：`max_rss_kb` 在 Windows 环境未稳定采集。CSV 列保留为空值，
> 汇总文档标注 `not measured on Windows`。Linux 用户可用 `/usr/bin/time -v` 采集。

---

## 8. 输出质量评估复现

> 详细文档：[`docs/quality_eval.md`](docs/quality_eval.md)
> 命令日志：[`command_logs/B_rpc_server_commands.md`](command_logs/B_rpc_server_commands.md)

### 8.1 脚本修复说明

B 在运行质量评估时发现并修复了 `scripts/run_quality_eval.py` 的两个问题：
- Windows 下 `-p` 传中文 prompt 存在编码问题 → 改为 `-f` + UTF-8 临时文件
- llama-cli 对 chat model 默认进入交互模式 → 添加 `--single-turn --simple-io`

### 8.2 运行 configA（较高随机性）

```bash
cd "C:/Code/rust/Lab4"
python scripts/run_quality_eval.py \
  --config-name configA \
  --threads 2 --temp 0.7 --top-p 0.9 \
  --ctx-size 2048 --n-predict 256
```

### 8.3 运行 configB（较低随机性）

```bash
cd "C:/Code/rust/Lab4"
python scripts/run_quality_eval.py \
  --config-name configB \
  --threads 2 --temp 0.2 --top-p 0.8 \
  --ctx-size 2048 --n-predict 256
```

输出：`results/quality_eval.csv`（10 行，5 prompt × 2 config）。
原始输出：`results/raw_quality_outputs/configA_P*.txt`、`configB_P*.txt`。

### 8.4 评分维度

| 维度 | 字段 | 范围 |
|---|---|---|
| 正确性 | `correctness_score` | 1–5 |
| 完整性 | `completeness_score` | 1–5 |
| 清晰度 | `clarity_score` | 1–5 |
| 幻觉程度 | `hallucination_level` | 低/中/高 |

---

## 9. RPC 分布式推理复现

> 详细文档：[`docs/rpc_deploy.md`](docs/rpc_deploy.md)
> 命令日志：[`command_logs/B_rpc_server_commands.md`](command_logs/B_rpc_server_commands.md)

### 9.1 机器拓扑

| 角色 | 主机名 | IP |
|---|---|---|
| Host（Master） | LAPTOP-CNRQSONN | 192.168.137.1 |
| Worker（Slave） | ljyUSTC | 192.168.137.70 |

通信通过 Host 创建的移动热点，RPC 端口 50052。

### 9.2 编译 llama.cpp（启用 RPC）

```bash
cd llama.cpp/build
export PATH="/c/Program Files/MinGW/mingw64/bin:$PATH"
cmake .. -DGGML_RPC=ON -DGGML_CUDA=OFF -DGGML_VULKAN=OFF -G "MinGW Makefiles"
mingw32-make -j4 llama-cli
```

### 9.3 启动 rpc-server（Worker 端）

在 Worker（ljyUSTC，192.168.137.70）上：

```bash
./llama.cpp/build/bin/rpc-server -p 50052
```

### 9.4 RPC 推理（Host 端）

```bash
export PATH="/c/Program Files/MinGW/mingw64/bin:$PATH"
./llama.cpp/build/bin/llama-cli.exe \
  -m ./models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  -f ./results/rpc_test_prompt.txt \
  --rpc 192.168.137.70:50052 \
  -n 128 --threads 2 --ctx-size 2048 --single-turn --simple-io
```

### 9.5 单机 vs RPC 对比

对比测试使用相同 prompt 和参数，分别在单机模式和 RPC 模式下各跑 3 次。

结果：`results/single_vs_rpc.csv`（12 行）、`results/single_vs_rpc_summary.md`。

关键发现：
- RPC 生成速度约为单机的 **1/5–1/6**（网络延迟 + 从机 CPU 性能差异）
- RPC 生成速度波动更大（受热点网络质量影响）
- Prompt 处理速度单机/RPC 接近（均在本地完成）

---

## 10. llama-server 并发测试复现

> 详细文档：[`docs/llama_server_concurrency.md`](docs/llama_server_concurrency.md)
> 命令日志：[`command_logs/B_rpc_server_commands.md`](command_logs/B_rpc_server_commands.md)

### 10.1 启动 llama-server

在独立终端中：

```bash
Lab4/llama.cpp/build/bin/llama-server.exe \
  -m Lab4/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 127.0.0.1 --port 8080 \
  --threads 2 --ctx-size 2048
```

验证服务正常：

```bash
curl http://127.0.0.1:8080/health
# 预期输出: {"status": "ok"}
```

### 10.2 运行并发测试

```bash
pip install requests  # 如尚未安装
python scripts/test_llama_server_concurrency.py \
  --server-url http://127.0.0.1:8080 \
  --prompts prompts/quality_prompts.jsonl \
  --output results/server_concurrency.csv \
  --concurrency-levels 1,2,4 \
  --requests-per-level 10 \
  --timeout 120 --n-predict 128
```

输出：`results/server_concurrency.csv`（30 行，3 档并发度 × 10 请求）。
原始响应：`results/server_concurrency_raw/`。

### 10.3 结果摘要

| 并发度 | 成功 | 平均延迟 | P95 延迟 | 吞吐 |
|---|---|---|---|---|
| 1 | 10/10 | 4.108s | 7.686s | 0.243 req/s |
| 2 | 10/10 | 4.560s | 4.727s | 0.438 req/s |
| 4 | 10/10 | 6.851s | 7.872s | 0.511 req/s |

> **线程限制说明**：B 的机器 CPU 散热有限，`--threads` 必须设为 2。
> 更高线程数会导致过载关机。这使并发场景下的排队效应更加明显。

---

## 11. Ray 批量推理复现

> 详细文档：[`docs/ray_task.md`](docs/ray_task.md)
> 命令日志：[`command_logs/C_ray_commands.md`](command_logs/C_ray_commands.md)

### 11.1 环境准备（Linux）

```bash
pip install ray requests pandas

# 启动 2 个 llama-server 实例（单机多进程模拟多节点）
./llama-server -m Lab4/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 0.0.0.0 --port 8080 --ctx-size 2048 --batch-size 256 \
  --threads 4 --n-gpu-layers 0 &

./llama-server -m Lab4/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 0.0.0.0 --port 8081 --ctx-size 2048 --batch-size 256 \
  --threads 4 --n-gpu-layers 0 &

# 验证服务
curl -s http://127.0.0.1:8080/health
curl -s http://127.0.0.1:8081/health
```

> **重要说明**：由于只有一台物理 Linux 机器可用，Ray 实验使用**单机多进程模拟**多节点部署。
> 两个 llama-server 运行在同一个 localhost 的不同端口（8080、8081）。
> Ray 使用 `ray.init()` local 模式（无需 `ray start`）。
> 这测试的是 Ray 调度逻辑和 Python 编排代码；不测试真实的跨节点网络延迟。
> 已在 `docs/ray_task.md` 和 `configs/server_ports.md` 中明确说明。

### 11.2 串行执行（baseline）

```bash
python3 Lab4/scripts/ray_batch_infer.py \
  --prompts Lab4/prompts/ray_prompts_20.jsonl \
  --server-urls http://127.0.0.1:8080 \
  --strategy serial \
  --output Lab4/results/ray_serial.csv \
  --timeout 120
```

### 11.3 Ray Round-Robin

```bash
python3 Lab4/scripts/ray_batch_infer.py \
  --prompts Lab4/prompts/ray_prompts_20.jsonl \
  --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
  --strategy ray_round_robin \
  --output Lab4/results/ray_round_robin.csv \
  --timeout 120
```

### 11.4 Ray Parallel（并发）

```bash
python3 Lab4/scripts/ray_batch_infer.py \
  --prompts Lab4/prompts/ray_prompts_20.jsonl \
  --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
  --strategy ray_parallel \
  --output Lab4/results/ray_parallel.csv \
  --timeout 120 --max-concurrency 8
```

### 11.5 三种策略对比

| 策略 | 说明 | 单请求延迟 | 批量吞吐 |
|---|---|---|---|
| serial | 顺序调用单个 server | 基准值 | 最低 |
| ray_round_robin | Ray Actor 轮询分发到 2 个 server | 接近基准 | 约 2× serial |
| ray_parallel | Ray Task 并发调用，max_concurrency=8 | 因排队略高 | 最高 |

> **重要区分**：Ray 可以提高批量吞吐（通过并发），但**不能加速单条 prompt 的推理**。
> 单条请求延迟受限于 llama-server 的推理速度，Ray 调度本身还会引入少量调度开销。

---

## 12. Ray 负载均衡复现

> 命令日志：[`command_logs/C_ray_commands.md`](command_logs/C_ray_commands.md) 第 7 节

### 12.1 异构 Server 设置

```bash
# Server 8080 (fast, 4 threads)
./llama-server -m Lab4/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 0.0.0.0 --port 8080 --ctx-size 2048 --batch-size 256 \
  --threads 4 --n-gpu-layers 0 &

# Server 8081 (slow, 2 threads)
./llama-server -m Lab4/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 0.0.0.0 --port 8081 --ctx-size 2048 --batch-size 256 \
  --threads 2 --n-gpu-layers 0 &
```

### 12.2 Round-Robin 策略

```bash
python3 Lab4/scripts/ray_load_balance.py \
  --prompts Lab4/prompts/ray_prompts_30.jsonl \
  --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
  --strategy round_robin \
  --output Lab4/results/ray_load_balance_round_robin.csv \
  --summary-output Lab4/results/ray_load_balance_round_robin_summary.csv \
  --timeout 120
```

### 12.3 Latency-Aware 策略

```bash
python3 Lab4/scripts/ray_load_balance.py \
  --prompts Lab4/prompts/ray_prompts_30.jsonl \
  --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
  --strategy latency_aware \
  --output Lab4/results/ray_load_balance_latency_aware.csv \
  --summary-output Lab4/results/ray_load_balance_latency_aware_summary.csv \
  --timeout 120
```

### 12.4 结果对比

| 策略 | worker_0 (fast) | worker_1 (slow) | 说明 |
|---|---|---|---|
| round_robin | 15 req, avg 8.33s | 15 req, avg 11.29s | 均匀分配，慢 worker 拖累整体 |
| latency_aware | 28 req, avg 5.62s | 2 req, avg 8.98s | 自动将流量导向快 worker |

> **结论**：Latency-aware 策略有效识别并避开了慢速 server，将 93% 的请求路由到快速 server。

---

## 13. Ray 失败重试复现

> 命令日志：[`command_logs/C_ray_commands.md`](command_logs/C_ray_commands.md) 第 8 节

### 13.1 Server 设置（两个相同配置）

```bash
# Server A (8080)
./llama-server -m Lab4/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 0.0.0.0 --port 8080 --ctx-size 2048 --batch-size 256 \
  --threads 4 --n-gpu-layers 0 &

# Server B (8081)
./llama-server -m Lab4/models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 0.0.0.0 --port 8081 --ctx-size 2048 --batch-size 256 \
  --threads 4 --n-gpu-layers 0 &
```

### 13.2 运行失败重试测试

```bash
python3 Lab4/scripts/ray_failure_retry.py \
  --prompts Lab4/prompts/ray_prompts_30.jsonl \
  --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
  --output Lab4/results/ray_failure_retry.csv \
  --log Lab4/results/ray_failure_retry.log \
  --timeout 60 --max-retries 2
```

### 13.3 注入失败

在脚本运行约 35 秒后（约 6–7 个 prompt 完成），手动 kill Server A：

```bash
kill -9 $(ss -tlnp | grep 8080 | grep -oP 'pid=\K[0-9]+')
```

### 13.4 结果

| 指标 | 值 |
|---|---|
| 总请求 | 30 |
| 首次成功 | 18 |
| 重试成功 | 12 |
| 最终失败 | 0 |
| **最终成功率** | **100%** |
| 总 wall time | 143.9s |
| 失败类型 | 全部为 `connection_refused` |
| 重试目标 | 全部为 worker_1 (8081) |

---

## 14. 结果文件说明

### 14.1 核心 CSV 文件

| 文件 | 行数 | 说明 | 角色 |
|---|---|---|---|
| `results/single_benchmark.csv` | 15 | 单机 baseline benchmark | A |
| `results/param_tuning.csv` | 24 | 参数调优结果 | A |
| `results/quality_eval.csv` | 10 | 输出质量评估（5 prompt × 2 config） | B |
| `results/server_concurrency.csv` | 30 | llama-server 并发测试（3 档 × 10 请求） | B |
| `results/single_vs_rpc.csv` | 12 | 单机 vs RPC 对比（2 prompt × 2 模式 × 3 重复） | B |
| `results/ray_serial.csv` | 30 | Ray 串行推理 | C |
| `results/ray_round_robin.csv` | 30 | Ray 轮询推理 | C |
| `results/ray_parallel.csv` | 30 | Ray 并行推理 | C |
| `results/ray_load_balance_round_robin.csv` | 30 | 负载均衡-轮询 | C |
| `results/ray_load_balance_latency_aware.csv` | 30 | 负载均衡-延迟感知 | C |
| `results/ray_failure_retry.csv` | 30 | 失败重试 | C |

### 14.2 汇总与原始输出

| 文件/目录 | 说明 |
|---|---|
| `results/*_summary.md` | CSV 分组汇总 |
| `results/raw_single_outputs/` | A 的单机推理原始输出 |
| `results/raw_quality_outputs/` | B 的质量评估原始输出 |
| `results/server_concurrency_raw/` | B 的并发测试原始 JSON 响应 |
| `results/p*_single_r*.txt` | B 的单机对比原始输出 |
| `results/p*_rpc_r*.txt` | B 的 RPC 对比原始输出 |
| `results/logs/` | A 的构建日志、参数调优分解数据 |

---

## 15. 截图位置说明

| 目录 | 内容 | 状态 |
|---|---|---|
| `screenshots/single_deploy/` | 编译成功、模型文件、单次推理、benchmark 运行、结果文件 | ✅ 5 张，见目录内 README |
| `screenshots/rpc_deploy/` | RPC server 启动、RPC 推理成功 | ❌ 待 B 补充 |
| `screenshots/llama_server_concurrency/` | server 启动、并发测试运行 | ❌ 待 B 补充 |
| `screenshots/ray_task/` | Ray 三种策略运行截图 | ❌ 待 C 补充 |
| `screenshots/ray_load_balance/` | 负载均衡实验截图 | ❌ 待 C 补充 |
| `screenshots/ray_failure_retry/` | 失败重试实验截图（含 kill server） | ❌ 待 C 补充 |

---

## 16. 注意事项

### 16.1 模型文件不提交

GGUF 模型文件（~469 MB）已被 `.gitignore` 排除。助教复现时需要自行下载模型放到 `Lab4/models/` 目录。

### 16.2 build 目录不提交

`llama.cpp/` 源码目录和所有 `build*/` 目录已被 `.gitignore` 排除。
助教需要自行 clone llama.cpp 并编译。推荐使用本项目记录的命令和 commit。

### 16.3 llama.cpp/ 子模块说明

当前仓库中 `llama.cpp/` 目录存在但已被 `.gitignore` 排除（不进入 Git 跟踪）。
这是为避免 clone 整个 llama.cpp 历史导致仓库过大。
如果助教需要精确复现，可使用 commit `60130d18f9ac7f42cb4d7f6060b088a45d8f242e`。

### 16.4 单机多进程模拟说明（重要）

Ray 实验（Role C）的 2 个 llama-server 运行在**同一台 Linux 机器**的不同端口（8080、8081），
模拟多节点部署。Ray 使用 `ray.init()` local 模式。

- ✅ **测试了**：Ray 调度逻辑、Actor/Task 分发、轮询/并行策略、负载均衡算法、失败重试机制
- ❌ **未测试**：真实跨节点网络延迟、异构硬件调度、集群扩缩容

这已在 `docs/ray_task.md` 和 `configs/server_ports.md` 中明确说明。

### 16.5 三台不同机器说明（重要）

A、B、C 使用了三台不同的物理机器：

| 角色 | 机器 | CPU |
|---|---|---|
| A | LAPTOP-G44Q460K | i9-14900HX |
| B | LAPTOP-CNRQSONN | Ultra 7 255HX |
| C | ljyUSTC | i7-10700 |

因此 **A 的 baseline 数字不能直接与 B/C 的结果按绝对值对比**。
所有对比（单机 vs RPC、单机 vs Ray）均已使用同一机器的测量值。

### 16.6 数据记录真实

所有失败尝试（编译失败、编码问题、脚本 bug 修复）均记录在命令日志中，
作为问题分析依据。所有 CSV 数据均为真实运行结果。

### 16.7 已知问题

1. **`max_rss_kb` 为空**：A 的 Windows 环境无法采集，已在 CSV 和文档中标注。
2. **线程数限制**：B 的机器因 CPU 散热限制必须使用 `--threads 2`，会影响绝对性能数字。
3. **RPC 网络不稳定**：B 的 RPC 实验通过移动热点连接，延迟方差大（1ms–538ms）。

### 16.8 复现路径建议

推荐按以下顺序复现：

1. **准备环境**：下载模型 → 编译 llama.cpp（CPU + RPC）
2. **单机验证**：`docs/deploy_llama_single.md` → 单次推理
3. **性能测试**：`docs/performance_metrics.md` → benchmark → 参数调优
4. **质量评估**：`docs/quality_eval.md` → 2 组配置
5. **服务化**：`docs/llama_server_concurrency.md` → 并发测试
6. **RPC**：`docs/rpc_deploy.md` → rpc-server → RPC 推理 → 对比
7. **Ray**：`docs/ray_task.md` → serial → round_robin → parallel
8. **选做**：负载均衡 → 失败重试

---

## 参考

- [OSH 2026 Lab4 实验说明](https://osh-2026.github.io/lab4/)
- [llama.cpp GitHub](https://github.com/ggml-org/llama.cpp)
- [Ray Documentation](https://docs.ray.io/)
- [Qwen2.5-0.5B-Instruct-GGUF](https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF)
