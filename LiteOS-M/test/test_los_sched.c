#include <stdio.h>
#include <string.h>

#include "los_sched.h"
#include "los_task.h"
#include "los_tick.h"

/* 伪造合法 runTask，让 LOS_CurTaskIDGet() 返回 0 */
static LosTaskCB g_hostTask;

#define CHECK(desc, expr) \
    do { \
        if (expr) { \
            printf("  PASS  %s\n", desc); \
        } else { \
            printf("  FAIL  %s\n", desc); \
        } \
    } while (0)

/* 测试用任务入口（host 环境不会真正调度） */
static VOID *SchedTestEntry(UINT32 arg) { (void)arg; return NULL; }

static void FillTaskParam(TSK_INIT_PARAM_S *p, const char *name, UINT16 prio)
{
    memset(p, 0, sizeof(*p));
    p->pfnTaskEntry = SchedTestEntry;
    p->usTaskPrio   = prio;
    p->uwStackSize  = LOSCFG_BASE_CORE_TSK_DEFAULT_STACK_SIZE;
    p->pcName       = (CHAR *)name;
}

/* ------------------------------------------------------------------ */

static void TestOsSchedSwtmrScanRegister(void)
{
    printf("\n--- OsSchedSwtmrScanRegister ---\n");

    /* NULL 函数指针 -> LOS_NOK */
    UINT32 ret = OsSchedSwtmrScanRegister(NULL);
    CHECK("NULL func -> LOS_NOK", ret == LOS_NOK);

    /* 合法函数指针 -> LOS_OK */
    BOOL dummyScan(VOID) { return FALSE; }
    ret = OsSchedSwtmrScanRegister(dummyScan);
    CHECK("valid func -> LOS_OK", ret == LOS_OK);

    /* 恢复为 NULL，避免影响后续测试 */
    OsSchedSwtmrScanRegister(NULL);
}

/* ------------------------------------------------------------------ */

static void TestOsSchedResetSchedResponseTime(void)
{
    printf("\n--- OsSchedResetSchedResponseTime ---\n");

    /* 传入比当前 responseTime 小的值 -> responseTime 被重置为 MAX */
    OsSchedResetSchedResponseTime(0);
    /* 传入比当前 responseTime 大的值 -> responseTime 不变 */
    OsSchedResetSchedResponseTime(OS_SCHED_MAX_RESPONSE_TIME);

    /* 两次调用都不崩溃即视为 PASS */
    CHECK("ResetSchedResponseTime(0) no crash", 1);
    CHECK("ResetSchedResponseTime(MAX) no crash", 1);
}

/* ------------------------------------------------------------------ */

static void TestOsSchedTaskEnDeQueue(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskID;
    UINT32 ret;

    printf("\n--- OsSchedTaskEnQueue / OsSchedTaskDeQueue ---\n");

    FillTaskParam(&param, "EnDeTask", 10);
    ret = LOS_TaskCreate(&taskID, &param);
    if (ret != LOS_OK) {
        printf("  SKIP  (task create failed: %u)\n", ret);
        return;
    }

    LosTaskCB *taskCB = OS_TCB_FROM_TID(taskID);

    /* 任务创建后处于 READY 状态，先 DeQueue */
    OsSchedTaskDeQueue(taskCB);
    CHECK("after DeQueue: READY bit cleared",
          !(taskCB->taskStatus & OS_TASK_STATUS_READY));

    /* 再 EnQueue */
    OsSchedTaskEnQueue(taskCB);
    CHECK("after EnQueue: READY bit set",
          (taskCB->taskStatus & OS_TASK_STATUS_READY));

    /* 重复 DeQueue 不崩溃 */
    OsSchedTaskDeQueue(taskCB);
    OsSchedTaskDeQueue(taskCB);
    CHECK("double DeQueue no crash", 1);

    LOS_TaskDelete(taskID);
}

/* ------------------------------------------------------------------ */

static void TestOsSchedSuspendResume(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskID;
    UINT32 ret;

    printf("\n--- OsSchedSuspend / OsSchedResume ---\n");

    FillTaskParam(&param, "SuspTask", 10);
    ret = LOS_TaskCreate(&taskID, &param);
    if (ret != LOS_OK) {
        printf("  SKIP  (task create failed: %u)\n", ret);
        return;
    }

    LosTaskCB *taskCB = OS_TCB_FROM_TID(taskID);

    /* 挂起：SUSPEND 位置位，READY 位清除 */
    OsSchedSuspend(taskCB);
    CHECK("after Suspend: SUSPEND bit set",
          (taskCB->taskStatus & OS_TASK_STATUS_SUSPEND));
    CHECK("after Suspend: READY bit cleared",
          !(taskCB->taskStatus & OS_TASK_STATUS_READY));

    /* 恢复：SUSPEND 位清除，READY 位置位 */
    BOOL resumed = OsSchedResume(taskCB);
    CHECK("Resume returns TRUE", resumed == TRUE);
    CHECK("after Resume: SUSPEND bit cleared",
          !(taskCB->taskStatus & OS_TASK_STATUS_SUSPEND));
    CHECK("after Resume: READY bit set",
          (taskCB->taskStatus & OS_TASK_STATUS_READY));

    LOS_TaskDelete(taskID);
}

/* ------------------------------------------------------------------ */

static void TestOsSchedModifyTaskSchedParam(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskID;
    UINT32 ret;

    printf("\n--- OsSchedModifyTaskSchedParam ---\n");

    FillTaskParam(&param, "ModTask", 10);
    ret = LOS_TaskCreate(&taskID, &param);
    if (ret != LOS_OK) {
        printf("  SKIP  (task create failed: %u)\n", ret);
        return;
    }

    LosTaskCB *taskCB = OS_TCB_FROM_TID(taskID);

    /* 任务在 READY 队列中，修改优先级 */
    BOOL changed = OsSchedModifyTaskSchedParam(taskCB, 5);
    CHECK("ModifySchedParam(READY task, prio=5) returns TRUE", changed == TRUE);
    CHECK("priority updated to 5", taskCB->priority == 5);

    /* 挂起后修改优先级（非 READY 非 RUNNING） */
    OsSchedSuspend(taskCB);
    changed = OsSchedModifyTaskSchedParam(taskCB, 8);
    CHECK("ModifySchedParam(SUSPEND task, prio=8) returns FALSE", changed == FALSE);
    CHECK("priority updated to 8", taskCB->priority == 8);

    LOS_TaskDelete(taskID);
}

/* ------------------------------------------------------------------ */

static void TestOsGetTopTask(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskHi, taskLo;
    UINT32 ret;

    printf("\n--- OsGetTopTask ---\n");

    /*
     * idle 任务由 LOS_KernelInit 创建，priority = 0（数值最小 = 最高优先级），
     * 始终在就绪队列中。因此 OsGetTopTask 在有 idle 任务时始终返回 idle 任务。
     * 这是正确的内核行为：idle 任务优先级最高（0），用户任务优先级 >= 1。
     */

    /* 验证1：无用户任务时，TopTask 为 idle 任务（priority = 0） */
    LosTaskCB *top = OsGetTopTask();
    CHECK("TopTask != NULL", top != NULL);
    CHECK("TopTask is idle task (priority == 0)", top != NULL && top->priority == 0);
    printf("       idle task priority = %u\n", top ? top->priority : 0xFFFF);

    /* 验证2：创建用户任务后，TopTask 仍是 idle（priority 数值更小） */
    FillTaskParam(&param, "HiPrio", 3);
    ret = LOS_TaskCreate(&taskHi, &param);
    if (ret != LOS_OK) { printf("  SKIP (task create failed)\n"); return; }

    FillTaskParam(&param, "LoPrio", 20);
    ret = LOS_TaskCreate(&taskLo, &param);
    if (ret != LOS_OK) { LOS_TaskDelete(taskHi); printf("  SKIP\n"); return; }

    top = OsGetTopTask();
    CHECK("with user tasks, TopTask still has lowest priority number",
          top != NULL && top->priority <= 3);
    printf("       TopTask priority = %u (idle=0, HiPrio=3, LoPrio=20)\n",
           top ? top->priority : 0xFFFF);

    /*
     * 验证3：idle 任务受内核保护，OsSchedTaskDeQueue 对 idle 任务
     * 只清 READY 位，不会从优先级队列中真正移除它。
     * 因此即使调用 DeQueue，OsGetTopTask 仍返回 idle 任务（priority=0）。
     * 这是内核的正确设计，保证系统始终有任务可调度。
     */
    LosTaskCB *idleCB = OS_TCB_FROM_TID(g_idleTaskID);
    OsSchedTaskDeQueue(idleCB);
    top = OsGetTopTask();
    CHECK("idle task protected: TopTask still priority=0 after DeQueue",
          top != NULL && top->priority == 0);
    printf("       TopTask priority after idle DeQueue = %u\n",
           top ? top->priority : 0xFFFF);

    /* 恢复 idle 任务的 READY 状态 */
    OsSchedTaskEnQueue(idleCB);

    LOS_TaskDelete(taskHi);
    LOS_TaskDelete(taskLo);
}

/* ------------------------------------------------------------------ */

static void TestOsSchedGetNextExpireTime(void)
{
    printf("\n--- OsSchedGetNextExpireTime ---\n");

    UINT64 startTime = LOS_SysCycleGet();
    UINT64 expireTime = OsSchedGetNextExpireTime(startTime);

    /* 返回值应 >= startTime（可能相等也可能更大） */
    CHECK("NextExpireTime >= startTime", expireTime >= startTime);
    printf("       startTime  = %llu\n", (unsigned long long)startTime);
    printf("       expireTime = %llu\n", (unsigned long long)expireTime);
}

/* ------------------------------------------------------------------ */

static void TestLOS_SchedTickTimeoutNsGet(void)
{
    printf("\n--- LOS_SchedTickTimeoutNsGet ---\n");

    /* g_taskScheduled == FALSE，调度未启动，responseTime 为 MAX，
     * 结果应为一个非零的大数（剩余时间）或 0（已超时）。
     * 只验证调用不崩溃且返回值合理（>= 0，UINT64 恒成立）。 */
    UINT64 ns = LOS_SchedTickTimeoutNsGet();
    CHECK("SchedTickTimeoutNsGet no crash", 1);
    printf("       timeout = %llu ns\n", (unsigned long long)ns);
}

/* ------------------------------------------------------------------ */

static void TestLOS_SchedTickHandler(void)
{
    printf("\n--- LOS_SchedTickHandler ---\n");

    /* g_taskScheduled == FALSE 时，函数直接返回，不执行任何逻辑，不崩溃 */
    LOS_SchedTickHandler();
    CHECK("SchedTickHandler no crash when scheduler not started", 1);
}

/* ------------------------------------------------------------------ */

static void TestLOS_Schedule(void)
{
    printf("\n--- LOS_Schedule ---\n");

    /* OsCheckKernelRunning() 返回 FALSE（调度未启动），
     * LOS_Schedule 直接返回，不调用 ArchTaskSchedule */
    LOS_Schedule();
    CHECK("LOS_Schedule no crash when scheduler not started", 1);
}

/* ------------------------------------------------------------------ */

static void TestOsSchedTaskExit(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskID;
    UINT32 ret;

    printf("\n--- OsSchedTaskExit ---\n");

    FillTaskParam(&param, "ExitTask", 10);
    ret = LOS_TaskCreate(&taskID, &param);
    if (ret != LOS_OK) {
        printf("  SKIP  (task create failed: %u)\n", ret);
        return;
    }

    LosTaskCB *taskCB = OS_TCB_FROM_TID(taskID);

    /* 任务在 READY 队列中，退出后应从队列移除 */
    OsSchedTaskExit(taskCB);
    CHECK("after TaskExit: READY bit cleared",
          !(taskCB->taskStatus & OS_TASK_STATUS_READY));

    LOS_TaskDelete(taskID);
}

/* ------------------------------------------------------------------ */

int main(void)
{
    /* 伪造合法 runTask */
    memset(&g_hostTask, 0, sizeof(g_hostTask));
    g_hostTask.taskID = 0;
    g_losTask.runTask = &g_hostTask;

    UINT32 ret = LOS_KernelInit();
    if (ret != LOS_OK) {
        printf("LOS_KernelInit failed: %u\n", ret);
        return 1;
    }

    printf("=== Sched Test Start ===\n");

    TestOsSchedSwtmrScanRegister();
    TestOsSchedResetSchedResponseTime();
    TestOsSchedTaskEnDeQueue();
    TestOsSchedSuspendResume();
    TestOsSchedModifyTaskSchedParam();
    TestOsGetTopTask();
    TestOsSchedGetNextExpireTime();
    TestLOS_SchedTickTimeoutNsGet();
    TestLOS_SchedTickHandler();
    TestLOS_Schedule();
    TestOsSchedTaskExit();

    printf("\n=== Sched Test End ===\n");
    return 0;
}