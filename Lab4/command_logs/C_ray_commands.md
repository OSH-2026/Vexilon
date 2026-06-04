# Role C — Ray Scheduling Command Log

All commands below reflect actual execution on the Linux experiment machine.

## Environment

- **Hostname**: ljyUSTC
- **OS**: Linux 6.17.0-23-generic (Ubuntu 24.04)
- **CPU**: 16 cores (x86_64)
- **Python**: 3.13.13 (miniconda3)
- **Working directory**: `/home/lijunyu/cs2024/Vexilon`

## 1. Install Python dependencies

```bash
# Install Ray, pandas (requests was already installed)
pip3 install ray pandas

# Verify installation
python3 -c "import ray; print('Ray version:', ray.__version__)"
python3 -c "import pandas; print('Pandas version:', pandas.__version__)"
python3 -c "import requests; print('Requests version:', requests.__version__)"
```

## 2. Verify llama-server nodes

```bash
# Check if llama-server instances are running
curl -s http://127.0.0.1:8080/health 2>&1 | head -c 200
curl -s http://127.0.0.1:8081/health 2>&1 | head -c 200

# If servers are not running, start them (see configs/server_ports.md for full command)
```

### Server URLs used in experiments

| Experiment | Server URL(s) |
|---|---|
| serial | http://127.0.0.1:8080 |
| ray_round_robin | http://127.0.0.1:8080, http://127.0.0.1:8081 |
| ray_parallel | http://127.0.0.1:8080, http://127.0.0.1:8081 |

All servers run locally (single-machine simulation). See `configs/server_ports.md` for details.

## 3. Start Ray

### Option A: Ray local mode (ray.init() without cluster)

The script uses `ray.init(ignore_reinit_error=True)` which starts a local
Ray instance in-process. No `ray start` command is needed.

```bash
# Ray local mode — automatically started by ray.init() in the script
# This is what our ray_batch_infer.py uses by default.
```

### Option B: Start a standalone Ray cluster (for multi-node)

```bash
# Head node (this machine)
ray start --head --port=6379

# Worker nodes (if available)
ray start --address='HEAD_IP:6379'

# Check cluster status
ray status

# Stop Ray when done
ray stop
```

For this experiment, **Option A (local mode)** is used because:
1. Only one machine is available.
2. `ray.init()` provides the same Actor/Task API as a cluster.
3. The scheduling logic is identical regardless of deployment mode.

### Option C: Virtual environment (if preferred)

```bash
python3 -m venv Lab4/.venv-ray
source Lab4/.venv-ray/bin/activate
pip install -U pip
pip install ray requests pandas
# Then run experiments inside the venv
```

## 4. Run experiments

### 4.1 Serial execution

```bash
cd /home/lijunyu/cs2024/Vexilon

python3 Lab4/scripts/ray_batch_infer.py \
  --prompts Lab4/prompts/ray_prompts_20.jsonl \
  --server-urls http://127.0.0.1:8080 \
  --strategy serial \
  --output Lab4/results/ray_serial.csv \
  --timeout 120
```

### 4.2 Ray Round-Robin

```bash
cd /home/lijunyu/cs2024/Vexilon

python3 Lab4/scripts/ray_batch_infer.py \
  --prompts Lab4/prompts/ray_prompts_20.jsonl \
  --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
  --strategy ray_round_robin \
  --output Lab4/results/ray_round_robin.csv \
  --timeout 120
```

### 4.3 Ray Parallel

```bash
cd /home/lijunyu/cs2024/Vexilon

python3 Lab4/scripts/ray_batch_infer.py \
  --prompts Lab4/prompts/ray_prompts_20.jsonl \
  --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
  --strategy ray_parallel \
  --output Lab4/results/ray_parallel.csv \
  --timeout 120 \
  --max-concurrency 8
```

### 4.4 Optional: 30-prompt runs

```bash
# Serial with 30 prompts
python3 Lab4/scripts/ray_batch_infer.py \
  --prompts Lab4/prompts/ray_prompts_30.jsonl \
  --server-urls http://127.0.0.1:8080 \
  --strategy serial \
  --output Lab4/results/ray_serial_30.csv \
  --timeout 120

# Round-robin with 30 prompts
python3 Lab4/scripts/ray_batch_infer.py \
  --prompts Lab4/prompts/ray_prompts_30.jsonl \
  --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
  --strategy ray_round_robin \
  --output Lab4/results/ray_round_robin_30.csv \
  --timeout 120

# Parallel with 30 prompts
python3 Lab4/scripts/ray_batch_infer.py \
  --prompts Lab4/prompts/ray_prompts_30.jsonl \
  --server-urls http://127.0.0.1:8080,http://127.0.0.1:8081 \
  --strategy ray_parallel \
  --output Lab4/results/ray_parallel_30.csv \
  --timeout 120 \
  --max-concurrency 8
```

## 5. Generate result summary

```bash
cd /home/lijunyu/cs2024/Vexilon

python3 Lab4/scripts/summarize_csv.py \
  --input Lab4/results/ray_serial.csv \
  --output Lab4/results/ray_serial_summary.md

python3 Lab4/scripts/summarize_csv.py \
  --input Lab4/results/ray_round_robin.csv \
  --output Lab4/results/ray_round_robin_summary.md

python3 Lab4/scripts/summarize_csv.py \
  --input Lab4/results/ray_parallel.csv \
  --output Lab4/results/ray_parallel_summary.md
```

## 6. Stop Ray cluster (if started with ray start)

```bash
ray stop
```
