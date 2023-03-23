use std::{ptr::NonNull, marker::PhantomData};

use crate::raw::*;

pub struct StringCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> StringCallReply<'root> {
    pub fn to_string(&self) -> Option<String> {
        call_reply_string(self.reply.as_ptr())
    }
}

impl<'root> Drop for StringCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

pub struct ErrorCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> ErrorCallReply<'root> {
    pub fn to_string(&self) -> Option<String> {
        call_reply_string(self.reply.as_ptr())
    }
}

impl<'root> Drop for ErrorCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

pub struct I64CallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> I64CallReply<'root> {
    pub fn to_i64(&self) -> i64 {
        call_reply_integer(self.reply.as_ptr())
    }
}

impl<'root> Drop for I64CallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

pub struct ArrayCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> Drop for ArrayCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

impl<'root> ArrayCallReply<'root> {
    pub fn iter(&self) -> ArrayCallReplyIterator<'root, '_> {
        ArrayCallReplyIterator{
            reply: self,
            index: 0,
        }
    }

    pub fn get(&self, idx: usize) -> Option<CallReply<'_>> {
        let res = NonNull::new(call_reply_array_element(self.reply.as_ptr(), idx))?;
        Some(create_call_reply(res))
    }

    pub fn len(&self) -> usize {
        call_reply_length(self.reply.as_ptr())
    }
}

pub struct ArrayCallReplyIterator<'root, 'curr> {
    reply: &'curr ArrayCallReply<'root>,
    index: usize,
}

impl<'root, 'curr> Iterator for ArrayCallReplyIterator<'root, 'curr> {
    type Item = CallReply<'curr>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.reply.get(self.index);
        if res.is_some() {
            self.index += 1;
        }
        res
    }
}

pub struct NullCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> Drop for NullCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

pub struct MapCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> MapCallReply<'root> {
    pub fn iter(&self) -> MapCallReplyIterator<'root, '_> {
        MapCallReplyIterator{
            reply: self,
            index: 0,
        }
    }

    pub fn get(&self, idx: usize) -> Option<(CallReply<'_>, CallReply<'_>)> {
        let (key, val) = call_reply_map_element(self.reply.as_ptr(), idx);
        Some((create_call_reply(NonNull::new(key)?), create_call_reply(NonNull::new(val)?))) 
    }

    pub fn len(&self) -> usize {
        call_reply_length(self.reply.as_ptr())
    }
}

pub struct MapCallReplyIterator<'root, 'curr> {
    reply: &'curr MapCallReply<'root>,
    index: usize,
}

impl<'root, 'curr> Iterator for MapCallReplyIterator<'root, 'curr> {
    type Item = (CallReply<'curr>, CallReply<'curr>);

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

pub struct SetCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> SetCallReply<'root> {
    pub fn iter(&self) -> SetCallReplyIterator<'root, '_> {
        SetCallReplyIterator{
            reply: self,
            index: 0,
        }
    }

    pub fn get(&self, idx: usize) -> Option<CallReply<'_>> {
        let res = NonNull::new(call_reply_set_element(self.reply.as_ptr(), idx))?;
        Some(create_call_reply(res))
    }

    pub fn len(&self) -> usize {
        call_reply_length(self.reply.as_ptr())
    }
}

pub struct SetCallReplyIterator<'root, 'curr> {
    reply: &'curr SetCallReply<'root>,
    index: usize,
}

impl<'root, 'curr> Iterator for SetCallReplyIterator<'root, 'curr> {
    type Item = CallReply<'curr>;

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

pub struct BoolCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> BoolCallReply<'root> {
    pub fn to_bool(&self) -> bool {
        call_reply_bool(self.reply.as_ptr())
    }
}

impl<'root> Drop for BoolCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

pub struct DoubleCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> DoubleCallReply<'root> {
    pub fn to_double(&self) -> f64 {
        call_reply_double(self.reply.as_ptr())
    }
}

impl<'root> Drop for DoubleCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

pub struct BigNumberCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> BigNumberCallReply<'root> {
    pub fn to_string(&self) -> Option<String> {
        call_reply_big_number(self.reply.as_ptr())
    }
}

impl<'root> Drop for BigNumberCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

pub struct VerbatimStringCallReply<'root> {
    reply: NonNull<RedisModuleCallReply>,
    _dummy: PhantomData<&'root i64>,
}

impl<'root> VerbatimStringCallReply<'root> {
    pub fn to_parts(&self) -> Option<(String, Vec<u8>)> {
        call_reply_verbatim_string(self.reply.as_ptr())
    }
}

impl<'root> Drop for VerbatimStringCallReply<'root> {
    fn drop(&mut self) {
        free_call_reply(self.reply.as_ptr());
    }
}

pub enum CallReply<'root> {
    Unknown,
    I64(I64CallReply<'root>),
    String(StringCallReply<'root>),
    Error(ErrorCallReply<'root>),
    Array(ArrayCallReply<'root>),
    Null(NullCallReply<'root>),
    Map(MapCallReply<'root>),
    Set(SetCallReply<'root>),
    Bool(BoolCallReply<'root>),
    Double(DoubleCallReply<'root>),
    BigNumber(BigNumberCallReply<'root>),
    VerbatimString(VerbatimStringCallReply<'root>),
}

fn create_call_reply<'root>(reply: NonNull<RedisModuleCallReply>) -> CallReply<'root> {
    let ty = call_reply_type(reply.as_ptr());
    match ty {
        ReplyType::Unknown => CallReply::Unknown, // unknown means NULL so no need to free free anything
        ReplyType::Integer => CallReply::I64(I64CallReply{ reply: reply, _dummy: PhantomData}),
        ReplyType::String => CallReply::String(StringCallReply{ reply: reply, _dummy: PhantomData}),
        ReplyType::Error => CallReply::Error(ErrorCallReply{ reply: reply, _dummy: PhantomData}),
        ReplyType::Array => CallReply::Array(ArrayCallReply{ reply: reply, _dummy: PhantomData}),
        ReplyType::Null => CallReply::Null(NullCallReply{ reply: reply, _dummy: PhantomData}),
        ReplyType::Map => CallReply::Map(MapCallReply{ reply: reply, _dummy: PhantomData}),
        ReplyType::Set => CallReply::Set(SetCallReply{ reply: reply, _dummy: PhantomData}),
        ReplyType::Bool => CallReply::Bool(BoolCallReply{ reply: reply, _dummy: PhantomData}),
        ReplyType::Double => CallReply::Double(DoubleCallReply{ reply: reply, _dummy: PhantomData}),
        ReplyType::BigNumber => CallReply::BigNumber(BigNumberCallReply{ reply: reply, _dummy: PhantomData}),
        ReplyType::VerbatimString => CallReply::VerbatimString(VerbatimStringCallReply{ reply: reply, _dummy: PhantomData}),
    }
}

pub(crate) fn create_root_call_reply<'root>(reply: Option<NonNull<RedisModuleCallReply>>) -> CallReply<'root> {
    reply.map_or(CallReply::Unknown, |v| create_call_reply(v))
}
