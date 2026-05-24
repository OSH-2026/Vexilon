#![crate_type = "staticlib"]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]

//! Rust rewrite of `c_kernel/src/los_membox.c`.
//!
//! This file keeps the LiteOS-M C ABI and uses the bindgen-generated Rust
//! headers already present under `src/include`.  The implementation mirrors the
//! configured, non-`LOSCFG_PLATFORM_EXC` build exposed by `los_membox_h.rs`.

mod include {
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(non_upper_case_globals)]
    #![allow(dead_code)]
    #![allow(unused_imports)]

    pub mod los_compiler_h;
    pub mod los_debug_h;
    pub mod los_interrupt_h;
    pub mod los_membox_h;
    pub mod los_task_h;
}

use crate::include::los_compiler_h::LOS_NOK;
use crate::include::los_debug_h::{printf, OsLogLevelCheck, LOG_INFO_LEVEL};
use crate::include::los_interrupt_h::{ArchIntLock, ArchIntRestore};
use crate::include::los_membox_h::{
    LOS_MEMBOX_INFO, LOS_MEMBOX_NODE, LOS_OK, UINT32, UINTPTR,
};
use crate::include::los_task_h::{LOS_CurTaskIDGet, LOSCFG_BASE_CORE_TSK_LIMIT};
use std::ffi::c_void;
use std::mem;
use std::os::raw::{c_char, c_int};
use std::ptr;

/*
 * The magic length is 32 bits, the lower 8 bits save the owner task ID,
 * and the other 24 bits are used as the verification magic number.
 */
const OS_MEMBOX_MAGIC: usize = 0xa55a5a00;
const OS_MEMBOX_TASKID_BITS: usize = 8;
const OS_MEMBOX_MAX_TASKID: usize = (1usize << OS_MEMBOX_TASKID_BITS) - 1;
const OS_MEMBOX_NODE_HEAD_SIZE: usize = mem::size_of::<LOS_MEMBOX_NODE>();

#[inline]
unsafe fn os_membox_taskid_get(addr: *mut LOS_MEMBOX_NODE) -> UINT32 {
    ((addr as usize) & OS_MEMBOX_MAX_TASKID) as UINT32
}

#[inline]
unsafe fn os_membox_set_magic(node: *mut LOS_MEMBOX_NODE) {
    let task_id = LOS_CurTaskIDGet() as usize;
    (*node).pstNext = (OS_MEMBOX_MAGIC | (task_id & OS_MEMBOX_MAX_TASKID)) as *mut LOS_MEMBOX_NODE;
}

#[inline]
unsafe fn os_membox_check_magic(node: *mut LOS_MEMBOX_NODE) -> UINT32 {
    let task_id = os_membox_taskid_get((*node).pstNext);

    if task_id > LOSCFG_BASE_CORE_TSK_LIMIT + 1 {
        LOS_NOK
    } else if (*node).pstNext as usize == (OS_MEMBOX_MAGIC | task_id as usize) {
        LOS_OK
    } else {
        LOS_NOK
    }
}

#[inline]
fn los_membox_aligned(mem_addr: UINT32) -> UINT32 {
    let align = mem::size_of::<UINTPTR>() as UINT32;
    mem_addr.wrapping_add(align - 1) & !(align - 1)
}

#[inline]
unsafe fn os_membox_next(addr: *mut LOS_MEMBOX_NODE, blk_size: UINT32) -> *mut LOS_MEMBOX_NODE {
    (addr as *mut u8).add(blk_size as usize) as *mut LOS_MEMBOX_NODE
}

#[inline]
unsafe fn os_membox_user_addr(addr: *mut LOS_MEMBOX_NODE) -> *mut c_void {
    (addr as *mut u8).add(OS_MEMBOX_NODE_HEAD_SIZE) as *mut c_void
}

#[inline]
unsafe fn os_membox_node_addr(addr: *mut c_void) -> *mut LOS_MEMBOX_NODE {
    (addr as *mut u8).sub(OS_MEMBOX_NODE_HEAD_SIZE) as *mut LOS_MEMBOX_NODE
}

#[inline]
unsafe fn os_check_box_mem(box_info: *const LOS_MEMBOX_INFO, node: *const c_void) -> UINT32 {
    if (*box_info).uwBlkSize == 0 {
        return LOS_NOK;
    }

    let start = box_info.add(1) as usize;
    let offset = (node as usize).wrapping_sub(start) as UINT32;

    if offset % (*box_info).uwBlkSize != 0 {
        return LOS_NOK;
    }

    if offset / (*box_info).uwBlkSize >= (*box_info).uwBlkNum {
        return LOS_NOK;
    }

    os_membox_check_magic(node as *mut LOS_MEMBOX_NODE)
}

#[inline]
unsafe fn membox_lock(state: &mut UINT32) {
    *state = ArchIntLock();
}

#[inline]
unsafe fn membox_unlock(state: UINT32) {
    ArchIntRestore(state);
}

#[inline]
unsafe fn print_info_enabled() -> bool {
    OsLogLevelCheck(LOG_INFO_LEVEL as c_int) == 0
}

/// Initialize a static memory box pool.
#[no_mangle]
pub unsafe extern "C" fn LOS_MemboxInit(
    pool: *mut c_void,
    poolSize: UINT32,
    blkSize: UINT32,
) -> UINT32 {
    if pool.is_null() || blkSize == 0 {
        return LOS_NOK;
    }

    if (poolSize as usize) < mem::size_of::<LOS_MEMBOX_INFO>() {
        return LOS_NOK;
    }

    let box_info = pool as *mut LOS_MEMBOX_INFO;
    let mut int_save: UINT32 = 0;

    membox_lock(&mut int_save);

    (*box_info).uwBlkSize = los_membox_aligned(blkSize.wrapping_add(OS_MEMBOX_NODE_HEAD_SIZE as UINT32));
    (*box_info).uwBlkNum =
        (poolSize - mem::size_of::<LOS_MEMBOX_INFO>() as UINT32) / (*box_info).uwBlkSize;
    (*box_info).uwBlkCnt = 0;

    if (*box_info).uwBlkNum == 0 {
        membox_unlock(int_save);
        return LOS_NOK;
    }

    let mut node = box_info.add(1) as *mut LOS_MEMBOX_NODE;
    (*box_info).stFreeList.pstNext = node;

    let last_index = (*box_info).uwBlkNum - 1;
    let mut index: UINT32 = 0;
    while index < last_index {
        (*node).pstNext = os_membox_next(node, (*box_info).uwBlkSize);
        node = (*node).pstNext;
        index += 1;
    }
    (*node).pstNext = ptr::null_mut();

    membox_unlock(int_save);
    LOS_OK
}

/// Allocate one block from a static memory box pool.
#[no_mangle]
pub unsafe extern "C" fn LOS_MemboxAlloc(pool: *mut c_void) -> *mut c_void {
    if pool.is_null() {
        return ptr::null_mut();
    }

    let box_info = pool as *mut LOS_MEMBOX_INFO;
    let mut node_tmp: *mut LOS_MEMBOX_NODE = ptr::null_mut();
    let mut int_save: UINT32 = 0;

    membox_lock(&mut int_save);

    let node = &mut (*box_info).stFreeList as *mut LOS_MEMBOX_NODE;
    if !(*node).pstNext.is_null() {
        node_tmp = (*node).pstNext;
        (*node).pstNext = (*node_tmp).pstNext;
        os_membox_set_magic(node_tmp);
        (*box_info).uwBlkCnt = (*box_info).uwBlkCnt.wrapping_add(1);
    }

    membox_unlock(int_save);

    if node_tmp.is_null() {
        ptr::null_mut()
    } else {
        os_membox_user_addr(node_tmp)
    }
}

/// Free one block back to a static memory box pool.
#[no_mangle]
pub unsafe extern "C" fn LOS_MemboxFree(pool: *mut c_void, box_: *mut c_void) -> UINT32 {
    if pool.is_null() || box_.is_null() {
        return LOS_NOK;
    }

    let box_info = pool as *mut LOS_MEMBOX_INFO;
    let mut ret: UINT32 = LOS_NOK;
    let mut int_save: UINT32 = 0;

    membox_lock(&mut int_save);

    let node = os_membox_node_addr(box_);
    if os_check_box_mem(box_info, node as *const c_void) == LOS_OK {
        (*node).pstNext = (*box_info).stFreeList.pstNext;
        (*box_info).stFreeList.pstNext = node;
        (*box_info).uwBlkCnt = (*box_info).uwBlkCnt.wrapping_sub(1);
        ret = LOS_OK;
    }

    membox_unlock(int_save);
    ret
}

/// Clear one allocated memory box block to zero.
#[no_mangle]
pub unsafe extern "C" fn LOS_MemboxClr(pool: *mut c_void, box_: *mut c_void) {
    if pool.is_null() || box_.is_null() {
        return;
    }

    let box_info = pool as *mut LOS_MEMBOX_INFO;
    let clear_size = (*box_info).uwBlkSize as usize - OS_MEMBOX_NODE_HEAD_SIZE;
    ptr::write_bytes(box_ as *mut u8, 0, clear_size);
}

/// Print memory box information and free/all-node lists.
#[no_mangle]
pub unsafe extern "C" fn LOS_ShowBox(pool: *mut c_void) {
    if pool.is_null() {
        return;
    }

    let box_info = pool as *mut LOS_MEMBOX_INFO;
    let mut int_save: UINT32 = 0;

    membox_lock(&mut int_save);

    if print_info_enabled() {
        printf(
            b"membox(%p, 0x%x, 0x%x):\r\n\0".as_ptr() as *const c_char,
            pool,
            (*box_info).uwBlkSize,
            (*box_info).uwBlkNum,
        );
        printf(b"free node list:\r\n\0".as_ptr() as *const c_char);

        let mut node = (*box_info).stFreeList.pstNext;
        let mut index: UINT32 = 0;
        while !node.is_null() {
            printf(
                b"(%u, %p)\r\n\0".as_ptr() as *const c_char,
                index,
                node as *mut c_void,
            );
            node = (*node).pstNext;
            index += 1;
        }

        printf(b"all node list:\r\n\0".as_ptr() as *const c_char);
        node = box_info.add(1) as *mut LOS_MEMBOX_NODE;
        index = 0;
        while index < (*box_info).uwBlkNum {
            printf(
                b"(%u, %p, %p)\r\n\0".as_ptr() as *const c_char,
                index,
                node as *mut c_void,
                (*node).pstNext as *mut c_void,
            );
            node = os_membox_next(node, (*box_info).uwBlkSize);
            index += 1;
        }
    }

    membox_unlock(int_save);
}

/// Get memory box statistics.
#[no_mangle]
pub unsafe extern "C" fn LOS_MemboxStatisticsGet(
    boxMem: *const c_void,
    maxBlk: *mut UINT32,
    blkCnt: *mut UINT32,
    blkSize: *mut UINT32,
) -> UINT32 {
    if boxMem.is_null() || maxBlk.is_null() || blkCnt.is_null() || blkSize.is_null() {
        return LOS_NOK;
    }

    let box_info = boxMem as *const LOS_MEMBOX_INFO;
    *maxBlk = (*box_info).uwBlkNum;
    *blkCnt = (*box_info).uwBlkCnt;
    *blkSize = (*box_info).uwBlkSize;

    LOS_OK
}
