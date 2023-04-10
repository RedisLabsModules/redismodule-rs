extern crate proc_macro;

mod api_versions;

use proc_macro::{TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseStream, Result};
use syn::punctuated::Punctuated;
use syn::token::{RArrow, Paren};
use syn::{self, parse_macro_input, Token, Type, ReturnType, TypeTuple};
use syn::ItemFn;
use syn::Ident;
use api_versions::{API_VERSION_MAPPING, API_OLDEST_VERSION, get_feature_flags};

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

#[derive(Debug)]
struct Args{
    requested_apis: Vec<Ident>
}

impl Parse for Args{
    fn parse(input: ParseStream) -> Result<Self> {
        // parses a,b,c, or a,b,c where a,b and c are Indent
        let vars = Punctuated::<Ident, Token![,]>::parse_terminated(input)?;
        Ok(Args {
            requested_apis: vars.into_iter().collect(),
        })
    }
}

#[proc_macro_attribute]
pub fn redismodule_api(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as Args);
    let minimum_require_version = args.requested_apis.iter().fold(*API_OLDEST_VERSION, |min_api_version, item|{
        // if we do not have a version mapping, we assume the API exists and return the minimum version.
        let api_version = API_VERSION_MAPPING.get(&item.to_string()).map(|v| *v).unwrap_or(*API_OLDEST_VERSION);
        api_version.max(min_api_version)
    });

    let requested_apis = args.requested_apis;
    let requested_apis_str: Vec<String> = requested_apis.iter().map(|e| e.to_string()).collect();

    let original_func = parse_macro_input!(item as ItemFn);
    let original_func_code = original_func.block;
    let original_func_sig = original_func.sig;
    let original_func_vis = original_func.vis;

    let inner_return_return_type = match original_func_sig.output.clone() {
        ReturnType::Default => Box::new(Type::Tuple(TypeTuple{paren_token: Paren::default(), elems: Punctuated::new()})),
        ReturnType::Type(_, t) => t,
    };
    let new_return_return_type = Type::Path(syn::parse(quote!(
        crate::apierror::APIResult<#inner_return_return_type>
    ).into()).unwrap());

    let mut new_func_sig = original_func_sig.clone();
    new_func_sig.output = ReturnType::Type(RArrow::default(), Box::new(new_return_return_type));

    let old_ver_func = quote!(
        #original_func_vis #new_func_sig {
            #(  
                #[allow(non_snake_case)]
                let #requested_apis = unsafe{crate::raw::#requested_apis.ok_or(concat!(#requested_apis_str, " does not exists"))?};
            )*
            let __callback__ = || {
                #original_func_code
            };
            Ok(__callback__())
        }
    );

    let new_ver_func = quote!(
        #original_func_vis #original_func_sig {
            #(
                #[allow(non_snake_case)]
                let #requested_apis = unsafe{crate::raw::#requested_apis.unwrap()};
            )*
            let __callback__ = || {
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
                compile_error!("min-redis-compatibility-version is not set correctly")
            }
        }
    };
    gen.into()
}
