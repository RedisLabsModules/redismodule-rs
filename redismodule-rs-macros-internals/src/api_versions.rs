use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::quote;

lazy_static::lazy_static! {
    pub(crate) static ref API_VERSION_MAPPING: HashMap<String, usize> = HashMap::from([
        ("RedisModule_AddPostNotificationJob".to_string(), 70200),
        ("RedisModule_SetCommandACLCategories".to_string(), 70200),
        ("RedisModule_GetOpenKeyModesAll".to_string(), 70200),
        ("RedisModule_CallReplyPromiseSetUnblockHandler".to_string(), 70200),
        ("RedisModule_CallReplyPromiseAbort".to_string(), 70200),
        ("RedisModule_Microseconds".to_string(), 70200),
        ("RedisModule_CachedMicroseconds".to_string(), 70200),
        ("RedisModule_RegisterAuthCallback".to_string(), 70200),
        ("RedisModule_BlockClientOnKeysWithFlags".to_string(), 70200),
        ("RedisModule_GetModuleOptionsAll".to_string(), 70200),
        ("RedisModule_BlockClientGetPrivateData".to_string(), 70200),
        ("RedisModule_BlockClientSetPrivateData".to_string(), 70200),
        ("RedisModule_BlockClientOnAuth".to_string(), 70200),
        ("RedisModule_ACLAddLogEntryByUserName".to_string(), 70200),
        ("RedisModule_GetCommand".to_string(), 70000),
        ("RedisModule_SetCommandInfo".to_string(), 70000),

    ]);

    pub(crate) static ref API_OLDEST_VERSION: usize = 60000;
    pub(crate) static ref ALL_VERSIONS: Vec<(usize, String)> = vec![
        (60000, "min-redis-compatibility-version-6-0".to_string()),
        (60200, "min-redis-compatibility-version-6-2".to_string()),
        (70000, "min-redis-compatibility-version-7-0".to_string()),
        (70200, "min-redis-compatibility-version-7-2".to_string()),
    ];
}

pub(crate) fn get_feature_flags(
    min_required_version: usize,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let all_lower_versions: Vec<&str> = ALL_VERSIONS
        .iter()
        .filter_map(|(v, s)| {
            if *v < min_required_version {
                Some(s.as_str())
            } else {
                None
            }
        })
        .collect();
    let all_upper_versions: Vec<&str> = ALL_VERSIONS
        .iter()
        .filter_map(|(v, s)| {
            if *v >= min_required_version {
                Some(s.as_str())
            } else {
                None
            }
        })
        .collect();
    (
        all_lower_versions
            .into_iter()
            .map(|s| quote!(feature = #s).into())
            .collect(),
        all_upper_versions
            .into_iter()
            .map(|s| quote!(feature = #s).into())
            .collect(),
    )
}
