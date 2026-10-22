use std::{marker::PhantomData, ptr};

use libc::{c_char, c_void};
use std::iter::Iterator;

use crate::{raw, Context, RedisString, Status};

struct Dictionary<'a, T> {
    ctx: &'a Context,
    inner: *mut raw::RedisModuleDict,
    value_type: PhantomData<T>,
}

impl<'a, T> Dictionary<'a, T> {
    pub fn new(ctx: &'a Context) -> Dictionary<T> {
        Dictionary {
            ctx: ctx,
            inner: unsafe { raw::RedisModule_CreateDict.unwrap()(ctx.ctx) },
            value_type: PhantomData,
        }
    }

    pub fn set(&mut self, key: RedisString, value: T) -> Status {
        let value = Box::into_raw(Box::new(value)).cast::<c_void>();
        unsafe { raw::RedisModule_DictSet.unwrap()(self.inner, key.inner, value).into() }
    }

    pub fn del(&mut self, key: RedisString) -> Option<T> {
        unsafe {
            let mut value: *mut T = ptr::null_mut();
            let ref_value: *mut *mut T = &mut value;
            let res = raw::RedisModule_DictDel.unwrap()(
                self.inner,
                key.inner,
                ref_value.cast::<c_void>(),
            )
            .into();

            match res {
                Status::Ok => {
                    let res: Box<T> = Box::from_raw(value);
                    Some(*res)
                }
                Status::Err => None,
            }
        }
    }

    pub fn iter(&self, op: Operator, key: RedisString) -> DictionaryIterator<T> {
        let iter = unsafe {
            raw::RedisModule_DictIteratorStart.unwrap()(
                self.inner,
                op.as_ptr(),
                key.inner,
            )
        };
        DictionaryIterator {
            ctx: self.ctx,
            inner: iter,
            value_type: PhantomData,
        }
    }
}

impl<T> Drop for Dictionary<'_, T> {
    // Frees resources appropriately as a Dictionary goes out of scope.
    fn drop(&mut self) {
        unsafe { raw::RedisModule_FreeDict.unwrap()(self.ctx.ctx, self.inner) };
    }
}

pub enum Operator {
    First,    // "^" – Seek the first (lexicographically smaller) key.
    Last,     // "$" – Seek the last (lexicographically biffer) key.
    FirstGT,  // ">" – Seek the first element greater than the specified key.
    FirstGTE, // ">=" – Seek the first element greater or equal than the specified key.
    FirstST,  // "<" – Seek the first element smaller than the specified key.
    FirstSTE, // "<=" – Seek the first element smaller or equal than the specified key.
    Equal,    // "==" – Seek the first element matching exactly the specified key.
}

impl Operator {
    pub fn as_ptr(&self) -> *const c_char {
        let op = match self {
            Operator::First => "^\0",
            Operator::Last => "$\0",
            Operator::FirstGT => ">\0",
            Operator::FirstGTE => ">=\0",
            Operator::FirstST => "<\0",
            Operator::FirstSTE => "<=\0",
            Operator::Equal => "==\0",
        };
        op.as_bytes().as_ptr() as *const c_char 
    }
}

struct DictionaryIterator<'a, T> {
    ctx: &'a Context,
    inner: *mut raw::RedisModuleDictIter,
    value_type: PhantomData<T>,
}

impl<T> Iterator for DictionaryIterator<'_, T> {
    type Item = (RedisString, T);

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {

        let mut value: *mut T = ptr::null_mut();
        let ref_value: *mut *mut T = &mut value;

        unsafe { 
            let res = raw::RedisModule_DictNext.unwrap()(self.ctx.ctx, self.inner, ref_value as *mut *mut c_void);
            if res.is_null() {
                let value: Box<T> = Box::from_raw(value);
                Some((RedisString::new(self.ctx.ctx, res), *value))
            } else {
                None
            }
        }
    }
}


impl<T> Drop for DictionaryIterator<'_ ,T> {
    // Frees resources appropriately as a DictionaryIterator goes out of scope.
    fn drop(&mut self) {
        unsafe { raw::RedisModule_DictIteratorStop.unwrap()(self.inner) };
    }
}