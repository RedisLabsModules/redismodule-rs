use crate::RedisString;

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

impl From<()> for RedisValue {
    fn from(_: ()) -> Self {
        RedisValue::Null
    }
}

impl From<i64> for RedisValue {
    fn from(i: i64) -> Self {
        RedisValue::Integer(i)
    }
}

impl From<usize> for RedisValue {
    fn from(i: usize) -> Self {
        (i as i64).into()
    }
}

impl From<f64> for RedisValue {
    fn from(f: f64) -> Self {
        RedisValue::Float(f)
    }
}

impl From<String> for RedisValue {
    fn from(s: String) -> Self {
        RedisValue::BulkString(s)
    }
}

impl From<RedisString> for RedisValue {
    fn from(s: RedisString) -> Self {
        RedisValue::BulkRedisString(s)
    }
}

impl From<Vec<u8>> for RedisValue {
    fn from(s: Vec<u8>) -> Self {
        RedisValue::StringBuffer(s)
    }
}

impl From<&RedisString> for RedisValue {
    fn from(s: &RedisString) -> Self {
        s.to_owned().into()
    }
}

impl From<&str> for RedisValue {
    fn from(s: &str) -> Self {
        s.to_owned().into()
    }
}

impl From<&String> for RedisValue {
    fn from(s: &String) -> Self {
        s.to_owned().into()
    }
}

impl<T: Into<RedisValue>> From<Option<T>> for RedisValue {
    fn from(s: Option<T>) -> Self {
        match s {
            Some(v) => v.into(),
            None => RedisValue::Null,
        }
    }
}

impl<T: Into<RedisValue>> From<Vec<T>> for RedisValue {
    fn from(items: Vec<T>) -> Self {
        RedisValue::Array(items.into_iter().map(|item| item.into()).collect())
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
    fn from_option_none() {
        assert_eq!(RedisValue::from(None::<()>), RedisValue::Null,);
    }
}
