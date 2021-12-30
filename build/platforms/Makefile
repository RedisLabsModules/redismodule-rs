
ROOT=../..
MK.pyver:=3
include $(ROOT)/deps/readies/mk/main

# OSNICK=bullseye|bionic|xenial|centos8|centos7
OSNICK ?= bullseye

REPO=redismodule-rs

REDIS_VER=6.2.5

INT_BRANCHES=

ART_DIR=$(ROOT)/bin/artifacts
ART_INT_DIR=/var/opt/redislabs/artifacts

include $(MK)/docker.native
