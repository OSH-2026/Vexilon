#include "test_common.h"
#include "los_event.h"

int test_event(void)
{
    EVENT_CB_S evt;

    TEST_ASSERT(LOS_EventInit(&evt) == LOS_OK,
                "event init failed");

    TEST_ASSERT(LOS_EventWrite(&evt, 0x01) == LOS_OK,
                "event write failed");

    UINT32 ret = LOS_EventRead(&evt,
                               0x01,
                               LOS_WAITMODE_OR,
                               0);

    TEST_ASSERT(ret == 0x01,
                "event read failed");

    TEST_PASS();
}
