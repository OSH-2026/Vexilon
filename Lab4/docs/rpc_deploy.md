# RPC Distributed Inference Deployment

## 1. Machine Topology

| Role | Hostname | IP | CPU | RAM | GPU |
|---|---|---|---|---|---|
| Host (Master) | LAPTOP-CNRQSONN | 192.168.137.1 | Intel(R) Core(TM) Ultra 7 255HX (20C/20T) | 16 GB | NVIDIA GeForce RTX 5060 Laptop |
| Worker (Slave) | ljyUSTC | 192.168.137.70 | Intel(R) Core(TM) i7-10700 @ 2.90GHz | 32 GB | — |

## 2. Network Setup

| Property | Value |
|---|---|
| Type | Mobile hotspot (host creates hotspot, worker connects) |
| Subnet | 192.168.137.0/24 |
| RPC Port | 50052 |
| Latency | 1ms–538ms range, avg ~46ms (high variance) |

**Note**: Hotspot network has high latency variance, less stable than wired LAN. This directly impacts RPC generation speed stability.

## 3. Software Stack

| Component | Host | Worker |
|---|---|---|
| llama.cpp version | b9502-6ddc9430b | b9502-6ddc9430b |
| Compiler | MinGW GCC 16.1.0 | — |
| Backend | CPU only, `GGML_RPC=ON` | CPU only, RPC server |
| Model | Qwen2.5-0.5B-Instruct Q4_K_M | — |

## 4. Build Instructions (Host)

The default MinGW build does not include RPC support. Rebuild with:

```bash
cd llama.cpp/build
export PATH="/c/Program Files/MinGW/mingw64/bin:$PATH"

# Reconfigure with RPC enabled
cmake .. -DGGML_RPC=ON -DGGML_CUDA=OFF -DGGML_VULKAN=OFF -G "MinGW Makefiles"

# Rebuild llama-cli only
mingw32-make -j4 llama-cli
```

The rebuilt binary will be at `llama.cpp/build/bin/llama-cli.exe`.

Verify RPC support:
```bash
./llama.cpp/build/bin/llama-cli.exe --help 2>&1 | grep rpc
# Expected: --rpc SERVERS  comma-separated list of RPC servers (host:port)
```

## 5. Worker Setup

On the worker machine (ljyUSTC), start the RPC server:

```bash
# Build llama.cpp with RPC support on worker
cd llama.cpp/build
cmake .. -DGGML_RPC=ON -G "..."  # Use worker's native compiler
make -j4 rpc-server

# Start rpc-server on port 50052
./bin/rpc-server --host 0.0.0.0 --port 50052
```

Verify connectivity from host:
```python
import socket
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.settimeout(5)
result = s.connect_ex(('192.168.137.70', 50052))
print('OPEN' if result == 0 else f'CLOSED({result})')
```

## 6. Usage

### RPC Inference Command

```bash
cd "C:/Code/rust/Lab4"
export PATH="/c/Program Files/MinGW/mingw64/bin:$PATH"

./llama.cpp/build/bin/llama-cli.exe \
  -m ./models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  -f ./prompt_file.txt \
  --rpc 192.168.137.70:50052 \
  -n 128 \
  --threads 2 \
  --ctx-size 2048 \
  --single-turn \
  --simple-io
```

### Key Parameters

| Parameter | Value | Note |
|---|---|---|
| `--rpc` | `192.168.137.70:50052` | Worker RPC server address |
| `--threads` | `2` | **Hard limit** due to host thermal constraints |
| `--ctx-size` | `2048` | Context window |
| `-n` | `128` | Tokens to generate |
| `--single-turn` | required | Prevents interactive mode hang |
| `--simple-io` | recommended | Clean output |

## 7. Thermal Management

**Critical**: The host CPU (Ultra 7 255HX) has limited cooling capacity. Running inference with more than `--threads 2` risks thermal shutdown.

Rules:
1. **Always `--threads 2`** — never increase
2. **Sleep 60 seconds** between consecutive inference runs
3. **Batch in small groups** — don't run 12+ inferences back-to-back without breaks
4. Monitor CPU temperature during extended testing

## 8. Performance Summary

See `results/single_vs_rpc_summary.md` for full comparison.

| Metric | Single (local) | RPC (remote) | Ratio |
|---|---|---|---|
| Generation speed (short prompt) | 44.6 t/s | 8.0 t/s | 5.5× |
| Generation speed (long prompt) | 41.6 t/s | 7.8 t/s | 5.3× |
| Prompt processing (short) | 215.8 t/s | 122.3 t/s | 1.8× |
| Prompt processing (long) | 203.0 t/s | 218.2 t/s | 0.9× |
| Success rate | 100% (6/6) | 100% (6/6) | — |

## 9. Known Issues

1. **Hotspot latency variance**: 1ms–538ms range causes RPC generation speed to fluctuate (5.3–10.1 t/s for short prompts)
2. **No computation offload benefit**: Both machines use only 2 threads; RPC adds network overhead without parallelizing computation
3. **Prompt processing not offloaded**: Tokenization and prompt evaluation happen locally even in RPC mode
4. **RDMA not available**: TCP-only transport on Windows/MinGW build
