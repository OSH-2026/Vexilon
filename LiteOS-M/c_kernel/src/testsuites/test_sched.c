#include "test_common.h"
#include "los_task.h"

static volatile int high_ran = 0;
static volatile int low_ran = 0;

static void HighTask(void)
{
    high_ran = 1;
}

static void LowTask(void)
{
    low_ran = 1;
    LOS_TaskDelay(10);
}

int test_sched(void)
{
    UINT32 tid1, tid2;
    TSK_INIT_PARAM_S init = {0};

    init.pfnTaskEntry = (TSK_ENTRY_FUNC)LowTask;
    init.pcName = "low";
    init.uwStackSize = 0x1000;
    init.usTaskPrio = 10;

    LOS_TaskCreate(&tid1, &init);

    init.pfnTaskEntry = (TSK_ENTRY_FUNC)HighTask;
    init.pcName = "high";
    init.usTaskPrio = 1;

    LOS_TaskCreate(&tid2, &init);

    LOS_TaskDelay(20);

    TEST_ASSERT(high_ran == 1,
                "high priority task not executed");

    TEST_ASSERT(low_ran == 1,
                "low priority task not executed");

    TEST_PASS();
}
