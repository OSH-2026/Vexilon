#include <stdio.h>
#include <string.h>

#include "los_sortlink.h"
#include "los_sched.h"
#include "los_task.h"
#include "los_tick.h"

/* 伪造合法 runTask */
static LosTaskCB g_hostTask;

#define CHECK(desc, expr) \
    do { \
        if (expr) { \
            printf("  PASS  %s\n", desc); \
        } else { \
            printf("  FAIL  %s\n", desc); \
        } \
    } while (0)

/* ------------------------------------------------------------------ */

static void TestOsSortLinkInit(void)
{
    printf("\n--- OsSortLinkInit ---\n");

    SortLinkAttribute link;
    UINT32 ret = OsSortLinkInit(&link);

    CHECK("OsSortLinkInit returns LOS_OK", ret == LOS_OK);
    CHECK("sortLink is empty after init", LOS_ListEmpty(&link.sortLink));
}

/* ------------------------------------------------------------------ */

static void TestOsGetSortLinkAttribute(void)
{
    printf("\n--- OsGetSortLinkAttribute ---\n");

    /* TASK 类型应返回 &g_taskSortLink */
    SortLinkAttribute *attr = OsGetSortLinkAttribute(OS_SORT_LINK_TASK);
    CHECK("OS_SORT_LINK_TASK -> non-NULL", attr != NULL);
    CHECK("OS_SORT_LINK_TASK -> &g_taskSortLink", attr == &g_taskSortLink);

    /* 无效类型应返回 NULL */
    attr = OsGetSortLinkAttribute((SortLinkType)0xFF);
    CHECK("invalid type -> NULL", attr == NULL);
}

/* ------------------------------------------------------------------ */

static void TestOsAdd2SortLink_and_Delete(void)
{
    printf("\n--- OsAdd2SortLink / OsDeleteSortLink ---\n");

    /* 使用独立的 SortLinkList 节点，不依赖任务 TCB */
    SortLinkList node1, node2, node3;
    memset(&node1, 0, sizeof(node1));
    memset(&node2, 0, sizeof(node2));
    memset(&node3, 0, sizeof(node3));

    /* 初始化节点的 responseTime 为无效值 */
    SET_SORTLIST_VALUE(&node1, OS_SORT_LINK_INVALID_TIME);
    SET_SORTLIST_VALUE(&node2, OS_SORT_LINK_INVALID_TIME);
    SET_SORTLIST_VALUE(&node3, OS_SORT_LINK_INVALID_TIME);

    /* 先清空 g_taskSortLink */
    OsSortLinkInit(&g_taskSortLink);
    CHECK("g_taskSortLink empty before test",
          LOS_ListEmpty(&g_taskSortLink.sortLink));

    UINT64 now = LOS_SysCycleGet();

    /* 插入三个节点，等待时间不同（waitTicks 对应延迟） */
    OsAdd2SortLink(&node1, now, 30, OS_SORT_LINK_TASK);  /* 最晚到期 */
    OsAdd2SortLink(&node2, now, 10, OS_SORT_LINK_TASK);  /* 最早到期 */
    OsAdd2SortLink(&node3, now, 20, OS_SORT_LINK_TASK);  /* 居中     */

    CHECK("after 3 inserts, list not empty",
          !LOS_ListEmpty(&g_taskSortLink.sortLink));

    /* 验证链表头（pstNext）是最早到期的节点（node2，waitTicks=10） */
    SortLinkList *first = LOS_DL_LIST_ENTRY(
        g_taskSortLink.sortLink.pstNext, SortLinkList, sortLinkNode);
    CHECK("first node is earliest (node2, waitTicks=10)",
          first == &node2);

    /* 验证排序顺序：node2 < node3 < node1 */
    SortLinkList *second = LOS_DL_LIST_ENTRY(
        first->sortLinkNode.pstNext, SortLinkList, sortLinkNode);
    SortLinkList *third = LOS_DL_LIST_ENTRY(
        second->sortLinkNode.pstNext, SortLinkList, sortLinkNode);
    CHECK("second node is node3 (waitTicks=20)", second == &node3);
    CHECK("third node is node1 (waitTicks=30)",  third  == &node1);
    CHECK("responseTime order: node2 <= node3 <= node1",
          node2.responseTime <= node3.responseTime &&
          node3.responseTime <= node1.responseTime);

    printf("       node2.responseTime = %llu\n", (unsigned long long)node2.responseTime);
    printf("       node3.responseTime = %llu\n", (unsigned long long)node3.responseTime);
    printf("       node1.responseTime = %llu\n", (unsigned long long)node1.responseTime);

    /* 删除中间节点 node3 */
    OsDeleteSortLink(&node3);
    CHECK("after delete node3: responseTime set to INVALID",
          node3.responseTime == OS_SORT_LINK_INVALID_TIME);

    /* 再次删除同一节点（responseTime 已为 INVALID，应跳过） */
    OsDeleteSortLink(&node3);
    CHECK("double delete node3: no crash", 1);

    /* 删除剩余节点 */
    OsDeleteSortLink(&node2);
    OsDeleteSortLink(&node1);
    CHECK("after all deletes, list empty",
          LOS_ListEmpty(&g_taskSortLink.sortLink));
}

/* ------------------------------------------------------------------ */

static void TestOsSortLinkGetNextExpireTime(void)
{
    printf("\n--- OsSortLinkGetNextExpireTime ---\n");

    /* 链表为空时应返回 0 */
    OsSortLinkInit(&g_taskSortLink);
    UINT64 expire = OsSortLinkGetNextExpireTime(&g_taskSortLink);
    CHECK("empty list -> 0", expire == 0);

    /* 插入一个节点后应返回非零值 */
    SortLinkList node;
    memset(&node, 0, sizeof(node));
    SET_SORTLIST_VALUE(&node, OS_SORT_LINK_INVALID_TIME);

    UINT64 now = LOS_SysCycleGet();
    OsAdd2SortLink(&node, now, 100, OS_SORT_LINK_TASK);

    expire = OsSortLinkGetNextExpireTime(&g_taskSortLink);
    /* 节点在未来到期，currTime < responseTime，所以返回剩余时间 > 0 */
    CHECK("non-empty list -> expire > 0", expire > 0);
    printf("       next expire time = %llu cycles\n", (unsigned long long)expire);

    OsDeleteSortLink(&node);
    OsSortLinkInit(&g_taskSortLink);
}

/* ------------------------------------------------------------------ */

static void TestOsSortLinkGetTargetExpireTime(void)
{
    printf("\n--- OsSortLinkGetTargetExpireTime ---\n");

    SortLinkList node;
    memset(&node, 0, sizeof(node));

    UINT64 currTime = LOS_SysCycleGet();

    /* 节点在未来到期：responseTime > currTime */
    SET_SORTLIST_VALUE(&node, currTime + 100000);
    UINT64 remain = OsSortLinkGetTargetExpireTime(currTime, &node);
    CHECK("future node: remain == responseTime - currTime",
          remain == node.responseTime - currTime);

    /* 节点已过期：responseTime <= currTime */
    SET_SORTLIST_VALUE(&node, currTime - 1);
    remain = OsSortLinkGetTargetExpireTime(currTime, &node);
    CHECK("expired node: remain == 0", remain == 0);

    printf("       future remain  = %llu cycles\n", (unsigned long long)(currTime + 100000 - currTime));
}

/* ------------------------------------------------------------------ */

static void TestOsGetNextExpireTime(void)
{
    printf("\n--- OsGetNextExpireTime (inline) ---\n");

    /* 确保 g_taskSortLink 为空 */
    OsSortLinkInit(&g_taskSortLink);

    UINT64 startTime    = LOS_SysCycleGet();
    UINT32 precision    = g_sysClock / LOSCFG_BASE_CORE_TICK_PER_SECOND_MINI;
    UINT64 expireTime   = OsGetNextExpireTime(startTime, precision);

    /* 空链表时返回 MAX - precision */
    CHECK("empty sortlink -> MAX - precision",
          expireTime == OS_SORT_LINK_UINT64_MAX - precision);

    /* 插入一个节点，到期时间在 precision 之外 */
    SortLinkList node;
    memset(&node, 0, sizeof(node));
    SET_SORTLIST_VALUE(&node, OS_SORT_LINK_INVALID_TIME);
    OsAdd2SortLink(&node, startTime, 200, OS_SORT_LINK_TASK);

    expireTime = OsGetNextExpireTime(startTime, precision);
    CHECK("with node beyond precision -> node.responseTime",
          expireTime == node.responseTime);

    printf("       expireTime = %llu\n", (unsigned long long)expireTime);

    OsDeleteSortLink(&node);
    OsSortLinkInit(&g_taskSortLink);
}

/* ------------------------------------------------------------------ */

static void TestOsSortLinkGetRemainTime(void)
{
    printf("\n--- OsSortLinkGetRemainTime (inline) ---\n");

    SortLinkList node;
    UINT64 currTime = LOS_SysCycleGet();

    /* 未来到期 */
    SET_SORTLIST_VALUE(&node, currTime + 5000);
    UINT64 remain = OsSortLinkGetRemainTime(currTime, &node);
    CHECK("future node: remain == 5000", remain == 5000);

    /* 已到期 */
    SET_SORTLIST_VALUE(&node, currTime - 1);
    remain = OsSortLinkGetRemainTime(currTime, &node);
    CHECK("expired node: remain == 0", remain == 0);
}

/* ------------------------------------------------------------------ */

static void TestSetGetSortListValue(void)
{
    printf("\n--- SET_SORTLIST_VALUE / GET_SORTLIST_VALUE ---\n");

    SortLinkList node;
    SET_SORTLIST_VALUE(&node, 0xDEADBEEFCAFEBABEULL);
    UINT64 val = GET_SORTLIST_VALUE(&node);
    CHECK("SET then GET returns same value",
          val == 0xDEADBEEFCAFEBABEULL);
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

    printf("=== SortLink Test Start ===\n");

    TestOsSortLinkInit();
    TestOsGetSortLinkAttribute();
    TestOsAdd2SortLink_and_Delete();
    TestOsSortLinkGetNextExpireTime();
    TestOsSortLinkGetTargetExpireTime();
    TestOsGetNextExpireTime();
    TestOsSortLinkGetRemainTime();
    TestSetGetSortListValue();

    printf("\n=== SortLink Test End ===\n");
    return 0;
}