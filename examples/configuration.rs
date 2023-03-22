#[macro_use]
extern crate redis_module;

use std::sync::{
    atomic::{AtomicBool, AtomicI64},
    Mutex,
};

use lazy_static::lazy_static;
use redis_module::{
    configuration::ConfigurationFlags, EnumConfigurationValue, RedisGILGuard, RedisString,
};

enum_configuration! {
    enum EnumConfiguration {
        Val1 = 1,
        Val2 = 2,
    }
}

lazy_static! {
    static ref CONFIGURATION_I64: RedisGILGuard<i64> = RedisGILGuard::default();
    static ref CONFIGURATION_ATOMIC_I64: AtomicI64 = AtomicI64::new(1);
    static ref CONFIGURATION_REDIS_STRING: RedisGILGuard<RedisString> =
        RedisGILGuard::new(RedisString::create(None, "default"));
    static ref CONFIGURATION_STRING: RedisGILGuard<String> = RedisGILGuard::new("default".into());
    static ref CONFIGURATION_MUTEX_STRING: Mutex<String> = Mutex::new("default".into());
    static ref CONFIGURATION_ATOMIC_BOOL: AtomicBool = AtomicBool::default();
    static ref CONFIGURATION_BOOL: RedisGILGuard<bool> = RedisGILGuard::default();
    static ref CONFIGURATION_ENUM: RedisGILGuard<EnumConfiguration> =
        RedisGILGuard::new(EnumConfiguration::Val1);
}

//////////////////////////////////////////////////////

redis_module! {
    name: "configuration",
    version: 1,
    data_types: [],
    commands: [],
    configurations: [
        i64: [
            ["i64", &*CONFIGURATION_I64, 10, 0, 1000, ConfigurationFlags::DEFAULT],
            ["atomic_i64", &*CONFIGURATION_ATOMIC_I64, 10, 0, 1000, ConfigurationFlags::DEFAULT],
        ],
        string: [
            ["redis_string", &*CONFIGURATION_REDIS_STRING, "default", ConfigurationFlags::DEFAULT],
            ["string", &*CONFIGURATION_STRING, "default", ConfigurationFlags::DEFAULT],
            ["mutex_string", &*CONFIGURATION_MUTEX_STRING, "default", ConfigurationFlags::DEFAULT],
        ],
        bool: [
            ["atomic_bool", &*CONFIGURATION_ATOMIC_BOOL, true, ConfigurationFlags::DEFAULT],
            ["bool", &*CONFIGURATION_BOOL, true, ConfigurationFlags::DEFAULT],
        ],
        enum: [
            ["enum", &*CONFIGURATION_ENUM, EnumConfiguration::Val1, ConfigurationFlags::DEFAULT],
        ],
    ]
}
