#[macro_use]
extern crate redismodule;

use redismodule::redisearch;
use redismodule::{Context, NextArg, RedisError, RedisResult};

fn hello_redisearch(_: &Context, args: Vec<String>) -> RedisResult {
    let mut args = args.into_iter().skip(1);
    let search_term = args.next_string()?;

    redisearch::initialize()?;

    // FT.CREATE my_index
    //     SCHEMA
    //         $a.b.c TEXT WEIGHT 5.0
    //         $a.b.d TEXT
    //         $b.a TEXT

    // see RediSearch: t_llapi.cpp

    let index_name = "index";
    let field_name = "foo";
    let score = 1.0;

    let index = redisearch::create_index(index_name).unwrap();
    redisearch::create_text_field(field_name, index);

    let doc = redisearch::create_document("doc1", score).unwrap();
    redisearch::document_add_text_field(field_name, "bar", doc);
    redisearch::spec_add_document(index, doc);

    let doc2 = redisearch::create_document("doc2", score).unwrap();
    redisearch::document_add_text_field(field_name, "quux", doc2);
    redisearch::spec_add_document(index, doc2);

    let results = redisearch::search(index, search_term.as_str(), field_name);

    Ok(results.into())
}

redis_module! {
    name: "hello_redisearch",
    version: 1,
    data_types: [],
    commands: [
        ["hello_redisearch", hello_redisearch, ""],
    ],
}
