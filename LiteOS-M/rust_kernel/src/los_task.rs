#![crate_type = "staticlib"]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(static_mut_refs)]

//! Rust rewrite of `c_kernel/src/los_task.c`.
//!
//! The file intentionally keeps the C ABI, global symbols and raw-pointer based
//! control-block manipulation used by LiteOS-M. Most functions are one-to-one
//! translations of `los_task.c`; compile-time disabled C branches (CPUP, task
//! switch monitor, hardware stack protection, kernel signal) are reduced to
//! no-op stubs or omitted paths according to the configuration present in the
//! supplied bindgen headers.

mod include {
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(non_upper_case_globals)]
    #![allow(dead_code)]
    #![allow(unused_imports)]
    pub mod los_task_h;
    pub mod los_sem_h;
    pub mod los_mux_h;
}

use crate::include::los_mux_h;
use crate::include::los_sem_h;
use crate::include::los_task_h as task;
use std::ffi::c_void;
use std::mem;
use std::ptr;

pub type UINT8 = task::UINT8;
pub type UINT16 = task::UINT16;
pub type UINT32 = task::UINT32;
pub type UINT64 = task::UINT64;
pub type UINTPTR = task::UINTPTR;
pub type INT32 = task::INT32;
pub type BOOL = task::BOOL;
pub type CHAR = task::CHAR;
pub type LOS_DL_LIST = task::LOS_DL_LIST;
pub type LosTaskCB = task::LosTaskCB;
pub type LosTask = task::LosTask;
pub type TSK_INIT_PARAM_S = task::TSK_INIT_PARAM_S;
pub type TSK_INFO_S = task::TSK_INFO_S;
pub type TSK_ENTRY_FUNC = task::TSK_ENTRY_FUNC;

const TRUE: BOOL = 1;
const FALSE: BOOL = 0;
const LOS_OK: UINT32 = task::LOS_OK;
const LOS_NOK: UINT32 = u32::MAX;
const OS_ERROR: UINT32 = u32::MAX;
const OS_INVALID: UINT32 = u32::MAX;
const OS_NULL_INT: UINT32 = u32::MAX;
const OS_NULL_SHORT: UINT16 = u16::MAX;
const EOK: i32 = 0;

const LOS_MOD_TSK: UINT32 = 0x2;
const fn los_errno_os_error(module_id: UINT32, errno: UINT32) -> UINT32 {
    0x0200_0000 | (module_id << 8) | errno
}
const fn los_errno_os_fatal(module_id: UINT32, errno: UINT32) -> UINT32 {
    0x0300_0000 | (module_id << 8) | errno
}

const LOS_ERRNO_TSK_NO_MEMORY: UINT32 = los_errno_os_fatal(LOS_MOD_TSK, 0x00);
const LOS_ERRNO_TSK_PTR_NULL: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x01);
const LOS_ERRNO_TSK_PRIOR_ERROR: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x03);
const LOS_ERRNO_TSK_ENTRY_NULL: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x04);
const LOS_ERRNO_TSK_NAME_EMPTY: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x05);
const LOS_ERRNO_TSK_STKSZ_TOO_SMALL: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x06);
const LOS_ERRNO_TSK_ID_INVALID: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x07);
const LOS_ERRNO_TSK_ALREADY_SUSPENDED: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x08);
const LOS_ERRNO_TSK_NOT_SUSPENDED: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x09);
const LOS_ERRNO_TSK_NOT_CREATED: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x0a);
const LOS_ERRNO_TSK_DELAY_IN_INT: UINT32 = los_errno_os_fatal(LOS_MOD_TSK, 0x0d);
const LOS_ERRNO_TSK_DELAY_IN_LOCK: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x0e);
const LOS_ERRNO_TSK_TCB_UNAVAILABLE: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x11);
const LOS_ERRNO_TSK_OPERATE_SYSTEM_TASK: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x14);
const LOS_ERRNO_TSK_OPERATE_IDLE: UINT32 = LOS_ERRNO_TSK_OPERATE_SYSTEM_TASK;
const LOS_ERRNO_TSK_SUSPEND_LOCKED: UINT32 = los_errno_os_fatal(LOS_MOD_TSK, 0x15);
const LOS_ERRNO_TSK_STKSZ_TOO_LARGE: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x20);
const LOS_ERRNO_TSK_SUSPEND_SWTMR_NOT_ALLOWED: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x21);
const LOS_ERRNO_TSK_OPERATE_SWTMR: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x22);
const LOS_ERRNO_TSK_NOT_JOIN: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x24);
const LOS_ERRNO_TSK_NOT_JOIN_SELF: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x25);
const LOS_ERRNO_TSK_NOT_ALLOW_IN_INT: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x26);
const LOS_ERRNO_TSK_ALREADY_EXIT: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x27);
const LOS_ERRNO_TSK_SCHED_LOCKED: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x28);
const LOS_ERRNO_TSK_PROCESS_SIGNAL: UINT32 = los_errno_os_error(LOS_MOD_TSK, 0x29);

const OS_TASK_STACK_TOP_OFFSET: UINT32 = 4;
const LOSCFG_BASE_CORE_TSK_MIN_STACK_SIZE: UINT32 = 0x80;
const OS_SORT_LINK_INVALID_TIME: UINT64 = u64::MAX;

#[no_mangle]
pub static mut g_taskCBArray: *mut LosTaskCB = ptr::null_mut();

#[no_mangle]
pub static mut g_losTask: LosTask = LosTask {
    runTask: ptr::null_mut(),
    newTask: ptr::null_mut(),
};

#[no_mangle]
pub static mut g_losTaskLock: UINT16 = 0;

#[no_mangle]
pub static mut g_taskMaxNum: UINT32 = 0;

#[no_mangle]
pub static mut g_idleTaskID: UINT32 = OS_INVALID;

#[no_mangle]
pub static mut g_swtmrTaskID: UINT32 = OS_INVALID;

#[no_mangle]
pub static mut g_losFreeTask: LOS_DL_LIST = LOS_DL_LIST {
    pstPrev: ptr::null_mut(),
    pstNext: ptr::null_mut(),
};

#[no_mangle]
pub static mut g_taskRecycleList: LOS_DL_LIST = LOS_DL_LIST {
    pstPrev: ptr::null_mut(),
    pstNext: ptr::null_mut(),
};

#[no_mangle]
pub static mut g_taskScheduled: BOOL = FALSE;

static mut PmEnter: Option<unsafe extern "C" fn()> = None;

static IDLE_TASK_NAME: &[u8; 12] = b"IdleCore000\0";
static STATUS_RUNNING: &[u8; 8] = b"Running\0";
static STATUS_READY: &[u8; 6] = b"Ready\0";
static STATUS_EXIT: &[u8; 5] = b"Exit\0";
static STATUS_SUSPEND: &[u8; 8] = b"Suspend\0";
static STATUS_DELAY: &[u8; 6] = b"Delay\0";
static STATUS_PEND_TIME: &[u8; 9] = b"PendTime\0";
static STATUS_PEND: &[u8; 5] = b"Pend\0";
static STATUS_IMPOSSIBLE: &[u8; 11] = b"Impossible\0";

unsafe extern "C" {
    fn LOS_MemAlloc(pool: *mut c_void, size: UINT32) -> *mut c_void;
    fn LOS_MemFree(pool: *mut c_void, ptr: *mut c_void) -> UINT32;
    fn LOS_MemAllocAlign(pool: *mut c_void, size: UINT32, boundary: UINT32) -> *mut c_void;

    fn OsSchedSetIdleTaskSchedParam(idleTask: *mut LosTaskCB);
    fn OsSchedTaskEnQueue(taskCB: *mut LosTaskCB);
    fn OsSchedTaskWake(resumedTask: *mut LosTaskCB);
    fn OsSchedTaskWait(list: *mut LOS_DL_LIST, timeout: UINT32);
    fn OsSchedModifyTaskSchedParam(taskCB: *mut LosTaskCB, priority: UINT16) -> BOOL;
    fn OsSchedDelay(runTask: *mut LosTaskCB, tick: UINT32);
    fn OsSchedYield();
    fn OsSchedTaskExit(taskCB: *mut LosTaskCB);
    fn OsSchedSuspend(taskCB: *mut LosTaskCB);
    fn OsSchedResume(taskCB: *mut LosTaskCB) -> BOOL;
    fn OsSchedInit() -> UINT32;
    fn OsGetTopTask() -> *mut LosTaskCB;
    fn LOS_Schedule();
}

macro_rules! print_err {
    ($($arg:tt)*) => {{
        #[cfg(feature = "LOSCFG_KERNEL_PRINTF")]
        {
            eprintln!($($arg)*);
        }
    }};
}

#[inline]
unsafe fn memset_s(dest: *mut c_void, dest_size: usize, value: i32, count: usize) -> i32 {
    if dest.is_null() || count > dest_size {
        return -1;
    }
    ptr::write_bytes(dest, value as u8, count);
    EOK
}

#[inline]
unsafe fn memcpy_s(dest: *mut c_void, dest_size: usize, src: *const c_void, count: usize) -> i32 {
    if dest.is_null() || src.is_null() || count > dest_size {
        return -1;
    }
    ptr::copy_nonoverlapping(src as *const u8, dest as *mut u8, count);
    EOK
}

#[inline]
unsafe fn strncpy_s(dest: *mut CHAR, dest_size: usize, src: *const CHAR, count: usize) -> i32 {
    if dest.is_null() || src.is_null() || dest_size == 0 {
        return -1;
    }
    let mut i = 0usize;
    while i < count && i + 1 < dest_size {
        let ch = *src.add(i);
        *dest.add(i) = ch;
        if ch == 0 {
            return EOK;
        }
        i += 1;
    }
    if i < dest_size {
        *dest.add(i) = 0;
    } else {
        *dest.add(dest_size - 1) = 0;
    }
    EOK
}

#[inline]
fn align(value: UINT32, boundary: UINT32) -> UINT32 {
    if boundary == 0 {
        value
    } else {
        (value + boundary - 1) & !(boundary - 1)
    }
}

#[inline]
fn truncate(value: UINT32, boundary: UINT32) -> UINT32 {
    if boundary == 0 {
        value
    } else {
        value & !(boundary - 1)
    }
}

#[inline]
fn has_flag(status: UINT16, flag: UINT32) -> bool {
    ((status as UINT32) & flag) != 0
}

#[inline]
unsafe fn os_task_stack_addr() -> *mut c_void {
    task::m_aucSysMem0 as *mut c_void
}

#[inline]
unsafe fn os_tcb_from_tid(task_id: UINT32) -> *mut LosTaskCB {
    g_taskCBArray.add(task_id as usize)
}

#[inline]
fn los_taskcb_pendlist_offset() -> usize {
    let uninit = mem::MaybeUninit::<LosTaskCB>::uninit();
    let base = uninit.as_ptr();
    unsafe { (ptr::addr_of!((*base).pendList) as usize).wrapping_sub(base as usize) }
}

#[inline]
unsafe fn os_tcb_from_pendlist(ptr: *mut LOS_DL_LIST) -> *mut LosTaskCB {
    (ptr as *mut u8).sub(los_taskcb_pendlist_offset()) as *mut LosTaskCB
}

#[inline]
unsafe fn LOS_DL_LIST_FIRST(list: *mut LOS_DL_LIST) -> *mut LOS_DL_LIST {
    (*list).pstNext
}

#[inline]
unsafe fn LOS_ListInit(list: *mut LOS_DL_LIST) {
    (*list).pstNext = list;
    (*list).pstPrev = list;
}

#[inline]
unsafe fn LOS_ListAdd(list: *mut LOS_DL_LIST, node: *mut LOS_DL_LIST) {
    (*node).pstNext = (*list).pstNext;
    (*node).pstPrev = list;
    (*(*list).pstNext).pstPrev = node;
    (*list).pstNext = node;
}

#[inline]
unsafe fn LOS_ListTailInsert(list: *mut LOS_DL_LIST, node: *mut LOS_DL_LIST) {
    LOS_ListAdd((*list).pstPrev, node);
}

#[inline]
unsafe fn LOS_ListDelete(node: *mut LOS_DL_LIST) {
    (*(*node).pstNext).pstPrev = (*node).pstPrev;
    (*(*node).pstPrev).pstNext = (*node).pstNext;
    (*node).pstNext = ptr::null_mut();
    (*node).pstPrev = ptr::null_mut();
}

#[inline]
unsafe fn LOS_ListEmpty(node: *const LOS_DL_LIST) -> bool {
    (*node).pstNext == node as *mut LOS_DL_LIST
}

#[inline]
unsafe fn OS_INT_ACTIVE() -> bool {
    task::ArchIsIntActive() != 0
}

#[inline]
unsafe fn LOS_IntLock() -> UINT32 {
    task::ArchIntLock()
}

#[inline]
unsafe fn LOS_IntRestore(int_save: UINT32) {
    task::ArchIntRestore(int_save);
}

#[inline]
unsafe fn os_task_id_check(task_id: UINT32) -> bool {
    task_id < g_taskMaxNum
}

#[inline]
unsafe fn os_check_tsk_pid_noidle(task_id: UINT32) -> bool {
    task_id >= g_taskMaxNum
}

#[inline]
unsafe fn set_sortlist_value(sort_list: *mut task::SortLinkList, value: UINT64) {
    (*sort_list).responseTime = value;
}

#[inline]
unsafe fn loscfg_task_create_extension_hook(_task_cb: *mut LosTaskCB) {}

#[inline]
unsafe fn loscfg_task_delete_extension_hook(_task_cb: *mut LosTaskCB) {}

#[inline]
unsafe fn os_hook_task_create(_task_cb: *const LosTaskCB) {}

#[inline]
unsafe fn os_hook_task_delete(_task_cb: *const LosTaskCB) {}

#[inline]
unsafe fn os_hook_task_delay(_tick: UINT32) {}

#[inline]
unsafe fn os_hook_moved_task_to_delayed_list(_task_cb: *const LosTaskCB) {}

unsafe fn OsCheckTaskIDValid(task_id: UINT32) -> UINT32 {
    if task_id == g_idleTaskID {
        return LOS_ERRNO_TSK_OPERATE_IDLE;
    }
    if task_id == g_swtmrTaskID {
        return LOS_ERRNO_TSK_SUSPEND_SWTMR_NOT_ALLOWED;
    }
    if task_id >= g_taskMaxNum {
        return LOS_ERRNO_TSK_ID_INVALID;
    }
    LOS_OK
}

unsafe fn OsInsertTCBToFreeList(task_cb: *mut LosTaskCB) {
    let task_id = (*task_cb).taskID;
    let _ = memset_s(
        task_cb as *mut c_void,
        mem::size_of::<LosTaskCB>(),
        0,
        mem::size_of::<LosTaskCB>(),
    );
    (*task_cb).taskID = task_id;
    (*task_cb).taskStatus = task::OS_TASK_STATUS_UNUSED as UINT16;
    LOS_ListAdd(ptr::addr_of_mut!(g_losFreeTask), ptr::addr_of_mut!((*task_cb).pendList));
}

unsafe fn OsRecycleTaskResources(task_cb: *mut LosTaskCB, stack_ptr: *mut UINTPTR) {
    if has_flag((*task_cb).taskStatus, task::OS_TASK_FLAG_STACK_FREE) && (*task_cb).topOfStack != 0 {
        *stack_ptr = (*task_cb).topOfStack;
        (*task_cb).topOfStack = 0;
        (*task_cb).taskStatus &= !(task::OS_TASK_FLAG_STACK_FREE as UINT16);
    }
    if !has_flag((*task_cb).taskStatus, task::OS_TASK_FLAG_JOINABLE) {
        OsInsertTCBToFreeList(task_cb);
    }
}

unsafe fn OsRecycleFinishedTask() {
    let mut int_save = LOS_IntLock();
    while !LOS_ListEmpty(ptr::addr_of!(g_taskRecycleList)) {
        let first = LOS_DL_LIST_FIRST(ptr::addr_of_mut!(g_taskRecycleList));
        let task_cb = os_tcb_from_pendlist(first);
        LOS_ListDelete(first);
        let mut stack_ptr: UINTPTR = 0;
        OsRecycleTaskResources(task_cb, &mut stack_ptr as *mut UINTPTR);
        LOS_IntRestore(int_save);

        let _ = LOS_MemFree(os_task_stack_addr(), stack_ptr as usize as *mut c_void);
        int_save = LOS_IntLock();
    }
    LOS_IntRestore(int_save);
}

#[no_mangle]
pub unsafe extern "C" fn OsPmEnterHandlerSet(func: Option<unsafe extern "C" fn()>) -> UINT32 {
    if func.is_none() {
        return LOS_NOK;
    }
    PmEnter = func;
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn OsIdleTask(_arg: UINT32) -> *mut c_void {
    loop {
        OsRecycleFinishedTask();
        if let Some(pm_enter) = PmEnter {
            pm_enter();
        } else {
            let _ = task::ArchEnterSleep();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn OsConvertTskStatus(task_status: UINT16) -> *mut UINT8 {
    if has_flag(task_status, task::OS_TASK_STATUS_RUNNING) {
        STATUS_RUNNING.as_ptr() as *mut UINT8
    } else if has_flag(task_status, task::OS_TASK_STATUS_READY) {
        STATUS_READY.as_ptr() as *mut UINT8
    } else if has_flag(task_status, task::OS_TASK_STATUS_EXIT) {
        STATUS_EXIT.as_ptr() as *mut UINT8
    } else if has_flag(task_status, task::OS_TASK_STATUS_SUSPEND) {
        STATUS_SUSPEND.as_ptr() as *mut UINT8
    } else if has_flag(task_status, task::OS_TASK_STATUS_DELAY) {
        STATUS_DELAY.as_ptr() as *mut UINT8
    } else if has_flag(task_status, task::OS_TASK_STATUS_PEND) {
        if has_flag(task_status, task::OS_TASK_STATUS_PEND_TIME) {
            STATUS_PEND_TIME.as_ptr() as *mut UINT8
        } else {
            STATUS_PEND.as_ptr() as *mut UINT8
        }
    } else {
        STATUS_IMPOSSIBLE.as_ptr() as *mut UINT8
    }
}

#[no_mangle]
pub unsafe extern "C" fn OsGetTaskWaterLine(task_id: UINT32) -> UINT32 {
    let task_cb = os_tcb_from_tid(task_id);
    let top = (*task_cb).topOfStack as usize as *mut UINT32;
    if !top.is_null() && *top == task::OS_TASK_MAGIC_WORD {
        let mut stack_ptr = ((*task_cb).topOfStack + OS_TASK_STACK_TOP_OFFSET) as usize as *mut UINT32;
        while (stack_ptr as usize) < ((*task_cb).stackPointer as usize)
            && *stack_ptr == task::OS_TASK_STACK_INIT
        {
            stack_ptr = stack_ptr.add(1);
        }
        (*task_cb)
            .stackSize
            .wrapping_sub((stack_ptr as UINT32).wrapping_sub((*task_cb).topOfStack))
    } else {
        print_err!("CURRENT task stack overflow!");
        OS_NULL_INT
    }
}

unsafe fn PrintTskInfo(_task_cb: *const LosTaskCB) {}
unsafe fn PrintTskInfoHeader() {}

#[no_mangle]
pub unsafe extern "C" fn OsGetAllTskInfo() -> UINT32 {
    PrintTskInfoHeader();
    let mut loop_num: UINT32 = 0;
    while loop_num < g_taskMaxNum {
        let task_cb = g_taskCBArray.add(loop_num as usize);
        if !has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_UNUSED) {
            PrintTskInfo(task_cb);
        }
        loop_num += 1;
    }
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn OsTaskInit() -> UINT32 {
    g_taskMaxNum = task::LOSCFG_BASE_CORE_TSK_LIMIT + 1;
    let size = (g_taskMaxNum + 1) * mem::size_of::<LosTaskCB>() as UINT32;
    g_taskCBArray = LOS_MemAlloc(task::m_aucSysMem0 as *mut c_void, size) as *mut LosTaskCB;
    if g_taskCBArray.is_null() {
        return LOS_ERRNO_TSK_NO_MEMORY;
    }

    let _ = memset_s(g_taskCBArray as *mut c_void, size as usize, 0, size as usize);
    LOS_ListInit(ptr::addr_of_mut!(g_losFreeTask));
    LOS_ListInit(ptr::addr_of_mut!(g_taskRecycleList));

    let mut index: UINT32 = 0;
    while index <= task::LOSCFG_BASE_CORE_TSK_LIMIT {
        let cb = g_taskCBArray.add(index as usize);
        (*cb).taskStatus = task::OS_TASK_STATUS_UNUSED as UINT16;
        (*cb).taskID = index;
        LOS_ListTailInsert(ptr::addr_of_mut!(g_losFreeTask), ptr::addr_of_mut!((*cb).pendList));
        index += 1;
    }

    g_losTask.runTask = g_taskCBArray.add(g_taskMaxNum as usize);
    g_losTask.newTask = ptr::null_mut();
    (*g_losTask.runTask).taskID = index;
    (*g_losTask.runTask).taskStatus = (task::OS_TASK_STATUS_UNUSED | task::OS_TASK_STATUS_RUNNING) as UINT16;
    (*g_losTask.runTask).priority = (task::OS_TASK_PRIORITY_LOWEST + 1) as UINT16;

    g_idleTaskID = OS_INVALID;
    OsSchedInit()
}

#[no_mangle]
pub unsafe extern "C" fn OsIdleTaskCreate() -> UINT32 {
    let mut task_init_param: TSK_INIT_PARAM_S = mem::zeroed();
    task_init_param.pfnTaskEntry = Some(OsIdleTask);
    task_init_param.uwStackSize = task::LOSCFG_BASE_CORE_TSK_IDLE_STACK_SIZE;
    task_init_param.pcName = IDLE_TASK_NAME.as_ptr() as *mut CHAR;
    task_init_param.usTaskPrio = task::OS_TASK_PRIORITY_LOWEST as UINT16;

    let ret_val = LOS_TaskCreateOnly(ptr::addr_of_mut!(g_idleTaskID), &mut task_init_param as *mut TSK_INIT_PARAM_S);
    if ret_val != LOS_OK {
        return ret_val;
    }

    OsSchedSetIdleTaskSchedParam(os_tcb_from_tid(g_idleTaskID));
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_CurTaskIDGet() -> UINT32 {
    if g_losTask.runTask.is_null() {
        return LOS_ERRNO_TSK_ID_INVALID;
    }
    (*g_losTask.runTask).taskID
}

#[no_mangle]
pub unsafe extern "C" fn LOS_NextTaskIDGet() -> UINT32 {
    let int_save = LOS_IntLock();
    let top_task = OsGetTopTask();
    let task_id = if top_task.is_null() {
        LOS_ERRNO_TSK_ID_INVALID
    } else {
        (*top_task).taskID
    };
    LOS_IntRestore(int_save);
    task_id
}

#[no_mangle]
pub unsafe extern "C" fn LOS_CurTaskNameGet() -> *mut CHAR {
    if g_losTask.runTask.is_null() {
        ptr::null_mut()
    } else {
        (*g_losTask.runTask).taskName
    }
}

#[no_mangle]
pub unsafe extern "C" fn OsTaskSwitchCheck() {}

#[no_mangle]
pub unsafe extern "C" fn OsTaskMonInit() {}

#[no_mangle]
pub unsafe extern "C" fn OsTaskEntry(task_id: UINT32) {
    let task_cb = os_tcb_from_tid(task_id);
    let ret_ptr = if let Some(entry) = (*task_cb).taskEntry {
        entry((*task_cb).arg)
    } else {
        ptr::null_mut()
    };
    (*task_cb).joinRetval = ret_ptr as usize as UINTPTR;
    let ret_val = LOS_TaskDelete((*task_cb).taskID);
    if ret_val != LOS_OK {
        print_err!("Delete Task[TID: {}] Failed!", (*task_cb).taskID);
    }
}

unsafe fn OsTaskInitParamCheck(task_init_param: *mut TSK_INIT_PARAM_S) -> UINT32 {
    if task_init_param.is_null() {
        return LOS_ERRNO_TSK_PTR_NULL;
    }
    if (*task_init_param).pcName.is_null() {
        return LOS_ERRNO_TSK_NAME_EMPTY;
    }
    if (*task_init_param).pfnTaskEntry.is_none() {
        return LOS_ERRNO_TSK_ENTRY_NULL;
    }
    if (*task_init_param).usTaskPrio as UINT32 > task::OS_TASK_PRIORITY_LOWEST {
        return LOS_ERRNO_TSK_PRIOR_ERROR;
    }

    if (*task_init_param).usTaskPrio as UINT32 == task::OS_TASK_PRIORITY_LOWEST {
        let entry = (*task_init_param).pfnTaskEntry.map(|f| f as usize).unwrap_or(0);
        if entry != (OsIdleTask as unsafe extern "C" fn(UINT32) -> *mut c_void) as usize {
            return LOS_ERRNO_TSK_PRIOR_ERROR;
        }
    }

    if (*task_init_param).uwStackSize > task::LOSCFG_SYS_HEAP_SIZE {
        return LOS_ERRNO_TSK_STKSZ_TOO_LARGE;
    }
    if (*task_init_param).uwStackSize == 0 {
        (*task_init_param).uwStackSize = task::LOSCFG_BASE_CORE_TSK_DEFAULT_STACK_SIZE;
    }
    if (*task_init_param).uwStackSize < LOSCFG_BASE_CORE_TSK_MIN_STACK_SIZE {
        return LOS_ERRNO_TSK_STKSZ_TOO_SMALL;
    }
    LOS_OK
}

unsafe fn OsNewTaskInit(task_cb: *mut LosTaskCB, task_init_param: *mut TSK_INIT_PARAM_S) -> UINT32 {
    (*task_cb).arg = (*task_init_param).uwArg;
    (*task_cb).stackSize = (*task_init_param).uwStackSize;
    (*task_cb).taskSem = ptr::null_mut();
    (*task_cb).taskMux = ptr::null_mut();
    (*task_cb).taskStatus = task::OS_TASK_STATUS_SUSPEND as UINT16;
    (*task_cb).priority = (*task_init_param).usTaskPrio;
    (*task_cb).timeSlice = 0;
    (*task_cb).waitTimes = 0;
    (*task_cb).taskEntry = (*task_init_param).pfnTaskEntry;
    (*task_cb).event.uwEventID = OS_NULL_INT;
    (*task_cb).eventMask = 0;
    (*task_cb).taskName = (*task_init_param).pcName;
    (*task_cb).msg = ptr::null_mut();
    (*task_cb).errorNo = 0;

    set_sortlist_value(ptr::addr_of_mut!((*task_cb).sortList), OS_SORT_LINK_INVALID_TIME);
    let _ = task::LOS_EventInit(ptr::addr_of_mut!((*task_cb).event));

    if ((*task_init_param).uwResved & task::LOS_TASK_ATTR_JOINABLE) != 0 {
        (*task_cb).taskStatus |= task::OS_TASK_FLAG_JOINABLE as UINT16;
        LOS_ListInit(ptr::addr_of_mut!((*task_cb).joinList));
    }

    if (*task_init_param).stackAddr == 0 {
        (*task_cb).stackSize = align((*task_init_param).uwStackSize, task::OS_TASK_STACK_ADDR_ALIGN);
        let stack_ptr = LOS_MemAllocAlign(
            os_task_stack_addr(),
            (*task_cb).stackSize,
            task::LOSCFG_STACK_POINT_ALIGN_SIZE,
        );
        (*task_cb).topOfStack = stack_ptr as usize as UINT32;
        if (*task_cb).topOfStack == 0 {
            return LOS_ERRNO_TSK_NO_MEMORY;
        }
        (*task_cb).taskStatus |= task::OS_TASK_FLAG_STACK_FREE as UINT16;
    } else {
        (*task_cb).topOfStack = align((*task_init_param).stackAddr, task::LOSCFG_STACK_POINT_ALIGN_SIZE);
        (*task_cb).stackSize = (*task_init_param)
            .uwStackSize
            .wrapping_sub((*task_cb).topOfStack.wrapping_sub((*task_init_param).stackAddr));
        (*task_cb).stackSize = truncate((*task_cb).stackSize, task::OS_TASK_STACK_ADDR_ALIGN);
    }

    let _ = memset_s(
        (*task_cb).topOfStack as usize as *mut c_void,
        (*task_cb).stackSize as usize,
        (task::OS_TASK_STACK_INIT & 0xff) as i32,
        (*task_cb).stackSize as usize,
    );
    *((*task_cb).topOfStack as usize as *mut UINT32) = task::OS_TASK_MAGIC_WORD;
    (*task_cb).stackPointer = task::ArchTskStackInit(
        (*task_cb).taskID,
        (*task_cb).stackSize,
        (*task_cb).topOfStack as usize as *mut c_void,
    );
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskCreateOnly(
    task_id: *mut UINT32,
    task_init_param: *mut TSK_INIT_PARAM_S,
) -> UINT32 {
    if task_id.is_null() {
        return LOS_ERRNO_TSK_ID_INVALID;
    }

    let mut ret_val = OsTaskInitParamCheck(task_init_param);
    if ret_val != LOS_OK {
        return ret_val;
    }

    OsRecycleFinishedTask();

    let mut int_save = LOS_IntLock();
    if LOS_ListEmpty(ptr::addr_of!(g_losFreeTask)) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_TCB_UNAVAILABLE;
    }

    let first = LOS_DL_LIST_FIRST(ptr::addr_of_mut!(g_losFreeTask));
    let task_cb = os_tcb_from_pendlist(first);
    LOS_ListDelete(first);
    LOS_IntRestore(int_save);

    ret_val = OsNewTaskInit(task_cb, task_init_param);
    if ret_val != LOS_OK {
        int_save = LOS_IntLock();
        OsInsertTCBToFreeList(task_cb);
        LOS_IntRestore(int_save);
        return ret_val;
    }

    loscfg_task_create_extension_hook(task_cb);
    *task_id = (*task_cb).taskID;
    os_hook_task_create(task_cb);
    ret_val
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskCreate(
    task_id: *mut UINT32,
    task_init_param: *mut TSK_INIT_PARAM_S,
) -> UINT32 {
    let ret_val = LOS_TaskCreateOnly(task_id, task_init_param);
    if ret_val != LOS_OK {
        return ret_val;
    }

    let task_cb = os_tcb_from_tid(*task_id);
    let int_save = LOS_IntLock();
    OsSchedTaskEnQueue(task_cb);
    LOS_IntRestore(int_save);

    if g_taskScheduled != FALSE {
        LOS_Schedule();
    }
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskResume(task_id: UINT32) -> UINT32 {
    if !os_task_id_check(task_id) {
        return LOS_ERRNO_TSK_ID_INVALID;
    }

    let task_cb = os_tcb_from_tid(task_id);
    let int_save = LOS_IntLock();
    let temp_status = (*task_cb).taskStatus;

    if has_flag(temp_status, task::OS_TASK_STATUS_UNUSED) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_NOT_CREATED;
    }
    if !has_flag(temp_status, task::OS_TASK_STATUS_SUSPEND) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_NOT_SUSPENDED;
    }

    let need_sched = OsSchedResume(task_cb);
    if need_sched != FALSE && g_taskScheduled != FALSE {
        LOS_IntRestore(int_save);
        LOS_Schedule();
        return LOS_OK;
    }

    LOS_IntRestore(int_save);
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskSuspend(task_id: UINT32) -> UINT32 {
    let mut ret_err = OsCheckTaskIDValid(task_id);
    if ret_err != LOS_OK {
        return ret_err;
    }

    let task_cb = os_tcb_from_tid(task_id);
    let int_save = LOS_IntLock();
    let temp_status = (*task_cb).taskStatus;

    if has_flag(temp_status, task::OS_TASK_STATUS_UNUSED) {
        ret_err = LOS_ERRNO_TSK_NOT_CREATED;
    } else if has_flag(temp_status, task::OS_TASK_FLAG_SYSTEM_TASK) {
        ret_err = LOS_ERRNO_TSK_OPERATE_SYSTEM_TASK;
    } else if has_flag(temp_status, task::OS_TASK_STATUS_SUSPEND) {
        ret_err = LOS_ERRNO_TSK_ALREADY_SUSPENDED;
    } else if has_flag(temp_status, task::OS_TASK_STATUS_RUNNING) && g_losTaskLock != 0 {
        ret_err = LOS_ERRNO_TSK_SUSPEND_LOCKED;
    }

    if ret_err != LOS_OK {
        LOS_IntRestore(int_save);
        return ret_err;
    }

    OsSchedSuspend(task_cb);
    if !g_losTask.runTask.is_null() && task_id == (*g_losTask.runTask).taskID {
        LOS_IntRestore(int_save);
        LOS_Schedule();
        return LOS_OK;
    }

    LOS_IntRestore(int_save);
    LOS_OK
}

unsafe fn OsTaskJoinPostUnsafe(task_cb: *mut LosTaskCB) {
    if has_flag((*task_cb).taskStatus, task::OS_TASK_FLAG_JOINABLE)
        && !LOS_ListEmpty(ptr::addr_of!((*task_cb).joinList))
    {
        let resumed_task = os_tcb_from_pendlist(LOS_DL_LIST_FIRST(ptr::addr_of_mut!((*task_cb).joinList)));
        OsSchedTaskWake(resumed_task);
    }
}

unsafe fn OsTaskJoinPendUnsafe(task_cb: *mut LosTaskCB) -> UINT32 {
    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_EXIT) {
        return LOS_OK;
    }
    if has_flag((*task_cb).taskStatus, task::OS_TASK_FLAG_JOINABLE)
        && LOS_ListEmpty(ptr::addr_of!((*task_cb).joinList))
    {
        OsSchedTaskWait(ptr::addr_of_mut!((*task_cb).joinList), task::LOS_WAIT_FOREVER);
        return LOS_OK;
    }
    LOS_NOK
}

unsafe fn OsTaskSetDetachUnsafe(task_cb: *mut LosTaskCB) -> UINT32 {
    if has_flag((*task_cb).taskStatus, task::OS_TASK_FLAG_JOINABLE) {
        if LOS_ListEmpty(ptr::addr_of!((*task_cb).joinList)) {
            LOS_ListDelete(ptr::addr_of_mut!((*task_cb).joinList));
            (*task_cb).taskStatus &= !(task::OS_TASK_FLAG_JOINABLE as UINT16);
            return LOS_OK;
        }
        return LOS_ERRNO_TSK_NOT_JOIN;
    }
    LOS_NOK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskJoin(task_id: UINT32, retval: *mut UINTPTR) -> UINT32 {
    let mut ret = OsCheckTaskIDValid(task_id);
    if ret != LOS_OK {
        return ret;
    }
    if OS_INT_ACTIVE() {
        return LOS_ERRNO_TSK_NOT_ALLOW_IN_INT;
    }
    if g_losTaskLock != 0 {
        return LOS_ERRNO_TSK_SCHED_LOCKED;
    }
    if task_id == LOS_CurTaskIDGet() {
        return LOS_ERRNO_TSK_NOT_JOIN_SELF;
    }

    let task_cb = os_tcb_from_tid(task_id);
    let mut int_save = LOS_IntLock();
    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_UNUSED) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_NOT_CREATED;
    }

    ret = OsTaskJoinPendUnsafe(task_cb);
    LOS_IntRestore(int_save);
    if ret == LOS_OK {
        LOS_Schedule();
        if !retval.is_null() {
            *retval = (*task_cb).joinRetval;
        }

        let mut stack_ptr: UINTPTR = 0;
        int_save = LOS_IntLock();
        (*task_cb).taskStatus &= !(task::OS_TASK_FLAG_JOINABLE as UINT16);
        OsRecycleTaskResources(task_cb, &mut stack_ptr as *mut UINTPTR);
        LOS_IntRestore(int_save);
        let _ = LOS_MemFree(os_task_stack_addr(), stack_ptr as usize as *mut c_void);
        return LOS_OK;
    }
    ret
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskDetach(task_id: UINT32) -> UINT32 {
    let mut ret = OsCheckTaskIDValid(task_id);
    if ret != LOS_OK {
        return ret;
    }
    if OS_INT_ACTIVE() {
        return LOS_ERRNO_TSK_NOT_ALLOW_IN_INT;
    }

    let task_cb = os_tcb_from_tid(task_id);
    let int_save = LOS_IntLock();
    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_UNUSED) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_NOT_CREATED;
    }
    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_EXIT) {
        LOS_IntRestore(int_save);
        return LOS_TaskJoin(task_id, ptr::null_mut());
    }

    ret = OsTaskSetDetachUnsafe(task_cb);
    LOS_IntRestore(int_save);
    ret
}

unsafe fn OsRunningTaskDelete(task_id: UINT32, task_cb: *mut LosTaskCB) {
    LOS_ListTailInsert(ptr::addr_of_mut!(g_taskRecycleList), ptr::addr_of_mut!((*task_cb).pendList));
    g_losTask.runTask = g_taskCBArray.add(g_taskMaxNum as usize);
    (*g_losTask.runTask).taskID = task_id;
    (*g_losTask.runTask).taskStatus = (*task_cb).taskStatus | task::OS_TASK_STATUS_RUNNING as UINT16;
    (*g_losTask.runTask).topOfStack = (*task_cb).topOfStack;
    (*g_losTask.runTask).taskName = (*task_cb).taskName;
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskDelete(task_id: UINT32) -> UINT32 {
    let ret = OsCheckTaskIDValid(task_id);
    if ret != LOS_OK {
        return ret;
    }

    let task_cb = os_tcb_from_tid(task_id);
    let int_save = LOS_IntLock();

    if has_flag((*task_cb).taskStatus, task::OS_TASK_FLAG_SYSTEM_TASK) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_OPERATE_SYSTEM_TASK;
    }
    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_UNUSED) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_NOT_CREATED;
    }
    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_EXIT) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_ALREADY_EXIT;
    }
    if has_flag((*task_cb).taskStatus, task::OS_TASK_FLAG_SIGNAL) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_PROCESS_SIGNAL;
    }

    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_RUNNING) && g_losTaskLock != 0 {
        g_losTaskLock = 0;
    }

    os_hook_task_delete(task_cb);
    OsTaskJoinPostUnsafe(task_cb);
    OsSchedTaskExit(task_cb);

    let _ = task::LOS_EventDestroy(ptr::addr_of_mut!((*task_cb).event));
    (*task_cb).event.uwEventID = OS_NULL_INT;
    (*task_cb).eventMask = 0;
    loscfg_task_delete_extension_hook(task_cb);

    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_RUNNING) {
        if !has_flag((*task_cb).taskStatus, task::OS_TASK_FLAG_JOINABLE) {
            (*task_cb).taskStatus |= task::OS_TASK_STATUS_UNUSED as UINT16;
            OsRunningTaskDelete(task_id, task_cb);
        }
        LOS_IntRestore(int_save);
        LOS_Schedule();
        return LOS_OK;
    }

    let mut stack_ptr: UINTPTR = 0;
    (*task_cb).joinRetval = LOS_CurTaskIDGet();
    OsRecycleTaskResources(task_cb, &mut stack_ptr as *mut UINTPTR);
    LOS_IntRestore(int_save);
    let _ = LOS_MemFree(os_task_stack_addr(), stack_ptr as usize as *mut c_void);
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskDelay(tick: UINT32) -> UINT32 {
    if OS_INT_ACTIVE() {
        return LOS_ERRNO_TSK_DELAY_IN_INT;
    }
    if g_losTaskLock != 0 {
        return LOS_ERRNO_TSK_DELAY_IN_LOCK;
    }
    if !g_losTask.runTask.is_null() && has_flag((*g_losTask.runTask).taskStatus, task::OS_TASK_FLAG_SYSTEM_TASK) {
        return LOS_ERRNO_TSK_OPERATE_SYSTEM_TASK;
    }

    os_hook_task_delay(tick);
    if tick == 0 {
        return LOS_TaskYield();
    }

    let int_save = LOS_IntLock();
    OsSchedDelay(g_losTask.runTask, tick);
    os_hook_moved_task_to_delayed_list(g_losTask.runTask);
    LOS_IntRestore(int_save);
    LOS_Schedule();
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskPriGet(task_id: UINT32) -> UINT16 {
    if os_check_tsk_pid_noidle(task_id) {
        return OS_INVALID as UINT16;
    }

    let task_cb = os_tcb_from_tid(task_id);
    let int_save = LOS_IntLock();
    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_UNUSED) {
        LOS_IntRestore(int_save);
        return OS_INVALID as UINT16;
    }
    let priority = (*task_cb).priority;
    LOS_IntRestore(int_save);
    priority
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskPriSet(task_id: UINT32, task_prio: UINT16) -> UINT32 {
    if task_prio as UINT32 > task::OS_TASK_PRIORITY_LOWEST {
        return LOS_ERRNO_TSK_PRIOR_ERROR;
    }
    if task_id == g_idleTaskID {
        return LOS_ERRNO_TSK_OPERATE_IDLE;
    }
    if task_id == g_swtmrTaskID {
        return LOS_ERRNO_TSK_OPERATE_SWTMR;
    }
    if os_check_tsk_pid_noidle(task_id) {
        return LOS_ERRNO_TSK_ID_INVALID;
    }

    let task_cb = os_tcb_from_tid(task_id);
    let int_save = LOS_IntLock();
    let temp_status = (*task_cb).taskStatus;
    if has_flag(temp_status, task::OS_TASK_STATUS_UNUSED) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_NOT_CREATED;
    }
    if has_flag(temp_status, task::OS_TASK_FLAG_SYSTEM_TASK) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_OPERATE_SYSTEM_TASK;
    }

    let is_ready = OsSchedModifyTaskSchedParam(task_cb, task_prio);
    LOS_IntRestore(int_save);
    if is_ready != FALSE {
        LOS_Schedule();
    }
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_CurTaskPriSet(task_prio: UINT16) -> UINT32 {
    if g_losTask.runTask.is_null() {
        return LOS_ERRNO_TSK_ID_INVALID;
    }
    LOS_TaskPriSet((*g_losTask.runTask).taskID, task_prio)
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskYield() -> UINT32 {
    let int_save = LOS_IntLock();
    OsSchedYield();
    LOS_IntRestore(int_save);
    LOS_Schedule();
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskLock() {
    let int_save = LOS_IntLock();
    g_losTaskLock = g_losTaskLock.wrapping_add(1);
    LOS_IntRestore(int_save);
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskUnlock() {
    let int_save = LOS_IntLock();
    if g_losTaskLock > 0 {
        g_losTaskLock -= 1;
        if g_losTaskLock == 0 {
            LOS_IntRestore(int_save);
            LOS_Schedule();
            return;
        }
    }
    LOS_IntRestore(int_save);
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskInfoGet(task_id: UINT32, task_info: *mut TSK_INFO_S) -> UINT32 {
    if task_info.is_null() {
        return LOS_ERRNO_TSK_PTR_NULL;
    }
    if os_check_tsk_pid_noidle(task_id) {
        return LOS_ERRNO_TSK_ID_INVALID;
    }

    let task_cb = os_tcb_from_tid(task_id);
    let int_save = LOS_IntLock();
    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_UNUSED) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_NOT_CREATED;
    }

    (*task_info).uwSP = (*task_cb).stackPointer as usize as UINT32;
    (*task_info).usTaskStatus = (*task_cb).taskStatus;
    (*task_info).usTaskPrio = (*task_cb).priority;
    (*task_info).uwStackSize = (*task_cb).stackSize;
    (*task_info).uwTopOfStack = (*task_cb).topOfStack;
    (*task_info).uwEvent = (*task_cb).event;
    (*task_info).uwEventMask = (*task_cb).eventMask;
    (*task_info).uwSemID = if !(*task_cb).taskSem.is_null() {
        (*( (*task_cb).taskSem as *mut los_sem_h::LosSemCB)).semID as UINT32
    } else {
        task::LOSCFG_BASE_IPC_SEM_LIMIT
    };
    (*task_info).uwMuxID = if !(*task_cb).taskMux.is_null() {
        (*( (*task_cb).taskMux as *mut los_mux_h::LosMuxCB)).muxID as UINT32
    } else {
        task::LOSCFG_BASE_IPC_MUX_LIMIT
    };
    (*task_info).pTaskSem = (*task_cb).taskSem;
    (*task_info).pTaskMux = (*task_cb).taskMux;
    (*task_info).uwTaskID = task_id;

    let _ = strncpy_s(
        (*task_info).acName.as_mut_ptr(),
        task::LOS_TASK_NAMELEN as usize,
        (*task_cb).taskName,
        (task::LOS_TASK_NAMELEN - 1) as usize,
    );
    (*task_info).acName[(task::LOS_TASK_NAMELEN - 1) as usize] = 0;

    (*task_info).uwBottomOfStack = truncate(
        (*task_cb).topOfStack.wrapping_add((*task_cb).stackSize),
        task::OS_TASK_STACK_ADDR_ALIGN,
    );
    (*task_info).uwCurrUsed = (*task_info).uwBottomOfStack.wrapping_sub((*task_info).uwSP);
    (*task_info).uwPeakUsed = OsGetTaskWaterLine(task_id);
    (*task_info).bOvf = if (*task_info).uwPeakUsed == OS_NULL_INT { TRUE } else { FALSE };

    LOS_IntRestore(int_save);
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskStatusGet(task_id: UINT32, task_status: *mut UINT32) -> UINT32 {
    if task_status.is_null() {
        return LOS_ERRNO_TSK_PTR_NULL;
    }
    if os_check_tsk_pid_noidle(task_id) {
        return LOS_ERRNO_TSK_ID_INVALID;
    }

    let task_cb = os_tcb_from_tid(task_id);
    let int_save = LOS_IntLock();
    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_UNUSED) {
        LOS_IntRestore(int_save);
        return LOS_ERRNO_TSK_NOT_CREATED;
    }
    *task_status = (*task_cb).taskStatus as UINT32;
    LOS_IntRestore(int_save);
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskSwitchInfoGet(_index: UINT32, task_switch_info: *mut UINT32) -> UINT32 {
    if task_switch_info.is_null() {
        return LOS_ERRNO_TSK_PTR_NULL;
    }
    *task_switch_info = 0;
    let name = task_switch_info.add(1) as *mut u8;
    ptr::write_bytes(name, 0, task::LOS_TASK_NAMELEN as usize);
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskInfoMonitor() -> UINT32 {
    OsGetAllTskInfo()
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskIsRunning() -> BOOL {
    g_taskScheduled
}

#[no_mangle]
pub unsafe extern "C" fn LOS_NewTaskIDGet() -> UINT32 {
    LOS_NextTaskIDGet()
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskNameGet(task_id: UINT32) -> *mut CHAR {
    if os_check_tsk_pid_noidle(task_id) {
        return ptr::null_mut();
    }
    let task_cb = os_tcb_from_tid(task_id);
    let int_save = LOS_IntLock();
    if has_flag((*task_cb).taskStatus, task::OS_TASK_STATUS_UNUSED) {
        LOS_IntRestore(int_save);
        return ptr::null_mut();
    }
    LOS_IntRestore(int_save);
    (*task_cb).taskName
}

#[no_mangle]
pub unsafe extern "C" fn LOS_Msleep(m_secs: UINT32) {
    if OS_INT_ACTIVE() {
        return;
    }

    let interval = if m_secs == 0 {
        0
    } else {
        let ticks = task::LOS_MS2Tick(m_secs);
        if ticks == 0 { 1 } else { ticks }
    };
    let _ = LOS_TaskDelay(interval);
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TaskResRecycle() {
    OsRecycleFinishedTask();
}
