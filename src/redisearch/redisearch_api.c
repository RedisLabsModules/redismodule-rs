#include "redismodule.h"
#include "redisearch_api.h"

// We write a wrapper function since the actual implementation is a macro,
// which we cannot call from Rust.
int Wrap_RediSearch_Initialize() {
    return RediSearch_Initialize();
}