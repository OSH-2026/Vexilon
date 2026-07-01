# Vexilon

## Logo

![Vexilon_Logo](./img/logo.png)

## 目录

- [Vexilon](#vexilon)
  - [Logo](#logo)
  - [目录](#目录)
  - [团队信息](#团队信息)
  - [课程信息](#课程信息)
  - [项目进度](#项目进度)
  - [贡献一览](#贡献一览)
  - [项目概览](#项目概览)
  - [项目背景与立项动机](#项目背景与立项动机)
  - [最终实现概述](#最终实现概述)
  - [技术路线与系统架构](#技术路线与系统架构)
    - [Rust 模块重构](#rust-模块重构)
    - [EcoPet 上板验证](#ecopet-上板验证)
    - [PC 端 Agent](#pc-端-agent)
  - [开发板与硬件平台](#开发板与硬件平台)
  - [测试与验证方案](#测试与验证方案)
  - [仓库结构说明](#仓库结构说明)
  - [相关文档](#相关文档)
  - [参考链接](#参考链接)

## 团队信息

* 付锦鹏
* 杨邵恺
* 陈安达
* 邱义家
* 李俊谕

## 课程信息
* 课程：OSH-2026 操作系统课程设计
* 学期：2026 春季学期
* 单位：中国科学技术大学

## 项目进度

| 项目阶段 | 日期 | 项目进展 |
|:---:|:---:|---|
| 选题探索 | 3.4 ~ 3.15 | 组内成员线下开展研讨会分析往年小组选题，提出1.增强型LLm嵌入文件系统、2.Rust改写系统内核、3.网络协议栈改写等多种选择 |
|   | 3.16 ~ 3.18 | 组内成员锁定Rust重写项目并对重写哪种具体系统开展探索 |
| 项目调研 | 3.19 ~ 3.29 | 通过比较多种选择，并于老师和助教进行激烈的线上讨论后，最终选定LiteOS-M |
| 路线收敛 | 3.30 ~ 4.1  | 与老师进行线下研讨，确定“LiteOs-M核心模块接力重构 + IronClaw 接口层”最终路线 |
| 报告撰写 | 4.6 ~ 4.19 | 开展项目仓库主页README、调研报告、可行性报告、项目介绍ppt的写作任务并在过程中不断修正错误、加深对任务理解 |
| 报告改进 | 4.19 ~ 4.27 | 经过课上老师对本小组工作的点评，新增选择开发板并实际上板验证工作，本周完成此任务的多报告内容补充  |
| 项目第一阶段 | 4.28 ~ 6.1 | 完成rust化los_membox.c (10KB), los_task.c (50KB), los_tick.c (15KB), los_sortlink.c (6KB), los_sched.c (19KB)模块，并搭建测试集，完成编译检验  |
| 项目第二阶段 | 6.10 ~ 6.30 | 学习STM32开发板，编译并烧写LiteOs和Ecoprt代码，完成PC端agent部署，完成完整闭环测试  |
| 项目收尾 | 7.1 | 完成final_report 和 结题PPT 并做汇报预演  |

## 贡献一览
| 姓名 | 主要贡献 | 参考百分比贡献率 | 
| --- | --- | --- | 
| 陈安达 | 1. 提供选题意见并最终被敲定<br>2. 持续更新 README<br>3.完善可行性和调研报告<br>4. 改写 20KB LiteOS 代码<br>5. 给出 Lab4 分工，完成 Lab4 的 llama.cpp 主线任务<br>6. 完成 2 次汇报提问（中期）<br>7. 参与 Ecopet 代码和烧写任务<br> 8.完善结题报告<br> 9.准备结题 PPT 并上台做结题报告 | 28%
| 付锦鹏 | 1. 提供选题意见<br>2. 编写可行性报告<br>3. 给出 Rust 改写分工<br>4. 买入开发板<br>5. 编译并验证Rust改写正确<br>6. 参与烧写任务<br>7. 编写结题报告并上台做结题报告 | 24% |  
| 李俊谕 | 1. 提供选题意见<br>2. 编写调研报告<br>3. 改写 40KB LiteOS 代码<br>4. 完成 Lab4 的 Ray 必做和选做任务<br>5. 学习 STM32 开发板使用并参与 Ecopet 代码和烧写任务<br> 6. 完成 5 次汇报提问（中期）<br> 7. 完成 2 次汇报提问（结题）|  23.5%   |
| 邱义家 | 1. 提供选题意见<br>2. 完成中期答辩 PPT（使用）并上台做中期汇报<br>3. 改写 40KB LiteOS 代码<br>4. 完成 Lab4 的 Ray 必做和选做任务<br>5. 学习 STM32 开发板使用并参与 Ecopet 代码和烧写任务 <br>6. 完成 2 次汇报提问（结题） |   22.5%    |
| 杨邵恺 | 1. 完成中期答辩 PPT（未使用）<br>2. 完成 3 次汇报提问（结题） |  2%  |
## 项目概览

Vexilon 是中国科学技术大学 2026 春季学期 OSH 操作系统课程设计项目。本项目延续 2024 届 RushToLight 团队对 LiteOS-M 动态内存模块的 Rust 改写工作，继续探索 OpenHarmony LiteOS-M 中更多关键模块的 Rust 化实现，并通过 STM32F407 开发板上的 EcoPet Demo 完成基础真机联动验证。

需要说明的是，早期调研报告和可行性报告中曾规划过 IronClaw 接口层、Hi3861V100 开发板和 WiFi 输入链路；这些内容保留为项目探索过程记录。最终实验结果、硬件平台、上板 Demo 和 Agent 方案均以 [结题报告](./docs/final_report.md) 为准。

## 项目背景与立项动机

LiteOS-M 原生内核主要由 C 语言实现，任务管理、调度、时钟、链表和内存块管理等路径中存在较多裸指针、手动资源管理和弱类型接口。Rust 的所有权、类型系统和 no_std 支持为嵌入式内核模块重构提供了可行方向。本项目选择若干关键模块进行渐进式替换，目标是在保留 LiteOS-M 轻量和实时特征的同时，降低常见内存误用和接口误用风险。

## 最终实现概述

项目最终包含两个主题任务：

1. **LiteOS-M 多模块 Rust 化重构**：围绕 `los_sortlink`、`los_tick`、`los_membox`、`los_task`、`los_sched` 五个模块完成 Rust 版本实现、C/Rust FFI 适配和模块测试。
2. **EcoPet 上板验证与自然语言 Agent**：在正点原子 ALIENTEK M144Z-M4 最小系统板（STM32F407ZGT6）上运行 EcoPet Demo，通过 USART1、LiteOS-M 任务、消息队列和 PF9/PF10 LED 验证任务调度、队列通信、定时衰减和硬件状态反馈；PC 端使用 Ollama + Qwen3:8B Agent 做自然语言指令转换和板端响应解释。

早期规划中的 IronClaw 未进入最终实现，原因是依赖和资源成本与课程周期、开发板条件不匹配。最终方案将安全边界放在板端固定协议、参数校验和状态钳位上，大模型只承担交互辅助角色，不直接绕过固件协议修改状态。

## 技术路线与系统架构

### Rust 模块重构

重构工作采用 Rust（no_std + alloc）与 C 混合开发方式：

1. Rust 模块通过 `#[repr(C)]` 保持结构体布局兼容；
2. 通过 `#[no_mangle]` 暴露 C 侧可链接符号；
3. 使用 bindgen/cbindgen 辅助管理 C/Rust 接口；
4. 保留 LiteOS-M 原有启动、硬件适配和部分 C 侧驱动逻辑；
5. 通过单元测试和联动测试验证模块行为。

具体模块原理、接口设计和测试结果见 [结题报告](./docs/final_report.md)，早期模块选择依据见 [调研报告](./docs/research_report.md) 与 [可行性报告](./docs/feasibility_report.md)。

### EcoPet 上板验证

EcoPet 是本项目的上板验证场景，不是独立产品。它用于把任务调度、队列通信、串口输入、周期任务和 GPIO 输出串成一个可观察的闭环。

Demo 包含三个 LiteOS-M 任务：

1. `UartRxTask`：从 USART1 中断写入的环形缓冲区读取命令，并写入队列；
2. `PetStateTask`：消费队列命令，更新宠物状态机；
3. `TelemetryTask`：周期执行状态衰减并输出低值告警。

宠物状态包含 `health`、`hunger`、`mood`、`energy` 四个字段，范围均限制在 `[0, 100]`。支持 `STATUS`、`FEED <1-100>`、`PLAY <1-100>`、`SLEEP`、`HEAL` 五类命令。`PLAY` 含随机事件，存在受伤概率，并带有 `ERR:PLAY_FATAL` 保护。

LED 状态反馈如下：

1. PF9 绿色 LED：`mood > 50` 时点亮，表示心情较好；
2. PF10 红色 LED：`hunger > 60`、`health < 50` 或 `energy < 20` 时点亮，表示需要关注。

### PC 端 Agent

`ecopet_agent_ollama.py` 使用 Python 编写，默认连接 `COM5`、115200 波特率，并调用本地 Ollama 的 `qwen3:8b` 模型：

1. 将中文/英文自然语言转换为板端串口命令；
2. 将 `OK:*`、`WARN:*`、`ERR:*` 和状态字段解释为自然语言；
3. 后台监听 `WARN:LOW_HEALTH`、`WARN:HIGH_HUNGER`、`WARN:LOW_MOOD`、`WARN:LOW_ENERGY` 等告警并立即提示。

## 开发板与硬件平台

最终使用的开发板为正点原子 ALIENTEK M144Z-M4 最小系统板，主控芯片为 STM32F407ZGT6。板卡标注资源包括 1MB Flash、192KB RAM、8Mb SRAM、128Mb SPI Flash、USB UART、USB Slave/Host、LCD/TF Card 接口、10 组扩展接口、2 个按键、2 个 LED 和 2Kb EEPROM。

仓库中的固件工程目录仍沿用 `stm32f429ig_firechallenger` 的历史命名，但最终链接脚本、栈顶地址和时钟配置按 STM32F407 资源约束进行适配。

## 测试与验证方案

项目验证包括三类：

1. **模块测试**：覆盖五个 Rust 重构模块的基础功能、边界参数和异常输入；
2. **C/Rust FFI 测试**：检查结构体布局、函数符号和参数传递是否符合 C 侧调用约定；
3. **真机联动测试**：通过 EcoPet Demo 验证任务创建、队列通信、串口收发、状态定时衰减和 LED 状态反馈。

当前测试结果说明项目模块在已有用例下能够保持功能兼容，并对部分非法输入和边界状态提供更明确的检查路径。它不等同于对完整 LiteOS-M 内核的形式化证明或长期工业级压力验证。

## 仓库结构说明

```text
Vexilon/
├── README.md                         项目主页与导航
├── docs/                             项目文档
│   ├── research_report.md            调研报告，记录早期选题、背景和技术方向
│   ├── feasibility_report.md         可行性报告，记录早期实现路径与上板设想
│   └── final_report.md               结题报告，最终实现与实验结果以此为准
├── img/                              README 和报告使用的图片资源
├── Lab4/                             课程 Lab4 相关内容
└── LiteOS-M/
    ├── rust_kernel/                  Rust 重构内核模块与相关构建产物
    ├── c_kernel/                     C 侧 LiteOS-M 相关代码
    ├── test/                         测试代码与测试结果
    └── on-board/
        ├── liteos_f407/              STM32F407 上板固件工程
        └── agent/                    EcoPet PC 端 Python Agent
```

## 相关文档

1. [调研报告](./docs/research_report.md)：记录项目早期背景、模块选择依据、行业调研和前瞻方案。
2. [可行性报告](./docs/feasibility_report.md)：记录早期技术路线、FFI 设想、EcoPet 上板可行性分析和风险评估。
3. [结题报告](./docs/final_report.md)：记录最终实现、实验结果、IronClaw 方案调整原因、EcoPet Demo 和 Agent 细节。项目最终结论以此文档为准。

## 参考链接

1. LiteOS-M（Gitee）：https://gitee.com/openharmony/kernel_liteos_m
2. Tock OS（GitHub）：https://github.com/tock/tock
3. Hubris（GitHub）：https://github.com/oxidecomputer/hubris
4. Microsoft: We need a safer systems programming language: https://www.microsoft.com/en-us/msrc/blog/2019/07/we-need-a-safer-systems-programming-language
5. NSA: Software Memory Safety: https://media.defense.gov/2022/Nov/10/2003112742/-1/-1/0/CSI_SOFTWARE_MEMORY_SAFETY.PDF