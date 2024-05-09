# Define variables
TARGET := xtools
INSTALL_PATH := /usr/local/bin

# Default target
.PHONY: all
all: build

# Build target
.PHONY: build
build:
	cargo build --release

# Install target
.PHONY: install
install: build
	@mkdir -p $(INSTALL_PATH)
	@cp target/release/$(TARGET) $(INSTALL_PATH)

# Clean target
.PHONY: clean
clean:
	cargo clean
