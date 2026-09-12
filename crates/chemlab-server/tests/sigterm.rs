//! systemd stops the unit with SIGTERM; the process must exit cleanly.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

fn free_bind_addr() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local addr");
    drop(listener);
    addr.to_string()
}

fn wait_until_listening(addr: &str, child: &mut std::process::Child) {
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        if TcpStream::connect(addr).is_ok() {
            return;
        }
        if let Some(status) = child.try_wait().expect("try_wait") {
            panic!("chemlab-server exited before listening: {status}");
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("timed out waiting for chemlab-server on {addr}");
        }
        thread::sleep(Duration::from_millis(30));
    }
}

fn health_ok(addr: &str) -> bool {
    let mut stream = match TcpStream::connect(addr) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
    if stream
        .write_all(b"GET /api/health HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .is_err()
    {
        return false;
    }
    let mut buf = String::new();
    let _ = stream.read_to_string(&mut buf);
    buf.starts_with("HTTP/1.1 200")
}

#[test]
fn server_exits_cleanly_on_sigterm() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("chemlab.db");
    let bind = free_bind_addr();

    let mut child = Command::new(env!("CARGO_BIN_EXE_chemlab-server"))
        .env("CHEMLAB_BIND", &bind)
        .env("CHEMLAB_DATABASE_URL", format!("sqlite://{}", db.display()))
        .env("RUST_LOG", "error")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn chemlab-server");

    wait_until_listening(&bind, &mut child);
    assert!(health_ok(&bind), "expected /api/health 200 before SIGTERM");

    let pid = child.id();
    let kill = Command::new("kill")
        .args(["-s", "TERM", &pid.to_string()])
        .status()
        .expect("send SIGTERM");
    assert!(kill.success(), "kill -s TERM failed");

    let deadline = Instant::now() + Duration::from_secs(8);
    let exit = loop {
        if let Some(status) = child.try_wait().expect("try_wait after SIGTERM") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            panic!("chemlab-server did not exit after SIGTERM");
        }
        thread::sleep(Duration::from_millis(20));
    };

    assert!(
        exit.success(),
        "expected exit code 0 after SIGTERM (graceful shutdown), got {exit}"
    );
}
