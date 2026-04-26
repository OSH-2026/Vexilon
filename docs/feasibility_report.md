# 基于Rust重构LiteOS内核核心模块：可行性报告

## 目录

- [基于Rust重构LiteOS内核核心模块：可行性报告](#基于rust重构liteos内核核心模块可行性报告)
  - [目录](#目录)
  - [1. 摘要](#1-摘要)
  - [2. 理论依据](#2-理论依据)
    - [2.1 LiteOS内核核心模块与C语言原生缺陷](#21-liteos内核核心模块与c语言原生缺陷)
    - [2.2 Rust重构的理论优势与模块适配性](#22-rust重构的理论优势与模块适配性)
    - [2.3 其他Rust改写的可行性详情](#23-其他rust改写的可行性详情)
  - [3. 技术依据与实现路径](#3-技术依据与实现路径)
    - [3.1 核心技术栈与Crates选型](#31-核心技术栈与crates选型)
    - [3.2 待改写核心模块的Rust适配性与优先级](#32-待改写核心模块的rust适配性与优先级)
    - [3.3 Rust改写示例和优化体现](#33-rust改写示例和优化体现)
      - [3.3.1 `los_sortlink.c`：固定容量排序链表，避免指针失误](#331-los_sortlinkc固定容量排序链表避免指针失误)
      - [3.3.2 `los_event.c`：事件标志位改成强类型位集](#332-los_eventc事件标志位改成强类型位集)
      - [3.3.3 `los_sem.c`：信号量用原子计数，减少手工加锁风险](#333-los_semc信号量用原子计数减少手工加锁风险)
      - [3.3.4 `los_tick.c` + `los_swtmr.c`：Tick 驱动定时器，逻辑更集中](#334-los_tickc--los_swtmrctick-驱动定时器逻辑更集中)
    - [3.4 实现路径](#34-实现路径)
    - [3.5 基于IronClaw的用户层-内核层对接与接口设计](#35-基于ironclaw的用户层-内核层对接与接口设计)
      - [3.5.1 IronClaw（Agent）植入部位](#351-ironclawagent植入部位)
      - [3.5.2 IronClaw（Agent）核心交互接口定义](#352-ironclawagent核心交互接口定义)
        - [（1）用户层 → 内核层：同步系统调用接口（SVC触发）](#1用户层--内核层同步系统调用接口svc触发)
        - [（2）内核层 → 用户层：异步回调/通知接口（内核主动触发）](#2内核层--用户层异步回调通知接口内核主动触发)
      - [3.5.3 接口实现机制](#353-接口实现机制)
      - [3.5.4 接口适配性](#354-接口适配性)
    - [3.6 编译与构建环境](#36-编译与构建环境)
  - [4. C/Rust FFI 设计示例](#4-crust-ffi-设计示例)
  - [5. 性能与安全性分析](#5-性能与安全性分析)
  - [6. 创新点与技术挑战](#6-创新点与技术挑战)
    - [6.1 预期创新点](#61-预期创新点)
    - [6.2 难点评估与应对](#62-难点评估与应对)
  - [7. 测试与验证方案](#7-测试与验证方案)

## 1. 摘要
LiteOS作为面向IoT领域的轻量级实时操作系统，采用C语言开发，存在缓冲区溢出、释放后使用、数据竞争等内存安全问题，难以满足IoT设备高安全需求。本项目计划以**已完成Rust改写的`los_memory.c`为基础**，继续用Rust重构剩余11个内核核心模块（含任务管理、调度器、IPC组件等），同时引入**IronClaw作为用户层与内核层的对接中间层（Agent）**，设计双向调用接口。本文聚焦可行性分析，阐述待改写模块的Rust适配性、分阶段实现路径、IronClaw接口设计，论证在保持轻量实时特性的同时，从语言层面提升内核内存安全与交互可靠性的可行性。

## 2. 理论依据
### 2.1 LiteOS内核核心模块与C语言原生缺陷
LiteOS内核核心模块包含动态内存管理、排序链表、任务管理、调度器、IPC通信（信号量、互斥锁、消息队列等），面向资源受限IoT设备，要求轻量、实时、高可靠。C语言开发存在原生缺陷：
1. 无内存安全检查，裸指针操作易引发越界、空指针、UAF等漏洞；
2. 并发安全依赖人工加锁，多核/多任务场景易出现线程安全问题；
3. 错误处理隐式，运行时崩溃难以提前规避；
4. 数据结构（如排序链表、任务队列）的边界校验依赖开发者手动实现，易出现逻辑漏洞。

### 2.2 Rust重构的理论优势与模块适配性
- **无运行时开销**：无GC，零成本抽象，适配IoT设备资源受限场景，与LiteOS轻量特性完全兼容；
- **编译期内存安全**：所有权、借用、生命周期机制，可直接消除排序链表、任务管理等模块中常见的内存越界、野指针问题；
- **强并发安全保障**：`Send`/`Sync` trait与类型安全的同步原语，可替代IPC模块中手动实现的锁机制，从编译期避免数据竞争；
- **模块化与可维护性**：Rust的包管理与模块系统，可对内核功能进行更清晰的解耦，便于后续扩展与审计。

### 2.3 其他Rust改写的可行性详情
- [可见往年RushToLight小组的可行性报告](https://github.com/Chanda666/RushToLight/blob/main/feasibility%20report/feasibility%20report.md)
其中详细阐述了**liteOS-m内核分析、Rust特性分析、C与Rust交互与互操作性分析**
## 3. 技术依据与实现路径
### 3.1 核心技术栈与Crates选型
- **底层环境**：`no_std`+`alloc`自定义分配器，适配无OS嵌入式环境；
- **硬件支持**：`cortex-m`/`riscv`、`embedded-hal`，兼容LiteOS主流架构；
- **跨语言交互**：`bindgen`生成C绑定，`libc`对接系统底层接口；
- **工具链**：`cargo`交叉编译，适配ARM/RISC-V嵌入式目标。

### 3.2 待改写核心模块的Rust适配性与优先级
结合项目调研报告中的修改需求，除已完成改写的`los_memory.c`外，剩余11个核心模块均具备Rust改写可行性。

| 模块文件          | 核心功能         | 改写可行性说明                                                                 |
|-------------------|------------------|--------------------------------------------------------------------------------|
| `los_sortlink.c`  | 排序链表         | 数据结构逻辑简单，无复杂硬件依赖，可作为Rust改写入门模块，快速验证FFI与数据结构适配 |
| `los_task.c`      | 任务管理         | 内核核心模块，涉及任务控制块、状态管理，Rust可通过结构体封装实现类型安全的任务对象 |
| `los_sched.c`     | 调度器           | 实时性关键模块，Rust的零成本抽象可保证调度性能，同时通过编译期检查避免调度逻辑漏洞 |
| `los_sem.c`       | 信号量           | IPC基础组件，可基于Rust `core::sync` 原语实现安全信号量，消除手动加锁风险       |
| `los_mux.c`       | 互斥锁           | 并发安全核心，Rust的`Mutex`/`SpinLock`可直接替代原生实现，编译期保证锁安全       |
| `los_queue.c`     | 消息队列         | 任务间通信模块，可通过Rust的通道/环形队列实现类型安全的消息传递，避免数据竞争   |
| `los_event.c`     | 事件标志组       | 事件同步模块，逻辑简单，可通过Rust枚举/位运算实现安全的事件管理                 |
| `mm/los_membox.c` | 静态内存池       | 静态内存管理，与已改写的`los_memory.c`逻辑同源，可复用内存安全设计模式         |
| `los_swtmr.c`     | 软件定时器       | 依赖Tick时钟与任务调度，需与调度器协同改写，可基于Rust时间库实现安全定时器逻辑   |
| `los_tick.c`      | Tick时钟         | 基础时钟模块，逻辑简单，可快速改写并与定时器/调度器联动                         |
| `los_init.c`      | 内核初始化       | 依赖所有核心模块，需在其他模块改写完成后适配，统一初始化流程                     |

### 3.3 Rust改写示例和优化体现
下面是3.2表格中一些模块的Rust改写示例和优化点。

#### 3.3.1 `los_sortlink.c`：固定容量排序链表，避免指针失误
**改动思路**：
C 版通常需要手动维护前驱/后继指针，Rust 改成“边界明确的固定数组 + 有序插入”，可以先验证数据结构迁移的可行性。
```rust
pub struct SortedLink<const N: usize> {
    buf: [u32; N],
    len: usize,
}

impl<const N: usize> SortedLink<N> {
    pub const fn new(init: u32) -> Self {
        Self { buf: [0; N], len: 0 }
    }

    pub fn insert(&mut self, value: u32) -> Result<(), ()> {
        if self.len == N {
            return Err(());
        }

        let mut i = self.len;
        while i > 0 && self.buf[i - 1] > value {
            self.buf[i] = self.buf[i - 1];
            i -= 1;
        }

        self.buf[i] = value;
        self.len += 1;
        Ok(())
    }
}
```
**优化体现**：
* 没有裸指针和链表断链风险。
* `len <= N` 在编译期/运行期都更容易约束。
* 更适合先做 Rust 适配验证，改动小、风险低。

#### 3.3.2 `los_event.c`：事件标志位改成强类型位集
**改动思路**：
C 里常见的是 `uint32_t flag` + 宏定义，Rust 可以用结构体封装，避免“写错位掩码”或“不同模块重复定义”。
```rust
use core::sync::atomic::{AtomicU32, Ordering};

#[derive(Copy, Clone)]
pub struct EventFlags(u32);

impl EventFlags {
    pub const RX_READY: Self = Self(1 << 0);
    pub const TX_DONE:  Self = Self(1 << 1);
    pub const ERROR:    Self = Self(1 << 2);
}

pub struct EventGroup {
    bits: AtomicU32,
}

impl EventGroup {
    pub const fn new() -> Self {
        Self { bits: AtomicU32::new(0) }
    }

    pub fn set(&self, flags: EventFlags) {
        self.bits.fetch_or(flags.0, Ordering::Release);
    }

    pub fn clear(&self, flags: EventFlags) {
        self.bits.fetch_and(!flags.0, Ordering::Release);
    }

    pub fn is_set(&self, flags: EventFlags) -> bool {
        (self.bits.load(Ordering::Acquire) & flags.0) != 0
    }
}
```
**优化体现**：
* 事件含义从“魔法数字”变成“显式常量”。
* 原子操作替代手动临界区拼接，降低并发错误。
* 适合展示“Rust 的类型安全可以直接提升同步模块可靠性”。

#### 3.3.3 `los_sem.c`：信号量用原子计数，减少手工加锁风险
**改动思路**：
信号量核心就是计数器。Rust 可把计数逻辑和同步逻辑封装在一个对象里，减少“计数减到负数”或“释放时遗漏加锁”的问题。
```rust
use core::sync::atomic::{AtomicU32, Ordering};

pub struct Semaphore {
    count: AtomicU32,
}

impl Semaphore {
    pub const fn new(init: u32) -> Self {
        Self { count: AtomicU32::new(init) }
    }

    pub fn try_pend(&self) -> bool {
        let mut cur = self.count.load(Ordering::Acquire);
        while cur > 0 {
            match self.count.compare_exchange_weak(
                cur,
                cur - 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return true,
                Err(v) => cur = v,
            }
        }
        false
    }

    pub fn post(&self) {
        self.count.fetch_add(1, Ordering::Release);
    }
}
```
**优化体现**：
* 信号量状态由原子变量统一管理。
* 避免 C 中“先判断再修改”的竞态窗口。
* 说明 Rust 不仅能“搬代码”，还能把同步原语做得更稳。

#### 3.3.4 `los_tick.c` + `los_swtmr.c`：Tick 驱动定时器，逻辑更集中
**改动思路**：
把 Tick 计数和软件定时器检查逻辑拆开，再用 Rust 的结构体连接，便于和调度器联动。
```rust
use core::sync::atomic::{AtomicU64, Ordering};

pub struct TickClock {
    tick: AtomicU64,
}

impl TickClock {
    pub const fn new() -> Self {
        Self { tick: AtomicU64::new(0) }
    }

    pub fn on_tick(&self) {
        self.tick.fetch_add(1, Ordering::Relaxed);
    }

    pub fn now(&self) -> u64 {
        self.tick.load(Ordering::Acquire)
    }
}

pub struct SoftTimer {
    deadline: u64,
    fired: bool,
    callback: fn(),
}

pub fn check_timers(clock: &TickClock, timers: &mut [SoftTimer]) {
    let now = clock.now();
    for t in timers.iter_mut() {
        if !t.fired && t.deadline <= now {
            t.fired = true;
            (t.callback)();
        }
    }
}
```
**优化体现**：
* Tick 源统一，定时器逻辑更清晰。
* 触发条件明确，避免 C 中多处共享状态导致的修改分散。
* 适合说明“定时器模块可与调度器协同改写”。

### 3.4 实现路径
1. 完成`los_sortlink.c`的Rust改写，验证数据结构、FFI交互与单元测试流程，为后续模块改写建立模板；
2. 重点改写`los_task.c`+`los_sched.c`，实现任务创建、状态管理与调度逻辑的Rust重构，完成任务切换核心功能；
3. 根据项目进度，选做1~2个IPC模块（如`los_sem.c`/`los_queue.c`），实现任务间安全通信；
4. 改写`los_tick.c`/`los_swtmr.c`，最后适配`los_init.c`，完成全流程初始化逻辑，实现内核完整运行。

### 3.5 基于IronClaw的用户层-内核层对接与接口设计
本项目引入**IronClaw轻量化中间层（Agent）**，作为用户层（应用程序）与Rust重构内核层的统一对接入口，屏蔽语言差异、权限隔离、调用规范，实现**用户→内核、内核→用户双向安全调用**，是项目核心交互组件。
#### 3.5.1 IronClaw（Agent）植入部位
IronClaw作为核心交互Agent，其植入位置严格锚定LiteOS原有系统调用链路，无侵入式修改原有框架，具体植入层级与部位如下：
1. **用户态-内核态边界层（核心植入点）**
挂载于LiteOS原生SVC（Supervisor Call）异常处理入口，替代原有C实现的系统调用分发逻辑：
- 物理位置：`los_syscall.c`（原有系统调用分发文件）的`SVC_Handler`中断处理函数内，将系统调用分发逻辑接管至IronClaw层；
- 作用：所有用户层发起的系统调用请求，先进入IronClaw进行权限校验、参数合法性检查，再转发至Rust重构内核模块，避免非法调用直接触达内核。
1. **内核层回调转发层（辅助植入点）**
植入于Rust内核模块的事件通知/回调触发逻辑处（如任务调度完成、定时器超时、IPC消息就绪等场景）：
- 物理位置：Rust改写的`los_sched.rs`（调度器）、`los_swtmr.rs`（软件定时器）、`los_queue.rs`（消息队列）等模块的事件触发节点；
- 作用：内核层产生的主动通知/回调请求，先经由IronClaw完成跨语言类型转换、用户态权限映射，再推送至用户层，保证回调的安全性与兼容性。
1. **编译构建链路（部署植入）**
在LiteOS构建系统（Makefile/CMake）中，将IronClaw编译单元（C/Rust混合实现）链接至内核镜像的`os_adapter`段，与原有C内核代码段隔离，保证内存布局兼容性，且支持独立编译调试。
#### 3.5.2 IronClaw（Agent）核心交互接口定义
IronClaw的接口设计遵循“兼容原有API、强类型校验、双向可追溯”原则，分为**用户→内核（同步调用）** 和**内核→用户（异步回调）** 两类，接口定义、参数、调用规范如下：
##### （1）用户层 → 内核层：同步系统调用接口（SVC触发）
所有接口均为C兼容格式（便于原有用户态应用直接调用），由IronClaw接管分发，核心接口定义如下（含参数校验规则）（暂定，可能会更改）：
|接口名称|功能描述|入参类型|入参校验规则（IronClaw层）|内核对接模块|
|---|---|---|---|---|
|`ironclaw_task_create`|创建任务|`u32 *task_id, TaskAttr *attr`|1. task_id非空；2. attr优先级∈[0,31]；3. 栈大小≥最小阈值|`los_task.rs`|
|`ironclaw_task_delete`|删除任务|`u32 task_id`|1. task_id存在；2. 非空闲/内核任务|`los_task.rs`|
|`ironclaw_task_suspend`|挂起任务|`u32 task_id`|1. task_id非当前运行任务；2. 任务状态为就绪/运行|`los_task.rs`|
|`ironclaw_sched_yield`|任务主动让出CPU|无|无（仅校验当前CPU模式为用户态）|`los_sched.rs`|
|`ironclaw_sched_set_priority`|设置任务优先级|`u32 task_id, u8 prio`|1. prio∈[0,31]；2. task_id非内核关键任务|`los_sched.rs`|
|`ironclaw_sem_pend`|获取信号量|`u32 sem_id, u32 timeout`|1. sem_id存在；2. timeout≤最大Tick数|`los_sem.rs`|
|`ironclaw_sem_post`|释放信号量|`u32 sem_id`|1. sem_id存在；2. 信号量未溢出|`los_sem.rs`|
|`ironclaw_queue_send`|发送消息至队列|`u32 queue_id, void *data, u32 len`|1. queue_id存在；2. data非空；3. len≤队列最大容量|`los_queue.rs`|
|`ironclaw_queue_recv`|从队列接收消息|`u32 queue_id, void *buf, u32 *len`|1. queue_id存在；2. buf非空；3. len指针非空|`los_queue.rs`|
|`ironclaw_mem_alloc`|动态内存分配|`u32 size, u32 align`|1. size>0；2. align为2的幂次|`los_memory.rs`|
|`ironclaw_mem_free`|动态内存释放|`void *ptr`|1. ptr为内核分配的有效地址；2. 非重复释放|`los_memory.rs`|
**调用流程**：
1. 用户态应用调用IronClaw接口 → 触发SVC中断 → 进入IronClaw SVC处理函数；
2. IronClaw校验参数合法性、用户态权限 → 转换参数为Rust兼容类型；
3. 调用Rust内核模块对应函数 → 执行结果通过IronClaw转换为C兼容返回值 → 返回到用户态。
##### （2）内核层 → 用户层：异步回调/通知接口（内核主动触发）
由Rust内核模块触发，IronClaw负责跨语言适配与安全转发，核心接口定义如下：
|接口名称|功能描述|入参类型|回调触发场景|用户层注册方式|
|---|---|---|---|---|
|`ironclaw_event_notify`|通用事件通知|`u32 event_id, void *data, u32 len`|任务调度完成、定时器超时、IPC消息就绪|预注册`event_callback_t`函数指针|
|`ironclaw_task_exit_notify`|任务退出通知|`u32 task_id, i32 exit_code`|任务异常退出/正常终止|`ironclaw_callback_register`|
|`ironclaw_isr_notify`|中断处理完成通知|`u32 irq_num`|外部中断处理完成（如GPIO/串口）|`ironclaw_callback_register`|
**回调流程**：
1. Rust内核模块触发事件 → 调用IronClaw回调转发函数；
2. IronClaw校验用户层注册的回调函数合法性（非空、权限匹配）；
3. 切换至用户态上下文 → 执行用户层回调函数 → 完成后返回内核态。
#### 3.5.3 接口实现机制
- 基于LiteOS原生异常中断（SVC）实现内核态切换；
- 通过Rust FFI完成IronClaw（C兼容）与Rust内核的函数映射；
- 接口参数强类型校验，内核层做权限隔离，杜绝非法调用；
- 所有接口调用均记录日志（可选编译开关），包含调用者PID、参数、返回值，便于问题追溯。
#### 3.5.4 接口适配性
兼容LiteOS原有用户层API规范，现有应用无需修改即可对接重构后的内核，保证项目兼容性。
### 3.6 编译与构建环境
1. 安装嵌入式目标交叉编译支持：`rustup target add thumbv7m-none-eabi riscv32imac-unknown-none-elf`；
2. 配置Rust模块与LiteOS原有构建系统的链接规则，将Rust编译生成的静态库与C代码打包；
3. 为每个改写模块编写独立编译单元，支持增量编译与单模块调试。

## 4. C/Rust FFI 设计示例
C 和 Rust 之间需要使用稳定 ABI，不能传 Rust 的 String、Vec、trait object，也不能让 C 直接释放 Rust 分配的对象。
下面是C侧头文件代码示例：
```c
// bridge/include/ic_pet_ffi.h

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

typedef enum {
    ECO_SRC_UART = 0,
    ECO_SRC_WIFI = 1,
} EcoCmdSource;

typedef struct {
    uint32_t kind;
    int32_t args[ECO_ARG_MAX];
    uint32_t arg_len;
    uint32_t source;
    uint32_t capability;
} EcoCommand;

typedef struct {
    int32_t health;
    int32_t hunger;
    int32_t mood;
    int32_t energy;
    int32_t comfort;
    uint32_t tick;
    uint32_t error_count;
} EcoPetState;

uint32_t ic_pet_dispatch(const EcoCommand *cmd);
uint32_t ic_pet_get_state(EcoPetState *out);
uint32_t ic_pet_mem_stress(uint32_t count, uint32_t block_size);
```
下面是Rust 侧 FFI 示例
```rust
#![no_std]

#[repr(C)]
pub struct EcoCommand {
    pub kind: u32,
    pub args: [i32; 4],
    pub arg_len: u32,
    pub source: u32,
    pub capability: u32,
}

#[repr(C)]
pub struct EcoPetState {
    pub health: i32,
    pub hunger: i32,
    pub mood: i32,
    pub energy: i32,
    pub comfort: i32,
    pub tick: u32,
    pub error_count: u32,
}

#[repr(u32)]
pub enum EcoError {
    Ok = 0,
    NullPtr = 1,
    InvalidCommand = 2,
    PermissionDenied = 3,
    InvalidArgument = 4,
    QueueFailed = 5,
    AllocFailed = 6,
}

const CAP_READ: u32 = 0x01;
const CAP_WRITE: u32 = 0x02;
const CAP_STRESS_IPC: u32 = 0x04;
const CAP_STRESS_MEM: u32 = 0x08;

#[unsafe(no_mangle)]
pub extern "C" fn ic_pet_dispatch(cmd: *const EcoCommand) -> u32 {
    if cmd.is_null() {
        return EcoError::NullPtr as u32;
    }

    let cmd = unsafe { &*cmd };

    if !check_capability(cmd) {
        return EcoError::PermissionDenied as u32;
    }

    if !check_args(cmd) {
        return EcoError::InvalidArgument as u32;
    }

    match enqueue_checked_command(cmd) {
        Ok(()) => EcoError::Ok as u32,
        Err(_) => EcoError::QueueFailed as u32,
    }
}

fn check_capability(cmd: &EcoCommand) -> bool {
    match cmd.kind {
        1 => cmd.capability & CAP_READ != 0,
        2 | 3 | 4 | 5 => cmd.capability & CAP_WRITE != 0,
        6 => cmd.capability & CAP_STRESS_IPC != 0,
        7 => cmd.capability & CAP_STRESS_MEM != 0,
        _ => false,
    }
}

fn check_args(cmd: &EcoCommand) -> bool {
    match cmd.kind {
        1 | 4 | 5 => cmd.arg_len == 0,
        2 | 3 => cmd.arg_len == 1 && (0..=100).contains(&cmd.args[0]),
        6 => cmd.arg_len == 1 && (1..=2000).contains(&cmd.args[0]),
        7 => cmd.arg_len == 2 && (1..=500).contains(&cmd.args[0]) && (1..=256).contains(&cmd.args[1]),
        _ => false,
    }
}

fn enqueue_checked_command(_cmd: &EcoCommand) -> Result<(), ()> {
    // 实际项目中这里调用 Rust 封装后的 LiteOS-M queue 接口，
    // 或通过 C FFI 调用 LOS_QueueWrite。
    Ok(())
}
```
## 5. 性能与安全性分析
- **性能保障**：Rust编译为原生机器码，无额外运行时开销，调度器、任务管理、IPC等核心模块的执行性能与原生C版本持平，满足RTOS实时性要求；IronClaw接口调用仅增加单次SVC中断开销，对整体性能影响可忽略；
- **安全性提升**：
  1. 内存安全：排序链表、任务控制块等数据结构的边界校验由Rust类型系统自动完成，消除缓冲区溢出、野指针风险；
  2. 并发安全：IPC模块的同步原语基于Rust `Send`/`Sync` trait实现，从编译期避免数据竞争；
  3. 调用安全：IronClaw接口提供参数校验与权限隔离，防止用户态非法调用内核接口；

## 6. 创新点与技术挑战
### 6.1 预期创新点
- **分模块渐进式重构**：以`los_sortlink.c`为起点，按优先级分步改写核心模块，兼顾兼容性与安全性；
- **类型安全内核原语**：用Rust结构体与trait封装任务、调度、IPC对象，杜绝非法操作；
- **IronClaw统一交互层**：标准化用户-内核双向调用接口，兼容原有生态的同时提升交互可靠性；
- **混合内核架构**：保留C语言内核框架，Rust重构核心逻辑，兼顾实时性与安全性。

### 6.2 难点评估与应对
- **no_std嵌入式环境适配**：需自定义内存分配器、panic处理与硬件抽象层，可复用`cortex-m-rt`、`embedded-hal`等成熟嵌入式生态组件；
- **C/Rust混合调用**：全局变量、裸指针、回调函数的类型映射与生命周期管理，通过`bindgen`自动生成绑定，配合`#[repr(C)]`结构体保证内存布局兼容；
- **实时性保障**：调度器、中断上下文的临界区管理，通过Rust `unsafe`块隔离硬件操作，确保无额外延迟；
- **模块协同验证**：多阶段改写模块的联调测试，通过单元测试+QEMU仿真+真机验证三级测试流程保障模块兼容性。

## 7. 测试与验证方案
1. **单模块单元测试**：对每个改写模块（如排序链表、任务管理）编写`cargo test`用例，验证数据结构与核心逻辑的正确性；
2. **FFI交互测试**：验证Rust模块与C代码的双向调用、参数传递与返回值正确性；
3. **QEMU仿真测试**：在ARM/RISC-V虚拟机加载重构内核，运行任务调度、IPC通信等核心场景，验证功能完整性；
4. **真机验证**：在STM32、RISC-V开发板烧录内核，测试任务切换、调度延迟、内存稳定性等指标，确保实时性与可靠性；
5. **IronClaw接口测试**：通过用户态应用调用系统接口，验证双向调用的安全性与兼容性。