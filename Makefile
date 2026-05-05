.PHONY: build-release install clean check test fmt lint

RUYI_HOME ?= $(HOME)/.ruyi
BIN_DIR = $(RUYI_HOME)/bin

build-release:
	cargo build --release
	@mkdir -p $(BIN_DIR)
	cp target/release/ruyic $(BIN_DIR)/
	@echo "ruyic installed to $(BIN_DIR)/ruyic"

install: build-release

clean:
	cargo clean
	rm -f $(BIN_DIR)/ruyic

check:
	cargo check --workspace

test:
	cargo test --workspace

fmt:
	cargo fmt

lint:
	cargo clippy --workspace
