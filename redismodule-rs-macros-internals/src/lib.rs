mod api_versions;

use api_versions::{get_feature_flags, API_OLDEST_VERSION, API_VERSION_MAPPING};
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::{self, Paren, RArrow};
use syn::Ident;
use syn::ItemFn;
use syn::{self, bracketed, parse_macro_input, ReturnType, Token, Type, TypeTuple};

#[derive(Debug)]
struct Args {
    requested_apis: Vec<Ident>,
    function: ItemFn,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let content;
        let _paren_token: token::Bracket = bracketed!(content in input);
        let vars: Punctuated<Ident, Token![,]> = content.parse_terminated(Ident::parse)?;
        input.parse::<Token![,]>()?;
        let function: ItemFn = input.parse()?;
        Ok(Args {
            requested_apis: vars.into_iter().collect(),
            function,
        })
    }
}

/// This proc macro allows specifying which RedisModuleAPI is required by some redismodue-rs
/// function. The macro finds, for a given set of RedisModuleAPI, what the minimal Redis version is
/// that contains all those APIs and decides whether or not the function might raise an [APIError].
///
/// In addition, for each RedisModuleAPI, the proc macro injects a code that extracts the actual
/// API function pointer and raises an error or panics if the API is invalid.
///
/// # Panics
///
/// Panics when an API is not available and if the function doesn't return [`Result`]. If it does
/// return a [`Result`], the panics are replaced with returning a [`Result::Err`].
///
/// # Examples
///
/// Creating a wrapper for the [`RedisModule_AddPostNotificationJob`]
/// ```rust,no_run,ignore
///    redismodule_api!(
///         [RedisModule_AddPostNotificationJob],
///         pub fn add_post_notification_job<F: Fn(&Context)>(&self, callback: F) -> Status {
///             let callback = Box::into_raw(Box::new(callback));
///             unsafe {
///                 RedisModule_AddPostNotificationJob(
///                     self.ctx,
///                     Some(post_notification_job::<F>),
///                     callback as *mut c_void,
///                     Some(post_notification_job_free_callback::<F>),
///                 )
///             }
///             .into()
///         }
///     );
/// ```
#[proc_macro]
pub fn api(item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(item as Args);
    let minimum_require_version =
        args.requested_apis
            .iter()
            .fold(*API_OLDEST_VERSION, |min_api_version, item| {
                // if we do not have a version mapping, we assume the API exists and return the minimum version.
                let api_version = API_VERSION_MAPPING
                    .get(&item.to_string())
                    .map(|v| *v)
                    .unwrap_or(*API_OLDEST_VERSION);
                api_version.max(min_api_version)
            });

    let requested_apis = args.requested_apis;
    let requested_apis_str: Vec<String> = requested_apis.iter().map(|e| e.to_string()).collect();

    let original_func = args.function;
    let original_func_attr = original_func.attrs;
    let original_func_code = original_func.block;
    let original_func_sig = original_func.sig;
    let original_func_vis = original_func.vis;

    let inner_return_return_type = match original_func_sig.output.clone() {
        ReturnType::Default => Box::new(Type::Tuple(TypeTuple {
            paren_token: Paren::default(),
            elems: Punctuated::new(),
        })),
        ReturnType::Type(_, t) => t,
    };
    let new_return_return_type = Type::Path(
        syn::parse(
            quote!(
                crate::apierror::APIResult<#inner_return_return_type>
            )
            .into(),
        )
        .unwrap(),
    );

    let mut new_func_sig = original_func_sig.clone();
    new_func_sig.output = ReturnType::Type(RArrow::default(), Box::new(new_return_return_type));

    let old_ver_func = quote!(
        #(#original_func_attr)*
        #original_func_vis #new_func_sig {
            #(
                #[allow(non_snake_case)]
                let #requested_apis = unsafe{crate::raw::#requested_apis.ok_or(concat!(#requested_apis_str, " does not exists"))?};
            )*
            let __callback__ = move || -> #inner_return_return_type {
                #original_func_code
            };
            Ok(__callback__())
        }
    );

    let new_ver_func = quote!(
        #(#original_func_attr)*
        #original_func_vis #original_func_sig {
            #(
                #[allow(non_snake_case)]
                let #requested_apis = unsafe{crate::raw::#requested_apis.unwrap()};
            )*
            let __callback__ = move || -> #inner_return_return_type {
                #original_func_code
            };
            __callback__()
        }
    );

    let (all_lower_features, all_upper_features) = get_feature_flags(minimum_require_version);

    let gen = quote! {
        cfg_if::cfg_if! {
            if #[cfg(any(#(#all_lower_features, )*))] {
                #old_ver_func
            } else if #[cfg(any(#(#all_upper_features, )*))] {
                #new_ver_func
            } else {
                compile_error!("Cannot generate the api! macro code. The \"min-redis-compatibility-version\" feature is not set up correctly")
            }
        }
    };
    gen.into()
}
