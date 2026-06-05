# Role A Single-machine Deployment Screenshots

This directory stores screenshots used as Role A evidence.

| File | Evidence |
|---|---|
| `01_build_success.png` | `llama-cli.exe --version` output and built binary paths. |
| `02_model_file.png` | GGUF model file under `Lab4/models/`: `qwen2.5-0.5b-instruct-q4_k_m.gguf`. |
| `03_single_inference.png` | Single `llama-cli` inference output, including model loading, generated text, speed line, and normal exit. |
| `04_benchmark_running.png` | Baseline benchmark summary generated from `results/single_benchmark.csv`. |
| `05_results_files.png` | Generated result files under `Lab4/results/`. |

The raw benchmark data remains in `Lab4/results/`. Model files and build artifacts are not committed to Git.