use std::sync::{
    atomic::{AtomicBool, AtomicI64},
    Mutex,
};

use lazy_static::lazy_static;
use redis_module::{
    configuration::{ConfigurationContext, ConfigurationFlags},
    enum_configuration, redis_module, ConfigurationValue, Context, RedisGILGuard, RedisString,
    RedisValue, RedisValueResult,
};

enum_configuration! {
    enum EnumConfiguration {
        Val1 = 1,
        Val2 = 2,
    }
}

lazy_static! {
    static ref NUM_OF_CONFIGURATION_CHANGES: RedisGILGuard<i64> = RedisGILGuard::default();
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
    static ref CONFIGURATION_MUTEX_ENUM: Mutex<EnumConfiguration> =
        Mutex::new(EnumConfiguration::Val1);
}

fn on_configuration_changed<G, T: ConfigurationValue<G>>(
    config_ctx: &ConfigurationContext,
    _name: &str,
    _val: &'static T,
) {
    let mut val = NUM_OF_CONFIGURATION_CHANGES.lock(config_ctx);
    *val += 1
}

fn num_changes(ctx: &Context, _: Vec<RedisString>) -> RedisValueResult {
    let val = NUM_OF_CONFIGURATION_CHANGES.lock(ctx);
    Ok(RedisValue::Integer(*val))
}

//////////////////////////////////////////////////////

redis_module! {
    name: "configuration",
    version: 1,
    allocator: (redis_module::alloc::RedisAlloc, redis_module::alloc::RedisAlloc),
    data_types: [],
    commands: [
        ["configuration.num_changes", num_changes, "", 0, 0, 0],
    ],
    configurations: [
        i64: [
            ["i64", &*CONFIGURATION_I64, 10, 0, 1000, ConfigurationFlags::DEFAULT, Some(Box::new(on_configuration_changed))],
            ["atomic_i64", &*CONFIGURATION_ATOMIC_I64, 10, 0, 1000, ConfigurationFlags::DEFAULT, Some(Box::new(on_configuration_changed))],
        ],
        string: [
            ["redis_string", &*CONFIGURATION_REDIS_STRING, "default", ConfigurationFlags::DEFAULT, Some(Box::new(on_configuration_changed))],
            ["string", &*CONFIGURATION_STRING, "default", ConfigurationFlags::DEFAULT, Some(Box::new(on_configuration_changed::<String, _>))],
            ["mutex_string", &*CONFIGURATION_MUTEX_STRING, "default", ConfigurationFlags::DEFAULT, Some(Box::new(on_configuration_changed::<String, _>))],
        ],
        bool: [
            ["atomic_bool", &*CONFIGURATION_ATOMIC_BOOL, true, ConfigurationFlags::DEFAULT, Some(Box::new(on_configuration_changed))],
            ["bool", &*CONFIGURATION_BOOL, true, ConfigurationFlags::DEFAULT, Some(Box::new(on_configuration_changed))],
        ],
        enum: [
            ["enum", &*CONFIGURATION_ENUM, EnumConfiguration::Val1, ConfigurationFlags::DEFAULT, Some(Box::new(on_configuration_changed))],
            ["enum_mutex", &*CONFIGURATION_MUTEX_ENUM, EnumConfiguration::Val1, ConfigurationFlags::DEFAULT, Some(Box::new(on_configuration_changed))],
        ],
        module_args_as_configuration: true,
    ]
}
