CC ?= cc
CFLAGS ?= -O2 -Wall -Wextra -Iinclude
TARGET_DIR ?= $(if $(CARGO_TARGET_DIR),$(CARGO_TARGET_DIR),target)
RELEASE_DIR := $(TARGET_DIR)/release

.PHONY: build-release gen-header c-smoke-shared c-smoke-static c-smoke

build-release:
	cargo build --release

gen-header:
	cbindgen --config cbindgen.toml --crate wfb_rs --output include/wfb_rs.h

c-smoke-shared: build-release
	$(CC) $(CFLAGS) -o examples_c/smoke_shared examples_c/smoke.c -L$(RELEASE_DIR) -lwfb_rs

c-smoke-static: build-release
	$(CC) $(CFLAGS) -o examples_c/smoke_static examples_c/smoke.c $(RELEASE_DIR)/libwfb_rs.a -lpcap -ldl -lpthread -lm

c-smoke: c-smoke-shared c-smoke-static
