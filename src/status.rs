use crate::{RedisError, RedisResult};
use enum_primitive_derive::Primitive;

const GENERIC_ERROR_MESSAGE: &str = "Generic error.";

#[derive(Primitive, Debug, PartialEq, Eq)]
pub enum Status {
    Ok = crate::raw::REDISMODULE_OK,
    Err = crate::raw::REDISMODULE_ERR,
}

impl From<Status> for RedisResult<()> {
    fn from(value: Status) -> Self {
        match value {
            Status::Ok => Ok(()),
            Status::Err => Err(RedisError::Str(GENERIC_ERROR_MESSAGE)),
        }
    }
}

impl From<Status> for Result<(), &str> {
    fn from(s: Status) -> Self {
        match s {
            Status::Ok => Ok(()),
            Status::Err => Err(GENERIC_ERROR_MESSAGE),
        }
    }
}
