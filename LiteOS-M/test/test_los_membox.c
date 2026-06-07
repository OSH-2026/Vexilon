#include <stdio.h>
#include <string.h>

#include "los_membox.h"
#include "los_task.h"

#define BLOCK_SIZE 32
#define BLOCK_NUM  8

static unsigned char g_pool[LOS_MEMBOX_SIZE(BLOCK_SIZE, BLOCK_NUM)];

int main(void)
{
    /* 伪造合法的 runTask，让 LOS_CurTaskIDGet() 返回 0，
     * 避免 OsMemBoxCheckMagic 因 taskID 非法而校验失败 */
    static LosTaskCB g_hostTask;
    memset(&g_hostTask, 0, sizeof(g_hostTask));
    g_hostTask.taskID = 0;
    g_losTask.runTask = &g_hostTask;

    UINT32 ret;
    UINT32 maxBlk;
    UINT32 blkCnt;
    UINT32 blkSize;
    void  *ptr[BLOCK_NUM];

    printf("=== Membox Test Start ===\n");

    /* 初始化 */
    ret = LOS_MemboxInit(g_pool, sizeof(g_pool), BLOCK_SIZE);
    printf("LOS_MemboxInit() = %u\n", ret);
    if (ret != LOS_OK) {
        printf("Init failed!\n");
        return 1;
    }

    /* 初始统计信息 */
    LOS_MemboxStatisticsGet(g_pool, &maxBlk, &blkCnt, &blkSize);
    printf("Statistics: maxBlk=%u  blkCnt=%u  blkSize=%u\n",
           maxBlk, blkCnt, blkSize);

    /* 连续分配 */
    printf("\nAllocating blocks:\n");
    for (int i = 0; i < BLOCK_NUM; i++) {
        ptr[i] = LOS_MemboxAlloc(g_pool);
        printf("  block[%d] = %p\n", i, ptr[i]);
        if (ptr[i] == NULL) {
            printf("Unexpected allocation failure!\n");
            return 1;
        }
    }

    /* 池已耗尽，再分配应返回 NULL */
    void *extra = LOS_MemboxAlloc(g_pool);
    printf("\nExtra allocation = %p (expect NULL)\n", extra);

    /* 全部分配后统计 */
    LOS_MemboxStatisticsGet(g_pool, &maxBlk, &blkCnt, &blkSize);
    printf("After full allocation: blkCnt=%u\n", blkCnt);

    /* 清零测试 */
    memset(ptr[0], 0xAA, BLOCK_SIZE);
    LOS_MemboxClr(g_pool, ptr[0]);
    int allZero = 1;
    for (int i = 0; i < BLOCK_SIZE; i++) {
        if (((unsigned char *)ptr[0])[i] != 0) {
            allZero = 0;
            break;
        }
    }
    printf("\nLOS_MemboxClr() = %s\n", allZero ? "PASS" : "FAIL");

    /* 释放一个块，再重新分配 */
    ret = LOS_MemboxFree(g_pool, ptr[0]);
    printf("\nFree block[0] = %u\n", ret);

    void *newBlock = LOS_MemboxAlloc(g_pool);
    printf("Realloc after free = %p\n", newBlock);
    if (newBlock == NULL) {
        printf("Reallocation failed!\n");
        return 1;
    }

    /* 释放所有块 */
    for (int i = 1; i < BLOCK_NUM; i++) {
        LOS_MemboxFree(g_pool, ptr[i]);
    }
    LOS_MemboxFree(g_pool, newBlock);

    /* 最终统计 */
    LOS_MemboxStatisticsGet(g_pool, &maxBlk, &blkCnt, &blkSize);
    printf("\nFinal Statistics: maxBlk=%u  blkCnt=%u  blkSize=%u\n",
           maxBlk, blkCnt, blkSize);

    printf("\n=== Membox Test End ===\n");
    return 0;
}