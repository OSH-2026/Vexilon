# llama-server 并发测试

## 1. 实验目标

本次实验旨在：
- 使用 llama-server 搭建本地推理服务，暴露 OpenAI 兼容 API
- 通过 Python 并发请求脚本测试不同并发度下的性能指标
- 分析单模型 CPU 推理服务在并发场景下的延迟变化和吞吐限制
- 为后续分布式部署（Ray）提供基线参考

## 2. Server 启动环境

### 2.1 硬件环境

| 项目 | 值 |
|---|---|
| CPU | Intel(R) Core(TM) Ultra 7 255HX（20 核 / 20 逻辑处理器） |
| 内存 | 16 GB |
| GPU | NVIDIA 独立显卡（CUDA 12.6） |
| OS | Windows 11 Home China（10.0.26200） |

### 2.2 软件环境

| 项目 | 值 |
|---|---|
| llama.cpp | build b9502-6ddc9430b |
| 编译方式 | MinGW（GCC），CPU only，无 GPU 加速，无 OpenMP |
| Python | Anaconda，3.9 |
| 关键依赖 | requests |

### 2.3 线程限制说明

**本机 CPU 散热有限，`--threads` 必须设为 2。** 更高线程数会导致 CPU 过载关机。这一硬性限制直接影响本次并发测试的结果——推理速度本身较慢（约 37 t/s @ 2 threads），因此并发场景下的排队效应更加明显。

## 3. 模型与量化格式

| 项目 | 值 |
|---|---|
| 模型名称 | Qwen2.5-0.5B-Instruct |
| 参数量 | 0.5B |
| 量化格式 | Q4_K_M（4-bit，中等质量） |
| 模型文件 | `models/qwen2.5-0.5b-instruct-q4_k_m.gguf` |
| 模型大小 | 约 469 MB |

## 4. Server 启动命令

llama-server 在独立的终端窗口中手动启动，命令如下：

```bash
Lab4\llama.cpp\build\bin\llama-server.exe \
  -m Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 127.0.0.1 \
  --port 8080 \
  --threads 2 \
  --ctx-size 2048
```

参数说明：

| 参数 | 值 | 说明 |
|---|---|---|
| `-m` | `models/qwen2.5-0.5b-instruct-q4_k_m.gguf` | 模型文件路径 |
| `--host` | `127.0.0.1` | 仅监听本地回环 |
| `--port` | `8080` | HTTP 服务端口 |
| `--threads` | `2` | CPU 推理线程数（散热限制） |
| `--ctx-size` | `2048` | 上下文窗口大小 |

## 5. 请求接口说明

使用 llama-server 提供的 OpenAI 兼容接口：

- **Endpoint：** `POST /v1/chat/completions`
- **请求体格式：**
  ```json
  {
    "messages": [{"role": "user", "content": "PROMPT内容"}],
    "max_tokens": 128
  }
  ```
- **响应格式：** 标准 OpenAI chat completion 格式，包含 `choices[0].message.content` 和 `usage` 字段
- **健康检查：** `GET /health` 返回 200 OK

接口测试结果：`/v1/chat/completions` 正常工作，返回标准 JSON 响应，包含 token 用量统计（`usage.completion_tokens`、`usage.prompt_tokens`、`usage.total_tokens`）以及推理耗时（`timings.predicted_ms`、`timings.prompt_ms`）。

## 6. 并发测试脚本说明

### 6.1 脚本路径

`scripts/test_llama_server_concurrency.py`

### 6.2 工作原理

1. 从 JSONL 文件加载 prompt 列表
2. 对每个并发度（1, 2, 4）依次测试：
   - 使用 `concurrent.futures.ThreadPoolExecutor(max_workers=并发度)` 创建线程池
   - 同时提交 N 个请求（N = `--requests-per-level`），prompt 循环使用
   - 记录每个请求的开始时间、结束时间、延迟、成功/失败状态、HTTP 状态码、输出字符数
   - 保存每个请求的原始 JSON 响应到 `results/server_concurrency_raw/`
3. 每个并发度测试完成后，计算并打印汇总指标：
   - 成功率、失败数
   - 平均延迟（avg_latency_s）
   - 第 95 百分位延迟（P95_latency_s）
   - 吞吐量（throughput_req_per_s = 成功请求数 / 总测试时间）

### 6.3 命令行参数

| 参数 | 默认值 | 说明 |
|---|---|---|
| `--server-url` | `http://127.0.0.1:8080` | llama-server 地址 |
| `--prompts` | `prompts/quality_prompts.jsonl` | Prompt 文件路径 |
| `--output` | `results/server_concurrency.csv` | 输出 CSV 路径 |
| `--concurrency-levels` | `1,2,4` | 测试的并发度（逗号分隔） |
| `--requests-per-level` | `10` | 每个并发度的请求数 |
| `--timeout` | `120` | 单请求超时（秒） |
| `--n-predict` | `128` | 每个请求的 max_tokens |

## 7. 并发度设置

| 项目 | 值 |
|---|---|
| 测试并发度 | 1, 2, 4 |
| 每档请求数 | 10 |
| 总请求数 | 30 |
| Prompt 数量 | 5（循环使用，第 6 个请求复用 P1） |
| max_tokens | 128 |

## 8. 测试结果表

### 8.1 汇总指标

| 并发度 | 总请求 | 成功数 | 失败数 | 平均延迟 (s) | P95 延迟 (s) | 吞吐 (req/s) | 总测试时间 (s) |
|---|---|---|---|---|---|---|---|
| 1 | 10 | 10 | 0 | 4.108 | 7.686 | 0.243 | 41.08 |
| 2 | 10 | 10 | 0 | 4.560 | 4.727 | 0.438 | 22.82 |
| 4 | 10 | 10 | 0 | 6.851 | 7.872 | 0.511 | 19.58 |

### 8.2 各请求详细数据

详见 `results/server_concurrency.csv`（30 行数据），关键字段：

- 并发度 1：延迟范围 3.201s ~ 7.686s
- 并发度 2：延迟范围 4.480s ~ 4.727s（非常均匀）
- 并发度 4：延迟范围 3.596s ~ 7.872s

### 8.3 并发度 1 延迟详情

| 请求 | prompt_id | 延迟 (s) | output_chars |
|---|---|---|---|
| c1_r001 | P1 | 4.013 | 232 |
| c1_r002 | P2 | **7.686** | 133 |
| c1_r003 | P3 | 4.566 | 279 |
| c1_r004 | P4 | 4.264 | 206 |
| c1_r005 | P5 | 4.079 | 206 |
| c1_r006 | P1 | 3.201 | 215 |
| c1_r007 | P2 | 3.454 | 214 |
| c1_r008 | P3 | 3.292 | 261 |
| c1_r009 | P4 | 3.285 | 234 |
| c1_r010 | P5 | 3.237 | 229 |

> 注：c1_r002（P2 摘要 prompt）延迟显著高于其他（7.69s vs 平均 3.5s），原因是 P2 的 prompt 文本约 1200 字（含大段技术材料），prompt 处理阶段耗时远超其他 prompt。

## 9. 结果分析

### 9.1 延迟变化分析

**平均延迟随并发度升高而增加：**

| 并发度 | 平均延迟 | 相对并发度 1 的变化 |
|---|---|---|
| 1 | 4.108s | 基准 |
| 2 | 4.560s | +11.0% |
| 4 | 6.851s | +66.8% |

**原因分析：**
llama-server 是单模型服务，CPU 推理是串行的——同一时刻只能处理一个推理任务（执行矩阵乘法等计算）。当多个请求同时到达时，后续请求必须排队等待前一个请求完成。线程池中的并发请求会同时建立 HTTP 连接，但 server 端逐个处理，导致排在后面的请求等待时间 = 前面请求的处理时间之和。因此：
- 并发度 2：第 2 个请求等待第 1 个完成（约 +0.5s），平均延迟略微增加
- 并发度 4：第 4 个请求需要等待前 3 个完成（约 +2-3s），延迟显著增加

**P95 延迟分析：**

| 并发度 | P95 延迟 | 说明 |
|---|---|---|
| 1 | 7.686s | 受 P2 长 prompt 拖高 |
| 2 | 4.727s | 延迟更均匀（重叠处理消除了方差） |
| 4 | 7.872s | 队尾请求等待时间长 |

并发度 1 的 P95（7.686s）由 P2 长 prompt 引起。并发度 2 中由于两个请求几乎同时提交和排队，每个请求的实际等待时间趋于均匀——长 prompt 和短 prompt 配对提交，server 连续处理，消除了空转时间。并发度 4 的 P95 回归高位，因为更多请求排队导致队尾请求等待更久。

### 9.2 吞吐量变化分析

| 并发度 | 吞吐 (req/s) | 相对提升 |
|---|---|---|
| 1 | 0.243 | 1.00x（基准） |
| 2 | 0.438 | **1.80x** |
| 4 | 0.511 | **2.10x** |

**吞吐提升存在明显边际递减：**
- 并发度 1→2：吞吐提升 80%（几乎线性 scaling），因为 2 个并发请求减少了请求间的空闲时间（HTTP 建连、等待响应的网络往返等），server 基本连续工作
- 并发度 2→4：吞吐提升仅 16.7%（远低于线性 scaling），因为 server 的推理计算本身已经饱和，更多并发只会增加排队，无法进一步压榨 CPU

**理论最大吞吐：** 以并发度 1 中去除 P2 异常值后的平均延迟（约 3.5s）为基准，单请求纯推理时间约 3.5 秒，理论最大串行吞吐约 1/3.5 ≈ 0.286 req/s。并发度 4 的实际吞吐 0.511 req/s 已接近 2 线程下该模型的理论上限。

### 9.3 失败请求分析

**30 个请求全部成功，失败数 = 0。**

这说明在测试的并发范围内（1-4），llama-server 运行稳定，`--threads 2 --ctx-size 2048` 配置可靠。没有出现超时、OOM、连接拒绝等问题。

### 9.4 瓶颈分析

在本次测试环境中，瓶颈层级如下：

1. **CPU 推理计算（主瓶颈）：** 0.5B 模型在 2 线程下纯生成速度约 37 t/s，128 token 生成需要约 3.5 秒。这是不可并行的串行计算瓶颈。
2. **Prompt 处理时间（次要瓶颈）：** 长 prompt（如 P2 约 1200 字）的处理耗时约 4 秒（约 300 tokens × ~75 t/s prompt eval），显著拉长了端到端延迟。
3. **单模型串行化（架构限制）：** llama-server 使用单个模型实例，不支持 batch 推理（或 batch 支持有限），并发请求本质上是排队串行处理。
4. **线程数限制（硬件限制）：** `--threads 2` 是散热约束的结果，如果能用 4-8 线程，推理速度可提升至约 10-11 t/s（参见成员 A 的 param_tuning 数据），每请求延迟可降至约 2 秒。

### 9.5 关键发现

1. **并发度 2 是最优配置：** 吞吐提升 80%，延迟仅增加 11%，且 P95 延迟反而降低（方差更小）。
2. **并发度 4 收益递减：** 吞吐仅额外提升 17%，但平均延迟增加 67%，P95 延迟反弹至 7.9 秒。
3. **P2（长 prompt 摘要）是性能热点：** 请求 c1_r002 的延迟（7.69s）是平均值的近 2 倍，说明 prompt 长度对总延迟影响很大。
4. **server 稳定性良好：** 30 个请求零失败，说明 llama-server 在有限并发下可靠。

## 10. 局限性

1. **单机资源限制：** 本机 CPU 散热瓶颈导致只能用 2 线程推理，推理速度慢（37 t/s）。在更好的散热条件下用 4-8 线程，并发测试的绝对指标会显著改善，但相对关系应类似。
2. **Prompt 数量有限：** 仅 5 个 prompt 循环使用（10 个请求中每个 prompt 出现 2 次），不同 prompt 的长度差异（P2 ~1200 字 vs 其他 ~50-300 字）导致延迟方差较大。
3. **无真实网络延迟：** 测试全部通过 `127.0.0.1` 本地回环，无网络往返延迟。生产环境中网络延迟通常为 10-100ms。
4. **0.5B 模型推理时间较短：** 128 token 生成约 3.5 秒，可能不足以充分暴露更高并发度下的瓶颈。如果使用 7B 模型且单次推理耗时 30 秒以上，并发排队效应会更明显。
5. **未测试更高并发度：** 仅测试了 1、2、4 三种并发度。未测试 8、16 等更高并发度。基于当前的吞吐递减趋势，推测 8 并发度下吞吐可能反而下降（过多的排队和上下文切换开销）。
6. **单模型实例：** llama-server 仅加载一个模型实例，无法利用 CPU 多核进行真正的并行推理。这是架构层面的限制，而非本实验的特定问题。
7. **无 GPU 加速：** CPU only 编译，推理速度远低于 GPU 推理。如果有 CUDA 支持，并发表现会有本质不同。

## 11. 复现命令

### 11.1 启动 llama-server

在独立终端中执行：

```bash
Lab4\llama.cpp\build\bin\llama-server.exe \
  -m Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 127.0.0.1 \
  --port 8080 \
  --threads 2 \
  --ctx-size 2048
```

确认 server 启动：

```bash
curl http://127.0.0.1:8080/health
# 预期输出: {"status": "ok"}
```

### 11.2 测试连通性

```python
import requests
r = requests.post('http://127.0.0.1:8080/v1/chat/completions',
    json={'messages':[{'role':'user','content':'hello'}],'max_tokens':16})
print(r.status_code, r.json()['choices'][0]['message']['content'])
```

### 11.3 运行并发测试

```bash
python scripts/test_llama_server_concurrency.py \
  --server-url http://127.0.0.1:8080 \
  --prompts prompts/quality_prompts.jsonl \
  --output results/server_concurrency.csv \
  --concurrency-levels 1,2,4 \
  --requests-per-level 10 \
  --timeout 120 \
  --n-predict 128
```

### 11.4 查看结果

```bash
# 查看 CSV
cat results/server_concurrency.csv

# 查看原始响应文件
ls results/server_concurrency_raw/

# 查看某个原始响应
cat results/server_concurrency_raw/c1_r001.txt
```

---
