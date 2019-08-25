pub mod raw;

use std::ffi::{c_void, CStr, CString};
use std::ptr;

use crate::redisearch::raw::{RSDoc, RSField, RSIndex, RSQueryNode, RSResultsIterator};
use crate::redismodule::raw::Status;
use crate::redismodule::RedisResult;
use crate::redismodule::{RedisError, RedisValue};
use std::os::raw::c_char;

pub fn initialize() -> RedisResult {
    let status: Status = unsafe { self::raw::Wrap_RediSearch_Initialize() }.into();
    if status == Status::Err {
        Err(RedisError::Str("Cannot initialize RediSearch"))
    } else {
        Ok(RedisValue::None)
    }
}

pub fn create_index(index_name: &str) -> Option<&mut RSIndex> {
    let c_index_name = CString::new(index_name).unwrap();

    let index = unsafe {
        self::raw::RediSearch_CreateIndex.unwrap()(c_index_name.as_ptr(), None, ptr::null_mut())
    };

    unsafe { index.as_mut() }
}

pub fn create_text_field<'a>(
    field_name: &'a str,
    index: &'a mut RSIndex,
) -> Option<&'a mut RSField> {
    let c_name = CString::new(field_name).unwrap();

    let field = unsafe { self::raw::RediSearch_CreateTextField.unwrap()(index, c_name.as_ptr()) };

    unsafe { field.as_mut() }
}

pub fn create_document(doc_id: &str, score: f64) -> Option<&mut RSDoc> {
    let c_doc_id = CString::new(doc_id).unwrap();

    let doc = {
        unsafe {
            self::raw::RediSearch_CreateDocument.unwrap()(
                c_doc_id.as_ptr() as *const c_void,
                doc_id.len(),
                score,
                ptr::null(), // Default language
            )
        }
    };

    unsafe { doc.as_mut() }
}

pub fn document_add_text_field(field_name: &str, field_value: &str, doc: &mut RSDoc) {
    let c_name = CString::new(field_name).unwrap();
    let c_value = CString::new(field_value).unwrap();

    unsafe {
        self::raw::RediSearch_DocumentAddTextField.unwrap()(
            doc,
            c_name.as_ptr(),
            c_value.as_ptr(),
            field_value.len(),
        )
    }
}

pub fn spec_add_document(index: &mut RSIndex, doc: &mut RSDoc) {
    unsafe { self::raw::RediSearch_SpecAddDocument.unwrap()(index, doc) }
}

pub fn search(index: &mut RSIndex, search_term: &str, field_name: &str) -> Vec<String> {
    use self::raw::*;

    let qn = create_query_node(index, search_term, field_name).unwrap();

    let iter = {
        let iter = unsafe { RediSearch_GetResultsIterator.unwrap()(qn, index) };
        unsafe { iter.as_mut() }
    };

    // FIXME: Return streaming results instead of aggregating them in memory as we do here.
    let mut results = vec![];

    let iter = match iter {
        Some(it) => it,
        None => return results,
    };

    loop {
        let id = results_iter_next(index, iter);
        match id {
            Some(doc_id) => results.push(doc_id),
            None => break,
        }
    }

    unsafe {
        RediSearch_ResultsIteratorFree.unwrap()(iter);
    }

    results
}

fn results_iter_next(index: &mut RSIndex, iter: &mut RSResultsIterator) -> Option<String> {
    let id = unsafe {
        let mut len = 0usize;

        self::raw::RediSearch_ResultsIteratorNext.unwrap()(iter, index, &mut len as *mut usize)
            as *const c_char
    };

    if id.is_null() {
        return None;
    }

    Some(unsafe { CStr::from_ptr(id) }.to_string_lossy().into_owned())
}

fn create_query_node<'a>(
    index: &'a mut RSIndex,
    search_term: &str,
    field_name: &str,
) -> Option<&'a mut RSQueryNode> {
    let c_field_name = CString::new(field_name).unwrap();
    let c_search_term = CString::new(search_term).unwrap();

    let qn = unsafe {
        self::raw::RediSearch_CreateTokenNode.unwrap()(
            index,
            c_field_name.as_ptr(),
            c_search_term.as_ptr(),
        )
    };

    unsafe { qn.as_mut() }
}
