# llama.cpp Server Ports Configuration (Role C — Ray Scheduling)

## Server URLs used by Role C

| Worker ID | Server URL | Host | Port | Notes |
|-----------|-----------|------|------|-------|
| worker_0 | http://127.0.0.1:8080 | localhost | 8080 | Primary llama-server instance |
| worker_1 | http://127.0.0.1:8081 | localhost | 8081 | Secondary llama-server instance |

## Multi-machine configuration (if available)

If multiple physical machines are available, replace the URLs above with actual host IPs:

```
http://主机A_IP:8080
http://主机B_IP:8080
```

## Limitation note (single-machine simulation)

Because only one physical machine is available for this experiment, both
llama-server instances run on the same host (localhost) on different ports.
This is a **single-machine multi-process simulation** of a multi-node
deployment:

- **What it tests**: Ray scheduling logic, round-robin distribution,
  concurrent request handling, and the Python orchestration code.
- **What it does NOT test**: Network latency between nodes, heterogeneous
  hardware, true distributed load balancing.
- **Resource contention**: Both server processes compete for the same CPU
  cores and memory bandwidth on the same machine.

This is acknowledged in `docs/ray_task.md`.

## llama-server startup commands (for reference)

These commands are executed by Role B or the experiment operator:

```bash
# Server 1 (port 8080)
./llama-server -m models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 0.0.0.0 --port 8080 \
  --ctx-size 2048 --batch-size 256 --threads 4 \
  --n-gpu-layers 0

# Server 2 (port 8081)
./llama-server -m models/qwen2.5-0.5b-instruct-q4_k_m.gguf \
  --host 0.0.0.0 --port 8081 \
  --ctx-size 2048 --batch-size 256 --threads 4 \
  --n-gpu-layers 0
```

## Verification

```bash
# Check if servers are running
curl -s http://127.0.0.1:8080/health | head -c 200
curl -s http://127.0.0.1:8081/health | head -c 200
```
