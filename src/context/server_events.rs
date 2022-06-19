use crate::context::Context;
use crate::raw;
use crate::RedisError;

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
pub struct RoleChangedEventData {
    pub role: ServerRole,
}

#[derive(Clone)]
pub enum ServerEventData {
    RoleChangedEvent(RoleChangedEventData),
    LoadingEvent(LoadingSubevent),
    FlushEvent(FlushSubevent),
}

pub enum ServerEvents {
    RuleChanged,
    Loading,
    Flush,
}

pub type ServerEventCallback = Box<dyn Fn(&Context, ServerEventData)>;

pub struct Subscribers {
    list: Option<Vec<ServerEventCallback>>,
    event_callback: raw::RedisModuleEventCallback,
    event: raw::RedisModuleEvent,
    event_str_rep: &'static str,
}

impl Subscribers {
    fn get_subscribers_list(&self) -> Option<&Vec<ServerEventCallback>> {
        self.list.as_ref()
    }

    fn get_or_create_subscribers_list(
        &mut self,
        ctx: &Context,
    ) -> Result<&mut Vec<ServerEventCallback>, RedisError> {
        if self.get_subscribers_list().is_none() {
            unsafe {
                if raw::RedisModule_SubscribeToServerEvent.unwrap()(
                    ctx.ctx,
                    self.event,
                    self.event_callback,
                ) != raw::REDISMODULE_OK as i32
                {
                    return Err(RedisError::String(format!(
                        "Failed subscribing to server event: '{}'",
                        self.event_str_rep
                    )));
                }
                self.list = Some(Vec::new());
            }
        }
        Ok(self.list.as_mut().unwrap())
    }

    fn subscribe_to_event(
        &mut self,
        ctx: &Context,
        callback: ServerEventCallback,
    ) -> Result<(), RedisError> {
        let subscribers_list = self.get_or_create_subscribers_list(ctx)?;
        subscribers_list.push(callback);
        Ok(())
    }

    fn get_subscribers(&self) -> &Vec<ServerEventCallback> {
        self.get_subscribers_list().unwrap()
    }
}

static mut ROLE_CHANGED_SUBSCRIBERS: Subscribers = Subscribers {
    list: None,
    event_callback: Some(role_changed_callback),
    event: raw::RedisModuleEvent {
        id: raw::REDISMODULE_EVENT_REPLICATION_ROLE_CHANGED,
        dataver: 1,
    },
    event_str_rep: "role_changed",
};

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
    for callback in unsafe { &ROLE_CHANGED_SUBSCRIBERS }.get_subscribers() {
        callback(
            &ctx,
            ServerEventData::RoleChangedEvent(RoleChangedEventData {
                role: new_role.clone(),
            }),
        );
    }
}

fn subscribe_to_role_changed_event(
    ctx: &Context,
    callback: ServerEventCallback,
) -> Result<(), RedisError> {
    unsafe { &mut ROLE_CHANGED_SUBSCRIBERS }.subscribe_to_event(ctx, callback)
}

static mut LOADING_SUBSCRIBERS: Subscribers = Subscribers {
    list: None,
    event_callback: Some(loading_event_callback),
    event: raw::RedisModuleEvent {
        id: raw::REDISMODULE_EVENT_LOADING,
        dataver: 1,
    },
    event_str_rep: "loading",
};

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
    for callback in unsafe { &LOADING_SUBSCRIBERS }.get_subscribers() {
        callback(
            &ctx,
            ServerEventData::LoadingEvent(loading_sub_event.clone()),
        );
    }
}

fn subscribe_to_loading_event(
    ctx: &Context,
    callback: ServerEventCallback,
) -> Result<(), RedisError> {
    unsafe { &mut LOADING_SUBSCRIBERS }.subscribe_to_event(ctx, callback)
}

static mut FLUSH_SUBSCRIBERS: Subscribers = Subscribers {
    list: None,
    event_callback: Some(flush_event_callback),
    event: raw::RedisModuleEvent {
        id: raw::REDISMODULE_EVENT_FLUSHDB,
        dataver: 1,
    },
    event_str_rep: "flush",
};

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
    for callback in unsafe { &FLUSH_SUBSCRIBERS }.get_subscribers() {
        callback(&ctx, ServerEventData::FlushEvent(flush_sub_event.clone()));
    }
}

fn subscribe_to_flush_event(
    ctx: &Context,
    callback: ServerEventCallback,
) -> Result<(), RedisError> {
    unsafe { &mut FLUSH_SUBSCRIBERS }.subscribe_to_event(ctx, callback)
}

pub fn subscribe_to_server_event(
    ctx: &Context,
    event: ServerEvents,
    callback: ServerEventCallback,
) -> Result<(), RedisError> {
    match event {
        ServerEvents::RuleChanged => subscribe_to_role_changed_event(ctx, callback),
        ServerEvents::Loading => subscribe_to_loading_event(ctx, callback),
        ServerEvents::Flush => subscribe_to_flush_event(ctx, callback),
    }
}
