#include "test_common.h"
#include "los_task.h"

static volatile int g_count = 0;

static void TaskEntry(void)
{
    for (int i = 0; i < 5; i++) {
        g_count++;
        LOS_TaskDelay(1);
    }
}

int test_task(void)
{
    UINT32 tid;
    TSK_INIT_PARAM_S init = {0};

    init.pfnTaskEntry = (TSK_ENTRY_FUNC)TaskEntry;
    init.pcName = "task_test";
    init.uwStackSize = 0x1000;
    init.usTaskPrio = 5;

    TEST_ASSERT(LOS_TaskCreate(&tid, &init) == LOS_OK,
                "task create failed");

    LOS_TaskDelay(20);

    TEST_ASSERT(g_count == 5,
                "task execution count mismatch");

    TEST_PASS();
}
