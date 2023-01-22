use crate::{RedisError, RedisString};

#[derive(Debug, PartialEq)]
pub enum RedisValue {
    SimpleStringStatic(&'static str),
    SimpleString(String),
    BulkString(String),
    BulkRedisString(RedisString),
    StringBuffer(Vec<u8>),
    Integer(i64),
    Float(f64),
    Array(Vec<RedisValue>),
    Null,
    NoReply, // No reply at all (as opposed to a Null reply)
}

impl From<RedisValue> for Result<String, RedisError> {
    fn from(val: RedisValue) -> Self {
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
        s.map_or(Self::Null, |v| v.into())
    }
}

impl<T: Into<Self>> From<Vec<T>> for RedisValue {
    fn from(items: Vec<T>) -> Self {
        Self::Array(items.into_iter().map(Into::into).collect())
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
