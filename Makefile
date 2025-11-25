CC := cargo

# Rust executable files
TARGET := static-server

# Colors
BOLD := \033[1m
BOLD_GREEN := \033[1;32m
RESET := \033[0m

# Directories
TARGET_DIR := ./build
DEBUG_DIR := $(TARGET_DIR)/debug
RELEASE_DIR := $(TARGET_DIR)/release

CUR_DIR := 

# Compile keys
RELEASE_KEY := --release

.PHONY: all clean debug build release run

all: debug release

build: debug

debug: $(DEBUG_DIR)
	@echo "=$(BOLD_GREEN)COMPILING$(RESET)=============================================================================="

	$(CC) build && \
	cp ./target/$@/$(TARGET) ./$^/
	
	@$(MAKE) log-info CUR_DIR=$^

release: $(RELEASE_DIR)
	@echo "=$(BOLD_GREEN)COMPILING$(RESET)=============================================================================="

	$(CC) build $(RELEASE_KEY) && \
	cp ./target/$@/$(TARGET) ./$^/
	
	@$(MAKE) log-info CUR_DIR=$^

BUILD := debug
LOG_LEVEL := info
run:
	@$(MAKE) $(BUILD)
	RUST_LOG=$(LOG_LEVEL) ./build/$(BUILD)/$(TARGET)

$(RELEASE_DIR):
	mkdir -p $(TARGET_DIR)
	mkdir -p $(RELEASE_DIR)

$(DEBUG_DIR):
	mkdir -p $(TARGET_DIR)
	mkdir -p $(DEBUG_DIR)

log-info:
	@echo "=$(BOLD_GREEN)INFO$(RESET)==================================================================================="
	@echo "$(BOLD)Исполнямые файлы:$(RESET)"
	@echo "  • ./$(CUR_DIR)/$(TARGET)"
	@echo "$(BOLD)Запуск:$(RESET)"
	@echo "  • Server: [RUST_LOG=<уровень>] ./$(CUR_DIR)/$(TARGET)"
	@echo ""
	@echo "$(BOLD)Доступные уровни логирования:$(RESET)"
	@echo "  • RUST_LOG=error   - только критические ошибки"
	@echo "  • RUST_LOG=warn    - ошибки и предупреждения"
	@echo "  • RUST_LOG=info    - основная информация о работе (по умолчанию)"
	@echo "  • RUST_LOG=debug   - технические детали для разработчиков"
	@echo "  • RUST_LOG=trace   - максимальная детализация"
	@echo ""
	@echo "$(BOLD)Альтереативный запуск:$(RESET)"
	@echo "make run [BUILD=<release|debug>] [LOG_LEVEL=<уровень>]"
	@echo ""
	@echo "$(BOLD)По умолчанию:$(RESET)"
	@echo "  • BUILD=debug"
	@echo "  • LOG_LEVEL=info"
	@echo =========================================================================================


clean:
	rm -rf $(TARGET_DIR) && \
	cargo clean
