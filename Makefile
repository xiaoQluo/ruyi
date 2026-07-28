# Ruyi Compiler Makefile
# Author: Ruyi Team
# Date: 2026-05-06

.PHONY: build build-release build-runtime install clean check test test-single fmt lint help \
        run-example compile-example build-debug

# Configuration
RUYI_HOME ?= $(HOME)/.ruyi
BIN_DIR = $(RUYI_HOME)/bin
LLVM_SYS_140_PREFIX ?= $(shell brew --prefix llvm@14 2>/dev/null || echo "/usr/local/opt/llvm@14")

# Default target
.DEFAULT_GOAL := help

##@ Build Targets

build-debug: ## Build debug binary (faster compilation, no optimizations)
	cargo build -p ruyic

build-release: ## Build release binary with optimizations
	cargo build --release
	@echo "Build complete: ./target/release/ruyic"

build: build-release ## Build release binary (alias for build-release)

build-runtime: ## Build runtime only (no LLVM required)
	cargo check -p ruyi_runtime --no-default-features

install: build-release ## Build and install ruyic to $(BIN_DIR)
	@mkdir -p $(BIN_DIR)
	cp target/release/ruyic $(BIN_DIR)/
	@echo "ruyic installed to $(BIN_DIR)/ruyic"

##@ Check & Test

check: ## Check workspace without linking (fast)
	cargo check --workspace

check-runtime: ## Check runtime only (no LLVM needed)
	cargo check -p ruyi_runtime --no-default-features

test: ## Run all workspace tests
	cargo test --workspace

test-single: ## Run single test file (usage: make test-single TEST=typechecker)
	@if [ -z "$(TEST)" ]; then \
		echo "Usage: make test-single TEST=<test_name>"; \
		exit 1; \
	fi
	cargo test -p ruyic --test $(TEST)

##@ Code Quality

fmt: ## Format code with rustfmt
	cargo fmt

fmt-check: ## Check code formatting without modifying
	cargo fmt -- --check

lint: ## Run clippy linter
	cargo clippy --workspace

lint-fix: ## Run clippy with auto-fix
	cargo clippy --workspace --fix --allow-dirty

##@ Examples & Compilation

run-example: ## Run an example file (usage: make run-example EXAMPLE=hello)
	@if [ -z "$(EXAMPLE)" ]; then \
		echo "Usage: make run-example EXAMPLE=<example_name>"; \
		exit 1; \
	fi
	@EXAMPLE_FILE=$$(find examples -name "$(EXAMPLE).ry" -not -path '*/target/*' | head -1); \
	if [ -z "$$EXAMPLE_FILE" ]; then \
		echo "Error: $(EXAMPLE).ry not found in examples/"; \
		exit 1; \
	fi; \
	mkdir -p examples/target; \
	./target/release/ruyic $$EXAMPLE_FILE -o examples/target/$(EXAMPLE) && \
	./examples/target/$(EXAMPLE)

compile-example: ## Compile an example to LLVM IR (usage: make compile-example EXAMPLE=hello)
	@if [ -z "$(EXAMPLE)" ]; then \
		echo "Usage: make compile-example EXAMPLE=<example_name>"; \
		exit 1; \
	fi
	@EXAMPLE_FILE=$$(find examples -name "$(EXAMPLE).ry" -not -path '*/target/*' | head -1); \
	if [ -z "$$EXAMPLE_FILE" ]; then \
		echo "Error: $(EXAMPLE).ry not found in examples/"; \
		exit 1; \
	fi; \
	./target/release/ruyic $$EXAMPLE_FILE --emit-llvm

compile-file: ## Compile a .ry file (usage: make compile-file FILE=path/to/file.ry)
	@if [ -z "$(FILE)" ]; then \
		echo "Usage: make compile-file FILE=<path_to_file.ry>"; \
		exit 1; \
	fi
	@if [ ! -f "$(FILE)" ]; then \
		echo "Error: $(FILE) not found"; \
		exit 1; \
	fi
	./target/release/ruyic $(FILE) -o $(FILE:.ry=) && \
	echo "Compiled: $(FILE) -> $(FILE:.ry=)"

##@ Maintenance

clean: ## Clean build artifacts
	cargo clean
	rm -f $(BIN_DIR)/ruyic
	rm -rf examples/target

clean-examples: ## Clean only example build outputs
	rm -rf examples/target

##@ Help

help: ## Display this help message
	@echo "Ruyi Compiler - Makefile Targets"
	@echo "================================"
	@echo ""
	@awk 'BEGIN {FS = ":.*##"; printf ""} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-20s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)
