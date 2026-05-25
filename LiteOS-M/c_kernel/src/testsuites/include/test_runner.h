#ifndef TEST_RUNNER_H
#define TEST_RUNNER_H

typedef int (*test_func_t)(void);

typedef struct {
    const char *name;
    test_func_t func;
} test_case_t;

#endif
