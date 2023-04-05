use crate::{context::call_reply::CallResult, CallReply, RedisError, RedisString};
use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum RedisValueKey {
    Integer(i64),
    String(String),
    BulkRedisString(RedisString),
    BulkString(Vec<u8>),
    Bool(bool),
}

#[derive(Debug, PartialEq, Clone)]
pub enum RedisValue {
    SimpleStringStatic(&'static str),
    SimpleString(String),
    BulkString(String),
    BulkRedisString(RedisString),
    StringBuffer(Vec<u8>),
    Integer(i64),
    Bool(bool),
    Float(f64),
    BigNumber(String),
    VerbatimString((String, Vec<u8>)),
    Array(Vec<RedisValue>),
    StaticError(&'static str),
    Map(HashMap<RedisValueKey, RedisValue>),
    Set(HashSet<RedisValueKey>),
    Null,
    NoReply, // No reply at all (as opposed to a Null reply)
}

impl TryFrom<RedisValue> for String {
    type Error = RedisError;
    fn try_from(val: RedisValue) -> Result<Self, RedisError> {
        match val {
            RedisValue::SimpleStringStatic(s) => Ok(s.to_string()),
            RedisValue::SimpleString(s) => Ok(s),
            RedisValue::BulkString(s) => Ok(s),
            RedisValue::BulkRedisString(s) => Ok(s.try_as_str()?.to_string()),
            RedisValue::StringBuffer(s) => Ok(std::str::from_utf8(&s)?.to_string()),
            _ => Err(RedisError::Str("Can not convert result to String")),
        }
    }
}

impl From<()> for RedisValue {
    fn from(_: ()) -> Self {
        Self::Null
    }
}

impl From<i64> for RedisValue {
    fn from(i: i64) -> Self {
        Self::Integer(i)
    }
}

impl From<usize> for RedisValue {
    fn from(i: usize) -> Self {
        (i as i64).into()
    }
}

impl From<f64> for RedisValue {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<String> for RedisValue {
    fn from(s: String) -> Self {
        Self::BulkString(s)
    }
}

impl From<RedisString> for RedisValue {
    fn from(s: RedisString) -> Self {
        Self::BulkRedisString(s)
    }
}

impl From<Vec<u8>> for RedisValue {
    fn from(s: Vec<u8>) -> Self {
        Self::StringBuffer(s)
    }
}

impl From<&RedisString> for RedisValue {
    fn from(s: &RedisString) -> Self {
        s.clone().into()
    }
}

impl From<&str> for RedisValue {
    fn from(s: &str) -> Self {
        s.to_owned().into()
    }
}

impl From<&String> for RedisValue {
    fn from(s: &String) -> Self {
        s.clone().into()
    }
}

impl<T: Into<Self>> From<Option<T>> for RedisValue {
    fn from(s: Option<T>) -> Self {
        s.map_or(Self::Null, Into::into)
    }
}

impl<T: Into<Self>> From<Vec<T>> for RedisValue {
    fn from(items: Vec<T>) -> Self {
        Self::Array(items.into_iter().map(Into::into).collect())
    }
}

impl<'root> TryFrom<&CallReply<'root>> for RedisValueKey {
    type Error = RedisError;
    fn try_from(reply: &CallReply<'root>) -> Result<Self, Self::Error> {
        match reply {
            CallReply::I64(reply) => Ok(RedisValueKey::Integer(reply.to_i64())),
            CallReply::String(reply) => Ok(reply
                .to_string()
                .map_or(RedisValueKey::BulkString(reply.as_bytes().to_vec()), |v| {
                    RedisValueKey::String(v)
                })),
            CallReply::Bool(b) => Ok(RedisValueKey::Bool(b.to_bool())),
            _ => Err(RedisError::String(format!(
                "Given CallReply can not be used as a map key or a set element, {:?}",
                reply
            ))),
        }
    }
}

impl<'root> From<&CallReply<'root>> for RedisValue {
    fn from(reply: &CallReply<'root>) -> Self {
        match reply {
            CallReply::Unknown => RedisValue::StaticError("Error on method call"),
            CallReply::Array(reply) => {
                RedisValue::Array(reply.iter().map(|v| (&v).into()).collect())
            }
            CallReply::I64(reply) => RedisValue::Integer(reply.to_i64()),
            CallReply::String(reply) => RedisValue::SimpleString(reply.to_string().unwrap()),
            CallReply::Null(_) => RedisValue::Null,
            CallReply::Map(reply) => RedisValue::Map(
                reply
                    .iter()
                    .map(|(key, val)| {
                        (
                            (&key)
                                .try_into()
                                .expect(&format!("Got unhashable map key from Redis, {:?}", key)),
                            (&val).into(),
                        )
                    })
                    .collect(),
            ),
            CallReply::Set(reply) => RedisValue::Set(
                reply
                    .iter()
                    .map(|v| {
                        (&v).try_into()
                            .expect(&format!("Got unhashable set element from Redis, {:?}", v))
                    })
                    .collect(),
            ),
            CallReply::Bool(reply) => RedisValue::Bool(reply.to_bool()),
            CallReply::Double(reply) => RedisValue::Float(reply.to_double()),
            CallReply::BigNumber(reply) => RedisValue::BigNumber(reply.to_string().unwrap()),
            CallReply::VerbatimString(reply) => {
                RedisValue::VerbatimString(reply.to_parts().unwrap())
            }
        }
    }
}

impl<'root> From<&CallResult<'root>> for RedisValue {
    fn from(reply: &CallResult<'root>) -> Self {
        reply.as_ref().map_or_else(
            |e| {
                // RedisValue does not support error, we can change that but to avoid.
                // drastic changes and try to keep backword compatability, currently
                // we will stansform the error into a simple String.
                RedisValue::SimpleString(e.to_string().unwrap())
            },
            |v| (v).into(),
        )
    }
}

impl<'root> TryFrom<&CallResult<'root>> for RedisValueKey {
    type Error = RedisError;
    fn try_from(reply: &CallResult<'root>) -> Result<Self, Self::Error> {
        reply.as_ref().map_or_else(
            |e| {
                Err(RedisError::String(
                    format!("Got an error reply which can not be translated into a map key or set element, {:?}", e),
                ))
            },
            |v| v.try_into(),
        )
    }
}

//////////////////////////////////////////////////////////

#[cfg(test)]
mod tests {
    use super::RedisValue;

    #[test]
    fn from_vec_string() {
        assert_eq!(
            RedisValue::from(vec!["foo".to_string()]),
            RedisValue::Array(vec![RedisValue::BulkString("foo".to_owned())])
        );
    }

    #[test]
    fn from_vec_str() {
        assert_eq!(
            RedisValue::from(vec!["foo"]),
            RedisValue::Array(vec![RedisValue::BulkString("foo".to_owned())])
        );
    }

    #[test]
    fn from_vec_string_ref() {
        assert_eq!(
            RedisValue::from(vec![&"foo".to_string()]),
            RedisValue::Array(vec![RedisValue::BulkString("foo".to_owned())])
        );
    }

    #[test]
    fn from_option_str() {
        assert_eq!(
            RedisValue::from(Some("foo")),
            RedisValue::BulkString("foo".to_owned())
        );
    }

    #[test]
    fn from_vec() {
        let v: Vec<u8> = vec![0, 3, 5, 21, 255];
        assert_eq!(
            RedisValue::from(v),
            RedisValue::StringBuffer(vec![0, 3, 5, 21, 255])
        );
    }

    #[test]
    fn from_option_none() {
        assert_eq!(RedisValue::from(None::<()>), RedisValue::Null,);
    }
}
