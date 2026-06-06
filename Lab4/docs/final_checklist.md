# Lab4 Final Checklist

> 整合检查时间：2026-06-06
> 整合人：整合 Agent
> A/B/C 均已完成各自实验，本文档为提交前最终检查清单。

---

## 一、评分点覆盖总表

### llama.cpp 主线（80 分）

| # | 评分项 | 分值 | 状态 | 对应文件 |
|---|---|---|---|---|
| 1 | 性能指标列表，不少于 5 个 | 8 | ✅ 已完成 | `docs/performance_metrics.md`（定义了 8 个指标） |
| 2 | GGUF 量化模型单机部署并成功推理 | 12 | ✅ 已完成 | `docs/deploy_llama_single.md`、`results/single_inference_output.txt` |
| 3 | 测试任务 + 至少 3 个指标实测 | 15 | ✅ 已完成 | `results/single_benchmark.csv`（15 次推理，测量 latency_s / tokens_per_second / output_chars / success） |
| 4 | 参数优化 | 15 | ✅ 已完成 | `results/param_tuning.csv`（24 次推理，比较 threads / batch-size / ctx-size / no-mmap） |
| 5 | 5 个 prompt 输出质量评估 | 10 | ✅ 已完成 | `docs/quality_eval.md`、`results/quality_eval.csv`（5 prompt × 2 config = 10 次） |
| 6 | RPC 多机分布式推理 | 10 | ✅ 已完成 | `docs/rpc_deploy.md`、`results/rpc_success_output.txt` |
| 7 | 单机 vs RPC 性能对比 | 10 | ✅ 已完成 | `results/single_vs_rpc.csv`（12 次推理）、`results/single_vs_rpc_summary.md` |

**llama.cpp 主线预计得分：80 / 80**

### llama.cpp 选做（10 分）

| # | 评分项 | 分值 | 状态 | 对应文件 |
|---|---|---|---|---|
| 1 | llama-server 服务 + 轻量并发测试 | 10 | ✅ 已完成 | `docs/llama_server_concurrency.md`、`results/server_concurrency.csv`（30 请求，3 档并发度） |

**llama.cpp 选做预计得分：10 / 10**

### Ray 必做（20 分）

| # | 评分项 | 分值 | 状态 | 对应文件 |
|---|---|---|---|---|
| 1 | Ray 环境部署 | 3 | ✅ 已完成 | `command_logs/C_ray_commands.md`、`docs/ray_task.md` |
| 2 | 至少 2 个 llama.cpp server 或合理模拟方案 | 4 | ✅ 已完成 | `configs/server_ports.md`（单机多进程模拟，2 个 server on port 8080/8081） |
| 3 | 不少于 20 个 prompt | 3 | ✅ 已完成 | `prompts/ray_prompts_20.jsonl`（20 prompt）、`prompts/ray_prompts_30.jsonl`（30 prompt） |
| 4 | Ray Task/Actor 分发并记录请求信息 | 4 | ✅ 已完成 | `scripts/ray_batch_infer.py`、`results/ray_*.csv`（含 worker_id / start_time / end_time / latency_s / success / error_message） |
| 5 | 至少两种执行方式对比 | 4 | ✅ 已完成 | serial vs ray_round_robin vs ray_parallel 三种策略，见 `docs/ray_task.md` |
| 6 | 系统现象分析 | 2 | ✅ 已完成 | `docs/ray_task.md` 第 5–8 节 |

**Ray 必做预计得分：20 / 20**

### Ray 选做（10 分）

| # | 评分项 | 分值 | 状态 | 对应文件 |
|---|---|---|---|---|
| 1 | 负载均衡调度 | 5 | ✅ 已完成 | `scripts/ray_load_balance.py`、`results/ray_load_balance_round_robin*.csv`、`results/ray_load_balance_latency_aware*.csv` |
| 2 | 失败重试 | 5 | ✅ 已完成 | `scripts/ray_failure_retry.py`、`results/ray_failure_retry.csv`、`results/ray_failure_retry.log` |

**Ray 选做预计得分：10 / 10**

### 总分预计：80 + 10 + 20 + 10 = 120 / 120

---

## 二、文件结构检查

### 文档（docs/）

| 文件 | 状态 |
|---|---|
| `docs/deploy_llama_single.md` | ✅ 存在 |
| `docs/performance_metrics.md` | ✅ 存在 |
| `docs/performance_analysis.md` | ✅ 存在 |
| `docs/quality_eval.md` | ✅ 存在 |
| `docs/rpc_deploy.md` | ✅ 存在 |
| `docs/llama_server_concurrency.md` | ✅ 存在 |
| `docs/ray_task.md` | ✅ 存在 |
| `docs/final_checklist.md` | ✅ 本文件 |

### 脚本（scripts/）

| 文件 | 状态 |
|---|---|
| `scripts/bench_single.py` | ✅ 存在 |
| `scripts/run_quality_eval.py` | ✅ 存在 |
| `scripts/test_llama_server_concurrency.py` | ✅ 存在 |
| `scripts/ray_batch_infer.py` | ✅ 存在 |
| `scripts/ray_load_balance.py` | ✅ 存在 |
| `scripts/ray_failure_retry.py` | ✅ 存在 |

### Prompt 文件（prompts/）

| 文件 | 状态 |
|---|---|
| `prompts/quality_prompts.jsonl` | ✅ 存在（5 prompt） |
| `prompts/ray_prompts_20.jsonl` | ✅ 存在（20 prompt） |
| `prompts/ray_prompts_30.jsonl` | ✅ 存在（30 prompt） |

### 结果文件（results/）

| 文件 | 状态 |
|---|---|
| `results/single_benchmark.csv` | ✅ 15 行，真数据 |
| `results/param_tuning.csv` | ✅ 24 行，真数据 |
| `results/quality_eval.csv` | ✅ 10 行，真数据 |
| `results/server_concurrency.csv` | ✅ 30 行，真数据 |
| `results/single_vs_rpc.csv` | ✅ 12 行，真数据 |
| `results/ray_serial.csv` | ✅ 30 行，真数据 |
| `results/ray_round_robin.csv` | ✅ 30 行，真数据 |
| `results/ray_parallel.csv` | ✅ 30 行，真数据 |
| `results/ray_load_balance_round_robin.csv` | ✅ 30 行，真数据 |
| `results/ray_load_balance_round_robin_summary.csv` | ✅ 存在 |
| `results/ray_load_balance_latency_aware.csv` | ✅ 30 行，真数据 |
| `results/ray_load_balance_latency_aware_summary.csv` | ✅ 存在 |
| `results/ray_failure_retry.csv` | ✅ 30 行，真数据 |

### 命令记录（command_logs/）

| 文件 | 状态 |
|---|---|
| `command_logs/A_single_benchmark_commands.md` | ✅ 包含编译、推理、benchmark、参数调优命令 |
| `command_logs/B_rpc_server_commands.md` | ✅ 包含质量评估、server 启动、并发测试、RPC 编译/启动/测试命令 |
| `command_logs/C_ray_commands.md` | ✅ 包含 Ray 环境、各策略运行、负载均衡、失败重试命令 |

---

## 三、数据真实性检查

### CSV 检查结果

| CSV 文件 | 行数 | 空值 | TODO | 全同时间 | 字段缺失 | 结论 |
|---|---|---|---|---|---|---|
| `single_benchmark.csv` | 15 | max_rss_kb 为空 | 无 | 无 | 无 | ✅ 真实（A 已说明 Windows 无法采集 RSS） |
| `param_tuning.csv` | 24 | max_rss_kb 为空 | 无 | 无 | 无 | ✅ 真实 |
| `quality_eval.csv` | 10 | 无 | 无 | 无 | 无 | ✅ 真实（分数有区分度） |
| `server_concurrency.csv` | 30 | 无 | 无 | 无 | 无 | ✅ 真实（c4 可见批量行为） |
| `single_vs_rpc.csv` | 12 | 无 | 无 | 无 | 无 | ✅ 真实（RPC gen t/s 明显低于单机） |
| `ray_serial.csv` | 30 | 无 | 无 | 无 | 无 | ✅ 真实 |
| `ray_round_robin.csv` | 30 | 无 | 无 | 无 | 无 | ✅ 真实 |
| `ray_parallel.csv` | 30 | 无 | 无 | 无 | 无 | ✅ 真实（worker_0/1 时间重叠） |
| `ray_load_balance_round_robin.csv` | 30 | 无 | 无 | 无 | 无 | ✅ 真实（worker_1 明显更慢） |
| `ray_load_balance_latency_aware.csv` | 30 | 无 | 无 | 无 | 无 | ✅ 真实（快 worker 分到 28/30） |
| `ray_failure_retry.csv` | 30 | 无 | 无 | 无 | 无 | ✅ 真实（有 retry_count=1 的记录，含 original_worker → final_worker 切换） |

**结论：所有 CSV 数据真实可信，不存在伪造数据。**

### 潜在不一致

1. **Ray CSV 数据条数与命令记录不一致**：`command_logs/C_ray_commands.md` 第 4.1–4.3 节记录使用 `ray_prompts_20.jsonl`（20 prompt），但 `ray_serial.csv`、`ray_round_robin.csv`、`ray_parallel.csv` 均为 30 行数据（R001–R030）。推测实际运行时使用了 30-prompt 文件，或 20-prompt 的 CSV 被 30-prompt 的结果覆盖。**建议成员 C 确认并统一命令记录与 CSV 的对应关系。**

2. **A/B 机器不同**：Role A 机器为 `LAPTOP-G44Q460K`（i9-14900HX），Role B 机器为 `LAPTOP-CNRQSONN`（Ultra 7 255HX），Role C 机器为 `ljyUSTC`（Linux Ubuntu 24.04, i7-10700）。三台机器硬件不同，但均使用同一模型 `qwen2.5-0.5b-instruct-q4_k_m.gguf`。这导致 A 的 baseline 与 B/C 的结果不能直接按数字对比——已在各自文档中说明不同机器的差异。**提交时需在 README 中明确说明三台机器不同。**

3. **RPC 从机充当 C 的实验机**：`ljyUSTC` 既是 B 的 RPC worker，也是 C 的 Ray 实验机（运行 2 个 llama-server）。提交时需在 README 中说明这一复用关系。

---

## 四、命令记录完整性

| 应记录命令 | A | B | C | 状态 |
|---|---|---|---|---|
| llama.cpp 编译 | ✅ 含 3 次尝试 | ✅ RPC 重编译 | — | ✅ |
| 单机推理 | ✅ | ✅ | — | ✅ |
| benchmark | ✅ | — | — | ✅ |
| llama-server 启动 | — | ✅ | ✅ | ✅ |
| RPC 编译 | — | ✅ | — | ✅ |
| rpc-server 启动 | — | ✅ | — | ✅ |
| RPC 推理 | — | ✅ | — | ✅ |
| Ray head/worker 启动 | — | — | ✅（说明 ray.init local 模式） | ✅ |
| Ray 脚本运行 | — | — | ✅ | ✅ |
| 失败重试注入 | — | — | ✅（kill -9 操作） | ✅ |

---

## 五、截图检查

| 截图目录 | 状态 | 备注 |
|---|---|---|
| `screenshots/single_deploy/` | ✅ 5 张 PNG + README | 覆盖编译成功、模型文件、单次推理、benchmark 运行、结果文件 |
| `screenshots/rpc_deploy/` | ❌ 缺失 | **需要成员 B 补充**：RPC server 启动截图、RPC 推理成功截图 |
| `screenshots/llama_server_concurrency/` | ❌ 缺失 | **需要成员 B 补充**：server 启动截图、并发测试运行截图 |
| `screenshots/ray_task/` | ❌ 缺失 | **需要成员 C 补充**：Ray 脚本运行截图、不同策略对比截图 |
| `screenshots/ray_load_balance/` | ❌ 缺失 | **需要成员 C 补充**：负载均衡脚本运行截图 |
| `screenshots/ray_failure_retry/` | ❌ 缺失 | **需要成员 C 补充**：失败重试实验截图（含 kill server 操作） |

---

## 六、提交前必须人工确认

### 6.1 必须确认的截图（由对应用户补充）

| # | 截图内容 | 负责人 |
|---|---|---|
| 1 | RPC server（ljyUSTC 上）运行状态 | B |
| 2 | RPC 推理成功输出（terminal 截图） | B |
| 3 | llama-server 启动后的 /health 返回 | B |
| 4 | 并发测试脚本运行中的 terminal | B |
| 5 | Ray serial / round_robin / parallel 脚本运行截图 | C |
| 6 | 负载均衡实验运行截图 | C |
| 7 | 失败重试实验中 kill server 后的 retry 日志 | C |

### 6.2 必须确认的真实多机信息

| # | 确认项 | 负责人 |
|---|---|---|
| 1 | ljyUSTC (192.168.137.70) 的 rpc-server 确实在运行 | B |
| 2 | B 的 RPC 推理确实通过热点网络连接到 ljyUSTC | B |
| 3 | C 的 2 个 llama-server (8080/8081) 确实在同一台 Linux 机器上运行 | C |
| 4 | C 的 Ray 实验确实在 ljyUSTC 上本地运行（ray.init local mode） | C |

---

## 七、README.md 检查

| 应含内容 | 状态 |
|---|---|
| 项目简介 | 🔧 已写入（本次整合） |
| 为什么选择 Ray | ✅ `docs/ray_task.md` 第 2 节已有，README 引用 |
| 目录结构 | 🔧 已写入 |
| 依赖环境 | 🔧 已写入 |
| 模型准备 | 🔧 已写入 |
| 单机部署复现 | 🔧 已写入 |
| 性能测试复现 | 🔧 已写入 |
| 输出质量评估复现 | 🔧 已写入 |
| RPC 复现 | 🔧 已写入 |
| llama-server 并发测试复现 | 🔧 已写入 |
| Ray 批量推理复现 | 🔧 已写入 |
| Ray 负载均衡复现 | 🔧 已写入 |
| Ray 失败重试复现 | 🔧 已写入 |
| 结果文件说明 | 🔧 已写入 |
| 截图位置说明 | 🔧 已写入 |
| 注意事项（模型/build 不提交、模拟说明、不伪装成功） | 🔧 已写入 |

---

## 八、风险项

| # | 风险 | 严重度 | 建议 |
|---|---|---|---|
| 1 | Ray CSV 条目数（30）与命令记录（20 prompt）不一致 | 中 | 成员 C 确认并修正命令记录或重跑 20-prompt 版本 |
| 2 | RPC / Ray / concurrency 实验缺少截图 | 高 | B 和 C 尽快补充（助教可能扣分） |
| 3 | 三台机器硬件不同，A 的 baseline 不能直接作为 B/C 的性能参考值 | 低 | README 已说明各机器差异 |
| 4 | RPC 实验依赖移动热点网络，网络不稳定 | 低 | B 已在 `docs/rpc_deploy.md` 中说明 |
| 5 | C 使用单机多进程模拟多节点，需明确说明不是真正的分布式 | 低 | 已在 `docs/ray_task.md` 和 `configs/server_ports.md` 中说明 |
| 6 | `docs/performance_analysis.md` 中 CPU 仍写为 i9-14900HX（A 的机器），但 B 已更正自己文档中的 CPU 为 Ultra 7 255HX | 低 | A 的机器确实是 i9-14900HX，无需修改 |

---

## 九、最终建议提交文件清单

### 必须提交（Git 跟踪）

```
Lab4/
├── README.md
├── .gitignore
├── configs/
│   ├── experiment_config.example.json
│   ├── machine_info_template.md
│   ├── model_info.md
│   ├── role_a_paths.env.example
│   └── server_ports.md
├── docs/
│   ├── deploy_llama_single.md
│   ├── performance_metrics.md
│   ├── performance_analysis.md
│   ├── quality_eval.md
│   ├── rpc_deploy.md
│   ├── llama_server_concurrency.md
│   ├── ray_task.md
│   └── final_checklist.md
├── scripts/
│   ├── bench_single.py
│   ├── run_quality_eval.py
│   ├── test_llama_server_concurrency.py
│   ├── ray_batch_infer.py
│   ├── ray_load_balance.py
│   ├── ray_failure_retry.py
│   ├── env_collect.sh
│   ├── run_llama_cli.sh
│   └── summarize_csv.py
├── prompts/
│   ├── quality_prompts.jsonl
│   ├── ray_prompts_20.jsonl
│   ├── ray_prompts_30.jsonl
│   ├── role_a_benchmark_prompts.jsonl
│   └── role_a_tuning_prompts.jsonl
├── results/
│   ├── README.md
│   ├── env_info.txt
│   ├── single_benchmark.csv
│   ├── single_benchmark_summary.md
│   ├── param_tuning.csv
│   ├── param_tuning_summary.md
│   ├── quality_eval.csv
│   ├── server_concurrency.csv
│   ├── single_vs_rpc.csv
│   ├── single_vs_rpc_summary.md
│   ├── ray_serial.csv
│   ├── ray_round_robin.csv
│   ├── ray_parallel.csv
│   ├── ray_load_balance_round_robin.csv
│   ├── ray_load_balance_round_robin_summary.csv
│   ├── ray_load_balance_latency_aware.csv
│   ├── ray_load_balance_latency_aware_summary.csv
│   ├── ray_failure_retry.csv
│   ├── ray_failure_retry.log
│   ├── rpc_network_info.txt
│   ├── rpc_success_output.txt
│   ├── single_inference_output.txt
│   ├── single_test_output.txt
│   └── raw_*_outputs/  (所有原始输出目录)
├── command_logs/
│   ├── A_single_benchmark_commands.md
│   ├── B_rpc_server_commands.md
│   └── C_ray_commands.md
└── screenshots/
    ├── single_deploy/
    │   ├── 01_build_success.png
    │   ├── 02_model_file.png
    │   ├── 03_single_inference.png
    │   ├── 04_benchmark_running.png
    │   ├── 05_results_files.png
    │   └── README.md
    ├── rpc_deploy/          ← 待补充
    ├── llama_server_concurrency/  ← 待补充
    ├── ray_task/            ← 待补充
    ├── ray_load_balance/    ← 待补充
    └── ray_failure_retry/   ← 待补充
```

### 不提交（已被 .gitignore 排除）

- `llama.cpp/` 源码和 build 目录
- `models/` 目录及所有 `.gguf` 文件
- `__pycache__/` 和 `.pyc` 文件
- `build/`、`build-*/` 目录

---

## 十、整合修正记录

本次整合做了以下修改（如需回滚可参考 git log）：

1. 创建 `docs/final_checklist.md`（本文件）
2. 创建 `README.md`（主 README）
3. （可选）修正文档间交叉引用路径
