LITEOSTOPDIR := ../../kernel_liteos_m
LITEOSTOPDIR := $(realpath $(LITEOSTOPDIR))

# Common
C_SOURCES     += $(wildcard $(LITEOSTOPDIR)/kernel/src/*.c) \
                 $(wildcard $(LITEOSTOPDIR)/kernel/src/mm/*.c) \
                 $(wildcard $(LITEOSTOPDIR)/components/cpup/*.c) \
                 $(wildcard $(LITEOSTOPDIR)/components/power/*.c) \
                 $(wildcard $(LITEOSTOPDIR)/components/backtrace/*.c) \
                 $(wildcard $(LITEOSTOPDIR)/components/exchook/*.c) \
                 $(wildcard $(LITEOSTOPDIR)/../third_party/bounds_checking_function/src/*.c) \
                 $(wildcard $(LITEOSTOPDIR)/utils/*.c) \
                 $(wildcard $(LITEOSTOPDIR)/kal/posix/src/*.c)

C_INCLUDES    += -I$(LITEOSTOPDIR)/utils \
                 -I$(LITEOSTOPDIR)/kal/posix/include \
				 -I$(LITEOSTOPDIR)/kal/posix/musl_src/internal \
                 -I$(LITEOSTOPDIR)/kernel/include \
                 -I$(LITEOSTOPDIR)/components/cpup \
                 -I$(LITEOSTOPDIR)/components/power \
                 -I$(LITEOSTOPDIR)/components/backtrace \
                 -I$(LITEOSTOPDIR)/components/exchook \
                 -I$(LITEOSTOPDIR)/components/signal \
                 -I$(LITEOSTOPDIR)/../third_party/cmsis/CMSIS/RTOS2/Include \
                 -I$(LITEOSTOPDIR)/../third_party/musl/porting/liteos_m/kernel/include \
                 -I$(LITEOSTOPDIR)/../third_party/bounds_checking_function/include

# TestSuite
# C_SOURCES     +=  $(wildcard $(LITEOSTOPDIR)/testsuits/sample/kernel/task/*.c) \
#                  $(wildcard $(LITEOSTOPDIR)/testsuits/src/*.c)
                #  $(wildcard $(LITEOSTOPDIR)/testsuits/sample/kernel/event/*.c) \
                #  $(wildcard $(LITEOSTOPDIR)/testsuits/sample/kernel/hwi/*.c) \
                #  $(wildcard $(LITEOSTOPDIR)/testsuits/sample/kernel/mux/*.c) \
                #  $(wildcard $(LITEOSTOPDIR)/testsuits/sample/kernel/queue/*.c) \
                #  $(wildcard $(LITEOSTOPDIR)/testsuits/sample/kernel/sem/*.c) \
                #  $(wildcard $(LITEOSTOPDIR)/testsuits/sample/kernel/swtmr/*.c) \
                #  $(wildcard $(LITEOSTOPDIR)/testsuits/sample/kernel/task/*.c) \
                #  $(wildcard $(LITEOSTOPDIR)/testsuits/src/*.c)

#C_INCLUDES    += -I$(LITEOSTOPDIR)/testsuits/include


# Related to arch
ASM_SOURCES   += $(wildcard $(LITEOSTOPDIR)/arch/arm/cortex-m4/gcc/*.s)

ASMS_SOURCES  += $(wildcard $(LITEOSTOPDIR)/arch/arm/cortex-m4/gcc/*.S)

C_SOURCES     += $(wildcard $(LITEOSTOPDIR)/arch/arm/cortex-m4/gcc/*.c) \
                 $(wildcard $(LITEOSTOPDIR)/arch/arm/common/*.c) \
                 $(wildcard $(LITEOSTOPDIR)/components/signal/*.c)

C_INCLUDES    += -I. \
                 -I$(LITEOSTOPDIR)/arch/include \
                 -I$(LITEOSTOPDIR)/arch/arm/cortex-m4/gcc \
                 -I$(LITEOSTOPDIR)/arch/arm/common


CFLAGS        += -nostdinc -nostdlib
ASFLAGS       += -imacros $(LITEOSTOPDIR)/kernel/include/los_config.h -DCLZ=CLZ

# list of ASM .S program objects
OBJECTS += $(addprefix $(BUILD_DIR)/,$(notdir $(ASMS_SOURCES:.S=.o)))
vpath %.S $(sort $(dir $(ASMS_SOURCES)))

$(BUILD_DIR)/%.o: %.S Makefile | $(BUILD_DIR)
	$(CC) -c $(CFLAGS) $(ASFLAGS) $< -o $@
