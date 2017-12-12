BUILD_DIR=build

PROJECT=testrom

ASM_SRC=$(PROJECT).s
ASM_OBJ=$(BUILD_DIR)/$(PROJECT)_s.o

C_SRC=$(PROJECT).c
C_OBJ=$(BUILD_DIR)/$(PROJECT)_c.o

PROJECT_ELF=$(BUILD_DIR)/$(PROJECT).elf
PROJECT_TARGET=$(BUILD_DIR)/$(PROJECT).vxe

LINKER_SCRIPT=linker_script.ld

AS=v810-as
GCC=v810-gcc
OBJCOPY=v810-objcopy

RM=rm
RM_FLAGS=-rf

.PHONY: all dirs clean

all: dirs $(PROJECT_TARGET)

dirs: $(BUILD_DIR)

$(BUILD_DIR):
	mkdir -p $(BUILD_DIR)

$(ASM_OBJ): $(ASM_SRC)
	$(AS) -o $(ASM_OBJ) $(ASM_SRC)

$(C_OBJ): $(C_SRC) $(C_HEADER)
	$(GCC) -nodefaultlibs -mv810 -O3 -c $(C_SRC) -o $(C_OBJ)

$(PROJECT_ELF): $(ASM_OBJ) $(C_OBJ) $(LINKER_SCRIPT)
	$(GCC) -o $(PROJECT_ELF) -nostartfiles $(ASM_OBJ) $(C_OBJ) -T$(LINKER_SCRIPT)

$(PROJECT_TARGET): $(PROJECT_ELF)
	$(OBJCOPY) -O binary $(PROJECT_ELF) $(PROJECT_TARGET)

clean:
	$(RM) $(RM_FLAGS) $(BUILD_DIR)
