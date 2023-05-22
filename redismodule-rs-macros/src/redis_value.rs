use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

pub fn redis_value(item: TokenStream) -> TokenStream {
    let struct_input: DeriveInput = parse_macro_input!(item);
    let struct_data = match struct_input.data {
        Data::Struct(s) => s,
        _ => {
            return quote! {compile_error!("RedisValue derive can only be apply on struct.")}.into()
        }
    };

    let struct_name = struct_input.ident;

    let fields = match struct_data.fields {
        Fields::Named(f) => f,
        _ => {
            return quote! {compile_error!("RedisValue derive can only be apply on struct with named fields.")}.into()
        }
    };

    let fields = fields
        .named
        .into_iter()
        .map(|v| {
            let name = v.ident.ok_or("Field without a name is not supported.")?;
            Ok(name)
        })
        .collect::<Result<Vec<_>, &str>>();

    let fields = match fields {
        Ok(f) => f,
        Err(e) => return quote! {compile_error!(#e)}.into(),
    };

    let fields_names: Vec<_> = fields.iter().map(|v| v.to_string()).collect();

    let res = quote! {
        impl From<#struct_name> for redis_module::redisvalue::RedisValue {
            fn from(val: #struct_name) -> redis_module::redisvalue::RedisValue {
                redis_module::redisvalue::RedisValue::OrderedMap(std::collections::BTreeMap::from([
                    #((
                        redis_module::redisvalue::RedisValueKey::String(#fields_names.to_owned()),
                        val.#fields.into()
                    ), )*
                ]))
            }
        }
    };
    res.into()
}
