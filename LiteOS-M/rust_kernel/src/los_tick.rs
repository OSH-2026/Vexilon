#![crate_type = "staticlib"]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(static_mut_refs)]

//! Rust rewrite of `c_kernel/src/los_tick.c`.
//!
//! This file keeps the LiteOS-M C ABI intact: exported function names,
//! global symbols and raw timer callbacks match the original C module.
//! It assumes the supplied bindgen header `include/los_tick_h.rs` is present
//! next to the other generated Rust headers.

mod include {
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    #![allow(non_upper_case_globals)]
    #![allow(dead_code)]
    #![allow(unused_imports)]

    pub mod los_tick_h;
}

use crate::include::los_tick_h as tick;
use core::ptr;

pub type UINT32 = tick::UINT32;
pub type UINT64 = tick::UINT64;
pub type UINTPTR = tick::UINTPTR;
pub type INT32 = tick::INT32;
pub type BOOL = tick::BOOL;
pub type DOUBLE = tick::DOUBLE;
pub type HWI_PROC_FUNC = tick::HWI_PROC_FUNC;
pub type SYS_TICK_FREQ_ADJUST_FUNC = tick::SYS_TICK_FREQ_ADJUST_FUNC;
pub type ArchTickTimer = tick::ArchTickTimer;
pub type CpuTick = tick::CpuTick;

const TRUE: BOOL = 1;
const FALSE: BOOL = 0;
const LOS_OK: UINT32 = tick::LOS_OK;
const OS_NULL_INT: UINT32 = u32::MAX;
const UINT32_MAX_VALUE: UINT32 = u32::MAX;

const LOS_MOD_SYS: UINT32 = 0;
const LOS_MOD_TICK: UINT32 = 4;

const fn los_errno_os_error(module_id: UINT32, errno: UINT32) -> UINT32 {
    0x0200_0000 | (module_id << 8) | errno
}

const LOS_ERRNO_TICK_CFG_INVALID: UINT32 = los_errno_os_error(LOS_MOD_TICK, 0x00);
const LOS_ERRNO_SYS_PTR_NULL: UINT32 = los_errno_os_error(LOS_MOD_SYS, 0x10);
const LOS_ERRNO_SYS_CLOCK_INVALID: UINT32 = los_errno_os_error(LOS_MOD_SYS, 0x11);
const LOS_ERRNO_SYS_TIMER_IS_RUNNING: UINT32 = los_errno_os_error(LOS_MOD_SYS, 0x15);
const LOS_ERRNO_SYS_HOOK_IS_NULL: UINT32 = los_errno_os_error(LOS_MOD_SYS, 0x16);
const LOS_ERRNO_SYS_TIMER_ADDR_FAULT: UINT32 = los_errno_os_error(LOS_MOD_SYS, 0x16);

const LOSCFG_BASE_CORE_TICK_PER_SECOND: UINT32 = tick::LOSCFG_BASE_CORE_TICK_PER_SECOND;
const LOSCFG_BASE_CORE_TICK_WTIMER: UINT32 = tick::LOSCFG_BASE_CORE_TICK_WTIMER;
const LOSCFG_BASE_CORE_TICK_RESPONSE_MAX: UINT64 = tick::LOSCFG_BASE_CORE_TICK_RESPONSE_MAX as UINT64;
const LOSCFG_PLATFORM_HWI_LIMIT: UINT32 = tick::LOSCFG_PLATFORM_HWI_LIMIT;
const OS_SYS_MS_PER_SECOND: UINT32 = tick::OS_SYS_MS_PER_SECOND;
const OS_SYS_US_PER_SECOND: UINT32 = tick::OS_SYS_US_PER_SECOND;
const OS_SYS_US_PER_MS: UINT32 = tick::OS_SYS_US_PER_MS;
const OS_SYS_NS_PER_SECOND: UINT64 = tick::OS_SYS_NS_PER_SECOND as UINT64;
const OS_SYS_NS_PER_MS: UINT64 = tick::OS_SYS_NS_PER_MS as UINT64;
const OS_SYS_MV_32_BIT: UINT32 = tick::OS_SYS_MV_32_BIT;

#[no_mangle]
pub static mut g_ticksPerSec: UINT32 = 0;

#[no_mangle]
pub static mut g_uwCyclePerSec: UINT32 = 0;

#[no_mangle]
pub static mut g_cyclesPerTick: UINT32 = 0;

#[no_mangle]
pub static mut g_sysClock: UINT32 = 0;

static mut G_SYS_TICK_TIMER: *mut ArchTickTimer = ptr::null_mut();
static mut G_SYS_TIMER_IS_INIT: BOOL = FALSE;
static mut G_TICK_TIMER_START_TIME: UINT64 = 0;

static mut G_TICK_TIMER_BASE: UINT64 = 0;
static mut G_OLD_TICK_TIMER_BASE: UINT64 = 0;
static mut G_TICK_TIMER_BASE_UPDATE: BOOL = FALSE;

unsafe extern "C" {
    fn ArchSysTickTimerGet() -> *mut ArchTickTimer;
    fn ArchIntLock() -> UINT32;
    fn ArchIntRestore(intSave: UINT32);
    fn LOS_SchedTickHandler();
    fn OsSchedTimeConvertFreq(oldFreq: UINT32);
}

#[inline]
unsafe fn sys_tick_timer() -> *mut ArchTickTimer {
    if G_SYS_TICK_TIMER.is_null() {
        G_SYS_TICK_TIMER = ArchSysTickTimerGet();
    }
    G_SYS_TICK_TIMER
}

#[inline]
unsafe fn call_init(timer: *mut ArchTickTimer, handler: HWI_PROC_FUNC) -> UINT32 {
    ((*timer).init.unwrap())(handler)
}

#[inline]
unsafe fn call_get_cycle(timer: *mut ArchTickTimer, period: *mut UINT32) -> UINT64 {
    ((*timer).getCycle.unwrap())(period)
}

#[inline]
unsafe fn call_reload(timer: *mut ArchTickTimer, period: UINT64) -> UINT64 {
    ((*timer).reload.unwrap())(period)
}

#[inline]
unsafe fn call_lock(timer: *mut ArchTickTimer) {
    ((*timer).lock.unwrap())();
}

#[inline]
unsafe fn call_unlock(timer: *mut ArchTickTimer) {
    ((*timer).unlock.unwrap())();
}

#[inline]
fn os_time_convert_freq(time: UINT64, old_freq: UINT32, new_freq: UINT32) -> UINT64 {
    if old_freq >= new_freq {
        time / ((old_freq / new_freq) as UINT64)
    } else {
        time * ((new_freq / old_freq) as UINT64)
    }
}

#[inline]
unsafe fn os_sys_cycle_to_tick(cycle: UINT64) -> UINT64 {
    cycle * (LOSCFG_BASE_CORE_TICK_PER_SECOND as UINT64) / (g_sysClock as UINT64)
}

unsafe fn os_update_sys_time_base() {
    let mut period: UINT32 = 0;

    if G_TICK_TIMER_BASE_UPDATE == FALSE {
        let timer = sys_tick_timer();
        call_get_cycle(timer, &mut period as *mut UINT32);
        G_TICK_TIMER_BASE = G_TICK_TIMER_BASE.wrapping_add(period as UINT64);
    }
    G_TICK_TIMER_BASE_UPDATE = FALSE;
}

#[no_mangle]
pub unsafe extern "C" fn OsTickTimerBaseReset(currTime: UINT64) {
    debug_assert!(currTime >= G_TICK_TIMER_BASE);
    G_TICK_TIMER_BASE = currTime;
}

#[no_mangle]
pub unsafe extern "C" fn OsTickHandler() {
    if LOSCFG_BASE_CORE_TICK_WTIMER == 0 {
        os_update_sys_time_base();
    }
    LOS_SchedTickHandler();
}

#[no_mangle]
pub unsafe extern "C" fn OsTickTimerReload(period: UINT64) -> UINT64 {
    if LOSCFG_BASE_CORE_TICK_WTIMER == 0 {
        G_TICK_TIMER_BASE = LOS_SysCycleGet();
    }
    call_reload(sys_tick_timer(), period)
}

#[no_mangle]
pub unsafe extern "C" fn LOS_SysCycleGet() -> UINT64 {
    let timer = sys_tick_timer();

    if LOSCFG_BASE_CORE_TICK_WTIMER == 1 {
        return call_get_cycle(timer, ptr::null_mut());
    }

    let mut period: UINT32 = 0;
    let int_save = ArchIntLock();
    let time = call_get_cycle(timer, &mut period as *mut UINT32);
    let mut sched_time = G_TICK_TIMER_BASE.wrapping_add(time);

    if sched_time < G_OLD_TICK_TIMER_BASE {
        G_TICK_TIMER_BASE = G_TICK_TIMER_BASE.wrapping_add(period as UINT64);
        sched_time = G_TICK_TIMER_BASE.wrapping_add(time);
        G_TICK_TIMER_BASE_UPDATE = TRUE;
    }

    debug_assert!(sched_time >= G_OLD_TICK_TIMER_BASE);
    G_OLD_TICK_TIMER_BASE = sched_time;
    ArchIntRestore(int_save);
    sched_time
}

unsafe fn tick_timer_check(timer: *const ArchTickTimer) -> UINT32 {
    if timer.is_null() {
        return LOS_ERRNO_SYS_PTR_NULL;
    }

    if (*timer).freq == 0
        || LOSCFG_BASE_CORE_TICK_PER_SECOND == 0
        || LOSCFG_BASE_CORE_TICK_PER_SECOND > (*timer).freq
    {
        return LOS_ERRNO_SYS_CLOCK_INVALID;
    }

    if (*timer).irqNum > LOSCFG_PLATFORM_HWI_LIMIT as INT32 {
        return LOS_ERRNO_TICK_CFG_INVALID;
    }

    if (*timer).periodMax == 0 {
        return LOS_ERRNO_TICK_CFG_INVALID;
    }

    if (*timer).init.is_none()
        || (*timer).reload.is_none()
        || (*timer).lock.is_none()
        || (*timer).unlock.is_none()
        || (*timer).getCycle.is_none()
    {
        return LOS_ERRNO_SYS_HOOK_IS_NULL;
    }

    if G_SYS_TIMER_IS_INIT != FALSE {
        return LOS_ERRNO_SYS_TIMER_IS_RUNNING;
    }

    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn OsTickTimerInit() -> UINT32 {
    G_SYS_TICK_TIMER = ArchSysTickTimerGet();
    let timer = G_SYS_TICK_TIMER;

    if timer.is_null()
        || (*timer).init.is_none()
        || (*timer).reload.is_none()
        || (*timer).lock.is_none()
        || (*timer).unlock.is_none()
        || (*timer).getCycle.is_none()
    {
        return LOS_ERRNO_SYS_HOOK_IS_NULL;
    }

    let mut tick_handler: HWI_PROC_FUNC = Some(OsTickHandler);
    if (*timer).tickHandler.is_some() {
        tick_handler = (*timer).tickHandler;
    }

    let int_save = ArchIntLock();
    let ret = call_init(timer, tick_handler);
    if ret != LOS_OK {
        ArchIntRestore(int_save);
        return ret;
    }

    if (*timer).freq == 0 || (*timer).freq < LOSCFG_BASE_CORE_TICK_PER_SECOND {
        ArchIntRestore(int_save);
        return LOS_ERRNO_SYS_CLOCK_INVALID;
    }

    if (*timer).irqNum > LOSCFG_PLATFORM_HWI_LIMIT as INT32 {
        ArchIntRestore(int_save);
        return LOS_ERRNO_TICK_CFG_INVALID;
    }

    g_sysClock = (*timer).freq;
    g_cyclesPerTick = (*timer).freq / LOSCFG_BASE_CORE_TICK_PER_SECOND;
    G_SYS_TIMER_IS_INIT = TRUE;

    ArchIntRestore(int_save);
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TickTimerRegister(
    timer: *const ArchTickTimer,
    tickHandler: HWI_PROC_FUNC,
) -> UINT32 {
    if timer.is_null() && tickHandler.is_none() {
        return LOS_ERRNO_SYS_PTR_NULL;
    }

    if !timer.is_null() {
        let ret = tick_timer_check(timer);
        if ret != LOS_OK {
            return ret;
        }

        let int_save = ArchIntLock();
        let dst = sys_tick_timer();
        if dst == timer as *mut ArchTickTimer {
            ArchIntRestore(int_save);
            return LOS_ERRNO_SYS_TIMER_ADDR_FAULT;
        }

        ptr::copy_nonoverlapping(timer, dst, 1);
        ArchIntRestore(int_save);
        return LOS_OK;
    }

    if G_SYS_TIMER_IS_INIT != FALSE {
        return LOS_ERRNO_SYS_TIMER_IS_RUNNING;
    }

    let int_save = ArchIntLock();
    let dst = sys_tick_timer();
    (*dst).tickHandler = tickHandler;
    ArchIntRestore(int_save);
    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_SysTickClockFreqAdjust(
    handler: SYS_TICK_FREQ_ADJUST_FUNC,
    param: UINTPTR,
) -> UINT32 {
    let old_freq = g_sysClock;

    let handler_fn = match handler {
        Some(f) => f,
        None => return LOS_ERRNO_SYS_HOOK_IS_NULL,
    };

    let int_save = ArchIntLock();
    let timer = sys_tick_timer();
    call_lock(timer);

    let curr_time_cycle = if LOSCFG_BASE_CORE_TICK_WTIMER == 0 {
        LOS_SysCycleGet()
    } else {
        0
    };

    let freq = handler_fn(param);
    if freq == 0 || freq == g_sysClock {
        call_unlock(timer);
        ArchIntRestore(int_save);
        return LOS_ERRNO_SYS_CLOCK_INVALID;
    }

    call_reload(timer, LOSCFG_BASE_CORE_TICK_RESPONSE_MAX);
    call_unlock(timer);

    if LOSCFG_BASE_CORE_TICK_WTIMER == 0 {
        G_TICK_TIMER_BASE = os_time_convert_freq(curr_time_cycle, old_freq, freq);
        G_OLD_TICK_TIMER_BASE = os_time_convert_freq(G_OLD_TICK_TIMER_BASE, old_freq, freq);
        G_TICK_TIMER_START_TIME = os_time_convert_freq(G_TICK_TIMER_START_TIME, old_freq, freq);
    }

    (*timer).freq = freq;
    g_sysClock = (*timer).freq;
    g_cyclesPerTick = (*timer).freq / LOSCFG_BASE_CORE_TICK_PER_SECOND;
    OsSchedTimeConvertFreq(old_freq);
    ArchIntRestore(int_save);

    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn OsTickSysTimerStartTimeSet(currTime: UINT64) {
    G_TICK_TIMER_START_TIME = currTime;
}

#[no_mangle]
pub unsafe extern "C" fn LOS_TickCountGet() -> UINT64 {
    os_sys_cycle_to_tick(LOS_SysCycleGet().wrapping_sub(G_TICK_TIMER_START_TIME))
}

#[no_mangle]
pub unsafe extern "C" fn LOS_CyclePerTickGet() -> UINT32 {
    g_cyclesPerTick
}

#[no_mangle]
pub unsafe extern "C" fn LOS_MS2Tick(millisec: UINT32) -> UINT32 {
    if millisec == OS_NULL_INT {
        return OS_NULL_INT;
    }

    ((millisec as UINT64 * LOSCFG_BASE_CORE_TICK_PER_SECOND as UINT64)
        / OS_SYS_MS_PER_SECOND as UINT64) as UINT32
}

#[no_mangle]
pub unsafe extern "C" fn LOS_Tick2MS(ticks: UINT32) -> UINT32 {
    ((ticks as UINT64 * OS_SYS_MS_PER_SECOND as UINT64)
        / LOSCFG_BASE_CORE_TICK_PER_SECOND as UINT64) as UINT32
}

#[no_mangle]
pub unsafe extern "C" fn OsCpuTick2MS(
    cpuTick: *mut CpuTick,
    msHi: *mut UINT32,
    msLo: *mut UINT32,
) -> UINT32 {
    if cpuTick.is_null() || msHi.is_null() || msLo.is_null() {
        return LOS_ERRNO_SYS_PTR_NULL;
    }

    if g_sysClock == 0 {
        return LOS_ERRNO_SYS_CLOCK_INVALID;
    }

    let mut tmp_cpu_tick = (((*cpuTick).cntHi as UINT64) << OS_SYS_MV_32_BIT) | (*cpuTick).cntLo as UINT64;
    let temp = (tmp_cpu_tick as DOUBLE) / ((g_sysClock as DOUBLE) / (OS_SYS_MS_PER_SECOND as DOUBLE));
    tmp_cpu_tick = temp as UINT64;

    *msLo = tmp_cpu_tick as UINT32;
    *msHi = (tmp_cpu_tick >> OS_SYS_MV_32_BIT) as UINT32;

    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn OsCpuTick2US(
    cpuTick: *mut CpuTick,
    usHi: *mut UINT32,
    usLo: *mut UINT32,
) -> UINT32 {
    if cpuTick.is_null() || usHi.is_null() || usLo.is_null() {
        return LOS_ERRNO_SYS_PTR_NULL;
    }

    if g_sysClock == 0 {
        return LOS_ERRNO_SYS_CLOCK_INVALID;
    }

    let mut tmp_cpu_tick = (((*cpuTick).cntHi as UINT64) << OS_SYS_MV_32_BIT) | (*cpuTick).cntLo as UINT64;
    let temp = (tmp_cpu_tick as DOUBLE) / ((g_sysClock as DOUBLE) / (OS_SYS_US_PER_SECOND as DOUBLE));
    tmp_cpu_tick = temp as UINT64;

    *usLo = tmp_cpu_tick as UINT32;
    *usHi = (tmp_cpu_tick >> OS_SYS_MV_32_BIT) as UINT32;

    LOS_OK
}

#[no_mangle]
pub unsafe extern "C" fn LOS_CurrNanosec() -> UINT64 {
    LOS_SysCycleGet() * (OS_SYS_NS_PER_SECOND / OS_SYS_NS_PER_MS) / (g_sysClock as UINT64 / OS_SYS_NS_PER_MS)
}

#[no_mangle]
pub unsafe extern "C" fn LOS_UDelay(microseconds: UINT64) {
    if microseconds == 0 {
        return;
    }

    let delta = (microseconds / OS_SYS_US_PER_SECOND as UINT64) * g_sysClock as UINT64
        + (microseconds % OS_SYS_US_PER_SECOND as UINT64) * g_sysClock as UINT64
            / OS_SYS_US_PER_SECOND as UINT64;
    let end_time = LOS_SysCycleGet().wrapping_add(delta);

    while LOS_SysCycleGet() < end_time {}
}

#[no_mangle]
pub unsafe extern "C" fn LOS_MDelay(mut millisec: UINT32) {
    let max_ms_once = UINT32_MAX_VALUE / OS_SYS_US_PER_MS;
    let delay_us = max_ms_once * OS_SYS_US_PER_MS;

    while millisec > max_ms_once {
        LOS_UDelay(delay_us as UINT64);
        millisec -= max_ms_once;
    }

    LOS_UDelay((millisec * OS_SYS_US_PER_MS) as UINT64);
}
