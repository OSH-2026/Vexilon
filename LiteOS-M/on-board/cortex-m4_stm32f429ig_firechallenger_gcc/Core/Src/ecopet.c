#include "ecopet.h"
#include "los_config.h"
#include "los_debug.h"
#include "los_task.h"
#include "los_queue.h"
#include "los_tick.h"
#include "los_interrupt.h"
#include "main.h"
#include "usart.h"
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

/* Ring buffer - written by ISR, read by UartRxTask */
volatile char g_uartRxBuf[UART_RX_BUF_SIZE];
volatile uint8_t g_uartRxHead = 0;
volatile uint8_t g_uartRxTail = 0;

/* Queue for passing parsed commands */
UINT32 g_cmdQueueId;

/* Global pet state - all full at startup */
EcoPetState g_petState = {
    .health = 100,
    .hunger = 0,
    .mood   = 100,
    .energy = 100,
};

/* ========== Helper Functions ========== */

static int16_t clamp(int16_t val, int16_t lo, int16_t hi)
{
    if (val < lo) return lo;
    if (val > hi) return hi;
    return val;
}

/* LCG pseudo-random number generator */
static uint32_t g_randSeed = 12345;

static uint16_t SimpleRand(void)
{
    g_randSeed = g_randSeed * 1103515245u + 12345u;
    return (uint16_t)((g_randSeed >> 16) & 0x7FFFu);
}

/* Returns random integer in [lo, hi] */
static int16_t RandRange(int16_t lo, int16_t hi)
{
    return lo + (int16_t)(SimpleRand() % (uint16_t)(hi - lo + 1));
}

static void SendResponse(const char *prefix, const char *cmd)
{
    printf("%s:%s health=%d hunger=%d mood=%d energy=%d\r\n",
           prefix, cmd,
           g_petState.health, g_petState.hunger,
           g_petState.mood, g_petState.energy);
}

static void UpdateLEDs(void)
{
    /* PF9 green: happy (mood > 50) */
    if (g_petState.mood > 50) {
        HAL_GPIO_WritePin(GPIOF, GPIO_PIN_9, GPIO_PIN_SET);
    } else {
        HAL_GPIO_WritePin(GPIOF, GPIO_PIN_9, GPIO_PIN_RESET);
    }

    /* PF10 red: needs attention (too hungry, low health, or exhausted) */
    if (g_petState.hunger > 60 || g_petState.health < 50 || g_petState.energy < 20) {
        HAL_GPIO_WritePin(GPIOF, GPIO_PIN_10, GPIO_PIN_SET);
    } else {
        HAL_GPIO_WritePin(GPIOF, GPIO_PIN_10, GPIO_PIN_RESET);
    }
}

/* ISR registered via LOS_HwiCreate - called by LiteOS vector table */
static void Usart1RxIsr(void)
{
    if (USART1->SR & USART_SR_RXNE) {
        char ch = (char)(USART1->DR & 0xFF);
        uint8_t next = (uint8_t)((g_uartRxHead + 1) % UART_RX_BUF_SIZE);
        if (next != g_uartRxTail) {
            g_uartRxBuf[g_uartRxHead] = ch;
            g_uartRxHead = next;
        }
    }
}

/* ========== Command Parser ========== */

static int ParseCommand(const char *line, EcoCmdMsg *msg)
{
    int val;

    if (strcmp(line, "STATUS") == 0) {
        msg->type = CMD_STATUS; msg->param = 0; return 0;
    }
    if (strcmp(line, "SLEEP") == 0) {
        msg->type = CMD_SLEEP; msg->param = 0; return 0;
    }
    if (strcmp(line, "HEAL") == 0) {
        msg->type = CMD_HEAL; msg->param = 0; return 0;
    }
    if (sscanf(line, "FEED %d", &val) == 1) {
        if (val < 1 || val > 100) return -2;
        msg->type = CMD_FEED; msg->param = (int16_t)val; return 0;
    }
    if (sscanf(line, "PLAY %d", &val) == 1) {
        if (val < 1 || val > 100) return -2;
        msg->type = CMD_PLAY; msg->param = (int16_t)val; return 0;
    }
    return -1;
}

/* ========== UART RX Task (polling ring buffer) ========== */

static VOID UartRxTask(VOID)
{
    char lineBuf[UART_RX_BUF_SIZE];
    uint8_t linePos = 0;
    uint8_t rxByte;

    while (1) {
        if (g_uartRxTail != g_uartRxHead) {
            rxByte = (uint8_t)g_uartRxBuf[g_uartRxTail];
            g_uartRxTail = (uint8_t)((g_uartRxTail + 1) % UART_RX_BUF_SIZE);
            char ch = (char)rxByte;

            if (ch == '\n' || ch == '\r') {
                if (linePos == 0) continue;
                lineBuf[linePos] = '\0';

                EcoCmdMsg msg;
                int ret = ParseCommand(lineBuf, &msg);
                if (ret == 0) {
                    UINT32 qret = LOS_QueueWriteCopy(g_cmdQueueId, &msg,
                                                     sizeof(EcoCmdMsg), 0);
                    if (qret != LOS_OK) {
                        printf("ERR:QUEUE_FULL\r\n");
                    }
                } else if (ret == -2) {
                    printf("ERR:INVALID_ARG\r\n");
                } else {
                    printf("ERR:INVALID_CMD\r\n");
                }

                linePos = 0;
            } else {
                if (linePos < UART_RX_BUF_SIZE - 1) {
                    lineBuf[linePos++] = ch;
                }
            }
        } else {
            LOS_TaskDelay(10);
        }
    }
}

/* ========== Pet State Task ========== */

static VOID PetStateTask(VOID)
{
    EcoCmdMsg msg;
    UINT32 bufSize;

    while (1) {
        bufSize = sizeof(EcoCmdMsg);
        UINT32 ret = LOS_QueueReadCopy(g_cmdQueueId, &msg, &bufSize, LOS_WAIT_FOREVER);
        if (ret != LOS_OK) continue;

        switch (msg.type) {
        case CMD_STATUS:
            SendResponse("OK", "STATUS");
            break;

        case CMD_FEED:
            if (g_petState.hunger < 20 && msg.param > 50) {
                /* Overfeed when not hungry: damages health */
                g_petState.health = clamp((int16_t)(g_petState.health - 15),            0, 100);
                g_petState.mood   = clamp((int16_t)(g_petState.mood   - 10),            0, 100);
                g_petState.hunger = clamp((int16_t)(g_petState.hunger - msg.param / 2), 0, 100);
                SendResponse("WARN", "OVERFED");
            } else {
                g_petState.hunger = clamp((int16_t)(g_petState.hunger - msg.param), 0, 100);
                g_petState.mood   = clamp((int16_t)(g_petState.mood   + 5),         0, 100);
                SendResponse("OK", "FEED");
            }
            break;

        case CMD_PLAY: {
            /* Random deltas [0, 30] */
            int16_t moodGain   = RandRange(0, 30);
            int16_t energyCost = RandRange(0, 30);
            int16_t hungerGain = RandRange(0, 30);

            if (g_petState.energy < 20) {
                /* Too tired to play: health penalty */
                int16_t newHealth = clamp((int16_t)(g_petState.health - 5), 0, 100);
                if (newHealth <= 0) {
                    printf("ERR:PLAY_FATAL health too low\r\n");
                    break;
                }
                g_petState.health = newHealth;
                g_petState.mood   = clamp((int16_t)(g_petState.mood   + moodGain / 2),   0, 100);
                g_petState.energy = clamp((int16_t)(g_petState.energy - energyCost / 3), 0, 100);
                SendResponse("WARN", "OVERTIRED");
            } else {
                /* Predict whether play would kill the pet */
                int16_t previewEnergy = clamp((int16_t)(g_petState.energy - energyCost), 0, 100);
                int16_t previewHealth = g_petState.health;
                if (previewEnergy < 10) previewHealth -= 2;
                if (previewHealth <= 0) {
                    printf("ERR:PLAY_FATAL would kill pet\r\n");
                    break;
                }

                g_petState.mood   = clamp((int16_t)(g_petState.mood   + moodGain),   0, 100);
                g_petState.energy = clamp((int16_t)(g_petState.energy - energyCost), 0, 100);
                g_petState.hunger = clamp((int16_t)(g_petState.hunger + hungerGain), 0, 100);

                /* 20% chance of random injury: health drops [5, 20] */
                if (SimpleRand() % 100 < 20) {
                    int16_t injury = RandRange(5, 20);
                    g_petState.health = clamp((int16_t)(g_petState.health - injury), 0, 100);
                    printf("WARN:INJURED health=%d\r\n", g_petState.health);
                }

                SendResponse("OK", "PLAY");
            }
            break;
        }

        case CMD_SLEEP:
            g_petState.energy = clamp((int16_t)(g_petState.energy + 40), 0, 100);
            g_petState.health = clamp((int16_t)(g_petState.health + 15), 0, 100);
            g_petState.mood   = clamp((int16_t)(g_petState.mood   +  5), 0, 100);
            SendResponse("OK", "SLEEP");
            break;

        case CMD_HEAL:
            g_petState.health = clamp((int16_t)(g_petState.health + 30), 0, 100);
            g_petState.mood   = clamp((int16_t)(g_petState.mood   -  5), 0, 100);
            SendResponse("OK", "HEAL");
            break;

        default:
            printf("ERR:UNKNOWN_CMD\r\n");
            break;
        }

        /* Post-command health penalties (all clamped) */
        if (g_petState.hunger > 80)
            g_petState.health = clamp((int16_t)(g_petState.health - 3), 0, 100);
        else if (g_petState.hunger > 70)
            g_petState.health = clamp((int16_t)(g_petState.health - 1), 0, 100);
        if (g_petState.energy < 10)
            g_petState.health = clamp((int16_t)(g_petState.health - 2), 0, 100);
        if (g_petState.mood < 20)
            g_petState.health = clamp((int16_t)(g_petState.health - 1), 0, 100);

        UpdateLEDs();
    }
}

/* ========== Telemetry Task ========== */

static uint8_t g_decayCounter = 0;

static VOID TelemetryTask(VOID)
{
    while (1) {
        LOS_TaskDelay(1000); /* 1-second tick */

        g_decayCounter++;

        /* Decay every 10 seconds */
        if (g_decayCounter >= 10) {
            g_decayCounter = 0;

            g_petState.hunger = clamp((int16_t)(g_petState.hunger + 2), 0, 100);
            g_petState.energy = clamp((int16_t)(g_petState.energy - 1), 0, 100);
            g_petState.mood   = clamp((int16_t)(g_petState.mood   - 1), 0, 100);

            /* Health penalties from bad state */
            if (g_petState.hunger > 80)
                g_petState.health = clamp((int16_t)(g_petState.health - 3), 0, 100);
            else if (g_petState.hunger > 70)
                g_petState.health = clamp((int16_t)(g_petState.health - 1), 0, 100);
            if (g_petState.energy < 10)
                g_petState.health = clamp((int16_t)(g_petState.health - 2), 0, 100);
            if (g_petState.mood < 20)
                g_petState.health = clamp((int16_t)(g_petState.health - 1), 0, 100);
        }

        /* Low-value warnings every second (threshold: 10% of max) */
        if (g_petState.health <= 10)
            printf("WARN:LOW_HEALTH health=%d\r\n", g_petState.health);
        if (g_petState.hunger >= 90)
            printf("WARN:HIGH_HUNGER hunger=%d\r\n", g_petState.hunger);
        if (g_petState.mood <= 10)
            printf("WARN:LOW_MOOD mood=%d\r\n", g_petState.mood);
        if (g_petState.energy <= 10)
            printf("WARN:LOW_ENERGY energy=%d\r\n", g_petState.energy);

        UpdateLEDs();
    }
}

/* ========== EcoPet Main Entry ========== */

void EcoPetMain(void)
{
    UINT32 ret;
    UINT32 taskId;
    TSK_INIT_PARAM_S taskParam = {0};

    /* Initialize LiteOS-M kernel */
    ret = LOS_KernelInit();
    if (ret != LOS_OK) {
        printf("ERR:KERNEL_INIT_FAILED\r\n");
        return;
    }

    /* Register USART1 RX interrupt with LiteOS (priority = 6) */
    ret = LOS_HwiCreate(USART1_IRQn, 6, 0, (HWI_PROC_FUNC)Usart1RxIsr, 0);
    if (ret != LOS_OK) {
        printf("ERR:HWI_CREATE %lu\r\n", ret);
        return;
    }

    /* Create command queue */
    ret = LOS_QueueCreate("cmd_q", 8, &g_cmdQueueId, 0, sizeof(EcoCmdMsg));
    if (ret != LOS_OK) {
        printf("ERR:QUEUE_CREATE_FAILED\r\n");
        return;
    }

    /* Create UartRxTask */
    taskParam.pfnTaskEntry = (TSK_ENTRY_FUNC)UartRxTask;
    taskParam.uwStackSize  = UART_RX_TASK_STACK;
    taskParam.pcName       = "UartRxTask";
    taskParam.usTaskPrio   = UART_RX_TASK_PRIO;
    ret = LOS_TaskCreate(&taskId, &taskParam);
    if (ret != LOS_OK) {
        printf("ERR:TASK_CREATE UartRxTask\r\n");
    }

    /* Create PetStateTask */
    memset(&taskParam, 0, sizeof(taskParam));
    taskParam.pfnTaskEntry = (TSK_ENTRY_FUNC)PetStateTask;
    taskParam.uwStackSize  = PET_STATE_TASK_STACK;
    taskParam.pcName       = "PetStateTask";
    taskParam.usTaskPrio   = PET_STATE_TASK_PRIO;
    ret = LOS_TaskCreate(&taskId, &taskParam);
    if (ret != LOS_OK) {
        printf("ERR:TASK_CREATE PetStateTask\r\n");
    }

    /* Create TelemetryTask */
    memset(&taskParam, 0, sizeof(taskParam));
    taskParam.pfnTaskEntry = (TSK_ENTRY_FUNC)TelemetryTask;
    taskParam.uwStackSize  = TELEMETRY_TASK_STACK;
    taskParam.pcName       = "TelemetryTask";
    taskParam.usTaskPrio   = TELEMETRY_TASK_PRIO;
    ret = LOS_TaskCreate(&taskId, &taskParam);
    if (ret != LOS_OK) {
        printf("ERR:TASK_CREATE TelemetryTask\r\n");
    }

    printf("[EcoPet] System initialized. Awaiting commands...\r\n");

    /* Start kernel scheduler */
    LOS_Start();
}
