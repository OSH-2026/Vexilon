#include "test_common.h"
#include "los_tick.h"
#include "los_task.h"

int test_tick(void)
{
    UINT64 start = LOS_TickCountGet();

    LOS_TaskDelay(10);

    UINT64 end = LOS_TickCountGet();

    TEST_ASSERT((end - start) >= 10,
                "tick not increasing");

    TEST_PASS();
}
