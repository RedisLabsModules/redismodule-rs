use crate::key::RedisKey;
use crate::raw;
use crate::RedisError;
use crate::RedisString;
use crate::Status;
use std::os::raw::c_long;
use std::ptr;

#[derive(Debug)]
pub struct StreamRecord {
    pub id: raw::RedisModuleStreamID,
    pub fields: Vec<(RedisString, RedisString)>,
}

#[derive(Debug)]
pub struct StreamIterator<'key> {
    key: &'key RedisKey,
}

impl<'key> StreamIterator<'key> {
    pub(crate) fn new(
        key: &RedisKey,
        mut from: Option<raw::RedisModuleStreamID>,
        mut to: Option<raw::RedisModuleStreamID>,
        exclusive: bool,
        reverse: bool,
    ) -> Result<StreamIterator, RedisError> {
        let mut flags = if exclusive {
            raw::REDISMODULE_STREAM_ITERATOR_EXCLUSIVE as i32
        } else {
            0
        };

        flags |= if reverse {
            raw::REDISMODULE_STREAM_ITERATOR_REVERSE as i32
        } else {
            0
        };

        let res = raw::stream_iterator_start(
            key.key_inner,
            flags,
            from.as_mut().map_or(ptr::null_mut(), |v| v),
            to.as_mut().map_or(ptr::null_mut(), |v| v),
        );
        if Status::Ok == res {
            Ok(StreamIterator { key })
        } else {
            Err(RedisError::Str("Failed creating stream iterator"))
        }
    }
}

impl<'key> Iterator for StreamIterator<'key> {
    type Item = StreamRecord;

    fn next(&mut self) -> Option<Self::Item> {
        let mut id = raw::RedisModuleStreamID { ms: 0, seq: 0 };
        let mut num_fields: c_long = 0;
        let mut field_name: *mut raw::RedisModuleString = ptr::null_mut();
        let mut field_val: *mut raw::RedisModuleString = ptr::null_mut();
        if Status::Ok != raw::stream_iterator_next_id(self.key.key_inner, &mut id, &mut num_fields)
        {
            return None;
        }
        let mut fields = Vec::new();
        while Status::Ok
            == raw::stream_iterator_next_field(self.key.key_inner, &mut field_name, &mut field_val)
        {
            fields.push((
                RedisString::from_redis_module_string(ptr::null_mut(), field_name),
                RedisString::from_redis_module_string(ptr::null_mut(), field_val),
            ));
        }
        Some(StreamRecord { id, fields })
    }
}

impl<'key> Drop for StreamIterator<'key> {
    fn drop(&mut self) {
        raw::stream_iterator_delete(self.key.key_inner);
    }
}
