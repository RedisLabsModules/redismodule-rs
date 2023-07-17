use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Data, DataStruct, DeriveInput, Fields};

mod supported_maps {
    /// A list of supported maps which can be converted to a dictionary for
    /// the [`redis_module::InfoContext`].
    const ALL: [&str; 2] = ["BTreeMap", "HashMap"];

    /// Returns `true` if the `type_string` provided is a type of a map
    /// supported by the [`crate::InfoSection`].
    pub fn is_supported(type_string: &str) -> bool {
        ALL.iter().any(|m| type_string.contains(&m.to_lowercase()))
    }
}

/// Generate a [`From`] implementation for this struct so that it is
/// possible to generate a [`redis_module::InfoContext`] information
/// from it.
///
/// A struct is compatible to be used with [`crate::InfoSection`] when
/// it has fields, whose types are convertible to
/// [`redis_module::InfoContextBuilderFieldTopLevelValue`] and (for
/// the dictionaries) if it has fields which are compatible maps of
/// objects, where a key is a [`String`] and a value is any type
/// convertible to
/// [`redis_module::InfoContextBuilderFieldBottomLevelValue`].
fn struct_info_section(struct_name: Ident, struct_data: DataStruct) -> TokenStream {
    let fields = match struct_data.fields {
        Fields::Named(f) => f,
        _ => {
            return quote! {compile_error!("The InfoSection can only be derived for structs with named fields.")}.into()
        }
    };

    let fields = fields
        .named
        .into_iter()
        .map(|v| {
            let is_dictionary = supported_maps::is_supported(
                &v.ty.clone().into_token_stream().to_string().to_lowercase(),
            );
            let name = v.ident.ok_or(
                "Structs with unnamed fields are not supported by the InfoSection.".to_owned(),
            )?;
            Ok((is_dictionary, name))
        })
        .collect::<Result<Vec<_>, String>>();

    let section_key_fields: Vec<_> = match fields {
        Ok(ref f) => f.iter().filter(|i| !i.0).map(|i| i.1.clone()).collect(),
        Err(e) => return quote! {compile_error!(#e)}.into(),
    };

    let section_dictionary_fields: Vec<_> = match fields {
        Ok(f) => f.iter().filter(|i| i.0).map(|i| i.1.clone()).collect(),
        Err(e) => return quote! {compile_error!(#e)}.into(),
    };

    let key_fields_names: Vec<_> = section_key_fields.iter().map(|v| v.to_string()).collect();

    let dictionary_fields_names: Vec<_> = section_dictionary_fields
        .iter()
        .map(|v| v.to_string())
        .collect();

    quote! {
        impl From<#struct_name> for redis_module::OneInfoSectionData {
            fn from(val: #struct_name) -> redis_module::OneInfoSectionData {
                let section_name = stringify!(#struct_name).to_owned();

                let fields = vec![
                    // The section's key => value pairs.
                    #((
                        #key_fields_names.to_owned(),
                        redis_module::InfoContextBuilderFieldTopLevelValue::from(val.#section_key_fields)
                    ), )*

                    // The dictionaries within this section.
                    #((
                        #dictionary_fields_names.to_owned(),
                        redis_module::InfoContextBuilderFieldTopLevelValue::Dictionary {
                            name: #dictionary_fields_names.to_owned(),
                            fields: redis_module::InfoContextFieldBottomLevelData(
                                val.#section_dictionary_fields
                                .into_iter()
                                .map(|d| d.into())
                                .collect()),
                        }
                    ), )*
                ];
                (section_name, fields)
            }
        }
    }
    .into()
}

/// Implementation for the [`crate::info_section`] derive macro.
/// Runs the relevant code generation base on the element
/// the macro was used on. Currently supports `struct`s only.
pub fn info_section(item: TokenStream) -> TokenStream {
    let input: DeriveInput = parse_macro_input!(item);
    let ident = input.ident;
    match input.data {
        Data::Struct(s) => struct_info_section(ident, s),
        _ => {
            quote! {compile_error!("The InfoSection derive can only be used with structs.")}.into()
        }
    }
}
