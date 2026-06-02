# Role A Command Log

All commands below reflect actual execution. Failed attempts are kept because they explain how the final working setup was reached.

## 1. Create local project directory

```powershell
$base = Join-Path $HOME 'Desktop\OSH2026_Lab4_Local'
$lab4 = Join-Path $base 'Lab4'
New-Item -ItemType Directory -Force -Path $lab4, `
  (Join-Path $lab4 'docs'), `
  (Join-Path $lab4 'scripts'), `
  (Join-Path $lab4 'configs'), `
  (Join-Path $lab4 'prompts'), `
  (Join-Path $lab4 'results'), `
  (Join-Path $lab4 'results\raw_single_outputs'), `
  (Join-Path $lab4 'results\logs'), `
  (Join-Path $lab4 'screenshots\single_deploy'), `
  (Join-Path $lab4 'command_logs'), `
  (Join-Path $lab4 'models')
```

Actual project path:

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4
```

## 2. Collect environment

```powershell
& 'C:\Program Files\Git\bin\bash.exe' -lc "chmod +x '/c/Users/cad/Desktop/OSH2026_Lab4_Local/Lab4/scripts/env_collect.sh' '/c/Users/cad/Desktop/OSH2026_Lab4_Local/Lab4/scripts/run_llama_cli.sh' '/c/Users/cad/Desktop/OSH2026_Lab4_Local/Lab4/scripts/bench_single.py' '/c/Users/cad/Desktop/OSH2026_Lab4_Local/Lab4/scripts/summarize_csv.py'; '/c/Users/cad/Desktop/OSH2026_Lab4_Local/Lab4/scripts/env_collect.sh'"
```

Output:

```text
results/env_info.txt
```

## 3. Clone and build llama.cpp

```powershell
git clone https://github.com/ggml-org/llama.cpp 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp'
git -C 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp' rev-parse HEAD
```

Commit:

```text
60130d18f9ac7f42cb4d7f6060b088a45d8f242e
```

Install CMake because it was not initially on PATH:

```powershell
python -m pip install --user cmake
```

Failed CPU build attempt with old MinGW 8.1:

```powershell
& 'C:\Users\cad\AppData\Roaming\Python\Python312\Scripts\cmake.exe' `
  -S 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp' `
  -B 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build' `
  -G 'MinGW Makefiles'

& 'C:\Users\cad\AppData\Roaming\Python\Python312\Scripts\cmake.exe' `
  --build 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build' `
  --config Release -j 8
```

Reason: old MinGW 8.1 failed while compiling `cpp-httplib`.

Failed UCRT build attempt without Windows target macro:

```powershell
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
& 'C:\Users\cad\AppData\Roaming\Python\Python312\Scripts\cmake.exe' `
  -S 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp' `
  -B 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt' `
  -G 'MinGW Makefiles' `
  -DCMAKE_C_COMPILER='C:\msys64\ucrt64\bin\gcc.exe' `
  -DCMAKE_CXX_COMPILER='C:\msys64\ucrt64\bin\g++.exe' `
  -DLLAMA_BUILD_UI=OFF

$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
& 'C:\Users\cad\AppData\Roaming\Python\Python312\Scripts\cmake.exe' `
  --build 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt' `
  --config Release -j 8
```

Reason: `cpp-httplib` detected Windows target lower than Windows 10.

Successful CPU build:

```powershell
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
& 'C:\Users\cad\AppData\Roaming\Python\Python312\Scripts\cmake.exe' `
  -S 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp' `
  -B 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10' `
  -G 'MinGW Makefiles' `
  -DCMAKE_C_COMPILER='C:\msys64\ucrt64\bin\gcc.exe' `
  -DCMAKE_CXX_COMPILER='C:\msys64\ucrt64\bin\g++.exe' `
  -DCMAKE_C_FLAGS='-D_WIN32_WINNT=0x0A00' `
  -DCMAKE_CXX_FLAGS='-D_WIN32_WINNT=0x0A00' `
  -DLLAMA_BUILD_UI=OFF

$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
& 'C:\Users\cad\AppData\Roaming\Python\Python312\Scripts\cmake.exe' `
  --build 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10' `
  --config Release -j 8
```

## 4. Prepare model

```powershell
curl.exe -L --fail --retry 3 `
  -o 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf' `
  'https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct-GGUF/resolve/main/qwen2.5-0.5b-instruct-q4_k_m.gguf?download=true'
```

## 5. Single inference

```powershell
$lab4 = 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4'
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
$cli = "$lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe"
$model = "$lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf"
$promptFile = "$lab4\results\single_prompt.txt"
[System.IO.File]::WriteAllText($promptFile, '请用三句话解释什么是虚拟内存，并说明页表和 TLB 的关系。', [System.Text.UTF8Encoding]::new($false))
& $cli -m $model -f $promptFile -n 128 --threads 4 --ctx-size 2048 --batch-size 256 --single-turn --simple-io
```

Output:

```text
results/single_inference_output.txt
results/single_run_log.txt
```

## 6. Baseline benchmark

```powershell
$lab4 = 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4'
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
python "$lab4\scripts\bench_single.py" `
  --model-path "$lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf" `
  --llama-cli "$lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe" `
  --prompts "$lab4\prompts\role_a_benchmark_prompts.jsonl" `
  --output "$lab4\results\single_benchmark.csv" `
  --n-predict 128 `
  --threads 4 `
  --ctx-size 2048 `
  --batch-size 256 `
  --repeat 3 `
  --timeout 300
```

Generate summary:

```powershell
$lab4 = 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4'
python "$lab4\scripts\summarize_csv.py" `
  --input "$lab4\results\single_benchmark.csv" `
  --output "$lab4\results\single_benchmark_summary.md" `
  --group-by "prompt_id,threads,ctx_size,batch_size,no_mmap"
```

## 7. Parameter tuning

Parameter tuning used `prompts/role_a_tuning_prompts.jsonl`, containing A001 and A002.

### threads

```powershell
# Tested:
--threads 1
--threads 2
--threads 4
--threads 8

# Fixed:
--ctx-size 2048 --batch-size 256 --n-predict 128
```

### batch-size

```powershell
# Tested:
--batch-size 128
--batch-size 256
--batch-size 512

# Fixed:
--threads 4 --ctx-size 2048 --n-predict 128
```

### ctx-size

```powershell
# Tested:
--ctx-size 512
--ctx-size 1024
--ctx-size 2048

# Fixed:
--threads 4 --batch-size 256 --n-predict 128
```

### no-mmap

```powershell
# Tested:
# default mmap
--no-mmap

# Fixed:
--threads 4 --ctx-size 2048 --batch-size 256 --n-predict 128
```

Combined output:

```text
results/param_tuning.csv
```

Generate summary:

```powershell
$lab4 = 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4'
python "$lab4\scripts\summarize_csv.py" `
  --input "$lab4\results\param_tuning.csv" `
  --output "$lab4\results\param_tuning_summary.md" `
  --group-by "threads,ctx_size,batch_size,no_mmap"
```
