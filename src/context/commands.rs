use crate::raw;
use crate::Context;
use crate::RedisError;
use crate::Status;
use bitflags::bitflags;
use linkme::distributed_slice;
use redis_module_macros_internals::redismodule_api;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::os::raw::c_int;
use std::ptr;

const COMMNAD_INFO_VERSION: raw::RedisModuleCommandInfoVersion =
    raw::RedisModuleCommandInfoVersion {
        version: 1,
        sizeof_historyentry: std::mem::size_of::<raw::RedisModuleCommandHistoryEntry>(),
        sizeof_keyspec: std::mem::size_of::<raw::RedisModuleCommandKeySpec>(),
        sizeof_arg: std::mem::size_of::<raw::RedisModuleCommandArg>(),
    };

bitflags! {
    /// Key spec flags
    ///
    /// The first four refer to what the command actually does with the value or
    /// metadata of the key, and not necessarily the user data or how it affects
    /// it. Each key-spec must have exactly one of these. Any operation
    /// that's not distinctly deletion, overwrite or read-only would be marked as
    /// RW.
    ///
    /// The next four refer to user data inside the value of the key, not the
    /// metadata like LRU, type, cardinality. It refers to the logical operation
    /// on the user's data (actual input strings or TTL), being
    /// used/returned/copied/changed. It doesn't refer to modification or
    /// returning of metadata (like type, count, presence of data). ACCESS can be
    /// combined with one of the write operations INSERT, DELETE or UPDATE. Any
    /// write that's not an INSERT or a DELETE would be UPDATE.
    pub struct KeySpecFlags : u32 {
        /// Read-Only. Reads the value of the key, but doesn't necessarily return it.
        const READ_ONLY = raw::REDISMODULE_CMD_KEY_RO;

        /// Read-Write. Modifies the data stored in the value of the key or its metadata.
        const READ_WRITE = raw::REDISMODULE_CMD_KEY_RW;

        /// Overwrite. Overwrites the data stored in the value of the key.
        const OVERWRITE = raw::REDISMODULE_CMD_KEY_OW;

        /// Deletes the key.
        const REMOVE = raw::REDISMODULE_CMD_KEY_RM;

        /// Returns, copies or uses the user data from the value of the key.
        const ACCESS = raw::REDISMODULE_CMD_KEY_ACCESS;

        /// Updates data to the value, new value may depend on the old value.
        const UPDATE = raw::REDISMODULE_CMD_KEY_UPDATE;

        /// Adds data to the value with no chance of modification or deletion of existing data.
        const INSERT = raw::REDISMODULE_CMD_KEY_INSERT;

        /// Explicitly deletes some content from the value of the key.
        const DELETE = raw::REDISMODULE_CMD_KEY_DELETE;

        /// The key is not actually a key, but should be routed in cluster mode as if it was a key.
        const NOT_KEY = raw::REDISMODULE_CMD_KEY_NOT_KEY;

        /// The keyspec might not point out all the keys it should cover.
        const INCOMPLETE = raw::REDISMODULE_CMD_KEY_INCOMPLETE;

        /// Some keys might have different flags depending on arguments.
        const VARIABLE_FLAGS = raw::REDISMODULE_CMD_KEY_VARIABLE_FLAGS;
    }
}

impl TryFrom<&str> for KeySpecFlags {
    type Error = RedisError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "READ_ONLY" => Ok(KeySpecFlags::READ_ONLY),
            "READ_WRITE" => Ok(KeySpecFlags::READ_WRITE),
            "OVERWRITE" => Ok(KeySpecFlags::OVERWRITE),
            "REMOVE" => Ok(KeySpecFlags::REMOVE),
            "ACCESS" => Ok(KeySpecFlags::ACCESS),
            "UPDATE" => Ok(KeySpecFlags::UPDATE),
            "INSERT" => Ok(KeySpecFlags::INSERT),
            "DELETE" => Ok(KeySpecFlags::DELETE),
            "NOT_KEY" => Ok(KeySpecFlags::NOT_KEY),
            "INCOMPLETE" => Ok(KeySpecFlags::INCOMPLETE),
            "VARIABLE_FLAGS" => Ok(KeySpecFlags::VARIABLE_FLAGS),
            _ => Err(RedisError::String(format!(
                "Value {value} is not a valid key spec flag."
            ))),
        }
    }
}

impl From<Vec<KeySpecFlags>> for KeySpecFlags {
    fn from(value: Vec<KeySpecFlags>) -> Self {
        value
            .into_iter()
            .fold(KeySpecFlags::empty(), |a, item| a | item)
    }
}

/// This struct represents how Redis should start looking for keys.
/// There are 2 possible options:
/// 1. Index - start looking for keys from a given position.
/// 2. Keyword - Search for a specific keyward and start looking for keys from this keyword
pub enum BeginSearch {
    Index(i32),
    Keyword((String, i32)), // (keyword, startfrom)
}

impl BeginSearch {
    fn get_redis_begin_search(
        &self,
    ) -> (
        raw::RedisModuleKeySpecBeginSearchType,
        raw::RedisModuleCommandKeySpec__bindgen_ty_1,
    ) {
        match self {
            BeginSearch::Index(i) => (
                raw::RedisModuleKeySpecBeginSearchType_REDISMODULE_KSPEC_BS_INDEX,
                raw::RedisModuleCommandKeySpec__bindgen_ty_1 {
                    index: raw::RedisModuleCommandKeySpec__bindgen_ty_1__bindgen_ty_1 { pos: *i },
                },
            ),
            BeginSearch::Keyword((k, i)) => {
                let keyword = CString::new(k.as_str()).unwrap().into_raw();
                (
                    raw::RedisModuleKeySpecBeginSearchType_REDISMODULE_KSPEC_BS_KEYWORD,
                    raw::RedisModuleCommandKeySpec__bindgen_ty_1 {
                        keyword: raw::RedisModuleCommandKeySpec__bindgen_ty_1__bindgen_ty_2 {
                            keyword,
                            startfrom: *i,
                        },
                    },
                )
            }
        }
    }
}

/// After Redis finds the location from where it needs to start looking for keys,
/// Redis will start finding keys base on the information in this struct.
/// There are 2 possible options:
/// 1. Range - A tuple represent a range of `(last_key, steps, limit)`.
/// 2. Keynum -  A tuple of 3 elements `(keynumidx, firstkey, keystep)`.
///              Redis will consider the argument at `keynumidx` as an indicator
///              to the number of keys that will follow. Then it will start
///              from `firstkey` and jump each `keystep` to find the keys.
pub enum FindKeys {
    Range((i32, i32, i32)),  // (last_key, steps, limit)
    Keynum((i32, i32, i32)), // (keynumidx, firstkey, keystep)
}

impl FindKeys {
    fn get_redis_find_keys(
        &self,
    ) -> (
        raw::RedisModuleKeySpecFindKeysType,
        raw::RedisModuleCommandKeySpec__bindgen_ty_2,
    ) {
        match self {
            FindKeys::Range((lastkey, keystep, limit)) => (
                raw::RedisModuleKeySpecFindKeysType_REDISMODULE_KSPEC_FK_RANGE,
                raw::RedisModuleCommandKeySpec__bindgen_ty_2 {
                    range: raw::RedisModuleCommandKeySpec__bindgen_ty_2__bindgen_ty_1 {
                        lastkey: *lastkey,
                        keystep: *keystep,
                        limit: *limit,
                    },
                },
            ),
            FindKeys::Keynum((keynumidx, firstkey, keystep)) => (
                raw::RedisModuleKeySpecFindKeysType_REDISMODULE_KSPEC_FK_KEYNUM,
                raw::RedisModuleCommandKeySpec__bindgen_ty_2 {
                    keynum: raw::RedisModuleCommandKeySpec__bindgen_ty_2__bindgen_ty_2 {
                        keynumidx: *keynumidx,
                        firstkey: *firstkey,
                        keystep: *keystep,
                    },
                },
            ),
        }
    }
}

pub struct KeySpec {
    notes: Option<String>,
    flags: KeySpecFlags,
    begin_search: BeginSearch,
    find_keys: FindKeys,
}

impl KeySpec {
    pub fn new(
        notes: Option<String>,
        flags: KeySpecFlags,
        begin_search: BeginSearch,
        find_keys: FindKeys,
    ) -> KeySpec {
        KeySpec {
            notes,
            flags,
            begin_search,
            find_keys,
        }
    }
    fn get_redis_key_spec(&self) -> raw::RedisModuleCommandKeySpec {
        let (begin_search_type, bs) = self.begin_search.get_redis_begin_search();
        let (find_keys_type, fk) = self.find_keys.get_redis_find_keys();
        raw::RedisModuleCommandKeySpec {
            notes: self
                .notes
                .as_ref()
                .map(|v| CString::new(v.as_str()).unwrap().into_raw())
                .unwrap_or(ptr::null_mut()),
            flags: self.flags.bits() as u64,
            begin_search_type,
            bs,
            find_keys_type,
            fk,
        }
    }
}

type CommnadCallback =
    extern "C" fn(*mut raw::RedisModuleCtx, *mut *mut raw::RedisModuleString, i32) -> i32;

/// A struct represent a CommandInfo
pub struct CommandInfo {
    name: String,
    flags: Option<String>,
    summary: Option<String>,
    complexity: Option<String>,
    since: Option<String>,
    tips: Option<String>,
    arity: i64,
    key_spec: Vec<KeySpec>,
    callback: CommnadCallback,
}

impl CommandInfo {
    pub fn new(
        name: String,
        flags: Option<String>,
        summary: Option<String>,
        complexity: Option<String>,
        since: Option<String>,
        tips: Option<String>,
        arity: i64,
        key_spec: Vec<KeySpec>,
        callback: CommnadCallback,
    ) -> CommandInfo {
        CommandInfo {
            name,
            flags,
            summary,
            complexity,
            since,
            tips,
            arity,
            key_spec,
            callback,
        }
    }
}

#[distributed_slice()]
pub static COMMNADS_LIST: [fn() -> Result<CommandInfo, RedisError>] = [..];

pub fn get_redis_key_spec(key_spec: Vec<KeySpec>) -> *mut raw::RedisModuleCommandKeySpec {
    let mut redis_key_spec: Vec<raw::RedisModuleCommandKeySpec> = key_spec
        .into_iter()
        .map(|v| v.get_redis_key_spec())
        .collect();
    let zerod: raw::RedisModuleCommandKeySpec = unsafe { MaybeUninit::zeroed().assume_init() };
    redis_key_spec.push(zerod);
    let res = redis_key_spec.as_ptr();
    std::mem::forget(redis_key_spec);
    res as *mut raw::RedisModuleCommandKeySpec
}

redismodule_api! {[
        RedisModule_CreateCommand,
        RedisModule_GetCommand,
        RedisModule_SetCommandInfo,
    ],
    /// Register all the commands located on `COMMNADS_LIST`.
    fn register_commands_internal(ctx: &Context) -> Result<(), RedisError> {
        COMMNADS_LIST.iter().try_for_each(|command| {
            let command_info = command()?;
            let name: CString = CString::new(command_info.name.as_str()).unwrap();
            let flags = CString::new(
                command_info
                    .flags
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or(""),
            )
            .unwrap();

            if unsafe {
                RedisModule_CreateCommand(
                    ctx.ctx,
                    name.as_ptr(),
                    Some(command_info.callback),
                    flags.as_ptr(),
                    0,
                    0,
                    0,
                )
            } == raw::Status::Err as i32
            {
                return Err(RedisError::String(format!(
                    "Failed register command {}.",
                    command_info.name
                )));
            }

            // Register the extra data of the command
            let command = unsafe { RedisModule_GetCommand(ctx.ctx, name.as_ptr()) };

            if command.is_null() {
                return Err(RedisError::String(format!(
                    "Failed finding command {} after registration.",
                    command_info.name
                )));
            }

            let summary = command_info
                .summary
                .as_ref()
                .map(|v| CString::new(v.as_str()).unwrap().into_raw())
                .unwrap_or(ptr::null_mut());
            let complexity = command_info
                .complexity
                .as_ref()
                .map(|v| CString::new(v.as_str()).unwrap().into_raw())
                .unwrap_or(ptr::null_mut());
            let since = command_info
                .since
                .as_ref()
                .map(|v| CString::new(v.as_str()).unwrap().into_raw())
                .unwrap_or(ptr::null_mut());
            let tips = command_info
                .tips
                .as_ref()
                .map(|v| CString::new(v.as_str()).unwrap().into_raw())
                .unwrap_or(ptr::null_mut());

            let redis_command_info = Box::into_raw(Box::new(raw::RedisModuleCommandInfo {
                version: &COMMNAD_INFO_VERSION,
                summary,
                complexity,
                since,
                history: ptr::null_mut(), // currently we will not support history
                tips,
                arity: command_info.arity as c_int,
                key_specs: get_redis_key_spec(command_info.key_spec),
                args: ptr::null_mut(),
            }));

            if unsafe { RedisModule_SetCommandInfo(command, redis_command_info) } == raw::Status::Err as i32 {
                return Err(RedisError::String(format!(
                    "Failed setting info for command {}.",
                    command_info.name
                )));
            }

            Ok(())
        })
    }
}

#[cfg(any(
    feature = "min-redis-compatibility-version-7-2",
    feature = "min-redis-compatibility-version-7-0"
))]
pub fn register_commands(ctx: &Context) -> Status {
    register_commands_internal(ctx).map_or_else(
        |e| {
            ctx.log_warning(&e.to_string());
            Status::Err
        },
        |_| Status::Ok,
    )
}

#[cfg(any(
    feature = "min-redis-compatibility-version-6-2",
    feature = "min-redis-compatibility-version-6-0"
))]
pub fn register_commands(ctx: &Context) -> Status {
    register_commands_internal(ctx).map_or_else(
        |e| {
            ctx.log_warning(&e.to_string());
            Status::Err
        },
        |v| {
            v.map_or_else(
                |e| {
                    ctx.log_warning(&e.to_string());
                    Status::Err
                },
                |_| Status::Ok,
            )
        },
    )
}
