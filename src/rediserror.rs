use std::fmt;

#[derive(Debug)]
pub enum RedisError {
    WrongArity,
    Str(&'static str),
    String(String),
}

impl RedisError {
    pub fn nonexistent_key() -> Self {
        RedisError::Str("ERR could not perform this operation on a key that doesn't exist")
    }
}

impl<T: std::error::Error> From<T> for RedisError {
    fn from(e: T) -> Self {
        RedisError::String(e.to_string())
    }
}

impl fmt::Display for RedisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let d = match self {
            RedisError::WrongArity => "Wrong Arity",
            RedisError::Str(s) => s,
            RedisError::String(s) => s.as_str(),
        };

        write!(f, "{}", d)
    }
}
