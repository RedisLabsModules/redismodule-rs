use std::{ffi::c_longlong, ptr::NonNull};

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
    reply: Option<NonNull<RedisModuleCallReply>>,
}

impl RootCallReply {
    pub(crate) fn new(reply: *mut RedisModuleCallReply) -> RootCallReply {
        RootCallReply {
            reply: NonNull::new(reply),
        }
    }
}

impl CallReply for RootCallReply {
    fn get_type(&self) -> ReplyType {
        self.reply
            .map_or(ReplyType::Unknown, |e| call_reply_type(e.as_ptr()))
    }

    fn get_string(&self) -> Option<String> {
        call_reply_string(self.reply?.as_ptr())
    }

    fn len(&self) -> usize {
        self.reply.map_or(0, |e| call_reply_length(e.as_ptr()))
    }

    fn get(&self, index: usize) -> Option<InnerCallReply> {
        // Redis will verify array boundaries so no need to veirfy it here.
        NonNull::new(call_reply_array_element(self.reply?.as_ptr(), index))
            .map(|inner_reply| InnerCallReply::new(self, inner_reply))
    }

    fn iter(&self) -> Box<dyn Iterator<Item = InnerCallReply> + '_> {
        Box::new(RootCallReplyIterator {
            reply: self,
            index: 0,
        })
    }

    fn get_int(&self) -> c_longlong {
        self.reply.map_or(0, |e| call_reply_integer(e.as_ptr()))
    }
}

impl Drop for RootCallReply {
    fn drop(&mut self) {
        self.reply.map(|e| free_call_reply(e.as_ptr()));
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
    reply: NonNull<RedisModuleCallReply>,
}

impl<'root> InnerCallReply<'root> {
    pub(crate) fn new(
        root: &'root RootCallReply,
        reply: NonNull<RedisModuleCallReply>,
    ) -> InnerCallReply<'root> {
        InnerCallReply { root, reply }
    }
}

impl<'a> CallReply for InnerCallReply<'a> {
    fn get_type(&self) -> ReplyType {
        call_reply_type(self.reply.as_ptr())
    }

    fn get_string(&self) -> Option<String> {
        call_reply_string(self.reply.as_ptr())
    }

    fn len(&self) -> usize {
        call_reply_length(self.reply.as_ptr())
    }

    fn get(&self, index: usize) -> Option<Self> {
        // Redis will verify array boundaries so no need to veirfy it here.
        NonNull::new(call_reply_array_element(self.reply.as_ptr(), index))
            .map(|inner_reply| Self::new(self.root, inner_reply))
    }

    fn iter(&self) -> Box<dyn Iterator<Item = InnerCallReply> + '_> {
        Box::new(InnerCallReplyIterator {
            reply: self,
            index: 0,
        })
    }

    fn get_int(&self) -> c_longlong {
        call_reply_integer(self.reply.as_ptr())
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
