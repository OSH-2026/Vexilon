#include "test_common.h"
#include "los_queue.h"

int test_queue(void)
{
    UINT32 qid;
    char send[] = "hello";
    char recv[16] = {0};
    UINT32 len = sizeof(recv);

    TEST_ASSERT(LOS_QueueCreate("q", 4, &qid,
                0, 32) == LOS_OK,
                "queue create failed");

    TEST_ASSERT(LOS_QueueWrite(qid,
                send,
                sizeof(send),
                0) == LOS_OK,
                "queue write failed");

    TEST_ASSERT(LOS_QueueRead(qid,
                recv,
                len,
                0) == LOS_OK,
                "queue read failed");

    TEST_ASSERT(strcmp(send, recv) == 0,
                "queue data mismatch");

    TEST_PASS();
}
