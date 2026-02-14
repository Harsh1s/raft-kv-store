//! Additional tests for the S3-compatible API
use reqwest::Client;
use std::fs;
use std::net::TcpListener;
use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};
use uuid::Uuid;

fn get_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn start_coord(http_port: u16, grpc_port: u16, test_id: &str) -> (Child, String) {
    let coord_bin =
        std::env::var("CARGO_BIN_EXE_minikv-coord").expect("CARGO_BIN_EXE_minikv-coord not set");
    let coord_data = format!("coord-s3extra-data-{}", test_id);
    let _ = fs::remove_dir_all(&coord_data);
    let _ = fs::create_dir_all(&coord_data);
    let config_path = format!("/tmp/minikv-config-{}.toml", test_id);
    fs::write(
        &config_path,
        format!(
            "node_id = 'coord-s3extra-{}'\nrole = 'coordinator'\nreplicas = 1\n",
            test_id
        ),
    )
    .expect("Failed to write config.toml");
    let mut cmd = Command::new(coord_bin);
    cmd.args([
        "serve",
        "--id",
        &format!("coord-s3extra-{}", test_id),
        "--bind",
        &format!("127.0.0.1:{}", http_port),
        "--grpc",
        &format!("127.0.0.1:{}", grpc_port),
        "--db",
        &coord_data,
    ]);
    cmd.env_clear();
    for (key, value) in std::env::vars() {
        if key != "MINIKV_CONFIG" && key != "RUST_LOG" && key != "RUST_BACKTRACE" {
            cmd.env(&key, &value);
        }
    }
    cmd.env("MINIKV_CONFIG", &config_path);
    cmd.env("RUST_LOG", "debug");
    cmd.env("RUST_BACKTRACE", "1");
    let log_path = format!("coord-s3extra-{}.log", test_id);
    let log = fs::File::create(&log_path).expect("Failed to create log file");
    let log_err = log.try_clone().expect("Failed to clone log file");
    cmd.stdout(Stdio::from(log));
    cmd.stderr(Stdio::from(log_err));
    (
        cmd.spawn().expect("Failed to launch minikv-coord server"),
        coord_data,
    )
}

fn start_volume(
    http_port: u16,
    grpc_port: u16,
    coord_http_port: u16,
    test_id: &str,
) -> (Child, String, String) {
    let volume_bin =
        std::env::var("CARGO_BIN_EXE_minikv-volume").expect("CARGO_BIN_EXE_minikv-volume not set");
    let vol_data = format!("vol-s3extra-data-{}", test_id);
    let vol_wal = format!("vol-s3extra-wal-{}", test_id);
    let _ = fs::remove_dir_all(&vol_data);
    let _ = fs::remove_dir_all(&vol_wal);
    let _ = fs::create_dir_all(&vol_data);
    let _ = fs::create_dir_all(&vol_wal);
    let mut cmd = Command::new(volume_bin);
    cmd.args([
        "serve",
        "--id",
        &format!("vol-s3extra-{}", test_id),
        "--bind",
        &format!("127.0.0.1:{}", http_port),
