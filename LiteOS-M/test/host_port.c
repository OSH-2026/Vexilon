/* host_port.c
 *
 * Linux Host Port for LiteOS-M
 *
 * 目标：
 * 1. 让 LiteOS-M 核心代码能够在 Linux 上编译和链接
 * 2. 用于测试内存管理、队列、链表、任务管理等逻辑
 * 3. 不实现真正的上下文切换和中断机制
 */

/*
 * 必须在包含任何 LiteOS 头文件之前先包含标准库，
 * 避免 LiteOS 类型宏与标准类型冲突。
 */
#include <stdlib.h>
#include <stdint.h>
#include <time.h>

/*
 * 包含 LiteOS 头文件，以便函数签名与声明严格一致。
 * los_compiler.h 定义了 UINT32 / VOID 等类型。
 */
#include "los_compiler.h"
#include "los_interrupt.h"
#include "los_context.h"
#include "los_timer.h"
#include "los_task.h"
#include "los_error.h"

/*----------------------------------------------------------
 * 中断相关
 *----------------------------------------------------------*/

UINT32 ArchIntLock(VOID)
{
    return 0;
}

VOID ArchIntRestore(UINT32 intSave)
{
    (VOID)intSave;
}

UINT32 ArchIsIntActive(VOID)
{
    return 0;
}

/*----------------------------------------------------------
 * 调度相关
 *----------------------------------------------------------*/

VOID ArchInit(VOID)
{
}

UINT32 ArchStartSchedule(VOID)
{
    return 0;
}

VOID ArchTaskSchedule(VOID)
{
}

VOID *ArchSignalContextInit(VOID *stackPointer, VOID *stackTop,
                            UINTPTR sigHandler, UINT32 param)
{
    (VOID)stackTop;
    (VOID)sigHandler;
    (VOID)param;
    return stackPointer;
}

/*----------------------------------------------------------
 * 系统相关
 *----------------------------------------------------------*/

VOID ArchSysExit(VOID)
{
    exit(0);
}

UINT32 ArchEnterSleep(VOID)
{
    return 0;
}

/*----------------------------------------------------------
 * Tick Timer
 *
 * OsTickTimerInit 要求 init/reload/lock/unlock/getCycle
 * 全部非 NULL，且 freq >= LOSCFG_BASE_CORE_TICK_PER_SECOND，
 * irqNum <= LOSCFG_PLATFORM_HWI_LIMIT。
 * host 环境用单调时钟模拟 1 MHz 计数器。
 *----------------------------------------------------------*/

static UINT32 HostTickInit(HWI_PROC_FUNC tickHandler)
{
    (VOID)tickHandler;
    return LOS_OK;
}

static UINT64 HostTickGetCycle(UINT32 *period)
{
    if (period != NULL) {
        *period = 0xFFFFFFFF;
    }
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ((UINT64)ts.tv_sec * 1000000ULL)
           + ((UINT64)ts.tv_nsec / 1000ULL);
}

static UINT64 HostTickReload(UINT64 time)
{
    (VOID)time;
    return 0;
}

static VOID HostTickLock(VOID)   {}
static VOID HostTickUnlock(VOID) {}

static ArchTickTimer g_hostTickTimer = {
    .freq      = 1000000,               /* 1 MHz，满足 >= LOSCFG_BASE_CORE_TICK_PER_SECOND(100) */
    .irqNum    = 0,                     /* 0 <= LOSCFG_PLATFORM_HWI_LIMIT(32) */
    .periodMax = 0xFFFFFFFFFFFFFFFFULL,
    .init      = HostTickInit,
    .getCycle  = HostTickGetCycle,
    .reload    = HostTickReload,
    .lock      = HostTickLock,
    .unlock    = HostTickUnlock,
};

ArchTickTimer *ArchSysTickTimerGet(VOID)
{
    return &g_hostTickTimer;
}

/*----------------------------------------------------------
 * 任务栈初始化
 *----------------------------------------------------------*/

VOID *ArchTskStackInit(UINT32 taskID, UINT32 stackSize, VOID *topStack)
{
    (VOID)taskID;
    (VOID)stackSize;
    return topStack;
}

/*----------------------------------------------------------
 * 错误处理
 *----------------------------------------------------------*/

UINT32 LOS_ErrHandle(CHAR *fileName, UINT32 lineNo,
                     UINT32 errorNo, UINT32 paraLen, VOID *para)
{
    (VOID)fileName;
    (VOID)lineNo;
    (VOID)errorNo;
    (VOID)paraLen;
    (VOID)para;
    return 0;
}

VOID OsDoExcHook(UINT32 excType, UINT32 faultAddr,
                 UINT32 pid, UINT32 tid, const CHAR *name)
{
    (VOID)excType;
    (VOID)faultAddr;
    (VOID)pid;
    (VOID)tid;
    (VOID)name;
}

/*----------------------------------------------------------
 * Log
 *----------------------------------------------------------*/

INT32 OsLogLevelCheck(UINT32 level)
{
    (VOID)level;
    return 0;
}