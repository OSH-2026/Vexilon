#ifndef TEST_COMMON_H
#define TEST_COMMON_H

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define TEST_PASS() \
    do { \
        printf("[PASS] %s\n", __func__); \
        return 0; \
    } while (0)

#define TEST_FAIL(msg) \
    do { \
        printf("[FAIL] %s : %s\n", __func__, msg); \
        return -1; \
    } while (0)

#define TEST_ASSERT(cond, msg) \
    do { \
        if (!(cond)) { \
            TEST_FAIL(msg); \
        } \
    } while (0)

#endif
