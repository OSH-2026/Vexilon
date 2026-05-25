#include "test_common.h"
#include "los_mux.h"

int test_mux(void)
{
    UINT32 mux;

    TEST_ASSERT(LOS_MuxCreate(&mux) == LOS_OK,
                "mux create failed");

    TEST_ASSERT(LOS_MuxPend(mux, LOS_WAIT_FOREVER) == LOS_OK,
                "mux lock failed");

    TEST_ASSERT(LOS_MuxPost(mux) == LOS_OK,
                "mux unlock failed");

    TEST_ASSERT(LOS_MuxDelete(mux) == LOS_OK,
                "mux delete failed");

    TEST_PASS();
}
