# Role A Single-machine Deployment Screenshots

This directory contains the screenshots for OSH 2026 Lab4 Role A.

The screenshots are evidence for the local llama.cpp single-machine deployment, model preparation, inference, benchmark completion, and generated result files.

| File | Evidence |
|---|---|
| `01_build_success.png` | `llama-cli.exe --version` output showing the successful llama.cpp build and build commit. |
| `02_model_file.png` | The GGUF model file under `Lab4/models/`: `qwen2.5-0.5b-instruct-q4_k_m.gguf`. |
| `03_single_inference.png` | Successful single-machine `llama-cli` inference, including model loading, generated output, prompt speed, generation speed, and normal exit. |
| `04_benchmark_running.png` | Baseline benchmark summary generated from `results/single_benchmark.csv`. |
| `05_results_files.png` | Generated result files under `Lab4/results/`, including baseline CSV, parameter tuning CSV, summaries, environment info, and inference output. |

Notes:

1. These screenshots are supplementary evidence. The authoritative raw data is stored in `Lab4/results/`.
2. Model files and llama.cpp build artifacts are not committed to Git, following the Lab4 guide.
3. The reproducible commands are recorded in `Lab4/command_logs/A_single_benchmark_commands.md`.
