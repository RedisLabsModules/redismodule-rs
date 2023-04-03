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
    let original_func = item.clone();
    let mut original_func = parse_macro_input!(original_func as ItemFn);
    let original_func_name = original_func.sig.ident.clone();
    let original_func_name: Ident = Ident::new(&format!("{}_inner", original_func_name.to_string()), original_func_name.span());
    original_func.sig.ident = original_func_name.clone();
    
    let mut use_self = false;
    let input_names:Vec<Ident> = original_func.sig.inputs.clone().into_iter().filter_map(|v| {
        match v {
            syn::FnArg::Receiver(_) => use_self = true,
            syn::FnArg::Typed(pat_type) => {
                if let syn::Pat::Ident(pat_ident) = *pat_type.pat.clone() {
                    return Some(pat_ident.ident)
                }
            }
        }
        None
    }).collect();
    let func = parse_macro_input!(item as ItemFn);

    let minimum_require_version = args.requested_apis.iter().fold(*API_OLDEST_VERSION, |min_api_version, item|{
        // if we do not have a version mapping, we assume the API exists and return the minimum version.
        let api_version = API_VERSION_MAPPING.get(&item.to_string()).map(|v| *v).unwrap_or(*API_OLDEST_VERSION);
        api_version.max(min_api_version)
    });

    if *API_OLDEST_VERSION == minimum_require_version {
        // all API exists on the older version supported so we can just return the function as is.
        return quote!(#original_func).into();
    }

    let requested_apis = args.requested_apis;
    let requested_apis_str: Vec<String> = requested_apis.iter().map(|e| e.to_string()).collect();
    let vis = func.vis;
    let inner_return_return_type = match func.sig.output.clone() {
        ReturnType::Default => Box::new(Type::Tuple(TypeTuple{paren_token: Paren::default(), elems: Punctuated::new()})),
        ReturnType::Type(_, t) => t,
    };
    let new_return_return_type = Type::Path(syn::parse(quote!(
        crate::apierror::APIResult<#inner_return_return_type>
    ).into()).unwrap());
    let mut sig = func.sig;
    sig.output = ReturnType::Type(RArrow::default(), Box::new(new_return_return_type));

    let original_function_call = if use_self {
        quote!(self.#original_func_name(#(#input_names, )*))
    } else {
        quote!(#original_func_name(#(#input_names, )*))
    };

    let new_func = quote!(
        #original_func

        #vis #sig {
            #(
                unsafe{crate::raw::#requested_apis.ok_or(concat!(#requested_apis_str, " does not exists"))?};
            )*

            Ok(#original_function_call)
        }
    );

    let (all_lower_features, all_upper_features) = get_feature_flags(minimum_require_version);

    let gen = quote! {
        cfg_if::cfg_if! {
            if #[cfg(any(#(#all_lower_features, )*))] {
                #new_func  
            } else if #[cfg(any(#(#all_upper_features, )*))] {
                #original_func
            } else {
                compile_error!("min-redis-compatibility-version is not set correctly")
            }
        }
    };
    gen.into()
}
