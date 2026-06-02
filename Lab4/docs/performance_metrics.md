# Performance Metrics for LLM Deployment

## 1. 模型加载时间

英文：load time

含义：从启动 llama-cli 到模型加载完成、开始推理前所花费的时间。

意义：反映模型文件读取、mmap、内存分配和初始化开销。对于频繁启动的服务场景很重要。

测量方法：解析 llama.cpp 输出中的 load time，或使用总启动时间近似。

---

## 2. 首 Token 延迟

英文：time to first token, TTFT

含义：从提交 prompt 到生成第一个 token 的时间。

意义：直接影响用户体感响应速度。

测量方法：如果 llama.cpp 输出可解析，则解析 prompt eval 与首 token 时间；否则在报告中说明无法稳定自动测量。

---

## 3. 总生成延迟

英文：total latency

含义：一次完整推理从开始到结束的总耗时。

意义：反映用户等待整个回答完成的时间。

测量方法：脚本在子进程开始前和结束后记录 wall-clock time。

---

## 4. 输出速度

英文：tokens per second

含义：模型每秒生成 token 的数量。

意义：是最核心的生成性能指标，直接反映推理速度。

测量方法：解析 llama.cpp 输出中的 `Generation: x t/s`、`tok/s` 或 `tokens per second`。

---

## 5. 内存占用

英文：peak RSS / RAM usage

含义：推理进程占用的最大物理内存。

意义：决定模型是否能在普通电脑上稳定运行，也能反映 ctx-size、batch-size 等参数带来的内存压力。

测量方法：优先使用 `/usr/bin/time -v` 的 Maximum resident set size；本次 Windows Git Bash 环境没有可用的 `/usr/bin/time -v`，CSV 中 `max_rss_kb` 留空。

---

## 6. CPU 利用率

英文：CPU utilization

含义：推理过程中 CPU 核心使用情况。

意义：用于分析 `--threads` 是否设置合理，是否出现线程不足或过度线程竞争。

测量方法：可通过 top、pidstat、psutil 或系统监控截图辅助记录；本次主要通过不同 `--threads` 的速度变化间接分析。

---

## 7. 吞吐量

英文：throughput

含义：批量任务中单位时间完成的请求数，或单位时间生成的总 token 数。

意义：对角色 B 的 llama-server 并发测试和角色 C 的 Ray 批量调度有参考价值。

测量方法：批量 prompt 总数除以总耗时，或总生成 token 数除以总耗时。

---

## 8. 成功率

英文：success rate

含义：测试中成功完成推理的请求占比。

意义：用于评估参数配置是否稳定，避免只看速度忽略失败。

测量方法：CSV 中 success=true 的数量除以总请求数量。
