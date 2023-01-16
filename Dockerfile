# BUILD redisfab/redismodule-rs:${VERSION}-${ARCH}-${OSNICK}

ARG REDIS_VER=6.2.7

# bullseye|bionic|xenial|centos8|centos7
ARG OSNICK=bullseye

# ARCH=x64|arm64v8|arm32v7
ARG ARCH=x64

ARG TEST=0

#----------------------------------------------------------------------------------------------
FROM redisfab/redis:${REDIS_VER}-${ARCH}-${OSNICK} AS redis
FROM debian:bullseye-slim AS builder

ARG OSNICK
ARG OS
ARG ARCH
ARG REDIS_VER
ARG TEST

RUN if [ -f /root/.profile ]; then sed -ie 's/mesg n/tty -s \&\& mesg -n/g' /root/.profile; fi
SHELL ["/bin/bash", "-l", "-c"]

RUN echo "Building for ${OSNICK} (${OS}) for ${ARCH} [with Redis ${REDIS_VER}]"
 
ADD . /build
WORKDIR /build

RUN ./sbin/setup
RUN make info
RUN make

RUN set -ex ;\
    if [ "$TEST" = "1" ]; then TEST= make test; fi

#----------------------------------------------------------------------------------------------
FROM redisfab/redis:${REDIS_VER}-${ARCH}-${OSNICK}

ARG REDIS_VER

ENV LIBDIR /usr/lib/redis/modules
WORKDIR /data
RUN mkdir -p "$LIBDIR"

COPY --from=builder /build/bin/artifacts/ /var/opt/redislabs/artifacts

EXPOSE 6379
CMD ["redis-server"]
