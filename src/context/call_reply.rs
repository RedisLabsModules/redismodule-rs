use core::slice;
use std::{
    ffi::c_char,
    fmt,
    fmt::{Debug, Formatter},
    marker::PhantomData,
    ptr::NonNull,
};

use crate::raw::*;

pub struct StringCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

impl<'root> StringCallReply<'root> {
    /// Convert StringCallReply to String.
    /// Return None data is not a valid utf8.
    pub fn to_string(&self) -> Option<String> {
        String::from_utf8(self.as_bytes().to_vec()).ok()
    }

    /// Return the StringCallReply data as &[u8]
    pub fn as_bytes(&self) -> &[u8] {
        let mut len: usize = 0;
        let reply_string: *mut u8 = unsafe {
            RedisModule_CallReplyStringPtr.unwrap()(self.reply.as_ptr(), &mut len) as *mut u8
        };
        unsafe { slice::from_raw_parts(reply_string, len) }
    }
}

impl<'root> Drop for StringCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> Debug for StringCallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.as_bytes())
    }
}

pub struct ErrorCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

#[derive(Debug)]
pub enum ErrorReply<'root> {
    Message(String),
    RedisError(ErrorCallReply<'root>),
}

impl<'root> ErrorCallReply<'root> {
    /// Convert ErrorCallReply to String.
    /// Return None data is not a valid utf8.
    pub fn to_utf8_string(&self) -> Option<String> {
        String::from_utf8(self.as_bytes().to_vec()).ok()
    }

    /// Return the ErrorCallReply data as &[u8]
    pub fn as_bytes(&self) -> &[u8] {
        let mut len: usize = 0;
        let reply_string: *mut u8 = unsafe {
            RedisModule_CallReplyStringPtr.unwrap()(self.reply.as_ptr(), &mut len) as *mut u8
        };
        unsafe { slice::from_raw_parts(reply_string, len) }
    }
}

impl<'root> ErrorReply<'root> {
    /// Convert [ErrorCallReply] to [String] or [None] if its not a valid utf8.
    pub fn to_utf8_string(&self) -> Option<String> {
        match self {
            ErrorReply::Message(s) => Some(s.clone()),
            ErrorReply::RedisError(r) => r.to_utf8_string(),
        }
    }

    /// Return the ErrorCallReply data as &[u8]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            ErrorReply::Message(s) => s.as_bytes(),
            ErrorReply::RedisError(r) => r.as_bytes(),
        }
    }
}

impl<'root> Drop for ErrorCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> Debug for ErrorCallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_utf8_string())
    }
}

pub struct I64CallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

impl<'root> I64CallReply<'root> {
    /// Return the i64 value of the [I64CallReply]
    pub fn to_i64(&self) -> i64 {
        call_reply_integer(self.reply.as_ptr())
    }
}

impl<'root> Drop for I64CallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> Debug for I64CallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_i64())
    }
}

pub struct ArrayCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

impl<'root> Drop for ArrayCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> ArrayCallReply<'root> {
    /// Return an Iterator that allows to iterate over the elements
    /// in the [ArrayCallReply].
    pub fn iter(&self) -> ArrayCallReplyIterator<'root, '_> {
        ArrayCallReplyIterator {
            reply: self,
            index: 0,
        }
    }

    /// Return the array element on the given index.
    pub fn get(&self, idx: usize) -> Option<CallResult<'_>> {
        let res = NonNull::new(call_reply_array_element(self.reply.as_ptr(), idx))?;
        Some(create_call_reply(res))
    }

    /// Return the number of elements in the [ArrayCallReply].
    pub fn len(&self) -> usize {
        call_reply_length(self.reply.as_ptr())
    }
}

pub struct ArrayCallReplyIterator<'root, 'curr> {
    reply: &'curr ArrayCallReply<'root>,
    index: usize,
}

impl<'root, 'curr> Iterator for ArrayCallReplyIterator<'root, 'curr> {
    type Item = CallResult<'curr>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.reply.get(self.index);
        if res.is_some() {
            self.index += 1;
        }
        res
    }
}

impl<'root> Debug for ArrayCallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let children: Vec<CallResult> = self.iter().collect();
        write!(f, "{:?}", children)
    }
}

pub struct NullCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

impl<'root> Drop for NullCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> Debug for NullCallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Null")
    }
}

pub struct MapCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

impl<'root> MapCallReply<'root> {
    /// Return an iterator over the elements in the [MapCallReply].
    /// The iterator return each element as a tuple representing the
    /// key and the value.
    pub fn iter(&self) -> MapCallReplyIterator<'root, '_> {
        MapCallReplyIterator {
            reply: self,
            index: 0,
        }
    }

    /// Return the map element on the given index.
    pub fn get(&self, idx: usize) -> Option<(CallResult<'_>, CallResult<'_>)> {
        let (key, val) = call_reply_map_element(self.reply.as_ptr(), idx);
        Some((
            create_call_reply(NonNull::new(key)?),
            create_call_reply(NonNull::new(val)?),
        ))
    }

    /// Return the number of elements in the [MapCallReply].
    pub fn len(&self) -> usize {
        call_reply_length(self.reply.as_ptr())
    }
}

pub struct MapCallReplyIterator<'root, 'curr> {
    reply: &'curr MapCallReply<'root>,
    index: usize,
}

impl<'root, 'curr> Iterator for MapCallReplyIterator<'root, 'curr> {
    type Item = (CallResult<'curr>, CallResult<'curr>);

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.reply.get(self.index);
        if res.is_some() {
            self.index += 1;
        }
        res
    }
}

impl<'root> Drop for MapCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> Debug for MapCallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let elements: Vec<(CallResult, CallResult)> = self.iter().collect();
        write!(f, "{:?}", elements)
    }
}

pub struct SetCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

impl<'root> SetCallReply<'root> {
    /// Return an iterator over the elements in the [SetCallReply].
    pub fn iter(&self) -> SetCallReplyIterator<'root, '_> {
        SetCallReplyIterator {
            reply: self,
            index: 0,
        }
    }

    /// Return the set element on the given index.
    pub fn get(&self, idx: usize) -> Option<CallResult<'_>> {
        let res = NonNull::new(call_reply_set_element(self.reply.as_ptr(), idx))?;
        Some(create_call_reply(res))
    }

    /// Return the number of elements in the [SetCallReply].
    pub fn len(&self) -> usize {
        call_reply_length(self.reply.as_ptr())
    }
}

pub struct SetCallReplyIterator<'root, 'curr> {
    reply: &'curr SetCallReply<'root>,
    index: usize,
}

impl<'root, 'curr> Iterator for SetCallReplyIterator<'root, 'curr> {
    type Item = CallResult<'curr>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.reply.get(self.index);
        if res.is_some() {
            self.index += 1;
        }
        res
    }
}

impl<'root> Drop for SetCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> Debug for SetCallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let elements: Vec<CallResult> = self.iter().collect();
        write!(f, "{:?}", elements)
    }
}

pub struct BoolCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

impl<'root> BoolCallReply<'root> {
    /// Return the boolean value of the [BoolCallReply].
    pub fn to_bool(&self) -> bool {
        call_reply_bool(self.reply.as_ptr())
    }
}

impl<'root> Drop for BoolCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> Debug for BoolCallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_bool())
    }
}

pub struct DoubleCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

impl<'root> DoubleCallReply<'root> {
    /// Return the double value of the [BoolCallReply] as f64.
    pub fn to_double(&self) -> f64 {
        call_reply_double(self.reply.as_ptr())
    }
}

impl<'root> Drop for DoubleCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> Debug for DoubleCallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_double())
    }
}

pub struct BigNumberCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

impl<'root> BigNumberCallReply<'root> {
    /// Return the big number value of the [BigNumberCallReply] as String.
    /// Return None if the data is not a valid utf8
    pub fn to_string(&self) -> Option<String> {
        call_reply_big_number(self.reply.as_ptr())
    }
}

impl<'root> Drop for BigNumberCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> Debug for BigNumberCallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_string())
    }
}

pub struct VerbatimStringCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root ()>,
}

impl<'root> VerbatimStringCallReply<'root> {
    /// Return the verbatim string value of the [VerbatimStringCallReply] as a tuple.
    /// The first entry represents the format, the second entry represent the data.
    /// Return None if the format is not a valid utf8
    pub fn to_parts(&self) -> Option<(String, Vec<u8>)> {
        self.as_parts()
            .map(|(format, data)| (format.to_string(), data.to_vec()))
    }

    /// Borrow the verbatim string value of the [VerbatimStringCallReply] as a tuple.
    /// The first entry represents the format as &str, the second entry represent the data as &[u8].
    /// Return None if the format is not a valid utf8.
    pub fn as_parts(&self) -> Option<(&str, &[u8])> {
        let mut len: usize = 0;
        let mut format: *const c_char = std::ptr::null();
        let reply_string: *mut u8 = unsafe {
            RedisModule_CallReplyVerbatim.unwrap()(self.reply.as_ptr(), &mut len, &mut format)
                as *mut u8
        };
        Some((
            std::str::from_utf8(unsafe { slice::from_raw_parts(format as *const u8, 3) })
                .ok()
                .unwrap(),
            unsafe { slice::from_raw_parts(reply_string, len) },
        ))
    }
}

impl<'root> Drop for VerbatimStringCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> Debug for VerbatimStringCallReply<'root> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.to_parts())
    }
}

#[derive(Debug)]
pub enum CallReply<'root> {
    Unknown,
    I64(I64CallReply<'root>),
    String(StringCallReply<'root>),
    Array(ArrayCallReply<'root>),
    Null(NullCallReply<'root>),
    Map(MapCallReply<'root>),
    Set(SetCallReply<'root>),
    Bool(BoolCallReply<'root>),
    Double(DoubleCallReply<'root>),
    BigNumber(BigNumberCallReply<'root>),
    VerbatimString(VerbatimStringCallReply<'root>),
}

fn create_call_reply<'root>(reply: NonNull<RedisModuleCallReply>) -> CallResult<'root> {
    let ty = call_reply_type(reply.as_ptr());
    match ty {
        ReplyType::Unknown => Ok(CallReply::Unknown), // unknown means NULL so no need to free free anything
        ReplyType::Integer => Ok(CallReply::I64(I64CallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
        ReplyType::String => Ok(CallReply::String(StringCallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
        ReplyType::Error => Err(ErrorReply::RedisError(ErrorCallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
        ReplyType::Array => Ok(CallReply::Array(ArrayCallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
        ReplyType::Null => Ok(CallReply::Null(NullCallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
        ReplyType::Map => Ok(CallReply::Map(MapCallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
        ReplyType::Set => Ok(CallReply::Set(SetCallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
        ReplyType::Bool => Ok(CallReply::Bool(BoolCallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
        ReplyType::Double => Ok(CallReply::Double(DoubleCallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
        ReplyType::BigNumber => Ok(CallReply::BigNumber(BigNumberCallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
        ReplyType::VerbatimString => Ok(CallReply::VerbatimString(VerbatimStringCallReply {
            reply: reply,
            _dummy: PhantomData,
        })),
    }
}

pub(crate) fn create_root_call_reply<'root>(
    reply: Option<NonNull<RedisModuleCallReply>>,
) -> CallResult<'root> {
    reply.map_or(Ok(CallReply::Unknown), |v| create_call_reply(v))
}

pub type CallResult<'root> = Result<CallReply<'root>, ErrorReply<'root>>;
