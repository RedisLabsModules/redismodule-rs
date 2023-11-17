ROOT=.

include $(ROOT)/deps/readies/mk/main

#----------------------------------------------------------------------------------------------

define HELPTEXT
make build
  DEBUG=1          # build debug variant
make clean         # remove binary files
  ALL=1            # remove binary directories

make all           # build all libraries and packages

make test          # run tests

make docker        # build for specific Linux distribution
  OSNICK=nick        # Linux distribution to build for
  REDIS_VER=ver      # use Redis version `ver`
  TEST=1             # test aftar build


endef

#----------------------------------------------------------------------------------------------

MK_CUSTOM_CLEAN=1
BINDIR=$(BINROOT)

include $(MK)/defs
include $(MK)/rules

#----------------------------------------------------------------------------------------------

MODULE_NAME=redismodule-rs.so

ifeq ($(DEBUG),1)
TARGET_DIR=target/debug
else
CARGO_FLAGS += --release
TARGET_DIR=target/release
endif

TARGET=$(TARGET_DIR)/$(MODULE_NAME)

#----------------------------------------------------------------------------------------------

lint:
	cargo fmt -- --check

.PHONY: lint

#----------------------------------------------------------------------------------------------

RUST_SOEXT.linux=so
RUST_SOEXT.freebsd=so
RUST_SOEXT.macos=dylib

build:
	cargo build --all --all-targets --no-default-features $(CARGO_FLAGS)
	# cp $(TARGET_DIR)/librejson.$(RUST_SOEXT.$(OS)) $(TARGET)

clean:
ifneq ($(ALL),1)
	cargo clean
else
	rm -rf target
endif

.PHONY: build clean

#----------------------------------------------------------------------------------------------

test: cargo_test cargo_deny

cargo_deny:
	cargo install cargo-deny
	cargo deny check licenses
	cargo deny check bans

cargo_test:
	cargo test --workspace --no-default-features $(CARGO_FLAGS)
	cargo test --doc --workspace --no-default-features $(CARGO_FLAGS)

.PHONY: test cargo_deny cargo_test

#----------------------------------------------------------------------------------------------

docker:
	@make -C build/docker build

info:
	gcc --version
	cmake --version
	clang --version
	rustc --version
	cargo --version
	rustup --version
	rustup show

.PHONY: docker info
