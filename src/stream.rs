use crate::Status;
use crate::key::RedisKey;
use crate::raw;
use crate::RedisError;
use crate::RedisString;
use std::os::raw::c_long;
use std::ptr;

pub struct StreamRecord {
    pub id: raw::RedisModuleStreamID,
    pub fields: Vec<(RedisString, RedisString)>,
}

pub struct StreamIterator<'key> {
    key: &'key RedisKey,
}

impl<'key> StreamIterator<'key> {
    pub(crate) fn new(
        key: &RedisKey,
        mut from: Option<raw::RedisModuleStreamID>,
        mut to: Option<raw::RedisModuleStreamID>,
        exclusive: bool,
    ) -> Result<StreamIterator, RedisError> {
        let flags = if exclusive {
            raw::REDISMODULE_STREAM_ITERATOR_EXCLUSIVE as i32
        } else {
            0
        };
        let res = unsafe {
            raw::RedisModule_StreamIteratorStart.unwrap()(
                key.key_inner,
                flags,
                from.as_mut().map_or(ptr::null_mut(), |v| v),
                to.as_mut().map_or(ptr::null_mut(), |v| v),
            )
        };
        if Status::Ok == res.into() {
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
        if Status::Ok
            != unsafe {
                raw::RedisModule_StreamIteratorNextID.unwrap()(
                    self.key.key_inner,
                    &mut id,
                    &mut num_fields,
                )
            }
            .into()
        {
            return None;
        }
        let mut fields = Vec::new();
        while Status::Ok
            == unsafe {
                raw::RedisModule_StreamIteratorNextField.unwrap()(
                    self.key.key_inner,
                    &mut field_name,
                    &mut field_val,
                )
                .into()
            }
        {
            fields.push((
                RedisString::from_redis_module_string(self.key.ctx, field_name),
                RedisString::from_redis_module_string(self.key.ctx, field_val),
            ));
        }
        Some(StreamRecord { id, fields })
    }
}

impl<'key> Drop for StreamIterator<'key> {
    fn drop(&mut self) {
        unsafe { raw::RedisModule_StreamIteratorDelete.unwrap()(self.key.key_inner) };
    }
}
