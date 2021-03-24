use anyhow::{Context, Result};

use redis::Connection;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

const LOG_DIR: &str = "tests/log";
const REDIS_PORT: u16 = 6666;

/// Ensure child process is killed both on normal exit and when panicking due to a failed test.
pub struct ChildGuard {
    name: &'static str,
    child: std::process::Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Err(e) = self.child.kill() {
            println!("Could not kill {}: {}", self.name, e)
        }
        if let Err(e) = self.child.wait() {
            println!("Could not wait for {}: {}", self.name, e)
        }
    }
}

pub fn prepare_dirs() -> Result<()> {
    let dirs = &[LOG_DIR];

    for dir in dirs {
        remove_tree(dir).context(dir)?;
        fs::create_dir(dir).context(dir)?;
    }

    Ok(())
}

fn remove_tree(dir: &str) -> Result<()> {
    match fs::remove_dir_all(dir) {
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
        res => res.with_context(|| dir.to_string()),
    }
}

pub fn start_redis_server_with_module(module_name: &str) -> Result<ChildGuard> {
    let extension = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };

    let module_path: PathBuf = [
        std::env::current_dir()?,
        PathBuf::from(format!(
            "target/debug/examples/lib{}.{}",
            module_name, extension
        )),
    ]
    .iter()
    .collect();

    assert!(fs::metadata(&module_path)
        .with_context(|| format!("Loading redis module: {}", module_path.display()))?
        .is_file());

    let module_path = format!("{}", module_path.display());
    let port = REDIS_PORT.to_string();

    #[rustfmt::skip]
        let args = &[
        "--port", port.as_str(),
        "--loadmodule", module_path.as_str(),
        "--dir", LOG_DIR,
        "--logfile", "redis.log",
    ];

    let redis_server = Command::new("redis-server")
        .args(args)
        .spawn()
        .map(|c| ChildGuard {
            name: "redis_server",
            child: c,
        })?;

    Ok(redis_server)
}

// Get connection to Redis
pub fn get_redis_connection() -> Result<Connection> {
    let client = redis::Client::open(format!("redis://127.0.0.1:{}/", REDIS_PORT))?;
    loop {
        let res = client.get_connection();
        match res {
            Ok(con) => return Ok(con),
            Err(e) => {
                if e.is_connection_refusal() {
                    // Redis not ready yet, sleep and retry
                    std::thread::sleep(Duration::from_millis(50));
                } else {
                    Err(e)?;
                }
            }
        }
    }
}
