//! Test the dashboard endpoint /admin/status

use reqwest::Client;
use serde_json::Value;
use std::env;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

/// Find a free TCP port on localhost
fn get_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Launch the minikv-coord server in the background, returns (Child, http_port, grpc_port)
fn start_server() -> (Child, u16, u16) {
    let http_port = get_free_port();
    let grpc_port = get_free_port();
    let _ = std::fs::remove_dir_all("coord-test-data");
    let _ = std::fs::create_dir_all("coord-test-data");
    std::fs::write(
        "config.toml",
        "node_id = 'coord-test'\nrole = 'coordinator'\n",
    )
    .expect("Failed to write config.toml");
    let mut cmd = Command::new(
        env::var("CARGO_BIN_EXE_minikv-coord")
            .expect("CARGO_BIN_EXE_minikv-coord not set by cargo test"),
    );
    cmd.args([
        "serve",
