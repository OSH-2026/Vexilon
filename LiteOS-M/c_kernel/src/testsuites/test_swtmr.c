#include "test_common.h"
#include "los_swtmr.h"
#include "los_task.h"

static volatile int fired = 0;

static void TimerCb(UINT32 arg)
{
    (void)arg;
    fired = 1;
}

int test_swtmr(void)
{
    UINT32 id;

    TEST_ASSERT(LOS_SwtmrCreate(10,
                                LOS_SWTMR_MODE_ONCE,
                                TimerCb,
                                &id,
                                0) == LOS_OK,
                "timer create failed");

    TEST_ASSERT(LOS_SwtmrStart(id) == LOS_OK,
                "timer start failed");

    LOS_TaskDelay(20);

    TEST_ASSERT(fired == 1,
                "timer callback failed");

    TEST_PASS();
}
