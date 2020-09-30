use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    hostname: String,
    port: u32,
}

fn main() {
    let c = Config {
        hostname: "".to_string(),
        port: 0,
    };
    println!("{:?}", c);
}
