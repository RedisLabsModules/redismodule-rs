use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

mod command;

/// This proc macro allow to specify that the follow function is a Redis command.
/// The macro accept the following arguments that discribe the command properties:
/// * name (optional) - The command name. in case not given, the function name will be taken.
/// * flags (optional) - Command flags such as `readonly`, for the full list please refer to https://redis.io/docs/reference/modules/modules-api-ref/#redismodule_createcommand
/// * summary (optional) - Command summary
/// * complexity (optional) - Command compexity
/// * since (optional) - At which module version the command was first introduce
/// * tips (optional) - Command tips for proxy, for more information please refer to https://redis.io/topics/command-tips
/// * arity - Number of arguments, including the command name itself. A positive number specifies an exact number of arguments and a negative number
///   specifies a minimum number of arguments.
/// * key_spec - A list of specs representing how to find the keys that the command might touch. the following options are available:
///    * notes (optional) - Some note about the key spec.
///    * flags - List of flags reprenting how the keys are accessed, the following options are available:
///       * Readonly - Read-Only. Reads the value of the key, but doesn't necessarily return it.
///       * ReadWrite - Read-Write. Modifies the data stored in the value of the key or its metadata.
///       * Overwrite - Overwrite. Overwrites the data stored in the value of the key.
///       * Remove - Deletes the key.
///       * Access - Returns, copies or uses the user data from the value of the key.
///       * Update - Updates data to the value, new value may depend on the old value.
///       * Insert - Adds data to the value with no chance of modification or deletion of existing data.
///       * Delete - Explicitly deletes some content from the value of the key.
///       * NotKey - The key is not actually a key, but should be routed in cluster mode as if it was a key.
///       * Incomplete - The keyspec might not point out all the keys it should cover.
///       * VariableFlags - Some keys might have different flags depending on arguments.
///    * begin_search - Represents how Redis should start looking for keys.
///      There are 2 possible options:
///       * Index - start looking for keys from a given position.
///       * Keyword - Search for a specific keyward and start looking for keys from this keyword
///    * FindKeys - After Redis finds the location from where it needs to start looking for keys,
///      Redis will start finding keys base on the information in this struct.
///      There are 2 possible options:
///       * Range - A tuple represent a range of `(last_key, steps, limit)`.
///          * last_key - Index of the last key relative to the result of the
///            begin search step. Can be negative, in which case it's not
///            relative. -1 indicates the last argument, -2 one before the
///            last and so on.
///          * steps - How many arguments should we skip after finding a
///            key, in order to find the next one.
///          * limit - If `lastkey` is -1, we use `limit` to stop the search
///            by a factor. 0 and 1 mean no limit. 2 means 1/2 of the
///            remaining args, 3 means 1/3, and so on.
///       * Keynum -  A tuple of 3 elements `(keynumidx, firstkey, keystep)`.
///          * keynumidx - Index of the argument containing the number of
///            keys to come, relative to the result of the begin search step.
///          * firstkey - Index of the fist key relative to the result of the
///            begin search step. (Usually it's just after `keynumidx`, in
///            which case it should be set to `keynumidx + 1`.)
///          * keystep - How many arguments should we skip after finding a
///            key, in order to find the next one?
///
/// Example:
/// The following example will register a command called `foo`.
/// ```rust,no_run,ignore
/// #[command(
/// {
///    name: "foo",
///    arity: 3,
///    key_spec: [
///        {
///            notes: "some notes",
///            flags: ["RW", "ACCESS"],
///            begin_search: Keyword(("foo", 1)),
///            find_keys: Range((1, 2, 3)),
///        }
///    ]
/// }
/// )]
/// fn test_command(_ctx: &Context, _args: Vec<RedisString>) -> RedisResult {
///     Ok(RedisValue::SimpleStringStatic("OK"))
/// }
/// ```
///
/// **Notice**, by default Redis does not validate the command spec. User should validate the command keys on the module command code. The command spec is used for validation on cluster so Redis can raise a cross slot error when needed.
#[proc_macro_attribute]
pub fn command(attr: TokenStream, item: TokenStream) -> TokenStream {
    command::redis_command(attr, item)
}

#[proc_macro_attribute]
pub fn role_changed_event_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = match syn::parse(item) {
        Ok(res) => res,
        Err(e) => return e.to_compile_error().into(),
    };
    let gen = quote! {
        #[linkme::distributed_slice(redis_module::server_events::ROLE_CHANGED_SERVER_EVENTS_LIST)]
        #ast
    };
    gen.into()
}

#[proc_macro_attribute]
pub fn loading_event_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = match syn::parse(item) {
        Ok(res) => res,
        Err(e) => return e.to_compile_error().into(),
    };
    let gen = quote! {
        #[linkme::distributed_slice(redis_module::server_events::LOADING_SERVER_EVENTS_LIST)]
        #ast
    };
    gen.into()
}

#[proc_macro_attribute]
pub fn flush_event_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = match syn::parse(item) {
        Ok(res) => res,
        Err(e) => return e.to_compile_error().into(),
    };
    let gen = quote! {
        #[linkme::distributed_slice(redis_module::server_events::FLUSH_SERVER_EVENTS_LIST)]
        #ast
    };
    gen.into()
}

#[proc_macro_attribute]
pub fn module_changed_event_handler(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let ast: ItemFn = match syn::parse(item) {
        Ok(res) => res,
        Err(e) => return e.to_compile_error().into(),
    };
    let gen = quote! {
        #[linkme::distributed_slice(redis_module::server_events::MODULE_CHANGED_SERVER_EVENTS_LIST)]
        #ast
    };
    gen.into()
}
