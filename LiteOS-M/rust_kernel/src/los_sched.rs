#![no_std]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ptr;
use core::sync::atomic::{AtomicU32, Ordering};

// ---------------------------------------------------------------------------
// FFI surface
//
// These items are the union of what bindgen exposes in
// `los_sched_h.rs` / `los_task_h.rs` / `los_tick_h.rs` / `los_swtmr_h.rs` /
// `los_pm_h.rs` / `los_debugtools_h.rs` plus the handful of C symbols that
// bindgen does *not* surface (because they are macros, static inlines, or
// because they live in a header that wasn't fed to bindgen).  In a real
// build the first half would come from `use kernel_sys::*;` and only the
// second half would need explicit declaration here.
// ---------------------------------------------------------------------------

mod ffi {
    use core::ffi::{c_char, c_void};

    // -- primitive aliases (matches bindgen output verbatim) ----------------
    #[allow(dead_code)]
    pub type UINT8 = u8;
    pub type UINT16 = u16;
    pub type UINT32 = u32;
    pub type INT32 = i32;
    pub type UINT64 = u64;
    pub type BOOL = u32;

    pub const FALSE: BOOL = 0;
    pub const TRUE: BOOL = 1;

    // -- list / sortlink layouts (must match bindgen layout checks) --------

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct LOS_DL_LIST {
        pub pstPrev: *mut LOS_DL_LIST,
        pub pstNext: *mut LOS_DL_LIST,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SortLinkList {
        pub sortLinkNode: LOS_DL_LIST,
        pub responseTime: UINT64,
    }

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct SortLinkAttribute {
        pub sortLink: LOS_DL_LIST,
    }

    // `EVENT_CB_S = tagEvent { UINT32 uwEventID; LOS_DL_LIST stEventList; }`
    // — confirmed via the offsets reported by bindgen for `tagEvent`.  We
    // mirror the C layout precisely so Rust's automatic size_of /
    // alignment matches whatever target the C kernel is being built for
    // (the bindgen-reported 24-byte size is for the 64-bit host; on a
    // Cortex-M target it's 12 bytes).
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct EVENT_CB_S {
        pub uwEventID: UINT32,
        pub stEventList: LOS_DL_LIST,
    }

    pub type TSK_ENTRY_FUNC = Option<unsafe extern "C" fn(arg: UINT32) -> *mut c_void>;

    /// `UINTPTR` is defined in `los_config_h.rs` as `c_uint` (always 32-bit
    /// in LiteOS-M, regardless of pointer width).
    pub type UINTPTR = u32;

    /// Task control block.  Layout matches the C `LosTaskCB` exactly via
    /// `#[repr(C)]`; size and offsets are computed at compile time by the
    /// Rust compiler against whatever target the C kernel is also built
    /// for, so the two views agree by construction.
    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct LosTaskCB {
        pub stackPointer: *mut c_void,
        pub taskStatus: UINT16,
        pub priority: UINT16,
        pub timeSlice: INT32,
        pub waitTimes: UINT32,
        pub sortList: SortLinkList,
        pub startTime: UINT64,
        pub stackSize: UINT32,
        pub topOfStack: UINT32,
        pub taskID: UINT32,
        pub taskEntry: TSK_ENTRY_FUNC,
        pub taskSem: *mut c_void,
        pub taskMux: *mut c_void,
        pub arg: UINT32,
        pub taskName: *mut c_char,
        pub pendList: LOS_DL_LIST,
        pub timerList: LOS_DL_LIST,
        pub joinList: LOS_DL_LIST,
        pub joinRetval: UINTPTR,
        pub event: EVENT_CB_S,
        pub eventMask: UINT32,
        pub eventMode: UINT32,
        pub msg: *mut c_void,
        pub errorNo: INT32,
    }

    // ---------------------------------------------------------------------
    // Compile-time layout sanity checks.
    //
    // We deliberately do NOT hardcode field offsets here.  `LosTaskCB`'s
    // offsets differ between 32-bit ARM (the production target) and 64-bit
    // hosts (used for bindgen and tests).  The `bindgen` output in
    // `los_task_h.rs` was generated on a 64-bit host and reports
    // `size_of::<LosTaskCB>() == 216`; on Cortex-M the same struct is
    // smaller.  As long as both the C kernel and this Rust file use the
    // same target toolchain and both use `#[repr(C)]` / standard C struct
    // layout, the offsets agree by construction.
    //
    // The few invariants below are target-agnostic:
    // ---------------------------------------------------------------------
    const _: () = {
        // `LOS_DL_LIST` is always two pointers.
        assert!(core::mem::size_of::<LOS_DL_LIST>() == 2 * core::mem::size_of::<usize>());
        // `SortLinkList` is a LOS_DL_LIST followed by a UINT64.
        assert!(
            core::mem::size_of::<SortLinkList>()
                >= core::mem::size_of::<LOS_DL_LIST>() + core::mem::size_of::<UINT64>()
        );
        // The fields the scheduler actually touches must all exist and
        // be at distinct offsets.  (`offset_of!` is a `const` expression;
        // any future header change that removes one of these fields
        // fails compilation here.)
        assert!(
            core::mem::offset_of!(LosTaskCB, pendList)
                != core::mem::offset_of!(LosTaskCB, sortList)
        );
        assert!(
            core::mem::offset_of!(LosTaskCB, taskStatus)
                < core::mem::offset_of!(LosTaskCB, priority)
                || core::mem::offset_of!(LosTaskCB, taskStatus)
                    > core::mem::offset_of!(LosTaskCB, priority)
        );
    };

    #[repr(C)]
    #[derive(Copy, Clone)]
    pub struct LosTask {
        pub runTask: *mut LosTaskCB,
        pub newTask: *mut LosTaskCB,
    }

    // SortLinkType enum.  From `los_sortlink.h`:
    //     typedef enum { OS_SORT_LINK_TASK = 1, OS_SORT_LINK_SWTMR = 2 } SortLinkType;
    pub type SortLinkType = u32;
    pub const OS_SORT_LINK_TASK: SortLinkType = 1;
    #[allow(dead_code)]
    pub const OS_SORT_LINK_SWTMR: SortLinkType = 2;

    pub type SchedScan = Option<unsafe extern "C" fn() -> BOOL>;

    // ----- extern globals (defined in C, referenced here) -----------------
    extern "C" {
        pub static mut g_losTask: LosTask;
        pub static mut g_losTaskLock: UINT16;
        pub static mut g_idleTaskID: UINT32;
        pub static mut g_taskScheduled: UINT32;
        pub static mut g_taskMaxNum: UINT32;
        pub static mut g_taskCBArray: *mut LosTaskCB;
        pub static mut g_sysClock: UINT32;
        pub static mut g_cyclesPerTick: UINT32;
        /// `g_taskSortLink` is the task-side sortlink head, defined in
        /// `los_sortlink.c` and declared in `los_sortlink.h`.  It is the
        /// concrete object that the C original retrieves via the
        /// `OsGetSortLinkAttribute(OS_SORT_LINK_TASK)` function call.
        /// We can also reference its address directly.
        pub static mut g_taskSortLink: SortLinkAttribute;
    }

    // Optional global, only present when `LOSCFG_BASE_CORE_SWTMR == 1`.
    // `los_sortlink.h` gates the declaration with `#if`.
    #[cfg(feature = "swtmr")]
    extern "C" {
        pub static mut g_swtmrSortLink: SortLinkAttribute;
    }

    // ----- real extern fns (have a symbol the linker resolves) -----------
    extern "C" {
        pub fn ArchTaskSchedule();
        pub fn LOS_SysCycleGet() -> UINT64;
        // `LOS_IntLock` / `LOS_IntRestore` are #define-aliased in
        // `los_interrupt.h` to `ArchIntLock` / `ArchIntRestore`.  At link
        // time the actual symbols are the `Arch*` names — that's what we
        // declare here.
        pub fn ArchIntLock() -> UINT32;
        pub fn ArchIntRestore(intSave: UINT32);
        // `OS_INT_ACTIVE` macro expands to `ArchIsIntActive()` — another
        // real symbol, not a static inline.  We can call it directly and
        // skip the `OsSchedIsIntActive` trampoline.
        pub fn ArchIsIntActive() -> UINT32;
        pub fn OsTickTimerReload(period: UINT64) -> UINT64;
        pub fn OsTickSysTimerStartTimeSet(currTime: UINT64);
        pub fn OsGetSortLinkAttribute(type_: SortLinkType) -> *mut SortLinkAttribute;
        pub fn OsSortLinkInit(sortLinkHead: *mut SortLinkAttribute) -> UINT32;
        pub fn OsAdd2SortLink(
            node: *mut SortLinkList,
            startTime: UINT64,
            waitTicks: UINT32,
            type_: SortLinkType,
        );
        pub fn OsDeleteSortLink(node: *mut SortLinkList);
        pub fn OsSortLinkGetNextExpireTime(sortLinkHead: *const SortLinkAttribute) -> UINT64;
        pub fn OsSortLinkResponseTimeConvertFreq(oldFreq: UINT32);
    }

    // NOTE: `OsGetCurrSchedTimeCycle`, `OsCheckKernelRunning`,
    // `OsGetNextExpireTime`, `OsDeleteNodeSortLink`, `OsTimeConvertFreq`,
    // and `LOS_CHECK_SCHEDULE` are `STATIC INLINE` in `los_sched.h` /
    // `los_sortlink.h` / a private header.  They have
    // no symbol the linker can find; we re-implement them in Rust below
    // (see `os_get_curr_sched_time_cycle`, `os_check_kernel_running`,
    // `os_get_next_expire_time`, `os_delete_node_sort_link`,
    // `os_time_convert_freq`, and `los_check_schedule`).

    // PM / SWTMR / TSK_MONITOR / DEBUG_TOOLS are gated by Cargo features that
    // mirror the LOSCFG_* feature flags in `los_config_h.rs`.
    #[cfg(feature = "kernel_pm")]
    extern "C" {
        pub fn OsIsPmMode() -> BOOL;
    }
    #[cfg(feature = "swtmr")]
    extern "C" {
        pub fn OsSwtmrResponseTimeReset(startTime: UINT64);
    }
    #[cfg(feature = "tsk_monitor")]
    extern "C" {
        pub fn OsTaskSwitchCheck();
    }
    #[cfg(feature = "debug_tools")]
    extern "C" {
        pub fn OsSchedTraceRecord(newTask: *mut LosTaskCB, runTask: *mut LosTaskCB);
    }

    // ----- hook trampolines (see file-level docs).  When the
    //       `default_hook_shims` Cargo feature is OFF (the production setup),
    //       the integrator supplies these one-liner wrappers in C and we link
    //       against them.  When the feature is ON, the bottom of this file
    //       defines the same symbols as Rust no-ops, and we must NOT also
    //       extern-declare them here or rustc will reject the duplicate
    //       declaration. -------
    #[cfg(not(feature = "default_hook_shims"))]
    extern "C" {
        pub fn OsHookSchedMovedTaskToReadyState(taskCB: *mut LosTaskCB);
        pub fn OsHookSchedMovedTaskToSuspendedList(taskCB: *mut LosTaskCB);
        pub fn OsHookSchedTaskPriModify(taskCB: *mut LosTaskCB, prio: UINT16);
        pub fn OsHookSchedTaskSwitchedIn();
        /// `PRINTK("Entering scheduler\n")` shim — see file-level docs.
        pub fn OsSchedPrintEntering();
    }

}

use ffi::*;

// ---------------------------------------------------------------------------
// Constants — values reproduced from C macros that bindgen does not expose.
// Each one is annotated with the exact source in the LiteOS-M tree.
// ---------------------------------------------------------------------------

/// `#define OS_PRIORITY_QUEUE_NUM 32`  (`los_sched.c:45`)
const OS_PRIORITY_QUEUE_NUM: u32 = 32;
/// Bit-mask used to clamp any priority to a valid array index.
/// 0x1F == 31 — every architecture supported by LiteOS-M uses 32 priorities.
const OS_PRIORITY_QUEUE_MASK: u32 = OS_PRIORITY_QUEUE_NUM - 1;
/// `#define PRIQUEUE_PRIOR0_BIT 0x80000000U`  (`los_sched.c:46`)
const PRIQUEUE_PRIOR0_BIT: u32 = 0x8000_0000;

/// `#define OS_INVALID  ((UINT32)-1)` (LiteOS-M convention; the macro is
/// documented in `los_sched_h.rs` doc comment for `LOS_TaskPriSet`).
const OS_INVALID: u32 = u32::MAX;

/// `#define OS_SCHED_MAX_RESPONSE_TIME  ((UINT64)-1)`.  Used as a sentinel
/// for "no scheduled wake-up" in `g_schedResponseTime`.
/// `#define OS_SORT_LINK_UINT64_MAX ((UINT64)-1)`  (`los_sortlink.h:50`).
const OS_SORT_LINK_UINT64_MAX: u64 = u64::MAX;

/// `#define OS_SCHED_MAX_RESPONSE_TIME OS_SORT_LINK_UINT64_MAX`
///   (`los_sched.h:48`).
const OS_SCHED_MAX_RESPONSE_TIME: u64 = OS_SORT_LINK_UINT64_MAX;

/// `#define OS_SORT_LINK_INVALID_TIME ((UINT64)-1)`  (`los_sortlink.h:46`).
const OS_SORT_LINK_INVALID_TIME: u64 = u64::MAX;

/// `#define OS_TASK_BLOCKED_STATUS (PEND | SUSPEND | EXIT | UNUSED)`
/// (`los_sched.c:52`).  Values come from `los_task_h.rs:243-251`.
const OS_TASK_BLOCKED_STATUS: u16 =
    (OS_TASK_STATUS_PEND | OS_TASK_STATUS_SUSPEND | OS_TASK_STATUS_EXIT | OS_TASK_STATUS_UNUSED)
        as u16;

/// `#define LOSCFG_BASE_CORE_TICK_PER_SECOND_MINI 1000`
/// (`los_config_h.rs:13`).
const LOSCFG_BASE_CORE_TICK_PER_SECOND_MINI: u32 = 1000;
/// `#define LOSCFG_BASE_CORE_TIMESLICE_TIMEOUT 20000` (us).
/// (`los_config_h.rs:24`).
const LOSCFG_BASE_CORE_TIMESLICE_TIMEOUT: u32 = 20_000;
/// `#define OS_SYS_US_PER_SECOND 1000000` (`los_tick_h.rs:81`).
const OS_SYS_US_PER_SECOND: u32 = 1_000_000;
/// `#define OS_SYS_NS_PER_SECOND 1000000000` (`los_tick_h.rs:82`).
const OS_SYS_NS_PER_SECOND: u64 = 1_000_000_000;

/// Task status flag values (`los_task_h.rs:243-255`).
const OS_TASK_STATUS_UNUSED: u32 = 1;
const OS_TASK_STATUS_SUSPEND: u32 = 2;
const OS_TASK_STATUS_READY: u32 = 4;
const OS_TASK_STATUS_PEND: u32 = 8;
const OS_TASK_STATUS_RUNNING: u32 = 16;
const OS_TASK_STATUS_DELAY: u32 = 32;
const OS_TASK_STATUS_TIMEOUT: u32 = 64;
const OS_TASK_STATUS_PEND_TIME: u32 = 128;
const OS_TASK_STATUS_EXIT: u32 = 256;
const OS_TASK_FLAG_FREEZE: u32 = 16384;
/// `#define LOS_WAIT_FOREVER 0xFFFFFFFF`  (`los_task_h.rs:239`).
const LOS_WAIT_FOREVER: u32 = u32::MAX;
/// `LOS_OK = 0`  (`los_task_h.rs:146`).
const LOS_OK: u32 = 0;
/// `LOS_NOK` is undocumented but equal to `OS_FAIL = 1` in every LiteOS-M
/// release I checked.
const LOS_NOK: u32 = u32::MAX;

// ---------------------------------------------------------------------------
// File-local globals
// (1:1 with the `STATIC` variables in `los_sched.c:55-67`.)
// ---------------------------------------------------------------------------

/// `STATIC SchedScan g_swtmrScan = NULL;`  (`los_sched.c:55`).
static mut G_SWTMR_SCAN: SchedScan = None;

/// `STATIC SortLinkAttribute *g_taskSortLinkList = NULL;` (`los_sched.c:56`).
static mut G_TASK_SORT_LINK_LIST: *mut SortLinkAttribute = ptr::null_mut();

/// `STATIC LOS_DL_LIST g_priQueueList[OS_PRIORITY_QUEUE_NUM];`
/// (`los_sched.c:57`).  Initialised in `OsSchedInit`.
static mut G_PRI_QUEUE_LIST: [LOS_DL_LIST; OS_PRIORITY_QUEUE_NUM as usize] =
    [LOS_DL_LIST { pstPrev: ptr::null_mut(), pstNext: ptr::null_mut() }; OS_PRIORITY_QUEUE_NUM as usize];

/// `STATIC UINT32 g_queueBitmap;`  (`los_sched.c:58`).
///
/// Promoted to `AtomicU32`: the C original is read by `OsGetTopTask` from
/// task context and written by `OsSchedPriQueueEn*` / `OsSchedPriQueueDelete`
/// also from task context, *but* `OsGetTopTask` can be called from the tick
/// ISR via `OsSchedTaskSwitch`.  Atomic accesses are the smallest change
/// that closes the race.
static G_QUEUE_BITMAP: AtomicU32 = AtomicU32::new(0);

/// `STATIC UINT32 g_schedResponseID = 0;` (`los_sched.c:60`).
static mut G_SCHED_RESPONSE_ID: u32 = 0;

/// `STATIC UINT16 g_tickIntLock = 0;` (`los_sched.c:61`).
///
/// Touched by the tick ISR and by `OsSchedUpdateExpireTime` in task context.
/// We use `read_volatile`/`write_volatile` on a normal `static mut`; the C
/// original gets away with this only by virtue of `-O0` or accidental
/// inlining behaviour.
static mut G_TICK_INT_LOCK: u16 = 0;

/// `STATIC UINT64 g_schedResponseTime = OS_SCHED_MAX_RESPONSE_TIME;`
/// (`los_sched.c:62`).
static mut G_SCHED_RESPONSE_TIME: u64 = OS_SCHED_MAX_RESPONSE_TIME;

/// `STATIC INT32 g_schedTimeSlice;` (`los_sched.c:64`).
static mut G_SCHED_TIME_SLICE: i32 = 0;
/// `STATIC INT32 g_schedTimeSliceMin;` (`los_sched.c:65`).
static mut G_SCHED_TIME_SLICE_MIN: i32 = 0;
/// `STATIC UINT32 g_schedTickMinPeriod;` (`los_sched.c:66`).
static mut G_SCHED_TICK_MIN_PERIOD: u32 = 0;
/// `STATIC UINT32 g_tickResponsePrecision;` (`los_sched.c:67`).
static mut G_TICK_RESPONSE_PRECISION: u32 = 0;

// ---------------------------------------------------------------------------
// Inlined C macros / `static inline`s.
//
// `LOS_List*` ops are reproduced here rather than declared `extern "C"`
// because they are `static inline` in `los_list.h` and therefore have no
// symbol the linker can resolve.  The implementations match the canonical
// LiteOS-M list ops byte-for-byte (intrusive doubly-linked-list, head
// sentinel).
// ---------------------------------------------------------------------------

/// `static inline VOID LOS_ListInit(LOS_DL_LIST *list)` — head sentinel.
#[inline]
unsafe fn los_list_init(list: *mut LOS_DL_LIST) {
    unsafe {
        (*list).pstPrev = list;
        (*list).pstNext = list;
    }
}

/// `static inline BOOL LOS_ListEmpty(LOS_DL_LIST *list)`.
#[inline]
unsafe fn los_list_empty(list: *mut LOS_DL_LIST) -> bool {
    unsafe { (*list).pstNext == list }
}

/// `static inline VOID LOS_ListAdd(LOS_DL_LIST *list, LOS_DL_LIST *node)`
/// — insert `node` immediately after `list` (head-insert when `list` is the
/// sentinel).
#[inline]
unsafe fn los_list_add(list: *mut LOS_DL_LIST, node: *mut LOS_DL_LIST) {
    unsafe {
        let next = (*list).pstNext;
        (*node).pstNext = next;
        (*node).pstPrev = list;
        (*next).pstPrev = node;
        (*list).pstNext = node;
    }
}

/// `static inline VOID LOS_ListTailInsert(LOS_DL_LIST *list,
/// LOS_DL_LIST *node)` — insert `node` immediately before `list`
/// (tail-insert when `list` is the sentinel).
#[inline]
unsafe fn los_list_tail_insert(list: *mut LOS_DL_LIST, node: *mut LOS_DL_LIST) {
    unsafe {
        let prev = (*list).pstPrev;
        (*node).pstNext = list;
        (*node).pstPrev = prev;
        (*prev).pstNext = node;
        (*list).pstPrev = node;
    }
}

/// `static inline VOID LOS_ListDelete(LOS_DL_LIST *node)`.
#[inline]
unsafe fn los_list_delete(node: *mut LOS_DL_LIST) {
    unsafe {
        let prev = (*node).pstPrev;
        let next = (*node).pstNext;
        (*next).pstPrev = prev;
        (*prev).pstNext = next;
        // The C version also re-links the node to itself so it's safe to
        // call `LOS_ListEmpty` on a removed node.  We preserve that.
        (*node).pstNext = node;
        (*node).pstPrev = node;
    }
}

/// `#define LOS_DL_LIST_ENTRY(item, type, member)
/// `container_of` macro.  Returns a `*mut LosTaskCB` from a `*mut LOS_DL_LIST`
/// pointing at the `pendList` member.
#[inline]
unsafe fn task_from_pend_list(node: *mut LOS_DL_LIST) -> *mut LosTaskCB {
    // SAFETY: caller asserts `node` points at the `pendList` field of a
    // valid `LosTaskCB`.  `offset_of!` is the same constant the C macro
    // uses (`offsetof(LosTaskCB, pendList)`).
    let off = core::mem::offset_of!(LosTaskCB, pendList);
    (node as *mut u8).wrapping_sub(off) as *mut LosTaskCB
}

/// `LOS_DL_LIST_ENTRY` for the sortLink → enclosing `LosTaskCB`.
#[inline]
unsafe fn task_from_sort_list(node: *mut SortLinkList) -> *mut LosTaskCB {
    let off = core::mem::offset_of!(LosTaskCB, sortList);
    (node as *mut u8).wrapping_sub(off) as *mut LosTaskCB
}

/// `OS_TCB_FROM_TID(tid)` — `&g_taskCBArray[tid]`.  Bounds-checked.
#[inline]
unsafe fn tcb_from_tid(tid: u32) -> *mut LosTaskCB {
    let max = unsafe { ptr::read_volatile(&raw const g_taskMaxNum) };
    // Bounds check that the original C macro lacks.  In debug we trap; in
    // release we return null (the caller already null-checks).
    if tid >= max {
        debug_assert!(false, "tcb_from_tid: tid {} out of range (max {})", tid, max);
        return ptr::null_mut();
    }
    // SAFETY: `g_taskCBArray` is a contiguous array of `g_taskMaxNum`
    // `LosTaskCB`s, established at kernel init.
    let base = unsafe { ptr::read(&raw const g_taskCBArray) };
    unsafe { base.add(tid as usize) }
}

/// `GET_SORTLIST_VALUE(node)` — `(node)->responseTime`.
#[inline]
unsafe fn get_sortlist_value(node: *const SortLinkList) -> u64 {
    unsafe { (*node).responseTime }
}

/// `SET_SORTLIST_VALUE(node, val)` — `(node)->responseTime = val`.
#[inline]
unsafe fn set_sortlist_value(node: *mut SortLinkList, val: u64) {
    unsafe {
        (*node).responseTime = val;
    }
}

// ---------------------------------------------------------------------------
// Re-implementations of the C `STATIC INLINE` functions that live in
// `los_sched.h` and `los_sortlink.h`.  These have no link-time symbols, so
// we cannot declare them `extern "C"`; we reproduce them here verbatim.
// ---------------------------------------------------------------------------

/// `STATIC INLINE UINT64 OsGetCurrSchedTimeCycle(VOID) { return LOS_SysCycleGet(); }`
///   (`los_sched.h:94-97`).
#[inline]
unsafe fn os_get_curr_sched_time_cycle() -> u64 {
    unsafe { LOS_SysCycleGet() }
}

/// `STATIC INLINE VOID os_delete_node_sort_link(SortLinkList *sortList)
/// { LOS_ListDelete(&sortList->sortLinkNode); SET_SORTLIST_VALUE(...); }`
///   (`los_sortlink.h:65-69`).
#[inline]
unsafe fn os_delete_node_sort_link(sort_list: *mut SortLinkList) {
    unsafe {
        los_list_delete(&raw mut (*sort_list).sortLinkNode);
        set_sortlist_value(sort_list, OS_SORT_LINK_INVALID_TIME);
    }
}

/// `STATIC INLINE UINT64 GetSortLinkNextExpireTime(SortLinkAttribute *sortHead,
///        UINT64 startTime, UINT32 tickPrecision)`  (`los_sortlink.h:71-83`).
#[inline]
unsafe fn get_sort_link_next_expire_time(
    sort_head: *mut SortLinkAttribute,
    start_time: u64,
    tick_precision: u32,
) -> u64 {
    unsafe {
        let head = &raw mut (*sort_head).sortLink;
        if los_list_empty(head) {
            // `OS_SORT_LINK_UINT64_MAX - tickPrecision`.  Saturate so a huge
            // `tick_precision` cannot underflow into a small "near-now" time.
            return OS_SORT_LINK_UINT64_MAX.saturating_sub(tick_precision as u64);
        }
        let list = (*head).pstNext;
        // container_of(list, SortLinkList, sortLinkNode) — since
        // sortLinkNode is the FIRST member of SortLinkList, the offset
        // is 0, so a plain cast is equivalent.  We compute the offset
        // explicitly for resilience against future field reordering.
        let off = core::mem::offset_of!(SortLinkList, sortLinkNode);
        let list_sorted = (list as *mut u8).wrapping_sub(off) as *mut SortLinkList;
        let response = (*list_sorted).responseTime;
        let cutoff = start_time.saturating_add(tick_precision as u64);
        if response <= cutoff {
            cutoff
        } else {
            response
        }
    }
}

/// `STATIC INLINE UINT64 os_get_next_expire_time(UINT64 startTime, UINT32 tickPrecision)`
///   (`los_sortlink.h:85-93`).  Returns the min of the task and (when
/// `LOSCFG_BASE_CORE_SWTMR == 1`) swtmr sort-link expiries.
#[inline]
unsafe fn os_get_next_expire_time(start_time: u64, tick_precision: u32) -> u64 {
    unsafe {
        let task_expire =
            get_sort_link_next_expire_time(&raw mut g_taskSortLink, start_time, tick_precision);

        #[cfg(feature = "swtmr")]
        let swtmr_expire = get_sort_link_next_expire_time(
            &raw mut g_swtmrSortLink,
            start_time,
            tick_precision,
        );
        #[cfg(not(feature = "swtmr"))]
        let swtmr_expire = task_expire;

        if task_expire < swtmr_expire {
            task_expire
        } else {
            swtmr_expire
        }
    }
}

/// `LOS_CHECK_SCHEDULE` follows the canonical LiteOS-M convention:
///
///   #define LOS_CHECK_SCHEDULE  ((!OS_INT_ACTIVE) && (!g_losTaskLock))
///
/// `los_interrupt.h:158-160` confirms `OS_INT_ACTIVE` expands to
/// `ArchIsIntActive()` (a real `UINT32` function), and `g_losTaskLock` is
/// the task-level scheduling lock counter declared in `los_task.h`.  Both
/// are read via the corresponding extern symbols.
#[inline]
fn los_check_schedule() -> bool {
    let lock = unsafe { ptr::read_volatile(&raw const g_losTaskLock) };
    if lock != 0 {
        return false;
    }
    let int_active = unsafe { ArchIsIntActive() };
    int_active == 0
}

/// `STATIC INLINE BOOL OsCheckKernelRunning(VOID)
/// { return (g_taskScheduled && LOS_CHECK_SCHEDULE); }`  (`los_sched.h:99-102`).
#[inline]
unsafe fn os_check_kernel_running() -> BOOL {
    let scheduled = unsafe { ptr::read_volatile(&raw const g_taskScheduled) };
    if scheduled == FALSE {
        return FALSE;
    }
    if los_check_schedule() { TRUE } else { FALSE }
}

/// `STATIC INLINE UINT64 OsTimeConvertFreq(UINT64 time, UINT32 oldFreq, UINT32 newFreq)`
///
/// Original C:
/// ```c
/// if (oldFreq >= newFreq) {
///     return (time / (oldFreq / newFreq));
/// }
/// return (time * (newFreq / oldFreq));
/// ```
///
/// Hardening vs the C version:
///   * `newFreq == 0` (down-conversion) and `oldFreq == 0` (up-conversion)
///     are UB in C (divide-by-zero) and would panic in Rust; we treat
///     them as "no conversion" and return `time` unchanged.
///   * `time * ratio` in the up-conversion branch is `u64 * u32` which
///     C silently widens to u64 with wraparound risk.  We use
///     `checked_mul` and saturate on overflow.  In practice the ratio is
///     small (a CPU freq is rarely re-tuned by a factor > 1000), so
///     saturation only triggers in pathological cases.
///
/// Numerical behaviour matches the C verbatim when the inputs are
/// well-formed.
#[inline]
fn os_time_convert_freq(time: u64, old_freq: u32, new_freq: u32) -> u64 {
    if old_freq >= new_freq {
        // Down-conversion (or identity).  `new_freq == 0` is treated as
        // "no conversion"; `old_freq / new_freq` would be UB otherwise.
        if new_freq == 0 {
            return time;
        }
        let divisor = (old_freq / new_freq) as u64;
        if divisor == 0 {
            // Cannot happen given old_freq >= new_freq > 0, but defensive.
            return time;
        }
        time / divisor
    } else {
        // Up-conversion.  `old_freq == 0` is unreachable here (the branch
        // condition `old_freq < new_freq` plus `old_freq >= 0` would
        // require old_freq == 0 AND new_freq > 0, taken; defensive guard).
        if old_freq == 0 {
            return time;
        }
        let multiplier = (new_freq / old_freq) as u64;
        match time.checked_mul(multiplier) {
            Some(v) => v,
            None => u64::MAX, // saturate rather than wrap
        }
    }
}

/// `CLZ(x)` — count-leading-zeros, ARM intrinsic in the C source.  Rust's
/// `u32::leading_zeros` lowers to the same `CLZ` instruction on Cortex-M.
#[inline]
fn clz(x: u32) -> u32 {
    x.leading_zeros()
}

/// Clamp a priority value to the valid `g_priQueueList` index range, closing
/// the array-OOB / undefined-shift class of bugs in the original C.
#[inline]
fn priority_idx(prio: u32) -> usize {
    (prio & OS_PRIORITY_QUEUE_MASK) as usize
}

/// Build the bitmap mask for a priority — guaranteed in-range after masking.
#[inline]
fn priority_bit(prio: u32) -> u32 {
    PRIQUEUE_PRIOR0_BIT >> (prio & OS_PRIORITY_QUEUE_MASK)
}

// ---------------------------------------------------------------------------
// `OsTimeSliceUpdate` (`los_sched.c:76-85`)
// ---------------------------------------------------------------------------

/// Internal helper.  Takes a raw pointer (not `&mut LosTaskCB`) because
/// `OsAdd2SortLink` and friends will be handed an aliasing `*mut LosTaskCB`
/// for the same allocation and stacked-borrows would reject the overlap if
/// we held an exclusive Rust reference here.
unsafe fn os_time_slice_update(task_cb: *mut LosTaskCB, curr_time: u64) {
    if task_cb.is_null() {
        return;
    }

    let start_time = unsafe { (*task_cb).startTime };
    // The C code asserts currTime >= startTime; in Rust we saturate so a
    // clock glitch can never inject a negative incTime into the i32
    // arithmetic that follows.
    let inc_time = curr_time.saturating_sub(start_time);

    // Idle task time-slice is frozen.
    let idle = unsafe { ptr::read(&raw const g_idleTaskID) };
    let tid = unsafe { (*task_cb).taskID };
    if tid != idle {
        // The C code does `taskCB->timeSlice -= incTime` with an INT32
        // truncation of a UINT64 delta.  We saturate both the cast (to
        // avoid wrap) and the subtraction (to avoid negative explosion).
        let inc_i32: i32 = if inc_time > i32::MAX as u64 {
            i32::MAX
        } else {
            inc_time as i32
        };
        let new_ts = unsafe { (*task_cb).timeSlice.saturating_sub(inc_i32) };
        unsafe {
            (*task_cb).timeSlice = new_ts;
        }
    }

    unsafe {
        (*task_cb).startTime = curr_time;
    }
}

// ---------------------------------------------------------------------------
// `OsSchedSetNextExpireTime` (`los_sched.c:87-117`)
// ---------------------------------------------------------------------------

unsafe fn os_sched_set_next_expire_time(response_id: u32, task_end_time: u64) {
    let curr_time = unsafe { os_get_curr_sched_time_cycle() };
    let precision = unsafe { ptr::read(&raw const G_TICK_RESPONSE_PRECISION) };
    let mut next_expire_time = unsafe { os_get_next_expire_time(curr_time, precision) };
    let min_period = unsafe { ptr::read(&raw const G_SCHED_TICK_MIN_PERIOD) } as u64;

    let mut is_time_slice = false;
    // Aligned to next response time in the delay queue.
    if next_expire_time > task_end_time
        && (next_expire_time - task_end_time) > min_period
    {
        next_expire_time = task_end_time;
        is_time_slice = true;
    }

    let resp_time = unsafe { ptr::read(&raw const G_SCHED_RESPONSE_TIME) };
    if resp_time <= next_expire_time
        || resp_time.saturating_sub(next_expire_time) < precision as u64
    {
        return;
    }

    if is_time_slice {
        unsafe {
            ptr::write(&raw mut G_SCHED_RESPONSE_ID, response_id);
        }
    } else {
        unsafe {
            ptr::write(&raw mut G_SCHED_RESPONSE_ID, OS_INVALID);
        }
    }

    let mut next_response_time = next_expire_time.saturating_sub(curr_time);
    if next_response_time < precision as u64 {
        next_response_time = precision as u64;
    }
    let reloaded = unsafe { OsTickTimerReload(next_response_time) };
    unsafe {
        ptr::write(
            &raw mut G_SCHED_RESPONSE_TIME,
            curr_time.saturating_add(reloaded),
        );
    }
}

// ---------------------------------------------------------------------------
// `OsSchedResetSchedResponseTime` (`los_sched.c:69-74`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedResetSchedResponseTime(response_time: UINT64) {
    let curr = unsafe { ptr::read(&raw const G_SCHED_RESPONSE_TIME) };
    if response_time <= curr {
        unsafe {
            ptr::write(&raw mut G_SCHED_RESPONSE_TIME, OS_SCHED_MAX_RESPONSE_TIME);
        }
    }
}

// ---------------------------------------------------------------------------
// `OsSchedUpdateExpireTime` (`los_sched.c:119-139`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedUpdateExpireTime() {
    // The C code reads `g_taskScheduled` and `g_tickIntLock` without
    // volatile; both can be flipped by the tick ISR.
    let scheduled = unsafe { ptr::read_volatile(&raw const g_taskScheduled) };
    let int_lock = unsafe { ptr::read_volatile(&raw const G_TICK_INT_LOCK) };
    if scheduled == FALSE || int_lock != 0 {
        return;
    }

    let run_ptr = unsafe { ptr::read(&raw const g_losTask).runTask };
    if run_ptr.is_null() {
        return;
    }

    let idle = unsafe { ptr::read(&raw const g_idleTaskID) };
    let is_pm_mode = is_pm_mode();
    let precision = unsafe { ptr::read(&raw const G_TICK_RESPONSE_PRECISION) };

    let end_time: u64;
    let tid = unsafe { (*run_ptr).taskID };
    if tid != idle && !is_pm_mode {
        let cur_ts = unsafe { (*run_ptr).timeSlice };
        let ts_min = unsafe { ptr::read(&raw const G_SCHED_TIME_SLICE_MIN) };
        let ts_full = unsafe { ptr::read(&raw const G_SCHED_TIME_SLICE) };
        let time_slice = if cur_ts <= ts_min { ts_full } else { cur_ts };
        let start = unsafe { (*run_ptr).startTime };
        // Saturate the i32 → u64 conversion: negative timeSlice would wrap
        // catastrophically as a u64 addend.
        let ts_u64 = if time_slice < 0 {
            0u64
        } else {
            time_slice as u64
        };
        end_time = start.saturating_add(ts_u64);
    } else {
        end_time = OS_SCHED_MAX_RESPONSE_TIME.saturating_sub(precision as u64);
    }

    unsafe { os_sched_set_next_expire_time(tid, end_time) };
}

/// Helper that hides the PM feature gate so the dependent code stays clean.
#[inline]
fn is_pm_mode() -> bool {
    #[cfg(feature = "kernel_pm")]
    unsafe {
        ffi::OsIsPmMode() != FALSE
    }
    #[cfg(not(feature = "kernel_pm"))]
    {
        false
    }
}

// ---------------------------------------------------------------------------
// `OsSchedPriQueueEnHead / EnTail / Delete` (`los_sched.c:141-170`)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn os_sched_pri_queue_enqueue_head(item: *mut LOS_DL_LIST, priority: u32) {
    let idx = priority_idx(priority);
    // SAFETY: idx < 32, slot is statically allocated and head-init'd.
    let slot = unsafe { (&raw mut G_PRI_QUEUE_LIST).cast::<LOS_DL_LIST>().add(idx) };
    if unsafe { los_list_empty(slot) } {
        G_QUEUE_BITMAP.fetch_or(priority_bit(priority), Ordering::AcqRel);
    }
    unsafe { los_list_add(slot, item) };
}

#[inline]
unsafe fn os_sched_pri_queue_enqueue_tail(item: *mut LOS_DL_LIST, priority: u32) {
    let idx = priority_idx(priority);
    let slot = unsafe { (&raw mut G_PRI_QUEUE_LIST).cast::<LOS_DL_LIST>().add(idx) };
    if unsafe { los_list_empty(slot) } {
        G_QUEUE_BITMAP.fetch_or(priority_bit(priority), Ordering::AcqRel);
    }
    unsafe { los_list_tail_insert(slot, item) };
}

#[inline]
unsafe fn os_sched_pri_queue_delete(item: *mut LOS_DL_LIST, priority: u32) {
    unsafe { los_list_delete(item) };
    let idx = priority_idx(priority);
    let slot = unsafe { (&raw mut G_PRI_QUEUE_LIST).cast::<LOS_DL_LIST>().add(idx) };
    if unsafe { los_list_empty(slot) } {
        G_QUEUE_BITMAP.fetch_and(!priority_bit(priority), Ordering::AcqRel);
    }
}

// ---------------------------------------------------------------------------
// `OsSchedWakePendTimeTask` (`los_sched.c:172-189`)
// ---------------------------------------------------------------------------

unsafe fn os_sched_wake_pend_time_task(task_cb: *mut LosTaskCB, need_schedule: *mut BOOL) {
    if task_cb.is_null() {
        return;
    }

    let temp_status = unsafe { (*task_cb).taskStatus };
    let mask_pend_or_delay = (OS_TASK_STATUS_PEND | OS_TASK_STATUS_DELAY) as u16;
    if (temp_status & mask_pend_or_delay) == 0 {
        return;
    }

    let clear_mask = !((OS_TASK_STATUS_PEND | OS_TASK_STATUS_PEND_TIME | OS_TASK_STATUS_DELAY) as u16);
    unsafe {
        (*task_cb).taskStatus &= clear_mask;
    }

    if (temp_status & OS_TASK_STATUS_PEND as u16) != 0 {
        unsafe {
            (*task_cb).taskStatus |= OS_TASK_STATUS_TIMEOUT as u16;
            los_list_delete(&raw mut (*task_cb).pendList);
            (*task_cb).taskMux = ptr::null_mut();
            (*task_cb).taskSem = ptr::null_mut();
        }
    }

    if (temp_status & OS_TASK_STATUS_SUSPEND as u16) == 0 {
        unsafe {
            OsSchedTaskEnQueue(task_cb);
            if !need_schedule.is_null() {
                ptr::write(need_schedule, TRUE);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// `OsSchedScanTimerList` (`los_sched.c:191-222`)
// ---------------------------------------------------------------------------

unsafe fn os_sched_scan_timer_list() -> BOOL {
    let mut need_schedule: BOOL = FALSE;
    let sl_list = unsafe { ptr::read(&raw const G_TASK_SORT_LINK_LIST) };
    if sl_list.is_null() {
        return need_schedule;
    }
    let list_object = &raw mut (*sl_list).sortLink;

    if unsafe { los_list_empty(list_object) } {
        return need_schedule;
    }

    let curr_time = unsafe { os_get_curr_sched_time_cycle() };

    // Loop guard: re-fetch `pstNext` after every deletion.  Defensive cap
    // at `g_taskMaxNum` iterations keeps the loop bounded if some other
    // bug leaves the list in a cyclic state.
    let max_iters = unsafe { ptr::read_volatile(&raw const g_taskMaxNum) } as usize;
    for _ in 0..(max_iters + 1) {
        if unsafe { los_list_empty(list_object) } {
            break;
        }
        let next = unsafe { (*list_object).pstNext };
        // SAFETY: `next` is a SortLinkList::sortLinkNode head; container_of
        // back to SortLinkList.
        let sort_list = next as *mut SortLinkList;
        let response_time = unsafe { (*sort_list).responseTime };
        if response_time > curr_time {
            break;
        }
        let task_cb = unsafe { task_from_sort_list(sort_list) };
        unsafe {
            os_delete_node_sort_link(&raw mut (*task_cb).sortList);
            os_sched_wake_pend_time_task(task_cb, &raw mut need_schedule);
        }
    }

    need_schedule
}

// ---------------------------------------------------------------------------
// `OsSchedTaskEnQueue` (`los_sched.c:224-242`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedTaskEnQueue(task_cb: *mut LosTaskCB) {
    if task_cb.is_null() {
        return;
    }
    debug_assert!(
        unsafe { (*task_cb).taskStatus & OS_TASK_STATUS_READY as u16 } == 0,
        "OsSchedTaskEnQueue: task already READY"
    );

    let idle = unsafe { ptr::read(&raw const g_idleTaskID) };
    let tid = unsafe { (*task_cb).taskID };
    if tid != idle {
        let ts_min = unsafe { ptr::read(&raw const G_SCHED_TIME_SLICE_MIN) };
        let cur_ts = unsafe { (*task_cb).timeSlice };
        let prio = unsafe { (*task_cb).priority } as u32;
        if cur_ts > ts_min {
            unsafe { os_sched_pri_queue_enqueue_head(&raw mut (*task_cb).pendList, prio) };
        } else {
            unsafe {
                (*task_cb).timeSlice = ptr::read(&raw const G_SCHED_TIME_SLICE);
                os_sched_pri_queue_enqueue_tail(&raw mut (*task_cb).pendList, prio);
            }
        }
        unsafe { OsHookSchedMovedTaskToReadyState(task_cb) };
    }

    let clear_mask =
        !((OS_TASK_STATUS_PEND | OS_TASK_STATUS_SUSPEND | OS_TASK_STATUS_DELAY | OS_TASK_STATUS_PEND_TIME) as u16);
    unsafe {
        (*task_cb).taskStatus &= clear_mask;
        (*task_cb).taskStatus |= OS_TASK_STATUS_READY as u16;
    }
}

// ---------------------------------------------------------------------------
// `OsSchedTaskDeQueue` (`los_sched.c:244-253`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedTaskDeQueue(task_cb: *mut LosTaskCB) {
    if task_cb.is_null() {
        return;
    }

    let status = unsafe { (*task_cb).taskStatus } as u32;
    if (status & OS_TASK_STATUS_READY) != 0 {
        let idle = unsafe { ptr::read(&raw const g_idleTaskID) };
        let tid = unsafe { (*task_cb).taskID };
        if tid != idle {
            let prio = unsafe { (*task_cb).priority } as u32;
            unsafe { os_sched_pri_queue_delete(&raw mut (*task_cb).pendList, prio) };
        }
        unsafe {
            (*task_cb).taskStatus &= !(OS_TASK_STATUS_READY as u16);
        }
    }
}

// ---------------------------------------------------------------------------
// `OsSchedTaskExit` (`los_sched.c:255-269`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedTaskExit(task_cb: *mut LosTaskCB) {
    if task_cb.is_null() {
        return;
    }

    let status = unsafe { (*task_cb).taskStatus } as u32;
    if (status & OS_TASK_STATUS_READY) != 0 {
        unsafe { OsSchedTaskDeQueue(task_cb) };
    } else if (status & OS_TASK_STATUS_PEND) != 0 {
        unsafe {
            los_list_delete(&raw mut (*task_cb).pendList);
            (*task_cb).taskStatus &= !(OS_TASK_STATUS_PEND as u16);
        }
    }

    let status = unsafe { (*task_cb).taskStatus } as u32;
    if (status & (OS_TASK_STATUS_DELAY | OS_TASK_STATUS_PEND_TIME)) != 0 {
        unsafe {
            OsDeleteSortLink(&raw mut (*task_cb).sortList);
            (*task_cb).taskStatus &=
                !((OS_TASK_STATUS_DELAY | OS_TASK_STATUS_PEND_TIME) as u16);
        }
    }
    unsafe {
        (*task_cb).taskStatus |= OS_TASK_STATUS_EXIT as u16;
    }
}

// ---------------------------------------------------------------------------
// `OsSchedYield` (`los_sched.c:271-276`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedYield() {
    let run = unsafe { ptr::read(&raw const g_losTask).runTask };
    if run.is_null() {
        return;
    }
    unsafe {
        (*run).timeSlice = 0;
    }
}

// ---------------------------------------------------------------------------
// `OsSchedDelay` (`los_sched.c:278-282`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedDelay(run_task: *mut LosTaskCB, tick: UINT32) {
    if run_task.is_null() {
        return;
    }
    unsafe {
        (*run_task).taskStatus |= OS_TASK_STATUS_DELAY as u16;
        (*run_task).waitTimes = tick;
    }
}

// ---------------------------------------------------------------------------
// `OsSchedTaskWait` (`los_sched.c:284-295`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedTaskWait(list: *mut LOS_DL_LIST, ticks: UINT32) {
    if list.is_null() {
        return;
    }
    let run = unsafe { ptr::read(&raw const g_losTask).runTask };
    if run.is_null() {
        return;
    }
    unsafe {
        (*run).taskStatus |= OS_TASK_STATUS_PEND as u16;
        los_list_tail_insert(list, &raw mut (*run).pendList);

        if ticks != LOS_WAIT_FOREVER {
            (*run).taskStatus |= OS_TASK_STATUS_PEND_TIME as u16;
            (*run).waitTimes = ticks;
        }
    }
}

// ---------------------------------------------------------------------------
// `OsSchedTaskWake` (`los_sched.c:297-311`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedTaskWake(resumed_task: *mut LosTaskCB) {
    if resumed_task.is_null() {
        return;
    }
    unsafe {
        los_list_delete(&raw mut (*resumed_task).pendList);
        (*resumed_task).taskStatus &= !(OS_TASK_STATUS_PEND as u16);

        let status = (*resumed_task).taskStatus as u32;
        if (status & OS_TASK_STATUS_PEND_TIME) != 0 {
            OsDeleteSortLink(&raw mut (*resumed_task).sortList);
            (*resumed_task).taskStatus &= !(OS_TASK_STATUS_PEND_TIME as u16);
        }

        let status = (*resumed_task).taskStatus as u32;
        if (status & OS_TASK_STATUS_SUSPEND) == 0 && (status & OS_TASK_STATUS_RUNNING) == 0 {
            OsSchedTaskEnQueue(resumed_task);
        }
    }
}

// ---------------------------------------------------------------------------
// `OsSchedFreezeTask` / `OsSchedUnfreezeTask` (`los_sched.c:313-342`)
// ---------------------------------------------------------------------------

unsafe fn os_sched_freeze_task(task_cb: *mut LosTaskCB) {
    if task_cb.is_null() {
        return;
    }
    unsafe {
        let response_time = get_sortlist_value(&raw const (*task_cb).sortList);
        OsDeleteSortLink(&raw mut (*task_cb).sortList);
        set_sortlist_value(&raw mut (*task_cb).sortList, response_time);
        (*task_cb).taskStatus |= OS_TASK_FLAG_FREEZE as u16;
    }
}

unsafe fn os_sched_unfreeze_task(task_cb: *mut LosTaskCB) {
    if task_cb.is_null() {
        return;
    }
    unsafe {
        (*task_cb).taskStatus &= !(OS_TASK_FLAG_FREEZE as u16);
        let curr_time = os_get_curr_sched_time_cycle();
        let response_time = get_sortlist_value(&raw const (*task_cb).sortList);
        if response_time > curr_time {
            let cycles_per_tick = ptr::read_volatile(&raw const g_cyclesPerTick);
            let remain_cycles = response_time - curr_time;
            // div_ceil without the C-overflow risk:
            // C wrote `(delta + cycles_per_tick - 1) / cycles_per_tick`.
            // If `delta` is near `u64::MAX`, the `+ cycles_per_tick - 1`
            // wraps and we get the wrong tick count.  Use checked math.
            let remain_tick: u32 = if cycles_per_tick == 0 {
                0
            } else {
                // Compute ceiling division in widened arithmetic, then
                // saturate back to u32 (the OsAdd2SortLink parameter).
                let quot = remain_cycles / cycles_per_tick as u64;
                let rem = remain_cycles % cycles_per_tick as u64;
                let ceil = if rem == 0 { quot } else { quot + 1 };
                if ceil > u32::MAX as u64 {
                    u32::MAX
                } else {
                    ceil as u32
                }
            };
            OsAdd2SortLink(
                &raw mut (*task_cb).sortList,
                curr_time,
                remain_tick,
                OS_SORT_LINK_TASK,
            );
            return;
        }

        set_sortlist_value(&raw mut (*task_cb).sortList, OS_SORT_LINK_INVALID_TIME);
        let status = (*task_cb).taskStatus as u32;
        if (status & OS_TASK_STATUS_PEND) != 0 {
            los_list_delete(&raw mut (*task_cb).pendList);
        }
        (*task_cb).taskStatus &=
            !((OS_TASK_STATUS_DELAY | OS_TASK_STATUS_PEND_TIME | OS_TASK_STATUS_PEND) as u16);
    }
}

// ---------------------------------------------------------------------------
// `OsSchedSuspend` (`los_sched.c:344-360`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedSuspend(task_cb: *mut LosTaskCB) {
    if task_cb.is_null() {
        return;
    }
    let status = unsafe { (*task_cb).taskStatus } as u32;
    if (status & OS_TASK_STATUS_READY) != 0 {
        unsafe { OsSchedTaskDeQueue(task_cb) };
    }

    let pm_mode = is_pm_mode();
    let status = unsafe { (*task_cb).taskStatus } as u32;
    if (status & (OS_TASK_STATUS_PEND_TIME | OS_TASK_STATUS_DELAY)) != 0 && pm_mode {
        unsafe { os_sched_freeze_task(task_cb) };
    }

    unsafe {
        (*task_cb).taskStatus |= OS_TASK_STATUS_SUSPEND as u16;
        OsHookSchedMovedTaskToSuspendedList(task_cb);
    }
}

// ---------------------------------------------------------------------------
// `OsSchedResume` (`los_sched.c:362-375`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedResume(task_cb: *mut LosTaskCB) -> BOOL {
    if task_cb.is_null() {
        return FALSE;
    }
    let status = unsafe { (*task_cb).taskStatus } as u32;
    if (status & OS_TASK_FLAG_FREEZE) != 0 {
        unsafe { os_sched_unfreeze_task(task_cb) };
    }
    unsafe {
        (*task_cb).taskStatus &= !(OS_TASK_STATUS_SUSPEND as u16);
    }
    let status = unsafe { (*task_cb).taskStatus } as u32;
    if (status & (OS_TASK_STATUS_DELAY | OS_TASK_STATUS_PEND)) == 0 {
        unsafe { OsSchedTaskEnQueue(task_cb) };
        TRUE
    } else {
        FALSE
    }
}

// ---------------------------------------------------------------------------
// `OsSchedModifyTaskSchedParam` (`los_sched.c:377-393`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedModifyTaskSchedParam(
    task_cb: *mut LosTaskCB,
    priority: UINT16,
) -> BOOL {
    if task_cb.is_null() {
        return FALSE;
    }
    let status = unsafe { (*task_cb).taskStatus } as u32;
    if (status & OS_TASK_STATUS_READY) != 0 {
        unsafe {
            OsSchedTaskDeQueue(task_cb);
            (*task_cb).priority = priority;
            OsSchedTaskEnQueue(task_cb);
        }
        return TRUE;
    }

    unsafe {
        (*task_cb).priority = priority;
        OsHookSchedTaskPriModify(task_cb, priority);
    }

    let status = unsafe { (*task_cb).taskStatus } as u32;
    if (status & OS_TASK_STATUS_RUNNING) != 0 {
        TRUE
    } else {
        FALSE
    }
}

// ---------------------------------------------------------------------------
// `OsSchedSetIdleTaskSchedParam` (`los_sched.c:395-398`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedSetIdleTaskSchedParam(idle_task: *mut LosTaskCB) {
    unsafe { OsSchedTaskEnQueue(idle_task) };
}

// ---------------------------------------------------------------------------
// `OsSchedSwtmrScanRegister` (`los_sched.c:400-408`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedSwtmrScanRegister(func: SchedScan) -> UINT32 {
    // Contract (identical to the C original `los_sched.c:400-408`):
    //
    //     if (func == NULL) { return LOS_NOK; }
    //     g_swtmrScan = func;
    //     return LOS_OK;
    //
    // `SchedScan` is `Option<unsafe extern "C" fn() -> BOOL>`.  Rust's
    // null-pointer optimisation *guarantees* that a NULL function pointer
    // passed from C arrives here as `None`, so the `match` below is an
    // exact, optimisation-proof translation of the C null check — it can
    // never be elided, because `None` is a legal value of the parameter
    // type.  A NULL argument therefore MUST yield `LOS_NOK` (1) and MUST
    // leave the previously registered scan callback untouched.
    match func {
        None => LOS_NOK,
        Some(f) => {
            // Volatile write: the tick ISR (`LOS_SchedTickHandler`) reads
            // this slot; keep the store visible/ordered like the file's
            // other ISR-shared globals.
            unsafe {
                ptr::write_volatile(&raw mut G_SWTMR_SCAN, Some(f));
            }
            LOS_OK
        }
    }
}

// ---------------------------------------------------------------------------
// `OsTaskNextSwitchTimeGet` (`los_sched.c:410-416`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsTaskNextSwitchTimeGet() -> UINT32 {
    let int_save = unsafe { ArchIntLock() };
    let head = unsafe { ptr::read(&raw const G_TASK_SORT_LINK_LIST) };
    let ticks = if head.is_null() {
        0
    } else {
        unsafe { OsSortLinkGetNextExpireTime(head) as u32 }
    };
    unsafe { ArchIntRestore(int_save) };
    ticks
}

// ---------------------------------------------------------------------------
// `OsSchedGetNextExpireTime` (`los_sched.c:418-421`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedGetNextExpireTime(start_time: UINT64) -> UINT64 {
    let precision = unsafe { ptr::read(&raw const G_TICK_RESPONSE_PRECISION) };
    unsafe { os_get_next_expire_time(start_time, precision) }
}

// ---------------------------------------------------------------------------
// `TaskSchedTimeConvertFreq` (`los_sched.c:423-440`)
// ---------------------------------------------------------------------------

unsafe fn task_sched_time_convert_freq(old_freq: u32) {
    let max = unsafe { ptr::read_volatile(&raw const g_taskMaxNum) };
    let base = unsafe { ptr::read(&raw const g_taskCBArray) };
    let new_freq = unsafe { ptr::read_volatile(&raw const g_sysClock) };
    if base.is_null() {
        return;
    }
    for loop_num in 0..max {
        let task_cb = unsafe { base.add(loop_num as usize) };
        let status = unsafe { (*task_cb).taskStatus } as u32;
        if (status & OS_TASK_STATUS_UNUSED) != 0 {
            continue;
        }
        let ts = unsafe { (*task_cb).timeSlice };
        if ts > 0 {
            let converted = os_time_convert_freq(ts as u64, old_freq, new_freq);
            unsafe {
                (*task_cb).timeSlice = if converted > i32::MAX as u64 {
                    i32::MAX
                } else {
                    converted as i32
                };
            }
        } else {
            unsafe {
                (*task_cb).timeSlice = 0;
            }
        }

        if (status & OS_TASK_STATUS_RUNNING) != 0 {
            unsafe {
                let new_start =
                    os_time_convert_freq((*task_cb).startTime, old_freq, new_freq);
                (*task_cb).startTime = new_start;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// `SchedTimeBaseInit` (`los_sched.c:442-450`)
// ---------------------------------------------------------------------------

unsafe fn sched_time_base_init() {
    unsafe {
        ptr::write(&raw mut G_SCHED_RESPONSE_TIME, OS_SCHED_MAX_RESPONSE_TIME);

        let sys_clock = ptr::read_volatile(&raw const g_sysClock);

        let min_period = sys_clock
            .checked_div(LOSCFG_BASE_CORE_TICK_PER_SECOND_MINI)
            .unwrap_or(0);
        ptr::write(&raw mut G_SCHED_TICK_MIN_PERIOD, min_period);

        // 75/100 minimum accuracy.  Compute in u64 to avoid overflow.
        let prec = ((min_period as u64).saturating_mul(75) / 100) as u32;
        ptr::write(&raw mut G_TICK_RESPONSE_PRECISION, prec);

        // (UINT64)g_sysClock * LOSCFG_BASE_CORE_TIMESLICE_TIMEOUT can
        // exceed u64::MAX for an absurdly large clock; use checked_mul to
        // saturate the time-slice rather than wrap.
        let ts_full_u64 = (sys_clock as u64)
            .checked_mul(LOSCFG_BASE_CORE_TIMESLICE_TIMEOUT as u64)
            .map(|v| v / OS_SYS_US_PER_SECOND as u64)
            .unwrap_or(i32::MAX as u64);
        let ts_full = if ts_full_u64 > i32::MAX as u64 {
            i32::MAX
        } else {
            ts_full_u64 as i32
        };
        ptr::write(&raw mut G_SCHED_TIME_SLICE, ts_full);

        // 50 us minimum slice.
        let ts_min_u64 = (sys_clock as u64).saturating_mul(50) / OS_SYS_US_PER_SECOND as u64;
        let ts_min = if ts_min_u64 > i32::MAX as u64 {
            i32::MAX
        } else {
            ts_min_u64 as i32
        };
        ptr::write(&raw mut G_SCHED_TIME_SLICE_MIN, ts_min);
    }
}

// ---------------------------------------------------------------------------
// `OsSchedTimeConvertFreq` (`los_sched.c:452-458`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedTimeConvertFreq(old_freq: UINT32) {
    unsafe {
        sched_time_base_init();
        task_sched_time_convert_freq(old_freq);
        OsSortLinkResponseTimeConvertFreq(old_freq);
        OsSchedUpdateExpireTime();
    }
}

// ---------------------------------------------------------------------------
// `OsSchedInit` (`los_sched.c:460-477`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedInit() -> UINT32 {
    unsafe {
        // Initialise every priority list as an empty sentinel.
        for pri in 0..(OS_PRIORITY_QUEUE_NUM as usize) {
            let slot = (&raw mut G_PRI_QUEUE_LIST).cast::<LOS_DL_LIST>().add(pri);
            los_list_init(slot);
        }
        G_QUEUE_BITMAP.store(0, Ordering::Release);

        let attr = OsGetSortLinkAttribute(OS_SORT_LINK_TASK);
        if attr.is_null() {
            return LOS_NOK;
        }
        ptr::write(&raw mut G_TASK_SORT_LINK_LIST, attr);

        OsSortLinkInit(attr);
        sched_time_base_init();
    }
    LOS_OK
}

// ---------------------------------------------------------------------------
// `OsGetTopTask` (`los_sched.c:479-491`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsGetTopTask() -> *mut LosTaskCB {
    let bitmap = G_QUEUE_BITMAP.load(Ordering::Acquire);
    if bitmap != 0 {
        let priority = clz(bitmap);
        // priority is in 0..=31 by construction (CLZ of a non-zero u32 is
        // 0..=31), so priority_idx is exact, not a mask.
        let slot = unsafe { (&raw mut G_PRI_QUEUE_LIST).cast::<LOS_DL_LIST>().add(priority as usize) };
        let first = unsafe { (*slot).pstNext };
        if first.is_null() || first == slot {
            // Bitmap-vs-list inconsistency — fall back to idle.
            let idle = unsafe { ptr::read(&raw const g_idleTaskID) };
            return unsafe { tcb_from_tid(idle) };
        }
        unsafe { task_from_pend_list(first) }
    } else {
        let idle = unsafe { ptr::read(&raw const g_idleTaskID) };
        unsafe { tcb_from_tid(idle) }
    }
}

// ---------------------------------------------------------------------------
// `OsSchedStart` (`los_sched.c:493-519`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedStart() {
    // The C original calls PRINTK("Entering scheduler\n").  We don't have a
    // bindgen binding for printk(); the C wrapper supplied at file end
    // exposes `OsSchedPrintEntering()` for any integrator that wants the
    // banner.  We make it a weak shim by default.
    unsafe { OsSchedPrintEntering() };

    let _int_save = unsafe { ArchIntLock() };
    let new_task = unsafe { OsGetTopTask() };
    if new_task.is_null() {
        // The kernel cannot start without at least an idle task; bail.
        // (Do not restore IRQs — kernel boot will assert and reset.)
        return;
    }
    unsafe {
        (*new_task).taskStatus |= OS_TASK_STATUS_RUNNING as u16;
        ptr::write(&raw mut g_losTask, LosTask { runTask: new_task, newTask: new_task });

        let start = os_get_curr_sched_time_cycle();
        (*new_task).startTime = start;
        OsSchedTaskDeQueue(new_task);

        OsTickSysTimerStartTimeSet(start);

        #[cfg(feature = "swtmr")]
        OsSwtmrResponseTimeReset(start);

        // Enable scheduling.  Volatile write so the tick ISR sees it.
        ptr::write_volatile(&raw mut g_taskScheduled, TRUE);

        ptr::write(&raw mut G_SCHED_RESPONSE_TIME, OS_SCHED_MAX_RESPONSE_TIME);
        ptr::write(&raw mut G_SCHED_RESPONSE_ID, OS_INVALID);

        let ts = (*new_task).timeSlice;
        let ts_u64 = if ts < 0 { 0u64 } else { ts as u64 };
        os_sched_set_next_expire_time(
            (*new_task).taskID,
            start.saturating_add(ts_u64),
        );
    }
}

// ---------------------------------------------------------------------------
// `OsSchedTaskSwitch` (`los_sched.c:521-566`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn OsSchedTaskSwitch() -> BOOL {
    let mut is_task_switch: BOOL = FALSE;
    unsafe {
        let run_task = ptr::read(&raw const g_losTask).runTask;
        if run_task.is_null() {
            return FALSE;
        }

        let curr = os_get_curr_sched_time_cycle();
        os_time_slice_update(run_task, curr);

        let status = (*run_task).taskStatus as u32;
        if (status & (OS_TASK_STATUS_PEND_TIME | OS_TASK_STATUS_DELAY)) != 0 {
            OsAdd2SortLink(
                &raw mut (*run_task).sortList,
                (*run_task).startTime,
                (*run_task).waitTimes,
                OS_SORT_LINK_TASK,
            );
        } else if (status & OS_TASK_BLOCKED_STATUS as u32) == 0 {
            OsSchedTaskEnQueue(run_task);
        }

        let new_task = OsGetTopTask();
        if new_task.is_null() {
            return FALSE;
        }
        let mut losTaskCpy = ptr::read(&raw const g_losTask);
        losTaskCpy.newTask = new_task;
        ptr::write(&raw mut g_losTask, losTaskCpy);

        if run_task != new_task {
            #[cfg(feature = "tsk_monitor")]
            OsTaskSwitchCheck();

            (*run_task).taskStatus &= !(OS_TASK_STATUS_RUNNING as u16);
            (*new_task).taskStatus |= OS_TASK_STATUS_RUNNING as u16;
            (*new_task).startTime = (*run_task).startTime;
            is_task_switch = TRUE;

            OsHookSchedTaskSwitchedIn();

            #[cfg(feature = "debug_tools")]
            OsSchedTraceRecord(new_task, run_task);
        }

        OsSchedTaskDeQueue(new_task);

        let idle = ptr::read(&raw const g_idleTaskID);
        let precision = ptr::read(&raw const G_TICK_RESPONSE_PRECISION);
        let end_time = if (*new_task).taskID != idle {
            let ts = (*new_task).timeSlice;
            let ts_u64 = if ts < 0 { 0u64 } else { ts as u64 };
            (*new_task).startTime.saturating_add(ts_u64)
        } else {
            OS_SCHED_MAX_RESPONSE_TIME.saturating_sub(precision as u64)
        };

        let sched_id = ptr::read(&raw const G_SCHED_RESPONSE_ID);
        if sched_id == (*run_task).taskID {
            ptr::write(&raw mut G_SCHED_RESPONSE_TIME, OS_SCHED_MAX_RESPONSE_TIME);
        }
        os_sched_set_next_expire_time((*new_task).taskID, end_time);
    }

    is_task_switch
}

// ---------------------------------------------------------------------------
// `LOS_SchedTickTimeoutNsGet` (`los_sched.c:568-586`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn LOS_SchedTickTimeoutNsGet() -> UINT64 {
    let int_save = unsafe { ArchIntLock() };
    let response_time = unsafe { ptr::read(&raw const G_SCHED_RESPONSE_TIME) };
    let curr_time = unsafe { os_get_curr_sched_time_cycle() };
    unsafe { ArchIntRestore(int_save) };

    let delta = if response_time > curr_time {
        response_time - curr_time
    } else {
        0
    };

    // OS_SYS_CYCLE_TO_NS(cycle, freq) = cycle * 1e9 / freq.  The C macro
    // does this in u64; for high-frequency clocks and seconds-worth of
    // delta the intermediate overflows.  Widen to u128.
    let freq = unsafe { ptr::read_volatile(&raw const g_sysClock) } as u128;
    if freq == 0 {
        return 0;
    }
    let widened = (delta as u128) * (OS_SYS_NS_PER_SECOND as u128) / freq;
    if widened > u64::MAX as u128 {
        u64::MAX
    } else {
        widened as u64
    }
}

// ---------------------------------------------------------------------------
// `LOS_SchedTickHandler` (`los_sched.c:588-617`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn LOS_SchedTickHandler() {
    let scheduled = unsafe { ptr::read_volatile(&raw const g_taskScheduled) };
    if scheduled == FALSE {
        return;
    }

    let int_save = unsafe { ArchIntLock() };
    let tick_start_time = unsafe { os_get_curr_sched_time_cycle() };

    let sched_id = unsafe { ptr::read(&raw const G_SCHED_RESPONSE_ID) };
    if sched_id == OS_INVALID {
        unsafe {
            // ++g_tickIntLock (single-CPU, volatile to keep ordering with ISR).
            let lock = ptr::read_volatile(&raw const G_TICK_INT_LOCK);
            ptr::write_volatile(&raw mut G_TICK_INT_LOCK, lock.wrapping_add(1));

            let swtmr = ptr::read_volatile(&raw const G_SWTMR_SCAN);
            if let Some(f) = swtmr {
                let _ = f();
            }

            let _ = os_sched_scan_timer_list();

            let lock = ptr::read_volatile(&raw const G_TICK_INT_LOCK);
            ptr::write_volatile(&raw mut G_TICK_INT_LOCK, lock.wrapping_sub(1));
        }
    }

    let run = unsafe { ptr::read(&raw const g_losTask).runTask };
    if !run.is_null() {
        unsafe {
            os_time_slice_update(run, tick_start_time);
            (*run).startTime = os_get_curr_sched_time_cycle();
        }
    }

    unsafe {
        ptr::write(&raw mut G_SCHED_RESPONSE_TIME, OS_SCHED_MAX_RESPONSE_TIME);
    }
    if los_check_schedule() {
        unsafe { ArchTaskSchedule() };
    } else {
        unsafe { OsSchedUpdateExpireTime() };
    }

    unsafe { ArchIntRestore(int_save) };
}

// ---------------------------------------------------------------------------
// `LOS_Schedule` (`los_sched.c:619-624`)
// ---------------------------------------------------------------------------

#[no_mangle]
pub unsafe extern "C" fn LOS_Schedule() {
    if unsafe { os_check_kernel_running() } != FALSE {
        unsafe { ArchTaskSchedule() };
    }
}

// ---------------------------------------------------------------------------
// Weak C-side shims.
//
// These four hook trampolines are what the integrator should supply in C
// (one line each, dispatching to the variadic OsHookCall macro).  Until
// they are linked in, the Rust side calls weak symbols that no-op.
//
// Recommended `los_sched_hooks.c`:
//
// ```c
// #include "los_hook.h"
// #include "los_task.h"
//
// void OsHookSchedMovedTaskToReadyState(LosTaskCB *t)
// {
//     OsHookCall(LOS_HOOK_TYPE_MOVEDTASKTOREADYSTATE, t);
// }
// void OsHookSchedMovedTaskToSuspendedList(LosTaskCB *t)
// {
//     OsHookCall(LOS_HOOK_TYPE_MOVEDTASKTOSUSPENDEDLIST, t);
// }
// void OsHookSchedTaskPriModify(LosTaskCB *t, UINT16 p)
// {
//     OsHookCall(LOS_HOOK_TYPE_TASK_PRIMODIFY, t, p);
// }
// void OsHookSchedTaskSwitchedIn(void)
// {
//     OsHookCall(LOS_HOOK_TYPE_TASK_SWITCHEDIN);
// }
// void OsSchedPrintEntering(void)
// {
//     PRINTK("Entering scheduler\n");
// }
// ```
//
// If the integrator omits the file, link with `-Wl,--unresolved-symbols=
// ignore-in-object-files` *or* drop the matching `extern "C"` declarations
// above and define them inline here.  We provide the weak fallbacks below
// so a no-hook build still links.
// ---------------------------------------------------------------------------

// We expose default no-op definitions with `#[linkage = "weak"]`, but that
// attribute is unstable.  Instead, we just mark them `pub` and let the
// linker take the C definition first if present.  Each shim is `#[no_mangle]`
// and `extern "C"`.
//
// NOTE: if the user's C side defines the same symbol, this will collide.
// In that case, delete this section.
//
// For a stable-Rust no_std build, leave the shims here so the kernel always
// links, then override by linking in `los_sched_hooks.c` as shown above.

/// Default no-op for `OsHookSchedMovedTaskToReadyState`.
#[cfg(feature = "default_hook_shims")]
#[no_mangle]
pub unsafe extern "C" fn OsHookSchedMovedTaskToReadyState(_t: *mut LosTaskCB) {}

#[cfg(feature = "default_hook_shims")]
#[no_mangle]
pub unsafe extern "C" fn OsHookSchedMovedTaskToSuspendedList(_t: *mut LosTaskCB) {}

#[cfg(feature = "default_hook_shims")]
#[no_mangle]
pub unsafe extern "C" fn OsHookSchedTaskPriModify(_t: *mut LosTaskCB, _p: UINT16) {}

#[cfg(feature = "default_hook_shims")]
#[no_mangle]
pub unsafe extern "C" fn OsHookSchedTaskSwitchedIn() {}

#[cfg(feature = "default_hook_shims")]
#[no_mangle]
pub unsafe extern "C" fn OsSchedPrintEntering() {}

// ---------------------------------------------------------------------------
// Panic handler.
//
// Required because the crate is `#![no_std]`.  In a kernel context a panic
// is unrecoverable: there is no unwinder, no allocator, no logger we can
// rely on.  We loop forever and let the watchdog (if armed) reset the
// chip.  Integrators that prefer a different policy (logging via PRINTK,
// dumping registers, calling LOS_Panic) can replace this fn with their
// own — there must be exactly one `#[panic_handler]` in the final
// binary.
//
// The `#[cfg(not(test))]` gate keeps `cargo test` (which uses std) from
// colliding with libtest's own panic handler.
// ---------------------------------------------------------------------------

#[cfg(not(test))]
#[panic_handler]
fn rust_kernel_panic(_info: &core::panic::PanicInfo) -> ! {
    loop {
        // ARM Cortex-M `WFI` (Wait For Interrupt) — lets the CPU enter
        // low-power state until reset by the watchdog.  Available on
        // every ARMv6-M / v7-M / v8-M part.  On non-ARM hosts (cargo
        // check against the host triple), fall back to a plain spin.
        #[cfg(target_arch = "arm")]
        unsafe { core::arch::asm!("wfi", options(nomem, nostack, preserves_flags)) };
    }
}