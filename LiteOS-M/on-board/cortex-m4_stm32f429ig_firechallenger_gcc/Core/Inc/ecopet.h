#ifndef __ECOPET_H
#define __ECOPET_H

#include <stdint.h>
#include "los_task.h"
#include "los_queue.h"

#ifdef __cplusplus
extern "C" {
#endif

/* Command types */
typedef enum {
    CMD_STATUS = 0,
    CMD_FEED,
    CMD_PLAY,
    CMD_SLEEP,
    CMD_HEAL,
} EcoCmdType;

/* Command message passed through queue */
typedef struct {
    EcoCmdType type;
    int16_t param;
} EcoCmdMsg;

/* Pet state */
typedef struct {
    int16_t health;
    int16_t hunger;
    int16_t mood;
    int16_t energy;
} EcoPetState;

/* UART RX ring buffer */
#define UART_RX_BUF_SIZE 128

extern volatile char g_uartRxBuf[UART_RX_BUF_SIZE];
extern volatile uint8_t g_uartRxHead;
extern volatile uint8_t g_uartRxTail;

/* Queue ID */
extern UINT32 g_cmdQueueId;

/* Pet state (only written by PetStateTask) */
extern EcoPetState g_petState;

/* Task priorities */
#define UART_RX_TASK_PRIO    6
#define PET_STATE_TASK_PRIO  8
#define TELEMETRY_TASK_PRIO  10

/* Task stack sizes */
#define UART_RX_TASK_STACK   0x1000
#define PET_STATE_TASK_STACK 0x1000
#define TELEMETRY_TASK_STACK 0x1000

/* Entry point called from main */
void EcoPetMain(void);

#ifdef __cplusplus
}
#endif

#endif /* __ECOPET_H */
