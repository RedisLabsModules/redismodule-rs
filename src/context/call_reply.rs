use std::ffi::c_longlong;

use crate::raw::*;

pub trait CallReply {
    /// Return the call reply type
    fn get_type(&self) -> ReplyType;

    /// Return the reply us rust string,
    /// Only relevant to the following types:
    /// * String
    /// * Error
    ///
    /// A none will also be returned if failed to convert the
    /// data into a string (data is binary).
    fn get_string(&self) -> Option<String>;

    /// Return lenght of the reply,
    /// Only relevant to the following types:
    /// * String
    /// * Error
    /// * Array
    ///
    /// Running this function on other type will return 0.
    fn len(&self) -> usize;

    /// Return the reply at the location of the given index,
    /// Only relevant to the following types:
    /// * Array
    ///
    /// Running this function on other type will return None.
    fn get(&self, index: usize) -> Option<InnerCallReply>;

    /// Return an iterator over the elements in the array
    /// Only relevant to the following types:
    /// * Array
    ///
    /// Running this function on other type will return an empty iterator.
    fn iter(&self) -> Box<dyn Iterator<Item = InnerCallReply> + '_>;

    /// Return integer value of the reply type
    /// Only relevant to the following types:
    /// * Integer
    ///
    /// Running this function on other type will return 0.
    fn get_int(&self) -> c_longlong;
}

pub struct RootCallReply {
    reply: *mut RedisModuleCallReply,
}

impl RootCallReply {
    pub(crate) fn new(reply: *mut RedisModuleCallReply) -> RootCallReply {
        RootCallReply { reply }
    }
}

impl CallReply for RootCallReply {
    fn get_type(&self) -> ReplyType {
        if self.reply.is_null() {
            return ReplyType::Unknown;
        }
        call_reply_type(self.reply)
    }

    fn get_string(&self) -> Option<String> {
        if self.reply.is_null() {
            return None;
        }
        call_reply_string(self.reply)
    }

    fn len(&self) -> usize {
        if self.reply.is_null() {
            return 0;
        }
        call_reply_length(self.reply)
    }

    fn get(&self, index: usize) -> Option<InnerCallReply> {
        if self.len() >= index {
            return None;
        }
        let res = call_reply_array_element(self.reply, index);
        if res.is_null() {
            return None;
        }
        Some(InnerCallReply::new(self, res))
    }

    fn iter(&self) -> Box<dyn Iterator<Item = InnerCallReply> + '_> {
        Box::new(RootCallReplyIterator {
            reply: self,
            index: 0,
        })
    }

    fn get_int(&self) -> c_longlong {
        if self.reply.is_null() {
            return 0;
        }
        call_reply_integer(self.reply)
    }
}

impl Drop for RootCallReply {
    fn drop(&mut self) {
        if !self.reply.is_null() {
            free_call_reply(self.reply)
        }
    }
}

pub struct RootCallReplyIterator<'root> {
    reply: &'root RootCallReply,
    index: usize,
}

impl<'root> Iterator for RootCallReplyIterator<'root> {
    type Item = InnerCallReply<'root>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.reply.get(self.index);
        if res.is_some() {
            self.index += 1;
        }
        res
    }
}

pub struct InnerCallReply<'root> {
    root: &'root RootCallReply,
    reply: *mut RedisModuleCallReply,
}

impl<'root> InnerCallReply<'root> {
    pub(crate) fn new(
        root: &'root RootCallReply,
        reply: *mut RedisModuleCallReply,
    ) -> InnerCallReply<'root> {
        InnerCallReply { root, reply }
    }
}

impl<'a> CallReply for InnerCallReply<'a> {
    fn get_type(&self) -> ReplyType {
        call_reply_type(self.reply)
    }

    fn get_string(&self) -> Option<String> {
        call_reply_string(self.reply)
    }

    fn len(&self) -> usize {
        call_reply_length(self.reply)
    }

    fn get(&self, index: usize) -> Option<Self> {
        if self.len() >= index {
            return None;
        }
        let res = call_reply_array_element(self.reply, index);
        if res.is_null() {
            return None;
        }
        Some(Self::new(self.root, res))
    }

    fn iter(&self) -> Box<dyn Iterator<Item = InnerCallReply> + '_> {
        Box::new(InnerCallReplyIterator {
            reply: self,
            index: 0,
        })
    }

    fn get_int(&self) -> c_longlong {
        call_reply_integer(self.reply)
    }
}

pub struct InnerCallReplyIterator<'root, 'curr: 'root> {
    reply: &'curr InnerCallReply<'root>,
    index: usize,
}

impl<'root, 'curr: 'root> Iterator for InnerCallReplyIterator<'root, 'curr> {
    type Item = InnerCallReply<'root>;

    fn next(&mut self) -> Option<Self::Item> {
        let res = self.reply.get(self.index);
        if res.is_some() {
            self.index += 1;
        }
        res
    }
}
