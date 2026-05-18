`los_memory.rs`是`RushToLight`小组的成果，但是有高风险代码存在。这些代码绕过了Rust内存管理的安全性，仍然存在内存风险。
### 高风险代码段 1
在第859 ~ 862行，`g_leakCheckRecord`被定义为全局可变的：
```rust
static mut g_leakCheckRecord: [OsMemLeakCheckInfo; LOSCFG_MEM_LEAKCHECK_RECORD_MAX_NUM as usize] = [OsMemLeakCheckInfo {
    node: null_mut(),                    // 初始化 node 为 null 指针
    linkReg: [0; LOSCFG_MEM_RECORD_LR_CNT as usize],        // 初始化 link_reg 数组为全 0
}; LOSCFG_MEM_LEAKCHECK_RECORD_MAX_NUM as usize];
```
而在第889行，对`g_leakCheckRecord`创建了引用：
```rust
g_leakCheckRecord.as_mut_ptr() as *mut c_void
```
对全局可变变量创建引用可能会导致数据竞争等问题。
### 高风险代码段 2
在第107行，`m_aucSysMem0`被定义为全局可变的：
```rust
static mut m_aucSysMem0: *mut u8 = null_mut();
```
而在2761行，对`m_aucSysMem0`创建了引用：
```rust
unsafe{m_aucSysMem0 = g_memStart.as_mut_ptr()};
```
并在2768行，共享了`m_aucSysMem0`的引用：
```rust
unsafe{println!("LiteOS heap memory address: {:p}, size: 0x{:x}", m_aucSysMem0, LOSCFG_SYS_HEAP_SIZE as usize)};
```
对全局可变变量创建引用可能会导致数据竞争等问题。