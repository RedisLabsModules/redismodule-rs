use crate::raw;
use crate::{context::Context, RedisError};
use linkme::distributed_slice;

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum ServerRole {
    Primary,
    Replica,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum LoadingSubevent {
    RdbStarted,
    AofStarted,
    ReplStarted,
    Ended,
    Failed,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum FlushSubevent {
    Started,
    Ended,
}

#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum ModuleChangeSubevent {
    Loaded,
    Unloaded,
}

#[derive(Clone)]
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
    ROLE_CHANGED_SERVER_EVENTS_LIST.iter().for_each(|callback| {
        callback(&ctx, new_role);
    });
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
    LOADING_SERVER_EVENTS_LIST.iter().for_each(|callback| {
        callback(&ctx, loading_sub_event);
    });
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
    FLUSH_SERVER_EVENTS_LIST.iter().for_each(|callback| {
        callback(&ctx, flush_sub_event);
    });
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
    MODULE_CHANGED_SERVER_EVENTS_LIST
        .iter()
        .for_each(|callback| {
            callback(&ctx, module_changed_sub_event);
        });
}

fn register_single_server_event_type<T>(
    ctx: &Context,
    callbacks: &[fn(&Context, T)],
    server_event: u64,
    inner_callback: raw::RedisModuleEventCallback,
) -> Result<(), RedisError> {
    if !callbacks.is_empty() {
        let res = unsafe {
            raw::RedisModule_SubscribeToServerEvent.unwrap()(
                ctx.ctx,
                raw::RedisModuleEvent {
                    id: server_event,
                    dataver: 1,
                },
                inner_callback,
            )
        };
        if res != raw::REDISMODULE_OK as i32 {
            return Err(RedisError::Str("Failed subscribing to server event"));
        }
    }

    Ok(())
}

pub fn register_server_events(ctx: &Context) -> Result<(), RedisError> {
    register_single_server_event_type(
        ctx,
        &ROLE_CHANGED_SERVER_EVENTS_LIST,
        raw::REDISMODULE_EVENT_REPLICATION_ROLE_CHANGED,
        Some(role_changed_callback),
    )?;
    register_single_server_event_type(
        ctx,
        &LOADING_SERVER_EVENTS_LIST,
        raw::REDISMODULE_EVENT_LOADING,
        Some(loading_event_callback),
    )?;
    register_single_server_event_type(
        ctx,
        &FLUSH_SERVER_EVENTS_LIST,
        raw::REDISMODULE_EVENT_FLUSHDB,
        Some(flush_event_callback),
    )?;
    register_single_server_event_type(
        ctx,
        &MODULE_CHANGED_SERVER_EVENTS_LIST,
        raw::REDISMODULE_EVENT_MODULE_CHANGE,
        Some(module_change_event_callback),
    )?;
    Ok(())
}
