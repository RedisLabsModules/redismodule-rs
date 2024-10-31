use crate::raw;
use crate::Context;
use crate::RedisError;
use crate::Status;
use bitflags::bitflags;
use libc::c_char;
use linkme::distributed_slice;
use redis_module_macros_internals::api;
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
        match value.to_lowercase().as_str() {
            "read_only" => Ok(KeySpecFlags::READ_ONLY),
            "read_write" => Ok(KeySpecFlags::READ_WRITE),
            "overwrite" => Ok(KeySpecFlags::OVERWRITE),
            "remove" => Ok(KeySpecFlags::REMOVE),
            "access" => Ok(KeySpecFlags::ACCESS),
            "update" => Ok(KeySpecFlags::UPDATE),
            "insert" => Ok(KeySpecFlags::INSERT),
            "delete" => Ok(KeySpecFlags::DELETE),
            "not_key" => Ok(KeySpecFlags::NOT_KEY),
            "incomplete" => Ok(KeySpecFlags::INCOMPLETE),
            "variable_flags" => Ok(KeySpecFlags::VARIABLE_FLAGS),
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

/// A version of begin search spec that finds the index
/// indicating where to start search for keys based on
/// an index.
pub struct BeginSearchIndex {
    index: i32,
}

/// A version of begin search spec that finds the index
/// indicating where to start search for keys based on
/// a keyword.
pub struct BeginSearchKeyword {
    keyword: String,
    startfrom: i32,
}

/// This struct represents how Redis should start looking for keys.
/// There are 2 possible options:
/// 1. Index - start looking for keys from a given position.
/// 2. Keyword - Search for a specific keyward and start looking for keys from this keyword
pub enum BeginSearch {
    Index(BeginSearchIndex),
    Keyword(BeginSearchKeyword),
}

impl BeginSearch {
    pub fn new_index(index: i32) -> BeginSearch {
        BeginSearch::Index(BeginSearchIndex { index })
    }

    pub fn new_keyword(keyword: String, startfrom: i32) -> BeginSearch {
        BeginSearch::Keyword(BeginSearchKeyword { keyword, startfrom })
    }
}

impl From<&BeginSearch>
    for (
        raw::RedisModuleKeySpecBeginSearchType,
        raw::RedisModuleCommandKeySpec__bindgen_ty_1,
    )
{
    fn from(value: &BeginSearch) -> Self {
        match value {
            BeginSearch::Index(index_spec) => (
                raw::RedisModuleKeySpecBeginSearchType_REDISMODULE_KSPEC_BS_INDEX,
                raw::RedisModuleCommandKeySpec__bindgen_ty_1 {
                    index: raw::RedisModuleCommandKeySpec__bindgen_ty_1__bindgen_ty_1 {
                        pos: index_spec.index,
                    },
                },
            ),
            BeginSearch::Keyword(keyword_spec) => {
                let keyword = CString::new(keyword_spec.keyword.as_str())
                    .unwrap()
                    .into_raw();
                (
                    raw::RedisModuleKeySpecBeginSearchType_REDISMODULE_KSPEC_BS_KEYWORD,
                    raw::RedisModuleCommandKeySpec__bindgen_ty_1 {
                        keyword: raw::RedisModuleCommandKeySpec__bindgen_ty_1__bindgen_ty_2 {
                            keyword,
                            startfrom: keyword_spec.startfrom,
                        },
                    },
                )
            }
        }
    }
}

/// A version of find keys base on range.
/// * `last_key` - Index of the last key relative to the result of the
///   begin search step. Can be negative, in which case it's not
///   relative. -1 indicates the last argument, -2 one before the
///   last and so on.
/// * `steps` - How many arguments should we skip after finding a
///   key, in order to find the next one.
/// * `limit` - If `lastkey` is -1, we use `limit` to stop the search
///   by a factor. 0 and 1 mean no limit. 2 means 1/2 of the
///   remaining args, 3 means 1/3, and so on.
pub struct FindKeysRange {
    last_key: i32,
    steps: i32,
    limit: i32,
}

/// A version of find keys base on some argument representing the number of keys
/// * keynumidx - Index of the argument containing the number of
///   keys to come, relative to the result of the begin search step.
/// * firstkey - Index of the fist key relative to the result of the
///   begin search step. (Usually it's just after `keynumidx`, in
///   which case it should be set to `keynumidx + 1`.)
/// * keystep - How many arguments should we skip after finding a
///   key, in order to find the next one?
pub struct FindKeysNum {
    key_num_idx: i32,
    first_key: i32,
    key_step: i32,
}

/// After Redis finds the location from where it needs to start looking for keys,
/// Redis will start finding keys base on the information in this enum.
/// There are 2 possible options:
/// 1. Range -   Required to specify additional 3 more values, `last_key`, `steps`, and `limit`.
/// 2. Keynum -  Required to specify additional 3 more values, `keynumidx`, `firstkey`, and `keystep`.
///              Redis will consider the argument at `keynumidx` as an indicator
///              to the number of keys that will follow. Then it will start
///              from `firstkey` and jump each `keystep` to find the keys.
pub enum FindKeys {
    Range(FindKeysRange),
    Keynum(FindKeysNum),
}

impl FindKeys {
    pub fn new_range(last_key: i32, steps: i32, limit: i32) -> FindKeys {
        FindKeys::Range(FindKeysRange {
            last_key,
            steps,
            limit,
        })
    }

    pub fn new_keys_num(key_num_idx: i32, first_key: i32, key_step: i32) -> FindKeys {
        FindKeys::Keynum(FindKeysNum {
            key_num_idx,
            first_key,
            key_step,
        })
    }
}

impl From<&FindKeys>
    for (
        raw::RedisModuleKeySpecFindKeysType,
        raw::RedisModuleCommandKeySpec__bindgen_ty_2,
    )
{
    fn from(value: &FindKeys) -> Self {
        match value {
            FindKeys::Range(range_spec) => (
                raw::RedisModuleKeySpecFindKeysType_REDISMODULE_KSPEC_FK_RANGE,
                raw::RedisModuleCommandKeySpec__bindgen_ty_2 {
                    range: raw::RedisModuleCommandKeySpec__bindgen_ty_2__bindgen_ty_1 {
                        lastkey: range_spec.last_key,
                        keystep: range_spec.steps,
                        limit: range_spec.limit,
                    },
                },
            ),
            FindKeys::Keynum(keynum_spec) => (
                raw::RedisModuleKeySpecFindKeysType_REDISMODULE_KSPEC_FK_KEYNUM,
                raw::RedisModuleCommandKeySpec__bindgen_ty_2 {
                    keynum: raw::RedisModuleCommandKeySpec__bindgen_ty_2__bindgen_ty_2 {
                        keynumidx: keynum_spec.key_num_idx,
                        firstkey: keynum_spec.first_key,
                        keystep: keynum_spec.key_step,
                    },
                },
            ),
        }
    }
}

/// A struct that specify how to find keys from a command.
/// It is devided into 2 parts:
/// 1. begin_search - indicate how to find the first command argument from where to start searching for keys.
/// 2. find_keys - the methose to use in order to find the keys.
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
}

impl From<&KeySpec> for raw::RedisModuleCommandKeySpec {
    fn from(value: &KeySpec) -> Self {
        let (begin_search_type, bs) = (&value.begin_search).into();
        let (find_keys_type, fk) = (&value.find_keys).into();
        raw::RedisModuleCommandKeySpec {
            notes: value
                .notes
                .as_ref()
                .map(|v| CString::new(v.as_str()).unwrap().into_raw())
                .unwrap_or(ptr::null_mut()),
            flags: value.flags.bits() as u64,
            begin_search_type,
            bs,
            find_keys_type,
            fk,
        }
    }
}

type CommandCallback =
    extern "C" fn(*mut raw::RedisModuleCtx, *mut *mut raw::RedisModuleString, i32) -> i32;

/// A struct represent a CommandInfo
pub struct CommandInfo {
    name: String,
    flags: Option<String>,
    enterprise_flags: Option<String>,
    summary: Option<String>,
    complexity: Option<String>,
    since: Option<String>,
    tips: Option<String>,
    arity: i64,
    key_spec: Vec<KeySpec>,
    callback: CommandCallback,
}

impl CommandInfo {
    pub fn new(
        name: String,
        flags: Option<String>,
        enterprise_flags: Option<String>,
        summary: Option<String>,
        complexity: Option<String>,
        since: Option<String>,
        tips: Option<String>,
        arity: i64,
        key_spec: Vec<KeySpec>,
        callback: CommandCallback,
    ) -> CommandInfo {
        CommandInfo {
            name,
            flags,
            enterprise_flags,
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
pub static COMMANDS_LIST: [fn() -> Result<CommandInfo, RedisError>] = [..];

pub fn get_redis_key_spec(key_spec: Vec<KeySpec>) -> Vec<raw::RedisModuleCommandKeySpec> {
    let mut redis_key_spec: Vec<raw::RedisModuleCommandKeySpec> =
        key_spec.into_iter().map(|v| (&v).into()).collect();
    let zerod: raw::RedisModuleCommandKeySpec = unsafe { MaybeUninit::zeroed().assume_init() };
    redis_key_spec.push(zerod);
    redis_key_spec
}

api! {[
        RedisModule_CreateCommand,
        RedisModule_GetCommand,
        RedisModule_SetCommandInfo,
    ],
    /// Register all the commands located on `COMMNADS_LIST`.
    fn register_commands_internal(ctx: &Context) -> Result<(), RedisError> {
        let is_enterprise = ctx.is_enterprise();
        COMMANDS_LIST.iter().try_for_each(|command| {
            let command_info = command()?;
            let name: CString = CString::new(command_info.name.as_str()).unwrap();
            let mut flags = command_info.flags.as_deref().unwrap_or("").to_owned();
            if is_enterprise {
                flags = format!("{flags} {}", command_info.enterprise_flags.as_deref().unwrap_or("")).trim().to_owned();
            }
            let flags = CString::new(flags).map_err(|e| RedisError::String(e.to_string()))?;

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
                .map(|v| Some(CString::new(v.as_str()).unwrap()))
                .unwrap_or(None);
            let complexity = command_info
                .complexity
                .as_ref()
                .map(|v| Some(CString::new(v.as_str()).unwrap()))
                .unwrap_or(None);
            let since = command_info
                .since
                .as_ref()
                .map(|v| Some(CString::new(v.as_str()).unwrap()))
                .unwrap_or(None);
            let tips = command_info
                .tips
                .as_ref()
                .map(|v| Some(CString::new(v.as_str()).unwrap()))
                .unwrap_or(None);

            let key_specs = get_redis_key_spec(command_info.key_spec);

            let mut redis_command_info = raw::RedisModuleCommandInfo {
                version: &COMMNAD_INFO_VERSION,
                summary: summary.as_ref().map(|v| v.as_ptr()).unwrap_or(ptr::null_mut()),
                complexity: complexity.as_ref().map(|v| v.as_ptr()).unwrap_or(ptr::null_mut()),
                since: since.as_ref().map(|v| v.as_ptr()).unwrap_or(ptr::null_mut()),
                history: ptr::null_mut(), // currently we will not support history
                tips: tips.as_ref().map(|v| v.as_ptr()).unwrap_or(ptr::null_mut()),
                arity: command_info.arity as c_int,
                key_specs: key_specs.as_ptr() as *mut raw::RedisModuleCommandKeySpec,
                args: ptr::null_mut(),
            };

            if unsafe { RedisModule_SetCommandInfo(command, &mut redis_command_info as *mut raw::RedisModuleCommandInfo) } == raw::Status::Err as i32 {
                return Err(RedisError::String(format!(
                    "Failed setting info for command {}.",
                    command_info.name
                )));
            }

            // the only CString pointers which are not freed are those of the key_specs, lets free them here.
            key_specs.into_iter().for_each(|v|{
                if !v.notes.is_null() {
                    drop(unsafe{CString::from_raw(v.notes as *mut c_char)});
                }
                if v.begin_search_type == raw::RedisModuleKeySpecBeginSearchType_REDISMODULE_KSPEC_BS_KEYWORD {
                    let keyword = unsafe{v.bs.keyword.keyword};
                    if !keyword.is_null() {
                        drop(unsafe{CString::from_raw(v.bs.keyword.keyword as *mut c_char)});
                    }
                }
            });

            Ok(())
        })
    }
}

#[cfg(all(
    any(
        feature = "min-redis-compatibility-version-7-4",
        feature = "min-redis-compatibility-version-7-2",
        feature = "min-redis-compatibility-version-7-0"
    ),
    not(any(
        feature = "min-redis-compatibility-version-6-2",
        feature = "min-redis-compatibility-version-6-0"
    ))
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

#[cfg(all(
    any(
        feature = "min-redis-compatibility-version-6-2",
        feature = "min-redis-compatibility-version-6-0"
    ),
    not(any(
        feature = "min-redis-compatibility-version-7-4",
        feature = "min-redis-compatibility-version-7-2",
        feature = "min-redis-compatibility-version-7-0"
    ))
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
