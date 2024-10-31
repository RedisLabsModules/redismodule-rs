use anyhow::{Context, Result};

use redis::Connection;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::AtomicU16;
use std::time::Duration;

/// Starts a redis instance with the module provided as a module name
/// and a port, returns the connection guards (`ChildGuard`) through
/// which the redis instance can be interacted with.
pub fn start_redis(module_name: &str, port: u16) -> Result<Vec<ChildGuard>, &'static str> {
    Ok(vec![start_redis_server_with_module(module_name, port)
        .map_err(|_| "failed to start redis server")?])
}

pub struct TestConnection {
    _guards: Vec<ChildGuard>,
    connection: Connection,
}

static TEST_PORT: AtomicU16 = AtomicU16::new(6479);

impl TestConnection {
    /// Creates a new connection to a Redis server with the module
    /// provided as a module name.
    pub fn new(module_name: &str) -> Self {
        let port = TEST_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        Self {
            _guards: start_redis(module_name, port).expect("Redis instance started."),
            connection: get_redis_connection(port).expect("Established connection to server."),
        }
    }
}

impl std::ops::Deref for TestConnection {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}

impl std::ops::DerefMut for TestConnection {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

/// Ensure child process is killed both on normal exit and when panicking due to a failed test.
#[derive(Debug)]
pub struct ChildGuard {
    name: &'static str,
    child: std::process::Child,
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Err(e) = self.child.kill() {
            println!("Could not kill {}: {e}", self.name);
        }
        if let Err(e) = self.child.wait() {
            println!("Could not wait for {}: {e}", self.name);
        }
    }
}

pub fn start_redis_server_with_module(module_name: &str, port: u16) -> Result<ChildGuard> {
    let extension = if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    };

    let profile = if cfg!(not(debug_assertions)) {
        "release"
    } else {
        "debug"
    };

    let module_path: PathBuf = [
        std::env::current_dir()?,
        PathBuf::from(format!(
            "target/{profile}/examples/lib{module_name}.{extension}"
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
        "--enable-debug-command",
        "yes",
    ];

    let redis_server = Command::new("redis-server")
        .args(args)
        .spawn()
        .map(|c| ChildGuard {
            name: "redis-server",
            child: c,
        })?;

    Ok(redis_server)
}

// Get connection to Redis
pub fn get_redis_connection(port: u16) -> Result<Connection> {
    let client = redis::Client::open(format!("redis://127.0.0.1:{port}/"))?;
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
