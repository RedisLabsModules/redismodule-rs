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
  PACK=1             # create packages
  ARTIFACTS=1        # copy artifacts from docker image
  PUBLISH=1          # publish (i.e. docker push) after build


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
	cargo build --features experimental-api,test --all --all-targets  $(CARGO_FLAGS)
	# cp $(TARGET_DIR)/librejson.$(RUST_SOEXT.$(OS)) $(TARGET)

clean:
ifneq ($(ALL),1)
	cargo clean
else
	rm -rf target
endif

.PHONY: build clean

#----------------------------------------------------------------------------------------------

test: cargo_test

cargo_test:
	cargo test --all-targets --features test,experimental-api $(CARGO_FLAGS)

.PHONY: test cargo_test

#----------------------------------------------------------------------------------------------

docker:
	@make -C build/docker build
ifeq ($(PUBLISH),1)
	@make -C build/docker publish
endif

info:
	gcc --version
	cmake --version
	clang --version
	rustc --version
	cargo --version
	rustup --version
	rustup show

.PHONY: docker info
