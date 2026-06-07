#include <stdio.h>
#include <string.h>

#include "los_task.h"

/*
 * host 环境下没有真实调度器，伪造 runTask 让 LOS_CurTaskIDGet()
 * 返回合法的 taskID=0，避免内部断言失败。
 */
static LosTaskCB g_hostTask;

/* 测试任务入口（host 环境下不会真正被调度执行） */
static VOID *TestTaskEntry(UINT32 arg)
{
    (void)arg;
    return NULL;
}

/* 辅助：打印 PASS / FAIL */
#define CHECK(desc, expr) \
    do { \
        if (expr) { \
            printf("  PASS  %s\n", desc); \
        } else { \
            printf("  FAIL  %s\n", desc); \
        } \
    } while (0)

/* 辅助：填充一个合法的任务初始化参数 */
static void FillTaskParam(TSK_INIT_PARAM_S *param, const char *name, UINT16 prio)
{
    memset(param, 0, sizeof(*param));
    param->pfnTaskEntry = TestTaskEntry;
    param->usTaskPrio   = prio;
    param->uwStackSize  = LOSCFG_BASE_CORE_TSK_DEFAULT_STACK_SIZE;
    param->pcName       = (CHAR *)name;
    param->uwResved     = 0;
}

/* ------------------------------------------------------------------ */

static void TestTaskCreate(UINT32 *taskID)
{
    TSK_INIT_PARAM_S param;
    UINT32 ret;

    printf("\n--- LOS_TaskCreate ---\n");

    /* NULL taskID */
    FillTaskParam(&param, "t_create", 10);
    ret = LOS_TaskCreate(NULL, &param);
    CHECK("NULL taskID -> LOS_ERRNO_TSK_ID_INVALID",
          ret == LOS_ERRNO_TSK_ID_INVALID);

    /* NULL param */
    ret = LOS_TaskCreate(taskID, NULL);
    CHECK("NULL param -> LOS_ERRNO_TSK_PTR_NULL",
          ret == LOS_ERRNO_TSK_PTR_NULL);

    /* NULL entry */
    FillTaskParam(&param, "t_create", 10);
    param.pfnTaskEntry = NULL;
    ret = LOS_TaskCreate(taskID, &param);
    CHECK("NULL entry -> LOS_ERRNO_TSK_ENTRY_NULL",
          ret == LOS_ERRNO_TSK_ENTRY_NULL);

    /* NULL name */
    FillTaskParam(&param, "t_create", 10);
    param.pcName = NULL;
    ret = LOS_TaskCreate(taskID, &param);
    CHECK("NULL name -> LOS_ERRNO_TSK_NAME_EMPTY",
          ret == LOS_ERRNO_TSK_NAME_EMPTY);

    /* 优先级越界 */
    FillTaskParam(&param, "t_create", 10);
    param.usTaskPrio = OS_TASK_PRIORITY_LOWEST + 1;
    ret = LOS_TaskCreate(taskID, &param);
    CHECK("priority out of range -> LOS_ERRNO_TSK_PRIOR_ERROR",
          ret == LOS_ERRNO_TSK_PRIOR_ERROR);

    /* 栈太小 */
    FillTaskParam(&param, "t_create", 10);
    param.uwStackSize = 1;
    ret = LOS_TaskCreate(taskID, &param);
    CHECK("stack too small -> LOS_ERRNO_TSK_STKSZ_TOO_SMALL",
          ret == LOS_ERRNO_TSK_STKSZ_TOO_SMALL);

    /* 正常创建 */
    FillTaskParam(&param, "TestTask", 10);
    ret = LOS_TaskCreate(taskID, &param);
    CHECK("normal create -> LOS_OK", ret == LOS_OK);
    if (ret == LOS_OK) {
        printf("       taskID = %u\n", *taskID);
    }
}

static void TestTaskCreateOnly(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskID;
    UINT32 ret;

    printf("\n--- LOS_TaskCreateOnly ---\n");

    /* 正常创建（挂起状态，不进就绪队列） */
    FillTaskParam(&param, "TaskOnly", 10);
    ret = LOS_TaskCreateOnly(&taskID, &param);
    CHECK("create only -> LOS_OK", ret == LOS_OK);
    if (ret == LOS_OK) {
        printf("       taskID = %u\n", taskID);

        /* 任务处于挂起状态 */
        UINT32 status = 0;
        LOS_TaskStatusGet(taskID, &status);
        CHECK("task status has SUSPEND bit",
              (status & OS_TASK_STATUS_SUSPEND) != 0);

        /* 恢复后删除 */
        LOS_TaskResume(taskID);
        LOS_TaskDelete(taskID);
    }
}

static void TestTaskDelete(UINT32 taskID)
{
    UINT32 ret;

    printf("\n--- LOS_TaskDelete ---\n");

    /* 无效 taskID */
    ret = LOS_TaskDelete(0xFFFFFFFF);
    CHECK("invalid ID -> LOS_ERRNO_TSK_ID_INVALID",
          ret == LOS_ERRNO_TSK_ID_INVALID);

    /* 正常删除 */
    ret = LOS_TaskDelete(taskID);
    CHECK("normal delete -> LOS_OK", ret == LOS_OK);

    /* 重复删除（已不存在） */
    ret = LOS_TaskDelete(taskID);
    CHECK("delete again -> LOS_ERRNO_TSK_NOT_CREATED",
          ret == LOS_ERRNO_TSK_NOT_CREATED);
}

static void TestTaskSuspendResume(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskID;
    UINT32 ret;
    UINT32 status;

    printf("\n--- LOS_TaskSuspend / LOS_TaskResume ---\n");

    FillTaskParam(&param, "SuspTask", 10);
    ret = LOS_TaskCreate(&taskID, &param);
    if (ret != LOS_OK) {
        printf("  SKIP  (task create failed: %u)\n", ret);
        return;
    }

    /* 挂起 */
    ret = LOS_TaskSuspend(taskID);
    CHECK("suspend -> LOS_OK", ret == LOS_OK);

    LOS_TaskStatusGet(taskID, &status);
    CHECK("status has SUSPEND bit after suspend",
          (status & OS_TASK_STATUS_SUSPEND) != 0);

    /* 重复挂起 */
    ret = LOS_TaskSuspend(taskID);
    CHECK("suspend again -> LOS_ERRNO_TSK_ALREADY_SUSPENDED",
          ret == LOS_ERRNO_TSK_ALREADY_SUSPENDED);

    /* 恢复 */
    ret = LOS_TaskResume(taskID);
    CHECK("resume -> LOS_OK", ret == LOS_OK);

    /* 对未挂起的任务恢复 */
    ret = LOS_TaskResume(taskID);
    CHECK("resume non-suspended -> LOS_ERRNO_TSK_NOT_SUSPENDED",
          ret == LOS_ERRNO_TSK_NOT_SUSPENDED);

    /* 无效 ID 挂起 */
    ret = LOS_TaskSuspend(0xFFFFFFFF);
    CHECK("suspend invalid ID -> LOS_ERRNO_TSK_ID_INVALID",
          ret == LOS_ERRNO_TSK_ID_INVALID);

    LOS_TaskDelete(taskID);
}

static void TestTaskPriority(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskID;
    UINT32 ret;
    UINT16 prio;

    printf("\n--- LOS_TaskPriGet / LOS_TaskPriSet ---\n");

    FillTaskParam(&param, "PrioTask", 10);
    ret = LOS_TaskCreate(&taskID, &param);
    if (ret != LOS_OK) {
        printf("  SKIP  (task create failed: %u)\n", ret);
        return;
    }

    /* 获取优先级 */
    prio = LOS_TaskPriGet(taskID);
    CHECK("PriGet returns initial priority 10", prio == 10);

    /* 设置合法优先级 */
    ret = LOS_TaskPriSet(taskID, 5);
    CHECK("PriSet(5) -> LOS_OK", ret == LOS_OK);
    prio = LOS_TaskPriGet(taskID);
    CHECK("PriGet returns updated priority 5", prio == 5);

    /* 设置越界优先级 */
    ret = LOS_TaskPriSet(taskID, OS_TASK_PRIORITY_LOWEST + 1);
    CHECK("PriSet(out of range) -> LOS_ERRNO_TSK_PRIOR_ERROR",
          ret == LOS_ERRNO_TSK_PRIOR_ERROR);

    /* 无效 taskID */
    prio = LOS_TaskPriGet(0xFFFFFFFF);
    CHECK("PriGet invalid ID -> OS_INVALID (0xFFFF)",
          prio == (UINT16)OS_INVALID);

    ret = LOS_TaskPriSet(0xFFFFFFFF, 10);
    CHECK("PriSet invalid ID -> LOS_ERRNO_TSK_ID_INVALID",
          ret == LOS_ERRNO_TSK_ID_INVALID);

    LOS_TaskDelete(taskID);
}

static void TestTaskStatusGet(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskID;
    UINT32 ret;
    UINT32 status;

    printf("\n--- LOS_TaskStatusGet ---\n");

    FillTaskParam(&param, "StatTask", 10);
    ret = LOS_TaskCreate(&taskID, &param);
    if (ret != LOS_OK) {
        printf("  SKIP  (task create failed: %u)\n", ret);
        return;
    }

    /* NULL 指针 */
    ret = LOS_TaskStatusGet(taskID, NULL);
    CHECK("NULL status ptr -> LOS_ERRNO_TSK_PTR_NULL",
          ret == LOS_ERRNO_TSK_PTR_NULL);

    /* 无效 ID */
    ret = LOS_TaskStatusGet(0xFFFFFFFF, &status);
    CHECK("invalid ID -> LOS_ERRNO_TSK_ID_INVALID",
          ret == LOS_ERRNO_TSK_ID_INVALID);

    /* 正常查询 */
    ret = LOS_TaskStatusGet(taskID, &status);
    CHECK("normal get -> LOS_OK", ret == LOS_OK);
    CHECK("status is not UNUSED", (status & OS_TASK_STATUS_UNUSED) == 0);

    LOS_TaskDelete(taskID);
}

static void TestTaskInfoGet(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskID;
    UINT32 ret;
    TSK_INFO_S info;

    printf("\n--- LOS_TaskInfoGet ---\n");

    FillTaskParam(&param, "InfoTask", 15);
    ret = LOS_TaskCreate(&taskID, &param);
    if (ret != LOS_OK) {
        printf("  SKIP  (task create failed: %u)\n", ret);
        return;
    }

    /* NULL info 指针 */
    ret = LOS_TaskInfoGet(taskID, NULL);
    CHECK("NULL info ptr -> LOS_ERRNO_TSK_PTR_NULL",
          ret == LOS_ERRNO_TSK_PTR_NULL);

    /* 无效 ID */
    ret = LOS_TaskInfoGet(0xFFFFFFFF, &info);
    CHECK("invalid ID -> LOS_ERRNO_TSK_ID_INVALID",
          ret == LOS_ERRNO_TSK_ID_INVALID);

    /* 正常获取 */
    ret = LOS_TaskInfoGet(taskID, &info);
    CHECK("normal get -> LOS_OK", ret == LOS_OK);
    if (ret == LOS_OK) {
        CHECK("info.uwTaskID matches", info.uwTaskID == taskID);
        CHECK("info.usTaskPrio == 15", info.usTaskPrio == 15);
        CHECK("info.uwStackSize > 0", info.uwStackSize > 0);
        printf("       name=%s  prio=%u  stackSize=%u\n",
               info.acName, info.usTaskPrio, info.uwStackSize);
    }

    LOS_TaskDelete(taskID);
}

static void TestTaskNameGet(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 taskID;
    UINT32 ret;
    CHAR  *name;

    printf("\n--- LOS_TaskNameGet ---\n");

    FillTaskParam(&param, "NamedTask", 10);
    ret = LOS_TaskCreate(&taskID, &param);
    if (ret != LOS_OK) {
        printf("  SKIP  (task create failed: %u)\n", ret);
        return;
    }

    /* 正常获取 */
    name = LOS_TaskNameGet(taskID);
    CHECK("name != NULL", name != NULL);
    if (name != NULL) {
        CHECK("name == \"NamedTask\"", strcmp(name, "NamedTask") == 0);
        printf("       name = \"%s\"\n", name);
    }

    /* 无效 ID */
    name = LOS_TaskNameGet(0xFFFFFFFF);
    CHECK("invalid ID -> NULL", name == NULL);

    LOS_TaskDelete(taskID);
}

static void TestTaskLockUnlock(void)
{
    printf("\n--- LOS_TaskLock / LOS_TaskUnlock ---\n");

    /* 加锁后 g_losTaskLock 应为 1 */
    LOS_TaskLock();
    CHECK("after Lock, g_losTaskLock == 1", g_losTaskLock == 1);

    /* 再次加锁，g_losTaskLock 应为 2 */
    LOS_TaskLock();
    CHECK("after 2nd Lock, g_losTaskLock == 2", g_losTaskLock == 2);

    /* 解锁一次 */
    LOS_TaskUnlock();
    CHECK("after 1st Unlock, g_losTaskLock == 1", g_losTaskLock == 1);

    /* 解锁归零 */
    LOS_TaskUnlock();
    CHECK("after 2nd Unlock, g_losTaskLock == 0", g_losTaskLock == 0);
}

static void TestTaskExhaustLimit(void)
{
    TSK_INIT_PARAM_S param;
    UINT32 ids[LOSCFG_BASE_CORE_TSK_LIMIT + 1];
    UINT32 ret;
    int i;
    int created = 0;

    printf("\n--- Task limit (LOSCFG_BASE_CORE_TSK_LIMIT=%d) ---\n",
           LOSCFG_BASE_CORE_TSK_LIMIT);

    for (i = 0; i <= LOSCFG_BASE_CORE_TSK_LIMIT; i++) {
        FillTaskParam(&param, "LimitTask", 10);
        ret = LOS_TaskCreate(&ids[i], &param);
        if (ret == LOS_OK) {
            created++;
        } else {
            CHECK("extra create beyond limit -> TCB_UNAVAILABLE or NO_MEMORY",
                  ret == LOS_ERRNO_TSK_TCB_UNAVAILABLE ||
                  ret == LOS_ERRNO_TSK_NO_MEMORY);
            break;
        }
    }
    printf("       created %d tasks before exhaustion\n", created);

    /* 清理 */
    for (i = 0; i < created; i++) {
        LOS_TaskDelete(ids[i]);
    }
}

/* ------------------------------------------------------------------ */

int main(void)
{
    UINT32 ret;
    UINT32 taskID = 0;

    /* 伪造合法 runTask */
    memset(&g_hostTask, 0, sizeof(g_hostTask));
    g_hostTask.taskID = 0;
    g_losTask.runTask = &g_hostTask;

    /* 初始化内核（分配 TCB 数组等） */
    ret = LOS_KernelInit();
    if (ret != LOS_OK) {
        printf("LOS_KernelInit failed: %u\n", ret);
        return 1;
    }

    printf("=== Task Test Start ===\n");

    TestTaskCreate(&taskID);
    TestTaskCreateOnly();
    TestTaskDelete(taskID);
    TestTaskSuspendResume();
    TestTaskPriority();
    TestTaskStatusGet();
    TestTaskInfoGet();
    TestTaskNameGet();
    TestTaskLockUnlock();
    TestTaskExhaustLimit();

    printf("\n=== Task Test End ===\n");
    return 0;
}