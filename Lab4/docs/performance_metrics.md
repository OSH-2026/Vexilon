# Performance Metrics for LLM Deployment

本文定义角色 A 单机部署和性能测试使用的指标。每个指标都说明测量依据，以及为什么选择该指标。未在本次 Windows 环境稳定采集到的指标不填估计值，只记录限制。

## 1. 模型加载时间

英文名：load time

定义：从启动 `llama-cli` 到模型加载完成、进入推理阶段前的耗时。

选择原因：加载时间决定冷启动成本。对于需要频繁启动进程、切换模型或做批处理任务的场景，模型文件读取、mmap、内存分配和初始化开销会直接影响任务总时间。

测量依据：优先解析 llama.cpp 输出中的 load time；如果当前输出没有稳定给出单独 load time，则只能用完整进程耗时作参考，不单独写入 CSV。

本次记录情况：本次 CSV 没有单独记录 load time。`single_inference_output.txt` 和命令日志保留了模型加载成功的原始输出。

## 2. 首 Token 延迟

英文名：time to first token, TTFT

定义：从提交 prompt 到生成第一个 token 的耗时。

选择原因：TTFT 直接对应交互式使用中的首次响应等待时间。即使最终 tokens/s 很高，如果 TTFT 很长，用户仍会感觉系统响应慢。

测量依据：需要解析 prompt eval 阶段结束和首 token 输出的时间点，或在调用层对输出流做时间戳记录。

本次记录情况：本次 `llama-cli --simple-io` 输出没有为首 token 提供稳定时间戳，脚本未单独记录 TTFT。报告中不使用推测值。

## 3. 总生成延迟

英文名：total latency

定义：一次完整推理从子进程启动到输出结束的 wall-clock 总耗时。

选择原因：总延迟是最稳定、最容易复现的端到端指标，可以直接比较不同 prompt 和不同参数配置下完成一次请求需要多久。

测量依据：`scripts/bench_single.py` 在启动 `llama-cli` 前记录开始时间，在进程退出后记录结束时间，二者差值写入 `total_latency_s`。

本次记录情况：已记录在 `results/single_benchmark.csv` 和 `results/param_tuning.csv`，汇总表使用平均值 `avg_latency_s`。

## 4. 输出速度

英文名：tokens per second

定义：模型生成阶段每秒输出 token 的数量。

选择原因：这是 llama.cpp 单机推理最核心的性能指标。它比总延迟更能反映生成阶段本身的速度，适合比较 `threads`、`batch-size`、`ctx-size` 等参数的影响。

测量依据：脚本解析 llama.cpp 输出中的 `Generation: x t/s`，写入 `tokens_per_second`。

本次记录情况：baseline 15 条记录和参数调优 24 条记录均成功解析 tokens/s。

## 5. 内存占用

英文名：peak RSS / RAM usage

定义：推理进程运行期间占用的最大物理内存。

选择原因：本实验使用本地 CPU 部署，内存是否足够决定模型能否稳定运行。`ctx-size`、`batch-size` 和 mmap 设置也会影响内存压力，因此 peak RSS 是判断配置可行性的关键指标。

测量依据：Linux 环境可用 `/usr/bin/time -v` 的 `Maximum resident set size`；Windows 环境可改用 PowerShell 进程监控或 psutil 采样。

本次记录情况：当前 Windows / Git Bash 环境没有可用的 `/usr/bin/time -v`，因此 `max_rss_kb` 原始列保留为空值，汇总文档标注为 `not measured on Windows`。没有填写估算内存。

## 6. CPU 线程影响

英文名：CPU thread scaling

定义：不同 `--threads` 设置下延迟和 tokens/s 的变化。

选择原因：角色 A 的主线是 CPU 单机部署。线程数通常是 CPU 推理最直接的调优参数，能够反映多核利用、线程调度开销和边际收益下降。

测量依据：在固定模型、prompt、`ctx-size` 和 `batch-size` 的前提下，对 `--threads 1/2/4/8` 做对比，观察 `avg_latency_s` 和 `avg_tokens_per_second`。

本次记录情况：参数调优 CSV 已包含 1、2、4、8 线程结果。4 线程到 8 线程提升很小，因此推荐配置保持 4 线程。

## 7. 吞吐量

英文名：throughput

定义：单位时间内完成的请求数，或单位时间内生成的总 token 数。

选择原因：角色 B 的服务化测试和角色 C 的 Ray 批量任务都需要吞吐量作为对比指标。它能描述系统处理多请求或批量任务的能力，但不能直接等同于单条请求延迟。

测量依据：批量测试中用总请求数除以总 wall-clock 时间，或用总生成 token 数除以总 wall-clock 时间。角色 A 当前使用单进程顺序推理，主要记录每次请求的 tokens/s 和成功率，给 B/C 作为单机 baseline。

本次记录情况：角色 A 没有做并发吞吐测试。B/C 若做服务端或 Ray 批量测试，应新增 `requests_per_second`、`total_requests`、`success_count`、`fail_count` 等字段。

## 8. 成功率

英文名：success rate

定义：测试中成功完成推理的请求数量占总请求数量的比例。

选择原因：只看速度可能掩盖崩溃、超时或输出失败。成功率用于判断某组参数是否稳定，是性能对比的前置条件。

测量依据：CSV 中 `success=true` 的数量除以总记录数。汇总脚本同时统计 `success` 和 `fail`。

本次记录情况：baseline 为 15/15 成功，参数调优为 24/24 成功。