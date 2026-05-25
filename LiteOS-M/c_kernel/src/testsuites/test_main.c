#include "test_runner.h"
#include <stdio.h>

extern int test_task(void);
extern int test_sched(void);
extern int test_sem(void);
extern int test_mux(void);
extern int test_queue(void);
extern int test_memory(void);
extern int test_event(void);
extern int test_tick(void);
extern int test_swtmr(void);
extern int test_exc(void);

static test_case_t g_tests[] = {
    {"task", test_task},
    {"sched", test_sched},
    {"sem", test_sem},
    {"mux", test_mux},
    {"queue", test_queue},
    {"memory", test_memory},
    {"event", test_event},
    {"tick", test_tick},
    {"swtmr", test_swtmr},
    {"exc", test_exc},
};

int main(void)
{
    int total = sizeof(g_tests) / sizeof(g_tests[0]);
    int pass = 0;

    printf("===== LiteOS-M Kernel Selftest =====\n");

    for (int i = 0; i < total; i++) {
        printf("\n[RUN ] %s\n", g_tests[i].name);

        int ret = g_tests[i].func();

        if (ret == 0) {
            pass++;
        }
    }

    printf("\n===== RESULT =====\n");
    printf("PASS: %d/%d\n", pass, total);

    return (pass == total) ? 0 : -1;
}
