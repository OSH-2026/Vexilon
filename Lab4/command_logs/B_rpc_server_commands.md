# Role B Command Log

All commands below reflect actual execution for member B's quality evaluation tasks.

## Project Environment

```text
Working directory: C:\Code\rust\Lab4
OS: Windows 11 Home China (10.0.26200)
Shell: Git Bash (MINGW64)
Python: Anaconda
MinGW: C:/Program Files/MinGW/mingw64/bin
```

---

## 质量评估

### 1. 检查 prompt 文件

```bash
cat prompts/quality_prompts.jsonl
```

结果：已存在，包含 5 条 prompt（P1-P5），覆盖中文问答、摘要、代码解释、推理题、课程相关全部类别。

### 2. 修复 run_quality_eval.py 脚本

脚本已存在但存在两个问题：
- `--threads` 默认值为 4，需改为 2（CPU 散热限制）
- 使用 `-p` 传参导致 Windows 命令行中文编码问题，改为 `-f` + 临时文件
- 缺少 `--single-turn` 导致 llama-cli 进入交互模式后挂起

修复内容：
1. 将 `default=4` 改为 `default=2`
2. 将 `-p prompt_text` 改为 `-f prompt_file`（避免编码问题）
3. 添加 `--single-turn` 和 `--simple-io` 标志

### 3. 运行 configA（较高随机性）

```bash
cd "C:/Code/rust/Lab4" && python scripts/run_quality_eval.py --config-name configA --threads 2 --temp 0.7 --top-p 0.9 --ctx-size 2048 --n-predict 256
```

- 运行时间：2026-06-05 10:17
- 模型：qwen2.5-0.5b-instruct-q4_k_m.gguf (Q4_K_M)
- 5 个 prompt 全部成功，生成速度约 34-40 t/s
- 输出字符数：P1=1221, P2=1068, P3=1719, P4=1270, P5=1306
- 首次运行因编码和交互模式问题全部超时，修复脚本后重跑成功

### 4. 运行 configB（较低随机性）

```bash
cd "C:/Code/rust/Lab4" && python scripts/run_quality_eval.py --config-name configB --threads 2 --temp 0.2 --top-p 0.8 --ctx-size 2048 --n-predict 256
```

- 运行时间：2026-06-05 10:19
- 模型：qwen2.5-0.5b-instruct-q4_k_m.gguf (Q4_K_M)
- 5 个 prompt 全部成功，生成速度约 35-37 t/s
- 输出字符数：P1=1200, P2=1056, P3=1656, P4=1297, P5=1277

### 5. 参考试运行（验证脚本修复）

修复脚本后先进行了一次测试：

```bash
export PATH="/c/Program Files/MinGW/mingw64/bin:$PATH"
"C:/Code/rust/Lab4/llama.cpp/build/bin/llama-cli.exe" \
  -m "C:/Code/rust/Lab4/models/qwen2.5-0.5b-instruct-q4_k_m.gguf" \
  -f "C:/Code/rust/Lab4/results/raw_quality_outputs/_test_prompt.txt" \
  -n 100 --threads 2 --ctx-size 2048 --temp 0.7 --top-p 0.9 --repeat-penalty 1.1 \
  --single-turn --simple-io --no-display-prompt
```

确认 `--single-turn` + `-f` 文件方式可正常工作后，才正式运行两组配置。

### 6. 关键发现

- llama-cli 在有 chat template 的模型上默认启用 conversation 模式，必须使用 `--single-turn` 才能在非交互场景下正常退出
- Windows 下通过 `-p` 传入中文参数存在编码问题（UTF-8 vs ANSI），使用 `-f` + UTF-8 临时文件是可靠替代方案
- 0.5B 模型在摘要类任务上表现可接受，但在逻辑推理和跨概念关联任务上表现很差

### 7. 生成文件清单

| 文件 | 说明 |
|---|---|
| `scripts/run_quality_eval.py` | 质量评估脚本（已修复） |
| `prompts/quality_prompts.jsonl` | 5 条评估 prompt（覆盖全部类别） |
| `results/quality_eval.csv` | 10 行评估结果（5 prompt × 2 配置） |
| `results/raw_quality_outputs/configA_P*.txt` | configA 原始输出（5 个文件） |
| `results/raw_quality_outputs/configB_P*.txt` | configB 原始输出（5 个文件） |
| `docs/quality_eval.md` | 质量评估文档 |

---

---

## llama-server 并发测试

### 1. 安装依赖

```bash
pip install requests
```

（`requests` 未在基础 Anaconda 环境中预装）

### 2. 测试 server 连通性

```python
import requests
r = requests.post('http://127.0.0.1:8080/v1/chat/completions',
    json={'messages':[{'role':'user','content':'hello'}],'max_tokens':16})
print(r.status_code)
# 输出: 200
print(r.json()['choices'][0]['message']['content'])
# 输出: Hello! How can I assist you today?
```

健康检查：
```bash
curl http://127.0.0.1:8080/health
# 输出: {"status": "ok"}
```

### 3. Server 配置

| 参数 | 值 |
|---|---|
| 可执行文件 | `llama.cpp/build/bin/llama-server.exe` |
| 模型 | `models/qwen2.5-0.5b-instruct-q4_k_m.gguf` |
| 监听地址 | `127.0.0.1:8080` |
| threads | 2 |
| ctx-size | 2048 |
| 编译 | MinGW, CPU only, OpenMP OFF |

Server 启动命令（在独立终端中手动执行）：
```bash
Lab4\llama.cpp\build\bin\llama-server.exe \
  -m Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 127.0.0.1 \
  --port 8080 \
  --threads 2 \
  --ctx-size 2048
```

### 4. 运行并发测试

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

- 运行时间：2026-06-05 11:12-11:13（UTC+8: ~19:12-19:13）
- 测试并发度：1, 2, 4
- 每档 10 个请求，共 30 个请求
- 全部成功，0 失败

### 5. 测试结果摘要

| 并发度 | 成功 | 平均延迟 | P95 延迟 | 吞吐 |
|---|---|---|---|---|
| 1 | 10/10 | 4.108s | 7.686s | 0.243 req/s |
| 2 | 10/10 | 4.560s | 4.727s | 0.438 req/s |
| 4 | 10/10 | 6.851s | 7.872s | 0.511 req/s |

### 6. 生成文件清单

| 文件 | 说明 |
|---|---|
| `scripts/test_llama_server_concurrency.py` | 并发测试脚本 |
| `results/server_concurrency.csv` | 30 行测试结果 |
| `results/server_concurrency_raw/c*_r*.txt` | 30 个原始 JSON 响应文件 |
| `docs/llama_server_concurrency.md` | 并发测试文档 |

---

## RPC 分布式推理 & 单机对比测试

### 1. 环境修正

**本机 CPU 修正**：之前 `rpc_network_info.txt` 和 `llama_server_concurrency.md` 中主机 CPU 错误地写为 `i9-14900HX`。PowerShell 验证真实 CPU 为：

```
Intel(R) Core(TM) Ultra 7 255HX (20 cores / 20 logical processors)
```

已修正 `results/rpc_network_info.txt` 和 `docs/llama_server_concurrency.md`。

### 2. 重新编译 llama.cpp（启用 RPC）

当前 MinGW build 的 `GGML_RPC=OFF`，需要重新编译：

```bash
cd llama.cpp/build
cmake .. -DGGML_RPC=ON -DGGML_CUDA=OFF -DGGML_VULKAN=OFF -G "MinGW Makefiles"
mingw32-make -j4 llama-cli
```

编译成功后 `llama-cli.exe` 输出到 `llama.cpp/build/bin/`，`--rpc` 标志可用。

### 3. RPC 连通性验证

```bash
export PATH="/c/Program Files/MinGW/mingw64/bin:$PATH"
python -c "import socket; s=socket.socket(); s.settimeout(5); r=s.connect_ex(('192.168.137.70',50052)); print('OPEN' if r==0 else f'CLOSED({r})')"
```

结果：Port 50052 OPEN（从机 ljyUSTC 已启动 rpc-server）。

### 4. RPC 功能验证（单次）

```bash
cd "C:/Code/rust/Lab4"
export PATH="/c/Program Files/MinGW/mingw64/bin:$PATH"
printf "%s" "请用三句话解释什么是虚拟内存，并说明页表和 TLB 的关系。" > ./results/rpc_test_prompt.txt
./llama.cpp/build/bin/llama-cli.exe \
  -m ./models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  -f ./results/rpc_test_prompt.txt \
  --rpc 192.168.137.70:50052 \
  -n 128 --threads 2 --ctx-size 2048 --single-turn --simple-io
```

- 运行时间：2026-06-05 ~15:55
- Prompt: 80.2 t/s | Generation: 9.1 t/s
- 结果：成功，输出保存到 `results/rpc_success_output.txt`

### 5. 单机功能验证（单次）

```bash
./llama.cpp/build/bin/llama-cli.exe \
  -m ./models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  -f ./results/rpc_test_prompt.txt \
  -n 128 --threads 2 --ctx-size 2048 --single-turn --simple-io
```

- 运行时间：2026-06-05 ~15:58
- Prompt: 178.7 t/s | Generation: 45.0 t/s
- 结果：成功，输出保存到 `results/single_test_output.txt`

### 6. 单机 vs RPC 对比测试（12 次推理）

**测试配置**：
- `--threads 2`, `--ctx-size 2048`, `--n-predict 128`, `--single-turn`, `--simple-io`
- 2 prompts (P1 中文问答 92 chars, P2 摘要 1159 chars)
- 每种 prompt × 2 模式(单机/RPC) × 3 次重复 = 12 次
- 每次推理后 sleep 60s（防止 CPU 过热）

**单机命令模板**：
```bash
./llama.cpp/build/bin/llama-cli.exe \
  -m ./models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  -f ./results/{p}_prompt.txt \
  -n 128 --threads 2 --ctx-size 2048 --single-turn --simple-io
```

**RPC 命令模板**：
```bash
./llama.cpp/build/bin/llama-cli.exe \
  -m ./models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  -f ./results/{p}_prompt.txt \
  --rpc 192.168.137.70:50052 \
  -n 128 --threads 2 --ctx-size 2048 --single-turn --simple-io
```

### 7. 对比测试结果摘要

| Prompt | 模式 | Prompt t/s avg | Gen t/s avg | Gen t/s range |
|---|---|---|---|---|
| P1 (短) | Single | 215.8 | **44.6** | 44.3–44.7 |
| P1 (短) | RPC | 122.3 | **8.0** | 5.3–10.1 |
| P2 (长) | Single | 203.0 | **41.6** | 41.5–41.7 |
| P2 (长) | RPC | 218.2 | **7.8** | 6.8–9.1 |

- 全部 12 次成功，零失败
- RPC 生成速度约为单机的 1/5–1/6（网络延迟 + 从机性能）
- RPC 稳定性受热点网络波动影响（P1 RPC gen 范围 5.3–10.1 t/s）
- Prompt 处理速度单机/RPC 接近（均在本地完成）

### 8. 生成文件清单

| 文件 | 说明 |
|---|---|
| `results/single_vs_rpc.csv` | 12 行对比测试详细数据 |
| `results/single_vs_rpc_summary.md` | 对比分析文档 |
| `results/rpc_success_output.txt` | RPC 功能验证输出 |
| `results/single_test_output.txt` | 单机功能验证输出 |
| `results/p1_single_r1~3.txt` | P1 单机原始输出 ×3 |
| `results/p1_rpc_r1~3.txt` | P1 RPC 原始输出 ×3 |
| `results/p2_single_r1~3.txt` | P2 单机原始输出 ×3 |
| `results/p2_rpc_r1~3.txt` | P2 RPC 原始输出 ×3 |

---

*日志记录时间：2026-06-05*
