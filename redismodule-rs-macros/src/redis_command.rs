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
pub enum FindKeys {
    Range((i32, i32, i32)),  // (last_key, steps, limit)
    Keynum((i32, i32, i32)), // (keynumidx, firstkey, keystep)
}

#[derive(Debug, Deserialize)]
pub enum BeginSearch {
    Index(i32),
    Keyword((String, i32)), // (keyword, startfrom)
}

#[derive(Debug, Deserialize)]
pub struct KeySpecArg {
    notes: Option<String>,
    flags: Vec<String>,
    begin_search: BeginSearch,
    find_keys: FindKeys,
}

#[derive(Debug, Deserialize)]
struct Args {
    name: String,
    flags: Option<String>,
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

    let name_literal = args.name;
    let flags_literal = to_token_stream(args.flags);
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
            let flags = &v.flags;
            quote! {
                vec![#(redis_module::commnads::KeySpecFlags::try_from(#flags)?, )*]
            }
        })
        .collect();

    let key_spec_begin_search: Vec<_> = args
        .key_spec
        .iter()
        .map(|v| match &v.begin_search {
            BeginSearch::Index(i) => {
                quote! {
                    redis_module::commnads::BeginSearch::Index(#i)
                }
            }
            BeginSearch::Keyword((k, i)) => {
                quote! {
                    redis_module::commnads::BeginSearch::Keyword((#k.to_owned(), #i))
                }
            }
        })
        .collect();

    let key_spec_find_keys: Vec<_> = args
        .key_spec
        .iter()
        .map(|v| match &v.find_keys {
            FindKeys::Keynum((keynumidx, firstkey, keystep)) => {
                quote! {
                    redis_module::commnads::FindKeys::Keynum((#keynumidx, #firstkey, #keystep))
                }
            }
            FindKeys::Range((last_key, steps, limit)) => {
                quote! {
                    redis_module::commnads::FindKeys::Range((#last_key, #steps, #limit))
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

        #[linkme::distributed_slice(redis_module::commnads::COMMNADS_LIST)]
        fn #get_command_info_function_name() -> Result<redis_module::commnads::CommandInfo, redis_module::RedisError> {
            let key_spec = vec![
                #(
                    redis_module::commnads::KeySpec::new(
                        #key_spec_notes,
                        #key_spec_flags.into(),
                        #key_spec_begin_search,
                        #key_spec_find_keys,
                    ),
                )*
            ];
            Ok(redis_module::commnads::CommandInfo::new(
                #name_literal.to_owned(),
                #flags_literal,
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
