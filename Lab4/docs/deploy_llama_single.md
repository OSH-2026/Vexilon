# llama.cpp Single-machine Deployment

## 1. 实验目标

完成 GGUF 量化模型在本地机器上的 llama.cpp 单机部署，并成功运行一次推理。

## 2. 相关链接

- Lab4: https://osh-2026.github.io/lab4/
- llama.cpp 主线任务: https://osh-2026.github.io/lab4/llama_cpp/
- llama.cpp GitHub: https://github.com/ggml-org/llama.cpp

## 3. 本地目录

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4
```

## 4. 环境信息

见 `results/env_info.txt`。

关键信息：

| Item | Value |
|---|---|
| Hostname | LAPTOP-G44Q460K |
| OS | Windows 11 / Git Bash MINGW64_NT-10.0-26200 |
| CPU | Intel Core i9-14900HX, 24 cores / 32 logical processors |
| Memory | 16,779,841,536 bytes, about 16 GB |
| GPU | NVIDIA driver detected by nvidia-smi, CUDA 12.6 reported |
| Build toolchain | MSYS2 UCRT64 GCC 14.1.0, CMake 4.3.2 installed by pip |

## 5. llama.cpp 编译

源码位置：

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp
```

llama.cpp commit：

```text
60130d18f9ac7f42cb4d7f6060b088a45d8f242e
```

第一次使用 PATH 中 MinGW 8.1.0 构建失败，原因是旧工具链无法正确编译 `cpp-httplib` 相关线程和 Windows API。第二次使用 MSYS2 UCRT64 GCC 14.1.0 后，又因 Windows 目标版本宏过低失败。最终使用以下配置成功：

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

CPU backend 构建结果：

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\llama.cpp\build-ucrt-win10\bin\llama-bench.exe
```

GPU 后端：未构建。虽然 `nvidia-smi` 检测到 NVIDIA 驱动和 CUDA 12.6，但角色 A 主线以 CPU backend 跑通为优先，GPU offload 不作为必需项。

## 6. 模型信息

见 `configs/model_info.md`。

本次使用：

```text
C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf
```

## 7. 单机推理命令

为避免 Windows 命令行参数编码导致中文 prompt 损坏，本次将 prompt 写入 UTF-8 文件后通过 `-f` 传给 llama-cli：

```powershell
$lab4 = 'C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4'
$env:PATH = 'C:\msys64\ucrt64\bin;' + $env:PATH
$cli = "$lab4\llama.cpp\build-ucrt-win10\bin\llama-cli.exe"
$model = "$lab4\models\qwen2.5-0.5b-instruct-q4_k_m.gguf"
$promptFile = "$lab4\results\single_prompt.txt"
[System.IO.File]::WriteAllText($promptFile, '请用三句话解释什么是虚拟内存，并说明页表和 TLB 的关系。', [System.Text.UTF8Encoding]::new($false))
& $cli -m $model -f $promptFile -n 128 --threads 4 --ctx-size 2048 --batch-size 256 --single-turn --simple-io
```

## 8. 推理结果

输出保存到：

```text
results/single_inference_output.txt
```

关键输出摘录：

```text
build      : b9478-60130d18f
model      : qwen2.5-0.5b-instruct-q4_k_m.gguf
Prompt: 16.1 t/s | Generation: 10.4 t/s
```

模型成功生成了中文回答，说明 GGUF 量化模型已经在单机 llama.cpp 上成功推理。

## 9. 截图

截图说明见 `screenshots/single_deploy/README.md`。当前没有自动截图，需人工按该 README 放置截图。

## 10. 问题与解决方法

| Problem | Solution |
|---|---|
| PATH 中 MinGW 8.1.0 编译失败 | 改用 MSYS2 UCRT64 GCC 14.1.0 |
| cpp-httplib 报 Windows 目标版本低于 Windows 10 | 添加 `-D_WIN32_WINNT=0x0A00` |
| 中文 prompt 通过 `-p` 传参时乱码 | 改为写入 UTF-8 prompt 文件并用 `-f` 读取 |
| Windows 下 `/usr/bin/time -v` 不可用 | `max_rss_kb` 留空并在文档说明 |

## 11. 复现步骤

1. 进入 `C:\Users\cad\Desktop\OSH2026_Lab4_Local\Lab4`。
2. 确认模型文件在 `models\qwen2.5-0.5b-instruct-q4_k_m.gguf`。
3. 确认 `C:\msys64\ucrt64\bin` 可用。
4. 使用上面的 CMake 命令构建 `build-ucrt-win10`。
5. 使用上面的单机推理命令运行 `llama-cli.exe`。
6. 查看 `results/single_inference_output.txt` 和 `results/single_run_log.txt`。
