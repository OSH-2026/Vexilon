# Single-machine vs RPC Distributed Inference Comparison

## 1. Experiment Overview

Compare llama.cpp inference performance between:
- **Single-machine (local)**: llama-cli running locally on host
- **RPC distributed**: llama-cli (RPC client) on host + rpc-server on worker via mobile hotspot

## 2. Environment

### Hardware

| Component | Host (LAPTOP-CNRQSONN) | Worker (ljyUSTC) |
|---|---|---|
| CPU | Intel(R) Core(TM) Ultra 7 255HX (20C/20T) | Intel(R) Core(TM) i7-10700 @ 2.90GHz |
| RAM | 16 GB | 32 GB |
| GPU | NVIDIA GeForce RTX 5060 Laptop | — |
| OS | Windows 11 Home China (10.0.26200) | — |

### Network

| Property | Value |
|---|---|
| Type | Mobile hotspot (host creates, worker connects) |
| Subnet | 192.168.137.0/24 |
| Host IP | 192.168.137.1 |
| Worker IP | 192.168.137.70 |
| RPC Port | 50052 |
| Latency | 1ms–538ms range, avg ~46ms (high variance) |

### Software

| Component | Value |
|---|---|
| llama.cpp build | b9502-6ddc9430b |
| Compiler | MinGW GCC 16.1.0 |
| Backend | CPU only (GGML_RPC=ON for RPC), no CUDA/Vulkan |
| Model | Qwen2.5-0.5B-Instruct Q4_K_M (~469 MB) |
| Threads | `--threads 2` (hard limit due to host thermal constraints) |

### Test Configuration

| Parameter | Value |
|---|---|
| `--threads` | 2 |
| `--ctx-size` | 2048 |
| `--n-predict` | 128 |
| `--single-turn` | enabled |
| `--simple-io` | enabled |
| Repeats per prompt/mode | 3 |
| Cooldown between runs | 60 seconds |

## 3. Results

### 3.1 P1: Short QA prompt (92 chars, ~30 tokens)

| Mode | Run | Prompt t/s | Generation t/s |
|---|---|---|---|
| Single | 1 | 218.6 | 44.7 |
| Single | 2 | 218.1 | 44.3 |
| Single | 3 | 210.7 | 44.7 |
| **Single avg** | | **215.8** | **44.6** |
| RPC | 1 | 133.1 | 10.1 |
| RPC | 2 | 97.3 | 5.3 |
| RPC | 3 | 136.6 | 8.7 |
| **RPC avg** | | **122.3** | **8.0** |

**Single/RPC ratio**: Prompt 1.8×, Generation **5.5×**

### 3.2 P2: Long summary prompt (1159 chars, ~300 tokens)

| Mode | Run | Prompt t/s | Generation t/s |
|---|---|---|---|
| Single | 1 | 197.7 | 41.6 |
| Single | 2 | 202.6 | 41.7 |
| Single | 3 | 208.7 | 41.5 |
| **Single avg** | | **203.0** | **41.6** |
| RPC | 1 | 205.0 | 7.6 |
| RPC | 2 | 207.0 | 6.8 |
| RPC | 3 | 242.5 | 9.1 |
| **RPC avg** | | **218.2** | **7.8** |

**Single/RPC ratio**: Prompt 0.9×, Generation **5.3×**

### 3.3 Overall Averages

| Metric | Single | RPC | Ratio |
|---|---|---|---|
| Prompt processing (short prompt) | 215.8 t/s | 122.3 t/s | 1.8× |
| Prompt processing (long prompt) | 203.0 t/s | 218.2 t/s | 0.9× |
| Token generation (short prompt) | 44.6 t/s | 8.0 t/s | **5.5×** |
| Token generation (long prompt) | 41.6 t/s | 7.8 t/s | **5.3×** |

## 4. Analysis

### 4.1 Prompt Processing

Prompt processing stays mostly local even in RPC mode (model loading, tokenization happen client-side). This explains why prompt t/s is similar between single and RPC — the bottleneck here is the local CPU, not the network.

### 4.2 Token Generation — The RPC Penalty

Token generation is **5–6× slower in RPC mode**. This is caused by:

1. **Network round-trip latency**: Each token generation step requires communication between host and worker. On the mobile hotspot, latency ranges from 1ms to 538ms with high variance.
2. **Worker CPU performance**: The worker's i7-10700 (2020 desktop CPU, 8C/16T) has different per-core performance characteristics compared to the host's Ultra 7 255HX (2024 mobile CPU, 20C/20T).
3. **No computation offload benefit**: Since both machines use only 2 threads, the RPC setup doesn't add computational parallelism — it only adds network overhead.

### 4.3 RPC Stability

RPC generation speed shows higher variance:
- P1: 5.3–10.1 t/s (range: 4.8)
- P2: 6.8–9.1 t/s (range: 2.3)

This variance is consistent with the hotspot network's latency instability documented in `rpc_network_info.txt`.

### 4.4 Prompt Length Impact

- **Single**: Long prompt (P2, 1159 chars) reduces generation speed from 44.6 → 41.6 t/s (−6.7%)
- **RPC**: Prompt length has minimal impact on generation speed (8.0 vs 7.8 t/s), suggesting network latency dominates over compute differences

## 5. Key Findings

1. **RPC inference is viable but slow** over mobile hotspot — 5–6× slower than local for token generation
2. **Prompt processing is largely unaffected** by RPC (done locally)
3. **Network quality is the critical factor** — hotspot latency variance directly impacts RPC generation stability
4. **Zero failures** across all 12 comparison runs — RPC protocol is reliable even over unstable network
5. **`--threads 2`** kept host CPU temperatures safe throughout; no thermal shutdown occurred

## 6. Limitations

1. Mobile hotspot introduces significant and variable latency; wired LAN would likely show better RPC performance
2. Both machines limited to `--threads 2` due to host thermal constraints; higher thread counts could change the single/RPC ratio
3. Only 0.5B model tested; larger models with longer per-token compute would amortize network overhead better
4. Only 2 prompts tested in the comparison (short QA + long summary); more diverse prompts would provide fuller picture

## 7. File Manifest

| File | Description |
|---|---|
| `results/single_vs_rpc.csv` | All 12 comparison runs with metrics |
| `results/single_vs_rpc_summary.md` | This document |
| `results/rpc_network_info.txt` | Network topology and host/worker specs |
| `results/rpc_success_output.txt` | Initial RPC verification run (step 3) |
| `results/single_test_output.txt` | Initial single verification run (step 4) |
| `results/p1_single_r1~3.txt` | P1 single-machine raw outputs |
| `results/p1_rpc_r1~3.txt` | P1 RPC raw outputs |
| `results/p2_single_r1~3.txt` | P2 single-machine raw outputs |
| `results/p2_rpc_r1~3.txt` | P2 RPC raw outputs |
| `results/p1_prompt.txt` | P1 prompt file |
| `results/p2_prompt.txt` | P2 prompt file |
| `results/rpc_test_prompt.txt` | Verification run prompt file |
| `command_logs/B_rpc_server_commands.md` | Role B command log |
