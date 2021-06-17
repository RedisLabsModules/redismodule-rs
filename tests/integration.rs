mod utils;

use anyhow::Result;
use utils::{get_redis_connection, prepare_dirs, start_redis_server_with_module};

#[test]
#[ignore] // test will be run explicitly by a script
fn test_hello() -> Result<()> {
    prepare_dirs()?;

    let _guards = vec![start_redis_server_with_module("hello")?];
    let mut con = get_redis_connection()?;

    let res: Vec<i32> = redis::cmd("hello.mul").arg(&[3, 4]).query(&mut con)?;
    assert_eq!(res, vec![3, 4, 12]);

    Ok(())
}
