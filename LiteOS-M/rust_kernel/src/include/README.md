## `los_config_h.rs`的手动修改
我们在`los_config_h.rs`中第68 ~ 69行保留了`RushToLight`小组自己定义的常数：
```rust
pub const MAX_SHRINK_PAGECACHE_TRY: u32 = 10;
pub const PAGE_SHIFT: u32 = 10;
```
## `los_compiler_h.rs`的手动修改
我们在`los_compiler_h.rs`中手动添加了第7行，因为bindgen工具无法识别在`los_compiler.h`中`#define LOS_NOK (UINT32)(-1)`这种自然溢出的转换。
```rust
pub const LOS_NOK: u32 = u32::MAX;
```
在`los_compiler.h`中还有`#define OS_ERROR (UINT32)(-1)`和`#define OS_INVALID (UINT32)(-1)`的宏定义，但是在`los_compiler_h.rs`都没有被bindgen工具自动生成，但是代码中暂时没有用到，所以没有手动添加。
## `los_interrupt_h.rs`的手动修改
我们在`los_interrupt_h.rs`中第200 ~ 250行保留了`RushToLight`小组手动添加的常数和函数:
```rust
// 定义函数类型别名
pub type LOS_IntLock = unsafe fn() -> u32;
pub type LOS_IntRestore = unsafe fn(intSave: u32);
pub type LOS_IntUnLock = unsafe fn() -> u32;
pub type LOS_HwiTrigger = unsafe fn(hwiNum: u32) -> u32;
pub type LOS_HwiEnable = unsafe fn(hwiNum: u32) -> u32;
pub type LOS_HwiDisable = unsafe fn(hwiNum: u32) -> u32;
pub type LOS_HwiClear = unsafe fn(hwiNum: u32) -> u32;
pub type LOS_HwiSetPriority = unsafe fn(hwiNum: u32, priority: u32) -> u32;
pub type LOS_HwiCurIrqNum = unsafe fn() -> u32;
pub type LOS_HwiOpsGet = unsafe fn() -> *mut HwiControllerOps;

// 包装外部C函数以匹配类型别名
pub unsafe fn LOS_IntLock() -> u32 {
    ArchIntLock()
}

pub unsafe fn LOS_IntRestore(intSave: u32) {
    ArchIntRestore(intSave)
}

pub unsafe fn LOS_IntUnLock() -> u32 {
    ArchIntUnLock()
}

pub unsafe fn LOS_HwiTrigger(hwiNum: u32) -> u32 {
    ArchIntTrigger(hwiNum)
}

pub unsafe fn LOS_HwiEnable(hwiNum: u32) -> u32 {
    ArchIntEnable(hwiNum)
}

pub unsafe fn LOS_HwiDisable(hwiNum: u32) -> u32 {
    ArchIntDisable(hwiNum)
}

pub unsafe fn LOS_HwiClear(hwiNum: u32) -> u32 {
    ArchIntClear(hwiNum)
}

pub unsafe fn LOS_HwiSetPriority(hwiNum: u32, priority: u32) -> u32 {
    ArchIntSetPriority(hwiNum, priority.try_into().unwrap())
}

pub unsafe fn LOS_HwiCurIrqNum() -> u32 {
    ArchIntCurIrqNum()
}

pub unsafe fn LOS_HwiOpsGet() -> *mut HwiControllerOps {
    ArchIntOpsGet()
}
```
事实上，`los_interrupt.h`中有关于它们的宏定义，但是是间接定义的，导致bindgen无法识别。部分代码如下：
```c
UINT32 ArchIntLock(VOID);
#define LOS_IntLock ArchIntLock
```
## `los_hook_h.rs`的手动修改
我们在`los_hook_h.rs`中第3 ~ 4行保留了`RushToLight`小组自己定义的常数：
```rust
pub const LOS_HOOK_TYPE_MEM_INIT: u32 = 1;
pub const LOS_HOOK_TYPE_MEM_ALLOC: u32 = 2;
```
## `los_memory_h.rs`中可能存在的风险
我们直接使用了`RushToLight`小组的`los_memory_h.rs`，但是可能会存在风险。

经过我们的检查，`los_memory_h.rs`中的第584 ~ 588行可能会导致内存崩溃：
```rust
pub struct OsMemNodeHead {
    pub ptr: OsMemNodeHead__bindgen_ty_1,
    pub linkReg: [usize; LOSCFG_MEM_RECORD_LR_CNT as usize], // LOSCFG_MEM_LEAKCHECK = 1 时生效
    pub sizeAndFlag: UINT32,
}
```
其中的`pub linkReg: [usize; LOSCFG_MEM_RECORD_LR_CNT as usize]`是手动添加的。

事实上，bindgen转化的Rust代码对这个结构体长度有静态断言：
```rust
["Size of OsMemNodeHead"][::std::mem::size_of::<OsMemNodeHead>() - 16usize];
```
`RushToLight`小组手动把这些静态断言删去了，可能会有未知风险。