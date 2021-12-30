# BUILD redisfab/redismodule-rs:${VERSION}-${ARCH}-${OSNICK}

ARG REDIS_VER=6.2.5

# bullseye|bionic|xenial|centos8|centos7
ARG OSNICK=bullseye

# ARCH=x64|arm64v8|arm32v7
ARG ARCH=x64

ARG PACK=0
ARG TEST=0

#----------------------------------------------------------------------------------------------
FROM redisfab/redis:${REDIS_VER}-${ARCH}-${OSNICK} AS builder

ARG OSNICK
ARG OS
ARG ARCH
ARG REDIS_VER
ARG PACK
ARG TEST

RUN echo "Building for ${OSNICK} (${OS}) for ${ARCH} [with Redis ${REDIS_VER}]"
 
ADD ./ /build
WORKDIR /build

RUN ./deps/readies/bin/getpy3
RUN ./sbin/system-setup.py
RUN bash -l -c "make info"
RUN bash -l -c make

RUN set -ex ;\
    if [ "$TEST" = "1" ]; then bash -l -c "TEST= make test"; fi
RUN set -ex ;\
    mkdir -p bin/artifacts ;\
    if [ "$PACK" = "1" ]; then bash -l -c "make pack"; fi

#----------------------------------------------------------------------------------------------
FROM redisfab/redis:${REDIS_VER}-${ARCH}-${OSNICK}

ARG REDIS_VER

ENV LIBDIR /usr/lib/redis/modules
WORKDIR /data
RUN mkdir -p "$LIBDIR"

COPY --from=builder /build/bin/artifacts/ /var/opt/redislabs/artifacts

EXPOSE 6379
CMD ["redis-server"]
