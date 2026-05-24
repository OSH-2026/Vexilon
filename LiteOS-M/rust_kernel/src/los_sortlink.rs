#![crate_type = "staticlib"]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(static_mut_refs)]

//! Rust rewrite of `c_kernel/src/los_sortlink.c`.
//!
//! The public symbols, global variables and raw-pointer list manipulation keep
//! the same C ABI as the original LiteOS-M implementation.  Helper functions
//! below mirror the `STATIC INLINE` list/sortlink/tick macros that bindgen does
//! not emit as callable Rust functions.

mod include {
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(non_upper_case_globals)]
    #![allow(dead_code)]
    #![allow(unused_imports)]
    pub mod los_config_h;
    pub mod los_sortlink_h;
}

use crate::include::los_config_h;
use crate::include::los_sortlink_h as sortlink;
use core::ptr;

pub type UINT32 = sortlink::UINT32;
pub type UINT64 = sortlink::UINT64;
pub type INT32 = sortlink::INT32;
pub type CHAR = sortlink::CHAR;
pub type LOS_DL_LIST = sortlink::LOS_DL_LIST;
pub type SortLinkType = sortlink::SortLinkType;
pub type SortLinkList = sortlink::SortLinkList;
pub type SortLinkAttribute = sortlink::SortLinkAttribute;

const LOS_OK: UINT32 = sortlink::LOS_OK;
const TRUE: UINT32 = 1;
const FALSE: UINT32 = 0;
const LOG_ERR_LEVEL: INT32 = 2;
const OS_SORT_LINK_INVALID_TIME: UINT64 = UINT64::MAX;

#[inline]
const fn empty_list_node() -> LOS_DL_LIST {
    LOS_DL_LIST {
        pstPrev: ptr::null_mut(),
        pstNext: ptr::null_mut(),
    }
}

#[no_mangle]
pub static mut g_taskSortLink: SortLinkAttribute = SortLinkAttribute {
    sortLink: empty_list_node(),
};

// `los_sortlink.h` exposes this global only when `LOSCFG_BASE_CORE_SWTMR == 1`.
// The supplied LiteOS-M configuration enables software timers, so the Rust
// replacement provides the symbol expected by the rest of the kernel.
#[no_mangle]
pub static mut g_swtmrSortLink: SortLinkAttribute = SortLinkAttribute {
    sortLink: empty_list_node(),
};

unsafe extern "C" {
    fn ArchIntLock() -> UINT32;
    fn ArchIntRestore(intSave: UINT32);
    fn LOS_SysCycleGet() -> UINT64;
    fn OsSchedResetSchedResponseTime(responseTime: UINT64);
    fn OsLogLevelCheck(level: INT32) -> INT32;
    fn printf(fmt: *const CHAR, ...) -> INT32;
    fn LOS_Panic(fmt: *const CHAR, ...) -> !;

    static mut g_sysClock: UINT32;
}

#[inline]
unsafe fn LOS_ListInit(list: *mut LOS_DL_LIST) {
    unsafe {
        (*list).pstNext = list;
        (*list).pstPrev = list;
    }
}

#[inline]
unsafe fn LOS_ListAdd(list: *mut LOS_DL_LIST, node: *mut LOS_DL_LIST) {
    unsafe {
        (*node).pstNext = (*list).pstNext;
        (*node).pstPrev = list;
        (*(*list).pstNext).pstPrev = node;
        (*list).pstNext = node;
    }
}

#[inline]
unsafe fn LOS_ListDelete(node: *mut LOS_DL_LIST) {
    unsafe {
        (*(*node).pstNext).pstPrev = (*node).pstPrev;
        (*(*node).pstPrev).pstNext = (*node).pstNext;
        (*node).pstNext = ptr::null_mut();
        (*node).pstPrev = ptr::null_mut();
    }
}

#[inline]
unsafe fn LOS_ListEmpty(node: *const LOS_DL_LIST) -> bool {
    unsafe { (*node).pstNext == node as *mut LOS_DL_LIST }
}

#[inline]
unsafe fn list_entry_sortlink_list(item: *mut LOS_DL_LIST) -> *mut SortLinkList {
    // In `SortLinkList`, `sortLinkNode` is the first field.  The bindgen
    // layout assertion in `los_sortlink_h.rs` checks this offset is zero.
    item as *mut SortLinkList
}

#[inline]
unsafe fn set_sortlist_value(sort_list: *mut SortLinkList, value: UINT64) {
    unsafe {
        (*sort_list).responseTime = value;
    }
}

#[inline]
unsafe fn OsDeleteNodeSortLink(sort_list: *mut SortLinkList) {
    unsafe {
        LOS_ListDelete(ptr::addr_of_mut!((*sort_list).sortLinkNode));
        set_sortlist_value(sort_list, OS_SORT_LINK_INVALID_TIME);
    }
}

#[inline]
unsafe fn os_sys_tick_to_cycle(ticks: UINT32) -> UINT64 {
    unsafe {
        (ticks as UINT64).wrapping_mul(g_sysClock as UINT64)
            / los_config_h::LOSCFG_BASE_CORE_TICK_PER_SECOND as UINT64
    }
}

#[inline]
unsafe fn OsGetCurrSchedTimeCycle() -> UINT64 {
    unsafe { LOS_SysCycleGet() }
}

#[inline]
unsafe fn OsTimeConvertFreq(time: UINT64, old_freq: UINT32, new_freq: UINT32) -> UINT64 {
    if old_freq >= new_freq {
        time / (old_freq / new_freq) as UINT64
    } else {
        time.wrapping_mul((new_freq / old_freq) as UINT64)
    }
}

#[inline]
unsafe fn print_err_invalid_sort_link_type() {
    unsafe {
        if OsLogLevelCheck(LOG_ERR_LEVEL) == 0 {
            printf(b"Invalid sort link type!\n\0".as_ptr() as *const CHAR);
        }
    }
}

#[inline]
unsafe fn OsAddNode2SortLink(sort_link_head: *mut SortLinkAttribute, sort_list: *mut SortLinkList) {
    unsafe {
        let head = ptr::addr_of_mut!((*sort_link_head).sortLink);

        if LOS_ListEmpty(head) {
            LOS_ListAdd(head, ptr::addr_of_mut!((*sort_list).sortLinkNode));
            return;
        }

        let mut list_sorted = list_entry_sortlink_list((*head).pstNext);
        if (*list_sorted).responseTime > (*sort_list).responseTime {
            LOS_ListAdd(head, ptr::addr_of_mut!((*sort_list).sortLinkNode));
            return;
        } else if (*list_sorted).responseTime == (*sort_list).responseTime {
            LOS_ListAdd((*head).pstNext, ptr::addr_of_mut!((*sort_list).sortLinkNode));
            return;
        }

        let mut prev_node = (*head).pstPrev;
        loop {
            list_sorted = list_entry_sortlink_list(prev_node);
            if (*list_sorted).responseTime <= (*sort_list).responseTime {
                LOS_ListAdd(prev_node, ptr::addr_of_mut!((*sort_list).sortLinkNode));
                break;
            }
            prev_node = (*prev_node).pstPrev;
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn OsSortLinkInit(sortLinkHead: *mut SortLinkAttribute) -> UINT32 {
    unsafe {
        LOS_ListInit(ptr::addr_of_mut!((*sortLinkHead).sortLink));
    }
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn OsAdd2SortLink(
    node: *mut SortLinkList,
    startTime: UINT64,
    waitTicks: UINT32,
    type_: SortLinkType,
) {
    unsafe {
        let sort_link_head = if type_ == sortlink::SortLinkType_OS_SORT_LINK_TASK {
            ptr::addr_of_mut!(g_taskSortLink)
        } else if los_config_h::LOSCFG_BASE_CORE_SWTMR == 1
            && type_ == sortlink::SortLinkType_OS_SORT_LINK_SWTMR
        {
            ptr::addr_of_mut!(g_swtmrSortLink)
        } else {
            LOS_Panic(
                b"Sort link type error : %u\n\0".as_ptr() as *const CHAR,
                type_,
            );
        };

        let int_save = ArchIntLock();
        set_sortlist_value(node, startTime.wrapping_add(os_sys_tick_to_cycle(waitTicks)));
        OsAddNode2SortLink(sort_link_head, node);
        ArchIntRestore(int_save);
    }
}

#[no_mangle]
pub unsafe extern "C" fn OsDeleteSortLink(node: *mut SortLinkList) {
    unsafe {
        let int_save = ArchIntLock();
        if (*node).responseTime != OS_SORT_LINK_INVALID_TIME {
            OsSchedResetSchedResponseTime((*node).responseTime);
            OsDeleteNodeSortLink(node);
        }
        ArchIntRestore(int_save);
    }
}

#[inline]
unsafe fn SortLinkNodeTimeUpdate(sort_link_head: *mut SortLinkAttribute, old_freq: UINT32) {
    unsafe {
        let head = ptr::addr_of_mut!((*sort_link_head).sortLink);

        if LOS_ListEmpty(head) {
            return;
        }

        let mut next_node = (*head).pstNext;
        loop {
            let list_sorted = list_entry_sortlink_list(next_node);
            (*list_sorted).responseTime = OsTimeConvertFreq(
                (*list_sorted).responseTime,
                old_freq,
                g_sysClock,
            );
            next_node = (*next_node).pstNext;
            if next_node == head {
                break;
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn OsSortLinkResponseTimeConvertFreq(oldFreq: UINT32) {
    unsafe {
        SortLinkNodeTimeUpdate(ptr::addr_of_mut!(g_taskSortLink), oldFreq);

        if los_config_h::LOSCFG_BASE_CORE_SWTMR == 1 {
            SortLinkNodeTimeUpdate(ptr::addr_of_mut!(g_swtmrSortLink), oldFreq);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn OsGetSortLinkAttribute(type_: SortLinkType) -> *mut SortLinkAttribute {
    unsafe {
        if type_ == sortlink::SortLinkType_OS_SORT_LINK_TASK {
            return ptr::addr_of_mut!(g_taskSortLink);
        }

        if los_config_h::LOSCFG_BASE_CORE_SWTMR == 1
            && type_ == sortlink::SortLinkType_OS_SORT_LINK_SWTMR
        {
            return ptr::addr_of_mut!(g_swtmrSortLink);
        }

        print_err_invalid_sort_link_type();
        ptr::null_mut()
    }
}

#[no_mangle]
pub unsafe extern "C" fn OsSortLinkGetTargetExpireTime(
    currTime: UINT64,
    targetSortList: *const SortLinkList,
) -> UINT64 {
    unsafe {
        if currTime >= (*targetSortList).responseTime {
            return 0;
        }

        (*targetSortList).responseTime - currTime
    }
}

#[no_mangle]
pub unsafe extern "C" fn OsSortLinkGetNextExpireTime(
    sortLinkHead: *const SortLinkAttribute,
) -> UINT64 {
    unsafe {
        let head = ptr::addr_of!((*sortLinkHead).sortLink) as *mut LOS_DL_LIST;

        if LOS_ListEmpty(head) {
            return 0;
        }

        let list_sorted = list_entry_sortlink_list((*head).pstNext) as *const SortLinkList;
        OsSortLinkGetTargetExpireTime(OsGetCurrSchedTimeCycle(), list_sorted)
    }
}
