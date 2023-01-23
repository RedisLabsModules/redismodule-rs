use crate::raw;
use crate::{context::Context, RedisError};
use linkme::distributed_slice;

#[derive(Clone)]
pub enum ServerRole {
    Primary,
    Replica,
}

#[derive(Clone)]
pub enum LoadingSubevent {
    RdbStarted,
    AofStarted,
    ReplStarted,
    Ended,
    Failed,
}

#[derive(Clone)]
pub enum FlushSubevent {
    Started,
    Ended,
}

#[derive(Clone)]
pub enum ModuleChangeSubevent {
    Loaded,
    Unloaded,
}

pub enum ServerEventHandler {
    RuleChanged(fn(&Context, ServerRole)),
    Loading(fn(&Context, LoadingSubevent)),
    Flush(fn(&Context, FlushSubevent)),
    ModuleChange(fn(&Context, ModuleChangeSubevent)),
}

#[distributed_slice()]
pub static ROLE_CHANGED_SERVER_EVENTS_LIST: [fn(&Context, ServerRole)] = [..];

#[distributed_slice()]
pub static LOADING_SERVER_EVENTS_LIST: [fn(&Context, LoadingSubevent)] = [..];

#[distributed_slice()]
pub static FLUSH_SERVER_EVENTS_LIST: [fn(&Context, FlushSubevent)] = [..];

#[distributed_slice()]
pub static MODULE_CHANGED_SERVER_EVENTS_LIST: [fn(&Context, ModuleChangeSubevent)] = [..];

extern "C" fn role_changed_callback(
    ctx: *mut raw::RedisModuleCtx,
    _eid: raw::RedisModuleEvent,
    subevent: u64,
    _data: *mut ::std::os::raw::c_void,
) {
    let new_role = if subevent == raw::REDISMODULE_EVENT_REPLROLECHANGED_NOW_MASTER {
        ServerRole::Primary
    } else {
        ServerRole::Replica
    };
    let ctx = Context::new(ctx);
    for callback in ROLE_CHANGED_SERVER_EVENTS_LIST.iter() {
        callback(&ctx, new_role.clone());
    }
}

extern "C" fn loading_event_callback(
    ctx: *mut raw::RedisModuleCtx,
    _eid: raw::RedisModuleEvent,
    subevent: u64,
    _data: *mut ::std::os::raw::c_void,
) {
    let loading_sub_event = match subevent {
        raw::REDISMODULE_SUBEVENT_LOADING_RDB_START => LoadingSubevent::RdbStarted,
        raw::REDISMODULE_SUBEVENT_LOADING_REPL_START => LoadingSubevent::ReplStarted,
        raw::REDISMODULE_SUBEVENT_LOADING_ENDED => LoadingSubevent::Ended,
        _ => LoadingSubevent::Failed,
    };
    let ctx = Context::new(ctx);
    for callback in LOADING_SERVER_EVENTS_LIST.iter() {
        callback(&ctx, loading_sub_event.clone());
    }
}

extern "C" fn flush_event_callback(
    ctx: *mut raw::RedisModuleCtx,
    _eid: raw::RedisModuleEvent,
    subevent: u64,
    _data: *mut ::std::os::raw::c_void,
) {
    let flush_sub_event = if subevent == raw::REDISMODULE_SUBEVENT_FLUSHDB_START {
        FlushSubevent::Started
    } else {
        FlushSubevent::Ended
    };
    let ctx = Context::new(ctx);
    for callback in FLUSH_SERVER_EVENTS_LIST.iter() {
        callback(&ctx, flush_sub_event.clone());
    }
}

extern "C" fn module_change_event_callback(
    ctx: *mut raw::RedisModuleCtx,
    _eid: raw::RedisModuleEvent,
    subevent: u64,
    _data: *mut ::std::os::raw::c_void,
) {
    let module_changed_sub_event = if subevent == raw::REDISMODULE_SUBEVENT_MODULE_LOADED {
        ModuleChangeSubevent::Loaded
    } else {
        ModuleChangeSubevent::Unloaded
    };
    let ctx = Context::new(ctx);
    for callback in MODULE_CHANGED_SERVER_EVENTS_LIST.iter() {
        callback(&ctx, module_changed_sub_event.clone());
    }
}

pub fn register_server_events(ctx: &Context) -> Result<(), RedisError> {
    if !ROLE_CHANGED_SERVER_EVENTS_LIST.is_empty() {
        let res = unsafe {
            raw::RedisModule_SubscribeToServerEvent.unwrap()(
                ctx.ctx,
                raw::RedisModuleEvent {
                    id: raw::REDISMODULE_EVENT_REPLICATION_ROLE_CHANGED,
                    dataver: 1,
                },
                Some(role_changed_callback),
            )
        };
        if res != raw::REDISMODULE_OK as i32 {
            return Err(RedisError::Str(
                "Failed subscribing to role changed server event",
            ));
        }
    }

    if !LOADING_SERVER_EVENTS_LIST.is_empty() {
        let res = unsafe {
            raw::RedisModule_SubscribeToServerEvent.unwrap()(
                ctx.ctx,
                raw::RedisModuleEvent {
                    id: raw::REDISMODULE_EVENT_LOADING,
                    dataver: 1,
                },
                Some(loading_event_callback),
            )
        };
        if res != raw::REDISMODULE_OK as i32 {
            return Err(RedisError::Str(
                "Failed subscribing to loading server event",
            ));
        }
    }

    if !FLUSH_SERVER_EVENTS_LIST.is_empty() {
        let res = unsafe {
            raw::RedisModule_SubscribeToServerEvent.unwrap()(
                ctx.ctx,
                raw::RedisModuleEvent {
                    id: raw::REDISMODULE_EVENT_FLUSHDB,
                    dataver: 1,
                },
                Some(flush_event_callback),
            )
        };
        if res != raw::REDISMODULE_OK as i32 {
            return Err(RedisError::Str("Failed subscribing to flush server event"));
        }
    }

    if !MODULE_CHANGED_SERVER_EVENTS_LIST.is_empty() {
        let res = unsafe {
            raw::RedisModule_SubscribeToServerEvent.unwrap()(
                ctx.ctx,
                raw::RedisModuleEvent {
                    id: raw::REDISMODULE_EVENT_MODULE_CHANGE,
                    dataver: 1,
                },
                Some(module_change_event_callback),
            )
        };
        if res != raw::REDISMODULE_OK as i32 {
            return Err(RedisError::Str(
                "Failed subscribing to module changed server event",
            ));
        }
    }

    Ok(())
}
