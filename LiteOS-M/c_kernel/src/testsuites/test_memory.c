#include "test_common.h"
#include "los_memory.h"

extern UINT8 *m_aucSysMem0;

int test_memory(void)
{
    unsigned char *p;

    p = LOS_MemAlloc(m_aucSysMem0, 128);

    TEST_ASSERT(p != NULL,
                "mem alloc failed");

    memset(p, 0xAA, 128);

    for (int i = 0; i < 128; i++) {
        TEST_ASSERT(p[i] == 0xAA,
                    "memory corruption");
    }

    TEST_ASSERT(LOS_MemFree(m_aucSysMem0, p) == LOS_OK,
                "mem free failed");

    TEST_PASS();
}
