use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
pub enum AclCategory {
    #[default]
    None,
    Keyspace,
    Read,
    Write,
    Set,
    SortedSet,
    List,
    Hash,
    String,
    Bitmap,
    HyperLogLog,
    Geo,
    Stream,
    PubSub,
    Admin,
    Fast,
    Slow,
    Blocking,
    Dangerous,
    Connection,
    Transaction,
    Scripting,
    Single(String),
    Multi(Vec<AclCategory>),
}

impl From<Vec<AclCategory>> for AclCategory {
    fn from(value: Vec<AclCategory>) -> Self {
        AclCategory::Multi(value)
    }
}

impl From<&str> for AclCategory {
    fn from(value: &str) -> Self {
        match value {
            "" => AclCategory::None,
            "keyspace" => AclCategory::Keyspace,
            "read" => AclCategory::Read,
            "write" => AclCategory::Write,
            "set" => AclCategory::Set,
            "sortedset" => AclCategory::SortedSet,
            "list" => AclCategory::List,
            "hash" => AclCategory::Hash,
            "string" => AclCategory::String,
            "bitmap" => AclCategory::Bitmap,
            "hyperloglog" => AclCategory::HyperLogLog,
            "geo" => AclCategory::Geo,
            "stream" => AclCategory::Stream,
            "pubsub" => AclCategory::PubSub,
            "admin" => AclCategory::Admin,
            "fast" => AclCategory::Fast,
            "slow" => AclCategory::Slow,
            "blocking" => AclCategory::Blocking,
            "dangerous" => AclCategory::Dangerous,
            "connection" => AclCategory::Connection,
            "transaction" => AclCategory::Transaction,
            "scripting" => AclCategory::Scripting,
            _ if !value.contains(" ") => AclCategory::Single(value.to_string()),
            _ => AclCategory::Multi(value.split_whitespace().map(AclCategory::from).collect()),
        }
    }
}

impl From<AclCategory> for String {
    fn from(value: AclCategory) -> Self {
        match value {
            AclCategory::None => "".to_string(),
            AclCategory::Keyspace => "keyspace".to_string(),
            AclCategory::Read => "read".to_string(),
            AclCategory::Write => "write".to_string(),
            AclCategory::Set => "set".to_string(),
            AclCategory::SortedSet => "sortedset".to_string(),
            AclCategory::List => "list".to_string(),
            AclCategory::Hash => "hash".to_string(),
            AclCategory::String => "string".to_string(),
            AclCategory::Bitmap => "bitmap".to_string(),
            AclCategory::HyperLogLog => "hyperloglog".to_string(),
            AclCategory::Geo => "geo".to_string(),
            AclCategory::Stream => "stream".to_string(),
            AclCategory::PubSub => "pubsub".to_string(),
            AclCategory::Admin => "admin".to_string(),
            AclCategory::Fast => "fast".to_string(),
            AclCategory::Slow => "slow".to_string(),
            AclCategory::Blocking => "blocking".to_string(),
            AclCategory::Dangerous => "dangerous".to_string(),
            AclCategory::Connection => "connection".to_string(),
            AclCategory::Transaction => "transaction".to_string(),
            AclCategory::Scripting => "scripting".to_string(),
            AclCategory::Single(s) => s,
            AclCategory::Multi(v) => v
                .into_iter()
                .map(String::from)
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

impl std::fmt::Display for AclCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", String::from(self.clone()))
    }
}

