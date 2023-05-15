use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use serde::Deserialize;
use serde_syn::{config, from_stream};
use syn::{
    parse,
    parse::{Parse, ParseStream},
    parse_macro_input, ItemFn,
};

#[derive(Debug, Deserialize)]
pub enum RedisCommandFlags {
    /// The command may modify the data set (it may also read from it).
    Write,

    /// The command returns data from keys but never writes.
    ReadOnly,

    /// The command is an administrative command (may change replication or perform similar tasks).
    Admin,

    /// The command may use additional memory and should be denied during out of memory conditions.
    DenyOOM,

    /// Don't allow this command in Lua scripts.
    DenyScript,

    /// Allow this command while the server is loading data. Only commands not interacting with the data set
    /// should be allowed to run in this mode. If not sure don't use this flag.
    AllowLoading,

    /// The command publishes things on Pub/Sub channels.
    PubSub,

    /// The command may have different outputs even starting from the same input arguments and key values.
    /// Starting from Redis 7.0 this flag has been deprecated. Declaring a command as "random" can be done using
    /// command tips, see https://redis.io/topics/command-tips.
    Random,

    /// The command is allowed to run on slaves that don't serve stale data. Don't use if you don't know what
    /// this means.
    AllowStale,

    /// Don't propagate the command on monitor. Use this if the command has sensitive data among the arguments.
    NoMonitor,

    /// Don't log this command in the slowlog. Use this if the command has sensitive data among the arguments.
    NoSlowlog,

    /// The command time complexity is not greater than O(log(N)) where N is the size of the collection or
    /// anything else representing the normal scalability issue with the command.
    Fast,

    /// The command implements the interface to return the arguments that are keys. Used when start/stop/step
    /// is not enough because of the command syntax.
    GetkeysApi,

    /// The command should not register in Redis Cluster since is not designed to work with it because, for
    /// example, is unable to report the position of the keys, programmatically creates key names, or any
    /// other reason.
    NoCluster,

    /// This command can be run by an un-authenticated client. Normally this is used by a command that is used
    /// to authenticate a client.
    NoAuth,

    /// This command may generate replication traffic, even though it's not a write command.
    MayReplicate,

    /// All the keys this command may take are optional
    NoMandatoryKeys,

    /// The command has the potential to block the client.
    Blocking,

    /// Permit the command while the server is blocked either by a script or by a slow module command, see
    /// RM_Yield.
    AllowBusy,

    /// The command implements the interface to return the arguments that are channels.
    GetchannelsApi,
}

impl From<&RedisCommandFlags> for &'static str {
    fn from(value: &RedisCommandFlags) -> Self {
        match value {
            RedisCommandFlags::Write => "write",
            RedisCommandFlags::ReadOnly => "readonly",
            RedisCommandFlags::Admin => "admin",
            RedisCommandFlags::DenyOOM => "deny-oom",
            RedisCommandFlags::DenyScript => "deny-script",
            RedisCommandFlags::AllowLoading => "allow-loading",
            RedisCommandFlags::PubSub => "pubsub",
            RedisCommandFlags::Random => "random",
            RedisCommandFlags::AllowStale => "allow-stale",
            RedisCommandFlags::NoMonitor => "no-monitor",
            RedisCommandFlags::NoSlowlog => "no-slowlog",
            RedisCommandFlags::Fast => "fast",
            RedisCommandFlags::GetkeysApi => "getkeys-api",
            RedisCommandFlags::NoCluster => "no-cluster",
            RedisCommandFlags::NoAuth => "no-auth",
            RedisCommandFlags::MayReplicate => "may-replicate",
            RedisCommandFlags::NoMandatoryKeys => "no-mandatory-keys",
            RedisCommandFlags::Blocking => "blocking",
            RedisCommandFlags::AllowBusy => "allow-busy",
            RedisCommandFlags::GetchannelsApi => "getchannels-api",
        }
    }
}

#[derive(Debug, Deserialize)]
pub enum RedisCommandKeySpecFlags {
    /// Read-Only. Reads the value of the key, but doesn't necessarily return it.
    ReadOnly,

    /// Read-Write. Modifies the data stored in the value of the key or its metadata.
    ReadWrite,

    /// Overwrite. Overwrites the data stored in the value of the key.
    Overwrite,

    /// Deletes the key.
    Remove,

    /// Returns, copies or uses the user data from the value of the key.
    Access,

    /// Updates data to the value, new value may depend on the old value.
    Update,

    /// Adds data to the value with no chance of modification or deletion of existing data.
    Insert,

    /// Explicitly deletes some content from the value of the key.
    Delete,

    /// The key is not actually a key, but should be routed in cluster mode as if it was a key.
    NotKey,

    /// The keyspec might not point out all the keys it should cover.
    Incomplete,

    /// Some keys might have different flags depending on arguments.
    VariableFlags,
}

impl From<&RedisCommandKeySpecFlags> for &'static str {
    fn from(value: &RedisCommandKeySpecFlags) -> Self {
        match value {
            RedisCommandKeySpecFlags::ReadOnly => "READ_ONLY",
            RedisCommandKeySpecFlags::ReadWrite => "READ_WRITE",
            RedisCommandKeySpecFlags::Overwrite => "OVERWRITE",
            RedisCommandKeySpecFlags::Remove => "REMOVE",
            RedisCommandKeySpecFlags::Access => "ACCESS",
            RedisCommandKeySpecFlags::Update => "UPDATE",
            RedisCommandKeySpecFlags::Insert => "INSERT",
            RedisCommandKeySpecFlags::Delete => "DELETE",
            RedisCommandKeySpecFlags::NotKey => "NOT_KEY",
            RedisCommandKeySpecFlags::Incomplete => "INCOMPLETE",
            RedisCommandKeySpecFlags::VariableFlags => "VARIABLE_FLAGS",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct FindKeysRange {
    last_key: i32,
    steps: i32,
    limit: i32,
}

#[derive(Debug, Deserialize)]
pub struct FindKeysNum {
    key_num_idx: i32,
    first_key: i32,
    key_step: i32,
}

#[derive(Debug, Deserialize)]
pub enum FindKeys {
    Range(FindKeysRange),
    Keynum(FindKeysNum),
}

#[derive(Debug, Deserialize)]
pub struct BeginSearchIndex {
    index: i32,
}

#[derive(Debug, Deserialize)]
pub struct BeginSearchKeyword {
    keyword: String,
    startfrom: i32,
}

#[derive(Debug, Deserialize)]
pub enum BeginSearch {
    Index(BeginSearchIndex),
    Keyword(BeginSearchKeyword), // (keyword, startfrom)
}

#[derive(Debug, Deserialize)]
pub struct KeySpecArg {
    notes: Option<String>,
    flags: Vec<RedisCommandKeySpecFlags>,
    begin_search: BeginSearch,
    find_keys: FindKeys,
}

#[derive(Debug, Deserialize)]
struct Args {
    name: Option<String>,
    flags: Vec<RedisCommandFlags>,
    summary: Option<String>,
    complexity: Option<String>,
    since: Option<String>,
    tips: Option<String>,
    arity: i64,
    key_spec: Vec<KeySpecArg>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> parse::Result<Self> {
        from_stream(config::JSONY, &input)
    }
}

fn to_token_stream(s: Option<String>) -> proc_macro2::TokenStream {
    s.map(|v| quote! {Some(#v.to_owned())})
        .unwrap_or(quote! {None})
}

pub(crate) fn redis_command(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as Args);
    let func: ItemFn = match syn::parse(item) {
        Ok(res) => res,
        Err(e) => return e.to_compile_error().into(),
    };

    let original_function_name = func.sig.ident.clone();

    let c_function_name = Ident::new(
        &format!("_inner_{}", func.sig.ident.to_string()),
        func.sig.ident.span(),
    );

    let get_command_info_function_name = Ident::new(
        &format!("_inner_get_command_info_{}", func.sig.ident.to_string()),
        func.sig.ident.span(),
    );

    let name_literal = args
        .name
        .unwrap_or_else(|| original_function_name.to_string());
    let flags_str = args
        .flags
        .into_iter()
        .fold(String::new(), |s, v| {
            format!("{} {}", s, Into::<&'static str>::into(&v))
        })
        .trim()
        .to_owned();
    let flags_literal = quote!(#flags_str);
    let summary_literal = to_token_stream(args.summary);
    let complexity_literal = to_token_stream(args.complexity);
    let since_literal = to_token_stream(args.since);
    let tips_literal = to_token_stream(args.tips);
    let arity_literal = args.arity;
    let key_spec_notes: Vec<_> = args
        .key_spec
        .iter()
        .map(|v| {
            v.notes
                .as_ref()
                .map(|v| quote! {Some(#v.to_owned())})
                .unwrap_or(quote! {None})
        })
        .collect();

    let key_spec_flags: Vec<_> = args
        .key_spec
        .iter()
        .map(|v| {
            let flags: Vec<&'static str> = v.flags.iter().map(|v| v.into()).collect();
            quote! {
                vec![#(redis_module::commands::KeySpecFlags::try_from(#flags)?, )*]
            }
        })
        .collect();

    let key_spec_begin_search: Vec<_> = args
        .key_spec
        .iter()
        .map(|v| match &v.begin_search {
            BeginSearch::Index(i) => {
                let i = i.index;
                quote! {
                    redis_module::commands::BeginSearch::new_index(#i)
                }
            }
            BeginSearch::Keyword(begin_search_keyword) => {
                let k = begin_search_keyword.keyword.as_str();
                let i = begin_search_keyword.startfrom;
                quote! {
                    redis_module::commands::BeginSearch::new_keyword(#k.to_owned(), #i)
                }
            }
        })
        .collect();

    let key_spec_find_keys: Vec<_> = args
        .key_spec
        .iter()
        .map(|v| match &v.find_keys {
            FindKeys::Keynum(find_keys_num) => {
                let keynumidx = find_keys_num.key_num_idx;
                let firstkey = find_keys_num.first_key;
                let keystep = find_keys_num.key_step;
                quote! {
                    redis_module::commands::FindKeys::new_keys_num(#keynumidx, #firstkey, #keystep)
                }
            }
            FindKeys::Range(find_keys_range) => {
                let last_key = find_keys_range.last_key;
                let steps = find_keys_range.steps;
                let limit = find_keys_range.limit;
                quote! {
                    redis_module::commands::FindKeys::new_range(#last_key, #steps, #limit)
                }
            }
        })
        .collect();

    let gen = quote! {
        #func

        extern "C" fn #c_function_name(
            ctx: *mut redis_module::raw::RedisModuleCtx,
            argv: *mut *mut redis_module::raw::RedisModuleString,
            argc: i32,
        ) -> i32 {
            let context = redis_module::Context::new(ctx);

            let args = redis_module::decode_args(ctx, argv, argc);
            let response = #original_function_name(&context, args);
            context.reply(response) as i32
        }

        #[linkme::distributed_slice(redis_module::commands::COMMNADS_LIST)]
        fn #get_command_info_function_name() -> Result<redis_module::commands::CommandInfo, redis_module::RedisError> {
            let key_spec = vec![
                #(
                    redis_module::commands::KeySpec::new(
                        #key_spec_notes,
                        #key_spec_flags.into(),
                        #key_spec_begin_search,
                        #key_spec_find_keys,
                    ),
                )*
            ];
            Ok(redis_module::commands::CommandInfo::new(
                #name_literal.to_owned(),
                Some(#flags_literal.to_owned()),
                #summary_literal,
                #complexity_literal,
                #since_literal,
                #tips_literal,
                #arity_literal,
                key_spec,
                #c_function_name,
            ))
        }
    };
    gen.into()
}
