# Vexilon 项目双任务工作流程

> 本文档将项目工作拆分为两条并行主线：
> - **任务 A**：Rust 内核模块改写
> - **任务 B**：IronClaw 接入（含板侧 IronClaw-Lite FFI 层 + PC 侧 IronClaw Agent）
>
> 两条主线在 **阶段三** 汇合，共同支撑 EcoPet 上板 Demo。

---

## 概念澄清（必读）

| 名称 | 位置 | 本质 |
|---|---|---|
| **IronClaw-Lite** | 固件内（Hi3861V100 板上） | 自实现的 C/Rust FFI 安全中间层，负责命令解析、能力检查、参数校验，编译进固件 |
| **IronClaw Agent** | PC/服务器端 | nearai/ironclaw 开源框架，WASM 沙箱 + AI Agent，通过 WiFi 向板子发命令、收日志、生成报告 |

两者不冲突，分别在不同位置工作，最终通过 WiFi 连接形成完整闭环。

---

## 任务 A：Rust 内核模块改写

### A-0：环境搭建（本地 PC，Windows + WSL2 或 Linux VM）

**目标**：能在本地编译 no_std Rust 静态库并与 C 代码链接。

1. 安装 Rust 工具链
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustup default stable
   ```

2. 添加嵌入式交叉编译目标（Hi3861V100 是 ARM Cortex-M，目标三元组为 `arm-none-eabi`）
   ```bash
   rustup target add arm-none-eabi
   # 备用：RISC-V 目标
   rustup target add riscv32imac-unknown-none-elf
   ```

3. 安装 ARM 交叉编译工具链（用于最终链接）
   ```bash
   # Ubuntu/Debian
   sudo apt install gcc-arm-none-eabi binutils-arm-none-eabi
   ```

4. 克隆 OpenHarmony LiteOS-M 源码
   ```bash
   git clone https://gitee.com/openharmony/kernel_liteos_m.git
   ```

5. 验证 C 侧能编译（先不加 Rust）：按官方文档配置 Hi3861 SDK，能 `make` 出固件即可。

6. 创建 Rust 改写工程目录
   ```
   vexilon-kernel/
   ├── Cargo.toml          # workspace
   ├── los_sortlink/       # 第一个改写模块
   ├── los_task/
   ├── los_sched/
   ├── los_sem/
   ├── los_mux/
   ├── los_queue/
   ├── los_event/
   ├── los_membox/
   ├── los_swtmr/
   ├── los_tick/
   ├── los_init/
   └── bridge/             # FFI 头文件与绑定
       └── include/
   ```

   `Cargo.toml` workspace 配置：
   ```toml
   [workspace]
   members = ["los_sortlink", "los_task", ...]

   [profile.release]
   opt-level = "s"   # 嵌入式优先代码体积
   lto = true
   ```

   每个子 crate 的 `Cargo.toml` 模板：
   ```toml
   [lib]
   crate-type = ["staticlib"]   # 编译为 .a 静态库供 C 链接

   [dependencies]
   # no_std，不依赖 std
   ```

   每个子 crate 的 `lib.rs` 顶部：
   ```rust
   #![no_std]
   #![no_main]
   ```

---

### A-1：第一个模块 `los_sortlink`（验证 FFI 流程）

**目标**：跑通"Rust 写逻辑 → 编译为 .a → C 调用"完整链路。

1. 阅读原始 C 代码 `kernel/src/los_sortlink.c`，理解数据结构（按超时排序的链表，约 200 行）。

2. 用 Rust 实现等价逻辑（参考 feasibility_report.md 3.3.1 节的示例），导出 `extern "C"` 函数：
   ```rust
   #[unsafe(no_mangle)]
   pub extern "C" fn los_sortlink_insert(link: *mut SortedLink, value: u32) -> u32 { ... }

   #[unsafe(no_mangle)]
   pub extern "C" fn los_sortlink_remove(link: *mut SortedLink, value: u32) -> u32 { ... }
   ```

3. 编写对应 C 头文件 `bridge/include/los_sortlink_rs.h`。

4. 在 LiteOS-M 构建系统中，将 Rust 编译产物 `.a` 加入链接：
   ```makefile
   LIBS += $(RUST_OUT)/liblos_sortlink.a
   ```

5. 写一个最小 C 测试程序调用 Rust 函数，在 PC 上（x86）先验证逻辑正确，再交叉编译到 ARM。

6. **验收标准**：C 调用 Rust 的 `los_sortlink_insert` / `los_sortlink_remove`，结果与原 C 版本一致，无链接错误。

---

### A-2：核心模块 `los_task` + `los_sched`（最重要，建议两人合作）

**目标**：任务创建、删除、挂起、恢复、调度逻辑全部用 Rust 实现。

1. 阅读 `los_task.c`（约 1200 行）和 `los_sched.c`（约 600 行），重点理解：
   - 任务控制块（TCB）结构体字段
   - 就绪队列（优先级位图 + 链表）
   - 上下文切换触发点（注意：寄存器保存/恢复在 `arch/` 目录，保留 C/汇编，不改写）

2. Rust 侧定义 TCB 结构体（`#[repr(C)]` 保证内存布局与 C 兼容）：
   ```rust
   #[repr(C)]
   pub struct LosTaskCB {
       pub stack_pointer: *mut u8,
       pub task_status: u32,
       pub priority: u32,
       pub task_id: u32,
       // ... 其余字段
   }
   ```

3. 全局就绪队列用 `static` + 临界区保护（Hi3861 单核，用关中断代替 Mutex）：
   ```rust
   static mut READY_QUEUE: ReadyQueue = ReadyQueue::new();
   ```

4. 导出 C 兼容接口：
   ```rust
   #[unsafe(no_mangle)]
   pub extern "C" fn LOS_TaskCreate(task_id: *mut u32, attr: *const TaskAttr) -> u32 { ... }

   #[unsafe(no_mangle)]
   pub extern "C" fn LOS_TaskDelete(task_id: u32) -> u32 { ... }
   ```

5. 上下文切换（`PendSV_Handler`）保留原 C/汇编实现，Rust 侧只负责调度决策（选哪个任务跑），不碰寄存器操作。

6. **验收标准**：在 QEMU（`qemu-system-arm -M mps2-an385`）上能创建 2 个任务并交替运行，UART 输出各自的 tick 计数。

---

### A-3：IPC 模块（可并行分工）

每个模块相对独立，可以分人同时推进。

#### `los_queue`（优先，EcoPet Demo 直接依赖）

1. 阅读 `los_queue.c`（约 700 行），理解环形缓冲区 + 阻塞/唤醒逻辑。
2. Rust 实现固定容量环形队列，导出：
   ```rust
   #[unsafe(no_mangle)]
   pub extern "C" fn LOS_QueueCreate(...) -> u32 { ... }

   #[unsafe(no_mangle)]
   pub extern "C" fn LOS_QueueWrite(queue_id: u32, buf: *const u8, len: u32, timeout: u32) -> u32 { ... }

   #[unsafe(no_mangle)]
   pub extern "C" fn LOS_QueueRead(queue_id: u32, buf: *mut u8, len: *mut u32, timeout: u32) -> u32 { ... }
   ```
3. **验收标准**：两个任务通过 Queue 传递 `EcoCommand` 结构体，无数据损坏。

#### `los_sem` / `los_mux` / `los_event`

参考 feasibility_report.md 3.3.2、3.3.3 节的示例，逐一实现，导出对应 C 接口。

---

### A-4：辅助模块

| 模块 | 依赖 | 建议时机 |
|---|---|---|
| `los_membox` | 无 | A-1 之后，逻辑简单，适合热身 |
| `los_tick` | 无 | A-2 之前，Tick 计数是调度器基础 |
| `los_swtmr` | `los_sortlink` + `los_sched` | A-2 完成后 |
| `los_init` | 所有模块 | 最后，串联初始化 |

---

### A-5：QEMU 集成验证

在上板之前，先在 QEMU 上跑通完整内核：

```bash
qemu-system-arm \
  -M mps2-an385 \
  -kernel build/liteos_m.elf \
  -serial stdio \
  -nographic
```

**验收标准**：
- 3 个任务（NetRxTask、PetStateTask、TelemetryTask）正常创建并调度
- Queue 传递命令无丢失
- UART 输出宠物状态

---

## 任务 B：IronClaw 接入

IronClaw 接入分为两个独立子任务：

- **B1**：板侧 IronClaw-Lite（固件内 FFI 安全层）
- **B2**：PC 侧 IronClaw Agent（nearai 框架，WiFi 驱动测试）

---

### B1：板侧 IronClaw-Lite

#### B1-0：环境搭建（与 A-0 共用）

与任务 A 的环境搭建完全相同，无需额外配置。IronClaw-Lite 是 Rust 静态库的一部分，编译方式与内核模块改写相同。

---

#### B1-1：定义 FFI 接口（C 头文件）

创建 `bridge/include/ic_pet_ffi.h`（已在 feasibility_report.md 4 节定义，直接使用）：

```c
#pragma once
#include <stdint.h>

#define ECO_ARG_MAX 4

typedef enum {
    ECO_CMD_STATUS     = 1,
    ECO_CMD_FEED       = 2,
    ECO_CMD_PLAY       = 3,
    ECO_CMD_SLEEP      = 4,
    ECO_CMD_CLEAN      = 5,
    ECO_CMD_STRESS_IPC = 6,
    ECO_CMD_STRESS_MEM = 7,
} EcoCmdKind;

typedef struct {
    uint32_t kind;
    int32_t  args[ECO_ARG_MAX];
    uint32_t arg_len;
    uint32_t source;
    uint32_t capability;
} EcoCommand;

typedef struct {
    int32_t  health, hunger, mood, energy, comfort;
    uint32_t tick, error_count;
} EcoPetState;

// 核心入口：命令解析 + 能力检查 + 参数校验 + 入队
uint32_t ic_pet_dispatch(const EcoCommand *cmd);

// 查询宠物当前状态
uint32_t ic_pet_get_state(EcoPetState *out);

// 受控内存压测（count ≤ 200，block_size ≤ 128 字节）
uint32_t ic_pet_mem_stress(uint32_t count, uint32_t block_size);
```

---

#### B1-2：Rust 侧实现 IronClaw-Lite 核心逻辑

创建 crate `ironclaw_lite/src/lib.rs`：

**能力位定义**：
```rust
const CAP_READ:       u32 = 0x01;  // STATUS 命令
const CAP_WRITE:      u32 = 0x02;  // FEED/PLAY/SLEEP/CLEAN
const CAP_STRESS_IPC: u32 = 0x04;  // STRESS_IPC
const CAP_STRESS_MEM: u32 = 0x08;  // STRESS_MEM
```

**三层校验逻辑**（按顺序执行，任一失败立即返回错误码）：
1. 空指针检查
2. 能力位检查（`cmd.capability & required_cap != 0`）
3. 参数范围检查（FEED/PLAY 的 args[0] 必须在 1~100，MEM_STRESS 的 count ≤ 200，block_size ≤ 128）

**入队**：校验通过后调用 `LOS_QueueWrite`（通过 C FFI 调用，或调用任务 A 改写的 Rust Queue 接口）。

**错误码**（`EcoError` 枚举，`#[repr(u32)]`）：
```
0 = Ok
1 = NullPtr
2 = InvalidCommand
3 = PermissionDenied
4 = InvalidArgument
5 = QueueFailed
6 = AllocFailed
```

---

#### B1-3：宠物状态机（Rust 实现）

在 `ironclaw_lite/src/pet_state.rs` 中实现：

```rust
pub struct EcoPetState {
    pub health:  i32,   // 0~100
    pub hunger:  i32,   // 0~100（越高越饿）
    pub mood:    i32,   // 0~100
    pub energy:  i32,   // 0~100
    pub comfort: i32,   // 0~100
    pub tick:    u32,
    pub error_count: u32,
}

impl EcoPetState {
    pub fn apply_feed(&mut self, amount: i32) { ... }
    pub fn apply_play(&mut self, amount: i32) { ... }
    pub fn apply_sleep(&mut self)             { ... }
    pub fn apply_clean(&mut self)             { ... }
    pub fn tick_decay(&mut self)              { ... }  // 每 Tick 自然衰减
}
```

所有字段值域钳位在 0~100，不允许溢出。

---

#### B1-4：SVC 接口层（理论设计，非当前上板路径）

> **重要说明**：以下 SVC 接口设计是面向未来有 MMU 的平台（如 Cortex-A 系列）的理论扩展方案。Hi3861V100（Cortex-M）无 MMU，用户态/内核态隔离不完整，当前上板验证**不走 SVC 路径**，直接使用 B1-2 的 FFI 直调方式。

理论设计（记录在 feasibility_report.md 3.5 节）：
- 在 `los_syscall.c` 的 `SVC_Handler` 中挂载 IronClaw 分发逻辑
- 用户态调用 `ironclaw_task_create` → 触发 SVC → IronClaw 校验 → 转发至 Rust 内核模块
- 适用于有完整特权级隔离的平台

当前 Hi3861V100 上板路径：C 任务直接调用 `ic_pet_dispatch()`，无需 SVC 中断。

---

#### B1-5：三任务架构（C 侧，调用 Rust 函数）

```
NetRxTask（优先级高）
  ├── 从 UART 或 WiFi socket 读取原始字节
  ├── 构造 EcoCommand 结构体（填写 kind/args/capability）
  ├── 调用 ic_pet_dispatch(cmd)  ← 进入 Rust IronClaw-Lite
  │     ├── 能力检查
  │     ├── 参数检查
  │     └── LOS_QueueWrite(cmd_queue, cmd)
  └── 打印错误码到 UART

PetStateTask（优先级中）
  ├── LOS_QueueRead(cmd_queue, &cmd, timeout=LOS_WAIT_FOREVER)
  ├── 调用 ic_pet_apply_command(cmd)  ← Rust 状态机
  └── 更新全局 EcoPetState

TelemetryTask（优先级低）
  ├── 每 500ms 调用 ic_pet_get_state(&state)
  └── 格式化输出到 UART：
      "H:85 HG:30 MD:70 EN:60 CF:75 T:1234"
```

---

#### B1-6：上板验证步骤（最短路径）

**Step 1**：UART 启动验证
- 烧录固件，串口工具（115200 baud）看到 boot log
- 确认 3 个任务创建成功

**Step 2**：C 调 Rust 验证
- UART 发送 `FEED 10`（手动构造 EcoCommand，capability=CAP_WRITE）
- 确认 `ic_pet_dispatch` 返回 `0`（Ok）
- UART 看到 PetStateTask 消费命令并更新状态

**Step 3**：Queue 验证
- 快速连发 5 条命令，确认无丢失、无乱序

**Step 4**：WiFi 输入
- PC 通过 TCP 连接板子 IP，发送文本命令
- 板子解析后走相同的 `ic_pet_dispatch` 路径

**Step 5**：内存压测
- 发送 `STRESS_MEM 100 64`（100 次 alloc/free，每块 64 字节）
- 确认无崩溃，UART 输出 `AllocFailed` 计数为 0

---

### B2：PC 侧 IronClaw Agent（nearai 框架）

#### B2-0：环境搭建（PC，macOS/Linux）

1. 安装 Rust（与 A-0 相同）

2. 克隆 IronClaw 仓库
   ```bash
   git clone https://github.com/nearai/ironclaw.git
   cd ironclaw
   cargo build --release
   ```

3. 配置 LLM 后端（选一个）：
   ```bash
   # 使用 Ollama 本地模型（推荐，无需 API Key）
   ollama pull llama3
   # 或配置 OpenAI/Anthropic API Key
   export OPENAI_API_KEY=sk-...
   ```

4. 验证 IronClaw Agent 能启动：
   ```bash
   ./target/release/ironclaw repl
   ```

---

#### B2-1：编写 WASM 工具（Rust → wasm32-wasip2）

IronClaw Agent 通过 WASM 工具与外部系统交互。为 EcoPet 编写两个工具：

**工具 1：`ecopet_send`**（向板子发送命令）

```rust
// tools/ecopet_send/src/main.rs
// 编译目标：wasm32-wasip2

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // args[1] = 命令字符串，如 "FEED 10"
    // 通过 TCP socket 连接板子 IP:PORT，发送命令，读取响应
    let response = tcp_send(&args[1]);
    println!("{}", response);
}
```

编译：
```bash
cargo build --target wasm32-wasip2 --release
cp target/wasm32-wasip2/release/ecopet_send.wasm ~/.ironclaw/tools/
```

**工具 2：`ecopet_status`**（查询宠物状态并解析）

```rust
// 连接板子，发送 STATUS，解析返回的状态字符串
// 输出结构化 JSON 供 Agent 分析
```

---

#### B2-2：编写 IronClaw Routine（自动化测试脚本）

在 `~/.ironclaw/routines/ecopet_stress_test.toml` 中定义自动化测试：

```toml
[routine]
name = "ecopet_stress_test"
description = "对 EcoPet 进行压力测试并生成报告"

[[steps]]
tool = "ecopet_send"
args = ["FEED 50"]

[[steps]]
tool = "ecopet_send"
args = ["PLAY 30"]

[[steps]]
tool = "ecopet_send"
args = ["STRESS_MEM 100 64"]

[[steps]]
tool = "ecopet_status"
```

---

#### B2-3：自然语言驱动测试（Demo 核心展示点）

在 IronClaw REPL 中输入自然语言：

```
> 测试在连续发送 10 次 FEED 命令后，宠物的 hunger 值是否正确下降，并生成报告
```

IronClaw Agent 自动：
1. 理解意图 → 生成测试计划
2. 调用 `ecopet_send` WASM 工具发送 10 次 `FEED 10`
3. 每次调用 `ecopet_status` 记录状态
4. 分析 hunger 变化趋势
5. 输出结构化报告（含时间戳、状态变化曲线、是否符合预期）

---

#### B2-4：UART 日志采集（可选增强）

IronClaw Agent 也可以通过串口工具采集 UART 日志，编写 `ecopet_uart_log` WASM 工具：

```rust
// 读取串口 /dev/ttyUSB0，缓存日志，供 Agent 分析
```

这样 Agent 可以同时分析 WiFi 响应 + UART 内核日志，形成更完整的验证闭环。

---

## 阶段汇总与里程碑

| 阶段 | 任务 A | 任务 B | 里程碑 |
|---|---|---|---|
| **阶段一**（本地验证） | A-0 环境搭建 + A-1 los_sortlink | B1-0 环境搭建 + B1-1 FFI 头文件 | C 调 Rust 静态库链接成功 |
| **阶段二**（核心模块） | A-2 los_task + los_sched + A-3 los_queue | B1-2 IronClaw-Lite 核心逻辑 + B1-3 状态机 | QEMU 上 3 任务 + Queue + IronClaw-Lite 跑通 |
| **阶段三**（上板） | A-5 QEMU 验证通过后上板 | B1-5 三任务架构 + B1-6 上板步骤 | Hi3861V100 上 EcoPet Demo 完整运行 |
| **阶段四**（PC Agent） | （A 任务完成） | B2-0~B2-3 IronClaw Agent + WASM 工具 | 自然语言驱动测试 + 自动生成报告 |

---

## 关键约束与注意事项

1. **Hi3861V100 内存限制**：352KB SRAM，固件总内存占用建议控制在 200KB 以内，为 WiFi 栈和运行时留余量。

2. **IronClaw-Lite 不是 nearai IronClaw**：板侧的 IronClaw-Lite 是自实现的轻量 FFI 层，不依赖 nearai 仓库，不需要 WASM 运行时，编译进固件约 2~5KB。

3. **SVC 接口是扩展方向，不是当前路径**：feasibility_report.md 3.5 节的 SVC 设计面向有 MMU 的平台，Hi3861V100 上板验证直接用 FFI 直调，不走 SVC。

4. **上下文切换保留 C/汇编**：`arch/` 目录下的 `PendSV_Handler` 等汇编代码不改写，Rust 只负责调度决策逻辑。

5. **WiFi 调试保底方案**：WiFi 不稳定时，所有命令可通过 UART 输入，Demo 不依赖 WiFi 才能运行。

6. **内存压测参数上限**：`STRESS_MEM` 命令的 count ≤ 200，block_size ≤ 128 字节，超出范围由 IronClaw-Lite 的参数检查拦截，返回 `InvalidArgument`。
