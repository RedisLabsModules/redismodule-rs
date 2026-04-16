#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

// Workaround for https://github.com/rust-lang/rust-bindgen/issues/1651#issuecomment-848479168
#[allow(deref_nullptr)]
pub mod bindings {
    #![expect(
        clippy::missing_safety_doc,
        reason = "The `RedisModuleSlotRangeArray` has an array field. And bindgen will try to generate an unsafe `as_slice()` method for this array."
    )]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

// See: https://users.rust-lang.org/t/bindgen-generate-options-and-some-are-none/14027
