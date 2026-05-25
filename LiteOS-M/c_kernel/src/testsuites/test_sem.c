#include "test_common.h"
#include "los_sem.h"
#include "los_task.h"

static UINT32 g_sem;
static volatile int wake = 0;

static void SemTask(void)
{
    LOS_SemPend(g_sem, LOS_WAIT_FOREVER);
    wake = 1;
}

int test_sem(void)
{
    UINT32 tid;
    TSK_INIT_PARAM_S init = {0};

    TEST_ASSERT(LOS_SemCreate(0, &g_sem) == LOS_OK,
                "sem create failed");

    init.pfnTaskEntry = (TSK_ENTRY_FUNC)SemTask;
    init.pcName = "sem_task";
    init.uwStackSize = 0x1000;
    init.usTaskPrio = 5;

    LOS_TaskCreate(&tid, &init);

    LOS_TaskDelay(10);

    TEST_ASSERT(LOS_SemPost(g_sem) == LOS_OK,
                "sem post failed");

    LOS_TaskDelay(10);

    TEST_ASSERT(wake == 1,
                "sem wake failed");

    TEST_PASS();
}
