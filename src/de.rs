use serde::de::{self, DeserializeSeed, SeqAccess, Visitor};
use serde::{forward_to_deserialize_any, Deserialize};
use std::collections::VecDeque;

use crate::key::RedisKey;
use crate::RedisError;
use crate::RedisString;

pub type Result<T> = std::result::Result<T, RedisError>;

pub struct Deserializer<'de> {
    key: &'de RedisKey,
    values: VecDeque<RedisString>,
}

impl<'de> Deserializer<'de> {
    pub fn from_hash(key: &'de RedisKey) -> Self {
        let values = VecDeque::new();

        Deserializer { key, values }
    }
}

pub fn from_hash<'a, T>(key: &'a RedisKey) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_hash(key);
    let t = T::deserialize(&mut deserializer)?;

    if !deserializer.values.is_empty() {
        return Err(RedisError::Str("Too many values returned"));
    }

    Ok(t)
}

impl de::Error for RedisError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        RedisError::String(msg.to_string())
    }
}

impl<'de> Deserializer<'de> {
    fn next_redis_string(&mut self) -> Result<RedisString> {
        let s = self
            .values
            .pop_front()
            .ok_or(RedisError::Str("missing value"))?;

        Ok(s)
    }
}

//noinspection RsSortImplTraitMembers
impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = RedisError;

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.values.extend(self.key.hash_get_multi(fields)?);
        self.deserialize_seq(visitor)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(RedisHashSeqAccess::new(&mut self))
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let s = self.next_redis_string()?;
        visitor.visit_string(s.try_as_str()?.to_string())
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let s = self.next_redis_string()?;
        visitor.visit_str(s.try_as_str()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let s = self.next_redis_string()?;
        visitor.visit_u32(s.try_as_str()?.parse()?)
    }

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u64 u128 f32 f64 char
        bytes byte_buf option unit unit_struct newtype_struct tuple
        tuple_struct map enum identifier ignored_any
    }
}

///////////////////////////////////////////////

struct RedisHashSeqAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> RedisHashSeqAccess<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> Self {
        Self { de }
    }
}

impl<'a, 'de> SeqAccess<'de> for RedisHashSeqAccess<'a, 'de> {
    type Error = RedisError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.de.values.is_empty() {
            return Ok(None);
        }

        seed.deserialize(&mut *self.de).map(Some)
    }
}

///////////////////////////////////////////////

/*
#[test]
fn test_struct() {
    #[derive(Debug, Deserialize)]
    struct Config {
        hostname: String,
        port: u32,
    }

    let c: Config = from_hash(&RedisKey).unwrap();
    println!("{:?}", c);
}
*/
