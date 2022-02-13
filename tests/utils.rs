use anyhow::{Context, Result};

use redis::Connection;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

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

pub fn start_redis_server_with_module(module_name: &str, port: u16) -> Result<ChildGuard> {
    let extension = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };

    let mut profile = "debug";
    if cfg!(not(debug_assertions)) {
        profile = "release";
    }
    let module_path: PathBuf = [
        std::env::current_dir()?,
        PathBuf::from(format!(
            "target/{}/examples/lib{}.{}",
            profile, module_name, extension
        )),
    ]
    .iter()
    .collect();

    assert!(fs::metadata(&module_path)
        .with_context(|| format!("Loading redis module: {}", module_path.display()))?
        .is_file());

    let module_path = format!("{}", module_path.display());

    let args = &[
        "--port",
        &port.to_string(),
        "--loadmodule",
        module_path.as_str(),
    ];

    let redis_server = Command::new("redis-server-6.2.5")
        .args(args)
        .spawn()
        .map(|c| ChildGuard {
            name: "redis-server-6.2.5",
            child: c,
        })?;

    Ok(redis_server)
}

// Get connection to Redis
pub fn get_redis_connection(port: u16) -> Result<Connection> {
    let client = redis::Client::open(format!("redis://127.0.0.1:{}/", port))?;
    loop {
        let res = client.get_connection();
        match res {
            Ok(con) => return Ok(con),
            Err(e) => {
                if e.is_connection_refusal() {
                    // Redis not ready yet, sleep and retry
                    std::thread::sleep(Duration::from_millis(50));
                } else {
                    return Err(e.into());
                }
            }
        }
    }
}
