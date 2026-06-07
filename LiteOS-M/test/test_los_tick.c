#include <stdio.h>
#include <string.h>
#include <unistd.h>   /* usleep */

#include "los_tick.h"
#include "los_task.h"

/* 伪造合法 runTask，避免内部断言失败 */
static LosTaskCB g_hostTask;

#define CHECK(desc, expr) \
    do { \
        if (expr) { \
            printf("  PASS  %s\n", desc); \
        } else { \
            printf("  FAIL  %s  (line %d)\n", desc, __LINE__); \
        } \
    } while (0)

/* ------------------------------------------------------------------ */

static void TestSysCycleGet(void)
{
    printf("\n--- LOS_SysCycleGet ---\n");

    UINT64 c1 = LOS_SysCycleGet();
    UINT64 c2 = LOS_SysCycleGet();

    CHECK("returns non-zero", c1 > 0);
    CHECK("second call >= first call", c2 >= c1);

    /* 等待一小段时间，cycle 应该增长 */
    usleep(1000); /* 1 ms */
    UINT64 c3 = LOS_SysCycleGet();
    CHECK("cycle increases over time", c3 > c1);
    printf("       c1=%llu  c3=%llu  delta=%llu\n",
           (unsigned long long)c1,
           (unsigned long long)c3,
           (unsigned long long)(c3 - c1));
}

static void TestCyclePerTickGet(void)
{
    printf("\n--- LOS_CyclePerTickGet ---\n");

    UINT32 cpt = LOS_CyclePerTickGet();

    CHECK("returns non-zero", cpt > 0);

    /* g_sysClock=1000000, TICK_PER_SECOND=100 → 期望 10000 */
    UINT32 expected = g_sysClock / LOSCFG_BASE_CORE_TICK_PER_SECOND;
    CHECK("value == g_sysClock / TICK_PER_SECOND", cpt == expected);
    printf("       cyclesPerTick = %u  (expected %u)\n", cpt, expected);
}

static void TestTickCountGet(void)
{
    printf("\n--- LOS_TickCountGet ---\n");

    UINT64 t1 = LOS_TickCountGet();

    /* tick count 在 kernel init 后从 0 开始计，host 环境无真实 tick 中断，
     * 但 LOS_SysCycleGet() 单调递增，所以 TickCountGet 也应单调递增 */
    usleep(20000); /* 20 ms — 超过 2 个 tick（每 tick 10 ms） */
    UINT64 t2 = LOS_TickCountGet();

    CHECK("tick count is non-negative", t1 >= 0);
    CHECK("tick count increases over ~20ms", t2 > t1);
    printf("       t1=%llu  t2=%llu  delta=%llu\n",
           (unsigned long long)t1,
           (unsigned long long)t2,
           (unsigned long long)(t2 - t1));
}

static void TestMS2Tick(void)
{
    printf("\n--- LOS_MS2Tick ---\n");

    /* TICK_PER_SECOND=100 → 1 tick = 10 ms */
    UINT32 t = LOS_MS2Tick(10);
    CHECK("10ms -> 1 tick", t == 1);

    t = LOS_MS2Tick(100);
    CHECK("100ms -> 10 ticks", t == 10);

    t = LOS_MS2Tick(1000);
    CHECK("1000ms -> 100 ticks", t == 100);

    t = LOS_MS2Tick(0);
    CHECK("0ms -> 0 ticks", t == 0);

    /* 特殊值 0xFFFFFFFF 应原样返回 0xFFFFFFFF */
    t = LOS_MS2Tick(0xFFFFFFFF);
    CHECK("0xFFFFFFFF ms -> 0xFFFFFFFF", t == 0xFFFFFFFF);
}

static void TestTick2MS(void)
{
    printf("\n--- LOS_Tick2MS ---\n");

    /* TICK_PER_SECOND=100 → 1 tick = 10 ms */
    UINT32 ms = LOS_Tick2MS(1);
    CHECK("1 tick -> 10ms", ms == 10);

    ms = LOS_Tick2MS(10);
    CHECK("10 ticks -> 100ms", ms == 100);

    ms = LOS_Tick2MS(100);
    CHECK("100 ticks -> 1000ms", ms == 1000);

    ms = LOS_Tick2MS(0);
    CHECK("0 ticks -> 0ms", ms == 0);
}

static void TestCurrNanosec(void)
{
    printf("\n--- LOS_CurrNanosec ---\n");

    UINT64 ns1 = LOS_CurrNanosec();
    usleep(1000); /* 1 ms */
    UINT64 ns2 = LOS_CurrNanosec();

    CHECK("returns non-zero", ns1 > 0);
    CHECK("increases over ~1ms", ns2 > ns1);
    printf("       ns1=%llu  ns2=%llu  delta=%llu ns\n",
           (unsigned long long)ns1,
           (unsigned long long)ns2,
           (unsigned long long)(ns2 - ns1));
}

static void TestOsCpuTick2MS(void)
{
    printf("\n--- OsCpuTick2MS ---\n");

    /* NULL 指针 */
    UINT32 hi, lo;
    UINT32 ret = OsCpuTick2MS(NULL, &hi, &lo);
    CHECK("NULL cpuTick -> LOS_ERRNO_SYS_PTR_NULL",
          ret == LOS_ERRNO_SYS_PTR_NULL);

    ret = OsCpuTick2MS((CpuTick *)&(CpuTick){0, 0}, NULL, &lo);
    CHECK("NULL msHi -> LOS_ERRNO_SYS_PTR_NULL",
          ret == LOS_ERRNO_SYS_PTR_NULL);

    ret = OsCpuTick2MS((CpuTick *)&(CpuTick){0, 0}, &hi, NULL);
    CHECK("NULL msLo -> LOS_ERRNO_SYS_PTR_NULL",
          ret == LOS_ERRNO_SYS_PTR_NULL);

    /* g_sysClock=1000000: 1000000 cycles = 1000 ms */
    CpuTick tick = { .cntHi = 0, .cntLo = 1000000 };
    ret = OsCpuTick2MS(&tick, &hi, &lo);
    CHECK("1000000 cycles -> LOS_OK", ret == LOS_OK);
    CHECK("1000000 cycles -> 1000ms (hi=0)", hi == 0);
    CHECK("1000000 cycles -> 1000ms (lo=1000)", lo == 1000);
    printf("       1000000 cycles = hi:%u lo:%u ms\n", hi, lo);
}

static void TestOsCpuTick2US(void)
{
    printf("\n--- OsCpuTick2US ---\n");

    UINT32 hi, lo;

    /* NULL 指针 */
    UINT32 ret = OsCpuTick2US(NULL, &hi, &lo);
    CHECK("NULL cpuTick -> LOS_ERRNO_SYS_PTR_NULL",
          ret == LOS_ERRNO_SYS_PTR_NULL);

    /* g_sysClock=1000000: 1000000 cycles = 1000000 us */
    CpuTick tick = { .cntHi = 0, .cntLo = 1000000 };
    ret = OsCpuTick2US(&tick, &hi, &lo);
    CHECK("1000000 cycles -> LOS_OK", ret == LOS_OK);
    CHECK("1000000 cycles -> 1000000us (hi=0)", hi == 0);
    CHECK("1000000 cycles -> 1000000us (lo=1000000)", lo == 1000000);
    printf("       1000000 cycles = hi:%u lo:%u us\n", hi, lo);
}

static void TestGlobalVars(void)
{
    printf("\n--- Global tick variables ---\n");

    CHECK("g_sysClock == 1000000", g_sysClock == 1000000);
    CHECK("g_cyclesPerTick == g_sysClock / TICK_PER_SECOND",
          g_cyclesPerTick == g_sysClock / LOSCFG_BASE_CORE_TICK_PER_SECOND);

    printf("       g_sysClock      = %u\n", g_sysClock);
    printf("       g_cyclesPerTick = %u\n", g_cyclesPerTick);
}

static void TestOsCycle2MSUS(void)
{
    printf("\n--- OsCycle2MS / OsCycle2US (inline) ---\n");

    /* g_sysClock=1000000: 1000000 cycles = 1000 ms = 1000000 us */
    UINT64 ms = OsCycle2MS(1000000);
    CHECK("OsCycle2MS(1000000) == 1000", ms == 1000);

    UINT64 us = OsCycle2US(1000000);
    CHECK("OsCycle2US(1000000) == 1000000", us == 1000000);

    UINT64 ms0 = OsCycle2MS(0);
    CHECK("OsCycle2MS(0) == 0", ms0 == 0);
}

/* ------------------------------------------------------------------ */

int main(void)
{
    /* 伪造合法 runTask */
    memset(&g_hostTask, 0, sizeof(g_hostTask));
    g_hostTask.taskID = 0;
    g_losTask.runTask = &g_hostTask;

    UINT32 ret = LOS_KernelInit();
    if (ret != LOS_OK) {
        printf("LOS_KernelInit failed: %u\n", ret);
        return 1;
    }

    printf("=== Tick Test Start ===\n");

    TestSysCycleGet();
    TestCyclePerTickGet();
    TestTickCountGet();
    TestMS2Tick();
    TestTick2MS();
    TestCurrNanosec();
    TestOsCpuTick2MS();
    TestOsCpuTick2US();
    TestGlobalVars();
    TestOsCycle2MSUS();

    printf("\n=== Tick Test End ===\n");
    return 0;
}