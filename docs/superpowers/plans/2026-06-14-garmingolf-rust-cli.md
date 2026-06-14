# Garmin Golf Rust CLI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the JavaScript/Electron runtime with a smaller Rust library plus CLI daemon that exposes a selectable-port OpenAPI server and optional Nova-style WebSocket shot feed.

**Architecture:** Build a library-first Rust crate with small modules for config, core state, Garmin/E6 protocol, GSPro payloads, OpenAPI, Nova WebSocket broadcast, and CLI wiring. Preserve the existing Garmin/E6 TCP bridge behavior while trimming away desktop UI concerns.

**Tech Stack:** Rust 2021, Tokio, Axum 0.8, clap derive with env support, serde, serde_json, utoipa, utoipa-swagger-ui, futures-util, tokio-tungstenite, tower test utilities.

---

## File Structure

- Create `Cargo.toml`: Rust crate manifest and dependency set.
- Create `src/lib.rs`: public module exports.
- Create `src/bin/garmingolf-connector.rs`: CLI entrypoint.
- Create `src/config.rs`: `AppConfig`, `CliArgs`, env/default parsing, socket address helpers.
- Create `src/core.rs`: normalized launch data, status models, app state, and shot broadcast bus.
- Create `src/garmin/mod.rs`: Garmin module exports.
- Create `src/garmin/protocol.rs`: typed E6/Garmin messages, responses, shot-state assembly, spin-axis conversion.
- Create `src/garmin/runtime.rs`: TCP listener that accepts Garmin clients and publishes `ShotEvent`s.
- Create `src/gspro/mod.rs`: GSPro module exports.
- Create `src/gspro/payload.rs`: `ShotEvent` to GSPro Open API JSON conversion.
- Create `src/gspro/runtime.rs`: TCP client runtime that subscribes to shot events and forwards payloads.
- Create `src/api.rs`: Axum router, OpenAPI handlers, Swagger UI, and server binding.
- Create `src/nova.rs`: optional WebSocket broadcast server and Nova shot serialization.
- Create `tests/config_contract.rs`: config defaults, env, and CLI precedence.
- Create `tests/garmin_protocol_contract.rs`: Garmin/E6 parser and response contracts.
- Create `tests/gspro_payload_contract.rs`: GSPro payload conversion contract.
- Create `tests/api_contract.rs`: OpenAPI endpoint contract.
- Create `tests/nova_ws_contract.rs`: Nova WebSocket shot broadcast contract.
- Create `tests/garmin_runtime_contract.rs`: local TCP Garmin runtime smoke contract.
- Modify `README.md`: Rust CLI usage, port flags, API, and Nova WebSocket instructions.
- Keep existing JavaScript files in place during this implementation; do not delete them in this plan.

## References

- Existing Garmin/E6 behavior: `src/garminConnect.js`, `src/helpers/simMessages.js`.
- Existing GSPro payload behavior: `src/gsProConnect.js`.
- SquareGolf backend template: `/Users/shane/repos/squaregolf-connector/src-tauri/src`.
- Context7 docs consulted:
  - Axum 0.8: `Router::with_state`, `TcpListener::bind`, `axum::serve`, WebSocket upgrade, port `0` local address.
  - clap: derive `Parser`, long flags, defaults, and env override behavior.
  - utoipa: Axum Swagger UI merge pattern.

---

### Task 1: Rust Crate, Config, And CLI Skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/config.rs`
- Create: `src/bin/garmingolf-connector.rs`
- Test: `tests/config_contract.rs`

- [ ] **Step 1: Write the failing config contract test**

Create `tests/config_contract.rs`:

```rust
use garmingolf_connector::config::{AppConfig, CliArgs};
use clap::Parser;

#[test]
fn default_config_matches_existing_connector_ports() {
    let config = AppConfig::from_cli_and_env(
        CliArgs::try_parse_from(["garmingolf-connector"]).expect("valid defaults"),
        std::iter::empty::<(&str, &str)>(),
    )
    .expect("config");

    assert_eq!(config.garmin_host, "0.0.0.0");
    assert_eq!(config.garmin_port, 2483);
    assert_eq!(config.api_host, "127.0.0.1");
    assert_eq!(config.api_port, 5178);
    assert!(!config.gspro_enabled);
    assert_eq!(config.gspro_host, "127.0.0.1");
    assert_eq!(config.gspro_port, 921);
    assert!(!config.nova_ws_enabled);
    assert_eq!(config.nova_ws_host, "127.0.0.1");
    assert_eq!(config.nova_ws_port, 8765);
}

#[test]
fn env_values_override_defaults_and_cli_values_override_env() {
    let env = [
        ("GARMINGOLF_API_PORT", "6000"),
        ("GARMINGOLF_GARMIN_PORT", "2500"),
        ("GARMINGOLF_ENABLE_NOVA_WS", "true"),
        ("GARMINGOLF_NOVA_WS_PORT", "9900"),
    ];
    let args = CliArgs::try_parse_from([
        "garmingolf-connector",
        "--api-port",
        "7000",
        "--enable-gspro",
        "--gspro-host",
        "192.0.2.10",
    ])
    .expect("valid cli");

    let config = AppConfig::from_cli_and_env(args, env).expect("config");

    assert_eq!(config.api_port, 7000);
    assert_eq!(config.garmin_port, 2500);
    assert!(config.gspro_enabled);
    assert_eq!(config.gspro_host, "192.0.2.10");
    assert!(config.nova_ws_enabled);
    assert_eq!(config.nova_ws_port, 9900);
}

#[test]
fn socket_addresses_are_formed_from_host_and_port() {
    let config = AppConfig::from_cli_and_env(
        CliArgs::try_parse_from([
            "garmingolf-connector",
            "--garmin-host",
            "127.0.0.1",
            "--garmin-port",
            "0",
            "--api-port",
            "0",
        ])
        .expect("valid cli"),
        std::iter::empty::<(&str, &str)>(),
    )
    .expect("config");

    assert_eq!(config.garmin_addr().to_string(), "127.0.0.1:0");
    assert_eq!(config.api_addr().to_string(), "127.0.0.1:0");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```sh
cargo test --test config_contract
```

Expected: fails because `Cargo.toml` and the Rust modules do not exist yet.

- [ ] **Step 3: Add the Rust crate manifest**

Create `Cargo.toml`:

```toml
[package]
name = "garmingolf-connector"
version = "0.1.0"
description = "Rust CLI/library Garmin Golf launch monitor bridge"
edition = "2021"
rust-version = "1.77"
license = "MIT"
default-run = "garmingolf-connector"

[lib]
name = "garmingolf_connector"
path = "src/lib.rs"

[[bin]]
name = "garmingolf-connector"
path = "src/bin/garmingolf-connector.rs"

[dependencies]
axum = { version = "0.8", features = ["ws"] }
clap = { version = "4.5", features = ["derive", "env"] }
futures-util = "0.3"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["macros", "net", "rt-multi-thread", "signal", "sync", "time"] }
tokio-tungstenite = "0.26"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
utoipa = { version = "5", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "9", features = ["axum", "vendored"] }

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
```

- [ ] **Step 4: Add library exports**

Create `src/lib.rs`:

```rust
pub mod config;
```

- [ ] **Step 5: Add config parsing**

Create `src/config.rs`:

```rust
use std::net::SocketAddr;

use clap::Parser;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Parser)]
#[command(version, about = "Garmin Golf launch monitor bridge", long_about = None)]
pub struct CliArgs {
    #[arg(long, env = "GARMINGOLF_GARMIN_HOST", default_value = "0.0.0.0")]
    pub garmin_host: String,
    #[arg(long, env = "GARMINGOLF_GARMIN_PORT", default_value_t = 2483)]
    pub garmin_port: u16,
    #[arg(long, env = "GARMINGOLF_API_HOST", default_value = "127.0.0.1")]
    pub api_host: String,
    #[arg(long, env = "GARMINGOLF_API_PORT", default_value_t = 5178)]
    pub api_port: u16,
    #[arg(long, env = "GARMINGOLF_ENABLE_GSPRO", default_value_t = false)]
    pub enable_gspro: bool,
    #[arg(long, env = "GARMINGOLF_GSPRO_HOST", default_value = "127.0.0.1")]
    pub gspro_host: String,
    #[arg(long, env = "GARMINGOLF_GSPRO_PORT", default_value_t = 921)]
    pub gspro_port: u16,
    #[arg(long, env = "GARMINGOLF_ENABLE_NOVA_WS", default_value_t = false)]
    pub enable_nova_ws: bool,
    #[arg(long, env = "GARMINGOLF_NOVA_WS_HOST", default_value = "127.0.0.1")]
    pub nova_ws_host: String,
    #[arg(long, env = "GARMINGOLF_NOVA_WS_PORT", default_value_t = 8765)]
    pub nova_ws_port: u16,
}

impl CliArgs {
    pub fn parse_cli() -> Self {
        Self::parse()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub garmin_host: String,
    pub garmin_port: u16,
    pub api_host: String,
    pub api_port: u16,
    pub gspro_enabled: bool,
    pub gspro_host: String,
    pub gspro_port: u16,
    pub nova_ws_enabled: bool,
    pub nova_ws_host: String,
    pub nova_ws_port: u16,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        Self::from_cli_and_env(CliArgs::parse_cli(), std::env::vars())
    }

    pub fn from_cli_and_env<I, K, V>(args: CliArgs, env: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut config = Self {
            garmin_host: args.garmin_host,
            garmin_port: args.garmin_port,
            api_host: args.api_host,
            api_port: args.api_port,
            gspro_enabled: args.enable_gspro,
            gspro_host: args.gspro_host,
            gspro_port: args.gspro_port,
            nova_ws_enabled: args.enable_nova_ws,
            nova_ws_host: args.nova_ws_host,
            nova_ws_port: args.nova_ws_port,
        };

        for (key, value) in env {
            let key = key.as_ref();
            let value = value.as_ref();
            match key {
                "GARMINGOLF_GARMIN_PORT" if config.garmin_port == 2483 => {
                    config.garmin_port = parse_port(key, value)?;
                }
                "GARMINGOLF_API_PORT" if config.api_port == 5178 => {
                    config.api_port = parse_port(key, value)?;
                }
                "GARMINGOLF_ENABLE_NOVA_WS" if !config.nova_ws_enabled => {
                    config.nova_ws_enabled = parse_bool(key, value)?;
                }
                "GARMINGOLF_NOVA_WS_PORT" if config.nova_ws_port == 8765 => {
                    config.nova_ws_port = parse_port(key, value)?;
                }
                _ => {}
            }
        }

        Ok(config)
    }

    pub fn garmin_addr(&self) -> SocketAddr {
        format!("{}:{}", self.garmin_host, self.garmin_port)
            .parse()
            .expect("validated garmin socket address")
    }

    pub fn api_addr(&self) -> SocketAddr {
        format!("{}:{}", self.api_host, self.api_port)
            .parse()
            .expect("validated api socket address")
    }

    pub fn nova_ws_addr(&self) -> SocketAddr {
        format!("{}:{}", self.nova_ws_host, self.nova_ws_port)
            .parse()
            .expect("validated nova websocket socket address")
    }
}

fn parse_port(name: &str, value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|err| format!("{name} must be a valid u16 port: {err}"))
}

fn parse_bool(name: &str, value: &str) -> Result<bool, String> {
    value
        .parse::<bool>()
        .map_err(|err| format!("{name} must be true or false: {err}"))
}
```

- [ ] **Step 6: Add a compiling CLI entrypoint**

Create `src/bin/garmingolf-connector.rs`:

```rust
use garmingolf_connector::config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), String> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "garmingolf_connector=info,tower_http=info".into()),
        )
        .try_init();

    let config = AppConfig::from_env()?;
    println!("garmingolf-connector configured: api={}", config.api_addr());
    Ok(())
}
```

- [ ] **Step 7: Run the config contract**

Run:

```sh
cargo test --test config_contract
```

Expected: all tests pass.

- [ ] **Step 8: Commit**

Run:

```sh
git add Cargo.toml src/lib.rs src/config.rs src/bin/garmingolf-connector.rs tests/config_contract.rs
git commit -m "feat: scaffold rust connector config"
```

---

### Task 2: Core Shot Models And Runtime State

**Files:**
- Modify: `src/lib.rs`
- Create: `src/core.rs`
- Test: `tests/core_contract.rs`

- [ ] **Step 1: Write the failing core model test**

Create `tests/core_contract.rs`:

```rust
use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::{AppState, ClubMetrics, ConnectionStatus, ShotEvent};

#[tokio::test]
async fn app_state_records_last_shot_and_broadcasts_events() {
    let config = AppConfig {
        garmin_host: "127.0.0.1".into(),
        garmin_port: 0,
        api_host: "127.0.0.1".into(),
        api_port: 0,
        gspro_enabled: true,
        gspro_host: "127.0.0.1".into(),
        gspro_port: 921,
        nova_ws_enabled: true,
        nova_ws_host: "127.0.0.1".into(),
        nova_ws_port: 0,
    };
    let state = AppState::new(&config);
    let mut rx = state.subscribe_shots();

    let shot = ShotEvent::test_shot(7);
    state.publish_shot(shot.clone()).await;

    assert_eq!(state.status().await.last_shot.as_ref().unwrap().shot_number, 7);
    assert_eq!(rx.recv().await.expect("shot").shot_number, 7);
}

#[test]
fn test_shot_contains_ball_and_optional_club_metrics() {
    let shot = ShotEvent::test_shot(3);

    assert_eq!(shot.shot_number, 3);
    assert_eq!(shot.device_id, "Garmin R10");
    assert_eq!(shot.units, "Yards");
    assert_eq!(shot.club_type, "7Iron");
    assert_eq!(shot.ball.ball_speed, 98.5);
    assert!(matches!(shot.club, Some(ClubMetrics { speed: Some(110.0), .. })));
}

#[tokio::test]
async fn status_starts_disconnected() {
    let state = AppState::new(&AppConfig {
        garmin_host: "0.0.0.0".into(),
        garmin_port: 2483,
        api_host: "127.0.0.1".into(),
        api_port: 5178,
        gspro_enabled: false,
        gspro_host: "127.0.0.1".into(),
        gspro_port: 921,
        nova_ws_enabled: false,
        nova_ws_host: "127.0.0.1".into(),
        nova_ws_port: 8765,
    });

    let status = state.status().await;
    assert_eq!(status.garmin.connection_status, ConnectionStatus::Disconnected);
    assert_eq!(status.api_port, 5178);
    assert!(!status.gspro.enabled);
    assert!(!status.nova_ws.enabled);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```sh
cargo test --test core_contract
```

Expected: fails because `src/core.rs` does not define the models yet.

- [ ] **Step 3: Export the core module**

Replace `src/lib.rs`:

```rust
pub mod config;
pub mod core;
```

- [ ] **Step 4: Add core models and state**

Create `src/core.rs`:

```rust
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use utoipa::ToSchema;

use crate::config::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    Disconnected,
    Listening,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BallMetrics {
    pub ball_speed: f64,
    pub spin_axis: f64,
    pub total_spin: f64,
    pub hla: f64,
    pub vla: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClubMetrics {
    pub speed: Option<f64>,
    pub angle_of_attack: Option<f64>,
    pub face_to_target: Option<f64>,
    pub lie: Option<f64>,
    pub loft: Option<f64>,
    pub path: Option<f64>,
    pub speed_at_impact: Option<f64>,
    pub vertical_face_impact: Option<f64>,
    pub horizontal_face_impact: Option<f64>,
    pub closure_rate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShotEvent {
    pub shot_number: u64,
    pub device_id: String,
    pub units: String,
    pub club_type: String,
    pub ball: BallMetrics,
    pub club: Option<ClubMetrics>,
    pub raw_ball_data: Option<serde_json::Value>,
    pub raw_club_data: Option<serde_json::Value>,
}

impl ShotEvent {
    pub fn test_shot(shot_number: u64) -> Self {
        Self {
            shot_number,
            device_id: "Garmin R10".into(),
            units: "Yards".into(),
            club_type: "7Iron".into(),
            ball: BallMetrics {
                ball_speed: 98.5,
                spin_axis: -10.2,
                total_spin: 2350.2,
                hla: 0.0,
                vla: 13.5,
            },
            club: Some(ClubMetrics {
                speed: Some(110.0),
                angle_of_attack: Some(0.0),
                face_to_target: Some(-2.4),
                lie: Some(0.0),
                loft: Some(0.0),
                path: Some(-10.2),
                speed_at_impact: Some(110.0),
                vertical_face_impact: Some(0.0),
                horizontal_face_impact: Some(0.0),
                closure_rate: Some(0.0),
            }),
            raw_ball_data: None,
            raw_club_data: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EndpointStatus {
    pub enabled: bool,
    pub connection_status: ConnectionStatus,
    pub host: String,
    pub port: u16,
    pub last_error: Option<String>,
    pub last_shot_number: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GarminStatus {
    pub connection_status: ConnectionStatus,
    pub host: String,
    pub port: u16,
    pub active_client: Option<String>,
    pub last_error: Option<String>,
    pub malformed_message_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub api_port: u16,
    pub garmin: GarminStatus,
    pub gspro: EndpointStatus,
    pub nova_ws: EndpointStatus,
    pub last_shot: Option<ShotEvent>,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<AppStatus>>,
    shots: broadcast::Sender<ShotEvent>,
}

impl AppState {
    pub fn new(config: &AppConfig) -> Self {
        let (shots, _) = broadcast::channel(128);
        Self {
            inner: Arc::new(RwLock::new(AppStatus {
                api_port: config.api_port,
                garmin: GarminStatus {
                    connection_status: ConnectionStatus::Disconnected,
                    host: config.garmin_host.clone(),
                    port: config.garmin_port,
                    active_client: None,
                    last_error: None,
                    malformed_message_count: 0,
                },
                gspro: EndpointStatus {
                    enabled: config.gspro_enabled,
                    connection_status: ConnectionStatus::Disconnected,
                    host: config.gspro_host.clone(),
                    port: config.gspro_port,
                    last_error: None,
                    last_shot_number: None,
                },
                nova_ws: EndpointStatus {
                    enabled: config.nova_ws_enabled,
                    connection_status: ConnectionStatus::Disconnected,
                    host: config.nova_ws_host.clone(),
                    port: config.nova_ws_port,
                    last_error: None,
                    last_shot_number: None,
                },
                last_shot: None,
            })),
            shots,
        }
    }

    pub async fn status(&self) -> AppStatus {
        self.inner.read().await.clone()
    }

    pub fn subscribe_shots(&self) -> broadcast::Receiver<ShotEvent> {
        self.shots.subscribe()
    }

    pub async fn publish_shot(&self, shot: ShotEvent) {
        {
            let mut status = self.inner.write().await;
            status.last_shot = Some(shot.clone());
        }
        let _ = self.shots.send(shot);
    }

    pub async fn update_garmin<F>(&self, update: F)
    where
        F: FnOnce(&mut GarminStatus),
    {
        let mut status = self.inner.write().await;
        update(&mut status.garmin);
    }

    pub async fn update_gspro<F>(&self, update: F)
    where
        F: FnOnce(&mut EndpointStatus),
    {
        let mut status = self.inner.write().await;
        update(&mut status.gspro);
    }

    pub async fn update_nova<F>(&self, update: F)
    where
        F: FnOnce(&mut EndpointStatus),
    {
        let mut status = self.inner.write().await;
        update(&mut status.nova_ws);
    }
}
```

- [ ] **Step 5: Run the core contract**

Run:

```sh
cargo test --test core_contract
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

Run:

```sh
git add src/lib.rs src/core.rs tests/core_contract.rs
git commit -m "feat: add connector core state"
```

---

### Task 3: Garmin/E6 Protocol Parser And Responses

**Files:**
- Modify: `src/lib.rs`
- Create: `src/garmin/mod.rs`
- Create: `src/garmin/protocol.rs`
- Test: `tests/garmin_protocol_contract.rs`

- [ ] **Step 1: Write the failing Garmin protocol contract**

Create `tests/garmin_protocol_contract.rs`:

```rust
use garmingolf_connector::garmin::protocol::{
    ack_json, authentication_json, handshake_json, parse_incoming, ping_json, sim_command_json,
    GarminIncoming, ShotAssembler,
};

#[test]
fn handshake_and_authentication_match_existing_e6_bridge() {
    assert_eq!(
        handshake_json(),
        r#"{"Challenge":"gQW3om37uK4OOU4FXQH9GWgljxOrNcL5MvubVHAtQC0x6Z1AwJTgAIKyamJJMzm9","E6Version":"2, 0, 0, 0","ProtocolVersion":"1.0.0.5","RequiredProtocolVersion":"1.0.0.0","Type":"Handshake"}"#
    );
    assert_eq!(authentication_json(), r#"{"Success":"true","Type":"Authentication"}"#);
    assert_eq!(ack_json("SetBallData"), r#"{"Details":"Success.","SubType":"SetBallData","Type":"ACK"}"#);
    assert_eq!(sim_command_json("Arm"), r#"{"SubType":"Arm","Type":"SimCommand"}"#);
    assert_eq!(ping_json(), r#"{"SubType":"Ping","Type":"SimCommand"}"#);
}

#[test]
fn parses_known_incoming_message_types() {
    assert!(matches!(
        parse_incoming(r#"{"Type":"Handshake"}"#).expect("message"),
        GarminIncoming::Handshake
    ));
    assert!(matches!(
        parse_incoming(r#"{"Type":"SetClubType","ClubType":"Driver"}"#).expect("message"),
        GarminIncoming::SetClubType { club_type } if club_type == "Driver"
    ));
}

#[test]
fn assembles_shot_with_spin_axis_conversion_and_club_metrics() {
    let mut assembler = ShotAssembler::default();
    assembler.apply(parse_incoming(r#"{"Type":"SetClubType","ClubType":"7Iron"}"#).unwrap());
    assembler.apply(parse_incoming(r#"{"Type":"SetBallData","BallData":{"BallSpeed":151.58,"SpinAxis":353.3982,"TotalSpin":4721.59,"LaunchDirection":-5.0065,"LaunchAngle":17.7736}}"#).unwrap());
    assembler.apply(parse_incoming(r#"{"Type":"SetClubData","ClubData":{"ClubHeadSpeed":110.31,"ClubAngleFace":-2.42,"ClubAnglePath":-10.28}}"#).unwrap());

    let shot = assembler.build_shot(9).expect("shot");

    assert_eq!(shot.shot_number, 9);
    assert_eq!(shot.club_type, "7Iron");
    assert!((shot.ball.spin_axis - 6.6018).abs() < 0.001);
    assert_eq!(shot.ball.total_spin, 4721.59);
    assert_eq!(shot.club.unwrap().face_to_target, Some(-2.42));
}

#[test]
fn malformed_json_returns_error_instead_of_panicking() {
    let err = parse_incoming("{").expect_err("invalid json");
    assert!(err.contains("invalid Garmin JSON"));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```sh
cargo test --test garmin_protocol_contract
```

Expected: fails because the Garmin protocol module does not exist.

- [ ] **Step 3: Export the Garmin module**

Replace `src/lib.rs`:

```rust
pub mod config;
pub mod core;
pub mod garmin;
```

- [ ] **Step 4: Add Garmin module export**

Create `src/garmin/mod.rs`:

```rust
pub mod protocol;
```

- [ ] **Step 5: Add protocol parsing and shot assembly**

Create `src/garmin/protocol.rs`:

```rust
use serde::Deserialize;
use serde_json::{json, Value};

use crate::core::{BallMetrics, ClubMetrics, ShotEvent};

const HANDSHAKE_CHALLENGE: &str =
    "gQW3om37uK4OOU4FXQH9GWgljxOrNcL5MvubVHAtQC0x6Z1AwJTgAIKyamJJMzm9";

#[derive(Debug, Clone, PartialEq)]
pub enum GarminIncoming {
    Handshake,
    Challenge,
    Disconnect,
    Pong,
    SetClubType { club_type: String },
    SetBallData { data: GarminBallData, raw: Value },
    SetClubData { data: GarminClubData, raw: Value },
    SendShot,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GarminBallData {
    pub ball_speed: f64,
    pub spin_axis: f64,
    pub total_spin: f64,
    pub launch_direction: f64,
    pub launch_angle: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GarminClubData {
    pub club_head_speed: Option<f64>,
    pub club_angle_face: Option<f64>,
    pub club_angle_path: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(rename = "Type")]
    message_type: String,
}

pub fn parse_incoming(text: &str) -> Result<GarminIncoming, String> {
    let value: Value =
        serde_json::from_str(text).map_err(|err| format!("invalid Garmin JSON: {err}"))?;
    let envelope: Envelope =
        serde_json::from_value(value.clone()).map_err(|err| format!("invalid Garmin message: {err}"))?;

    match envelope.message_type.as_str() {
        "Handshake" => Ok(GarminIncoming::Handshake),
        "Challenge" => Ok(GarminIncoming::Challenge),
        "Disconnect" => Ok(GarminIncoming::Disconnect),
        "Pong" => Ok(GarminIncoming::Pong),
        "SetClubType" => Ok(GarminIncoming::SetClubType {
            club_type: required_string(&value, "ClubType")?,
        }),
        "SetBallData" => {
            let raw = value
                .get("BallData")
                .cloned()
                .ok_or_else(|| "SetBallData missing BallData".to_string())?;
            let data = serde_json::from_value(raw.clone())
                .map_err(|err| format!("invalid BallData: {err}"))?;
            Ok(GarminIncoming::SetBallData { data, raw })
        }
        "SetClubData" => {
            let raw = value
                .get("ClubData")
                .cloned()
                .ok_or_else(|| "SetClubData missing ClubData".to_string())?;
            let data = serde_json::from_value(raw.clone())
                .map_err(|err| format!("invalid ClubData: {err}"))?;
            Ok(GarminIncoming::SetClubData { data, raw })
        }
        "SendShot" => Ok(GarminIncoming::SendShot),
        other => Ok(GarminIncoming::Unknown(other.to_string())),
    }
}

pub fn handshake_json() -> String {
    json!({
        "Challenge": HANDSHAKE_CHALLENGE,
        "E6Version": "2, 0, 0, 0",
        "ProtocolVersion": "1.0.0.5",
        "RequiredProtocolVersion": "1.0.0.0",
        "Type": "Handshake"
    })
    .to_string()
}

pub fn authentication_json() -> String {
    json!({ "Success": "true", "Type": "Authentication" }).to_string()
}

pub fn ack_json(subtype: &str) -> String {
    json!({ "Details": "Success.", "SubType": subtype, "Type": "ACK" }).to_string()
}

pub fn sim_command_json(subtype: &str) -> String {
    json!({ "SubType": subtype, "Type": "SimCommand" }).to_string()
}

pub fn ping_json() -> String {
    sim_command_json("Ping")
}

#[derive(Debug, Clone)]
pub struct ShotAssembler {
    club_type: String,
    ball: Option<(GarminBallData, Value)>,
    club: Option<(GarminClubData, Value)>,
}

impl Default for ShotAssembler {
    fn default() -> Self {
        Self {
            club_type: "7Iron".into(),
            ball: None,
            club: None,
        }
    }
}

impl ShotAssembler {
    pub fn apply(&mut self, incoming: GarminIncoming) {
        match incoming {
            GarminIncoming::SetClubType { club_type } => self.club_type = club_type,
            GarminIncoming::SetBallData { data, raw } => self.ball = Some((data, raw)),
            GarminIncoming::SetClubData { data, raw } => self.club = Some((data, raw)),
            _ => {}
        }
    }

    pub fn build_shot(&self, shot_number: u64) -> Result<ShotEvent, String> {
        let (ball, raw_ball) = self
            .ball
            .clone()
            .ok_or_else(|| "cannot build shot without ball data".to_string())?;
        let (club, raw_club) = self.club.clone().unzip();

        Ok(ShotEvent {
            shot_number,
            device_id: "Garmin R10".into(),
            units: "Yards".into(),
            club_type: self.club_type.clone(),
            ball: BallMetrics {
                ball_speed: ball.ball_speed,
                spin_axis: convert_spin_axis(ball.spin_axis),
                total_spin: ball.total_spin,
                hla: ball.launch_direction,
                vla: ball.launch_angle,
            },
            club: club.map(|club| ClubMetrics {
                speed: club.club_head_speed,
                angle_of_attack: Some(0.0),
                face_to_target: club.club_angle_face,
                lie: Some(0.0),
                loft: Some(0.0),
                path: club.club_angle_path,
                speed_at_impact: club.club_head_speed,
                vertical_face_impact: Some(0.0),
                horizontal_face_impact: Some(0.0),
                closure_rate: Some(0.0),
            }),
            raw_ball_data: Some(raw_ball),
            raw_club_data: raw_club,
        })
    }
}

fn convert_spin_axis(mut spin_axis: f64) -> f64 {
    if spin_axis > 90.0 {
        spin_axis -= 360.0;
    }
    spin_axis * -1.0
}

fn required_string(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("missing string field {field}"))
}
```

- [ ] **Step 6: Run the Garmin protocol contract**

Run:

```sh
cargo test --test garmin_protocol_contract
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

Run:

```sh
git add src/lib.rs src/garmin/mod.rs src/garmin/protocol.rs tests/garmin_protocol_contract.rs
git commit -m "feat: add garmin e6 protocol contracts"
```

---

### Task 4: GSPro Payload Conversion

**Files:**
- Modify: `src/lib.rs`
- Create: `src/gspro/mod.rs`
- Create: `src/gspro/payload.rs`
- Test: `tests/gspro_payload_contract.rs`

- [ ] **Step 1: Write the failing GSPro payload contract**

Create `tests/gspro_payload_contract.rs`:

```rust
use garmingolf_connector::core::ShotEvent;
use garmingolf_connector::gspro::payload::GsProShotPayload;

#[test]
fn converts_test_shot_to_gspro_open_api_payload() {
    let shot = ShotEvent::test_shot(12);
    let payload = GsProShotPayload::from_shot(&shot, true);
    let json = serde_json::to_value(payload).expect("json");

    assert_eq!(json["DeviceID"], "Garmin R10");
    assert_eq!(json["Units"], "Yards");
    assert_eq!(json["ShotNumber"], 12);
    assert_eq!(json["APIversion"], "1");
    assert_eq!(json["BallData"]["Speed"], 98.5);
    assert_eq!(json["BallData"]["SpinAxis"], -10.2);
    assert_eq!(json["BallData"]["TotalSpin"], 2350.2);
    assert_eq!(json["BallData"]["HLA"], 0.0);
    assert_eq!(json["BallData"]["VLA"], 13.5);
    assert_eq!(json["ShotDataOptions"]["ContainsBallData"], true);
    assert_eq!(json["ShotDataOptions"]["ContainsClubData"], true);
    assert_eq!(json["ClubData"]["Speed"], 110.0);
}

#[test]
fn can_omit_club_data_even_when_shot_has_club_metrics() {
    let shot = ShotEvent::test_shot(1);
    let json = serde_json::to_value(GsProShotPayload::from_shot(&shot, false)).unwrap();

    assert_eq!(json["ShotDataOptions"]["ContainsClubData"], false);
    assert!(json.get("ClubData").is_none());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```sh
cargo test --test gspro_payload_contract
```

Expected: fails because GSPro modules do not exist.

- [ ] **Step 3: Export the GSPro module**

Replace `src/lib.rs`:

```rust
pub mod config;
pub mod core;
pub mod garmin;
pub mod gspro;
```

- [ ] **Step 4: Add GSPro module export**

Create `src/gspro/mod.rs`:

```rust
pub mod payload;
```

- [ ] **Step 5: Add GSPro payload conversion**

Create `src/gspro/payload.rs`:

```rust
use serde::Serialize;

use crate::core::{ClubMetrics, ShotEvent};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GsProShotPayload {
    #[serde(rename = "DeviceID")]
    pub device_id: String,
    pub units: String,
    pub shot_number: u64,
    #[serde(rename = "APIversion")]
    pub api_version: String,
    pub ball_data: GsProBallData,
    pub shot_data_options: ShotDataOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub club_data: Option<GsProClubData>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GsProBallData {
    pub speed: f64,
    pub spin_axis: f64,
    pub total_spin: f64,
    #[serde(rename = "HLA")]
    pub hla: f64,
    #[serde(rename = "VLA")]
    pub vla: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ShotDataOptions {
    pub contains_ball_data: bool,
    pub contains_club_data: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GsProClubData {
    pub speed: Option<f64>,
    pub angle_of_attack: Option<f64>,
    pub face_to_target: Option<f64>,
    pub lie: Option<f64>,
    pub loft: Option<f64>,
    pub path: Option<f64>,
    pub speed_at_impact: Option<f64>,
    pub vertical_face_impact: Option<f64>,
    pub horizontal_face_impact: Option<f64>,
    pub closure_rate: Option<f64>,
}

impl GsProShotPayload {
    pub fn from_shot(shot: &ShotEvent, include_club_data: bool) -> Self {
        let club_data = include_club_data
            .then(|| shot.club.as_ref().map(GsProClubData::from))
            .flatten();

        Self {
            device_id: shot.device_id.clone(),
            units: shot.units.clone(),
            shot_number: shot.shot_number,
            api_version: "1".into(),
            ball_data: GsProBallData {
                speed: shot.ball.ball_speed,
                spin_axis: shot.ball.spin_axis,
                total_spin: shot.ball.total_spin,
                hla: shot.ball.hla,
                vla: shot.ball.vla,
            },
            shot_data_options: ShotDataOptions {
                contains_ball_data: true,
                contains_club_data: club_data.is_some(),
            },
            club_data,
        }
    }
}

impl From<&ClubMetrics> for GsProClubData {
    fn from(club: &ClubMetrics) -> Self {
        Self {
            speed: club.speed,
            angle_of_attack: club.angle_of_attack,
            face_to_target: club.face_to_target,
            lie: club.lie,
            loft: club.loft,
            path: club.path,
            speed_at_impact: club.speed_at_impact,
            vertical_face_impact: club.vertical_face_impact,
            horizontal_face_impact: club.horizontal_face_impact,
            closure_rate: club.closure_rate,
        }
    }
}
```

- [ ] **Step 6: Run the GSPro payload contract**

Run:

```sh
cargo test --test gspro_payload_contract
```

Expected: all tests pass.

- [ ] **Step 7: Commit**

Run:

```sh
git add src/lib.rs src/gspro/mod.rs src/gspro/payload.rs tests/gspro_payload_contract.rs
git commit -m "feat: add gspro payload conversion"
```

---

### Task 5: OpenAPI Server And Test-Shot Endpoint

**Files:**
- Modify: `src/lib.rs`
- Create: `src/api.rs`
- Test: `tests/api_contract.rs`

- [ ] **Step 1: Write the failing API contract**

Create `tests/api_contract.rs`:

```rust
use axum::body::Body;
use axum::http::{Request, StatusCode};
use garmingolf_connector::api::router;
use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::AppState;
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        garmin_host: "127.0.0.1".into(),
        garmin_port: 0,
        api_host: "127.0.0.1".into(),
        api_port: 0,
        gspro_enabled: false,
        gspro_host: "127.0.0.1".into(),
        gspro_port: 921,
        nova_ws_enabled: false,
        nova_ws_host: "127.0.0.1".into(),
        nova_ws_port: 8765,
    }
}

fn test_state() -> AppState {
    AppState::new(&test_config())
}

#[tokio::test]
async fn health_returns_ok() {
    let response = router(test_config(), test_state())
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn status_returns_json_status() {
    let response = router(test_config(), test_state())
        .oneshot(Request::builder().uri("/status").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn config_returns_json_config() {
    let response = router(test_config(), test_state())
        .oneshot(Request::builder().uri("/config").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn swagger_ui_is_mounted() {
    let response = router(test_config(), test_state())
        .oneshot(Request::builder().uri("/api-docs/openapi.json").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_shot_updates_status() {
    let state = test_state();
    let app = router(test_config(), state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/shots/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(state.status().await.last_shot.unwrap().shot_number, 1);
}
```

- [ ] **Step 2: Run the API contract to verify it fails**

Run:

```sh
cargo test --test api_contract
```

Expected: fails because `src/api.rs` is empty or missing.

- [ ] **Step 3: Export the API module**

Replace `src/lib.rs`:

```rust
pub mod api;
pub mod config;
pub mod core;
pub mod garmin;
pub mod gspro;
```

- [ ] **Step 4: Add the API router and bind helper**

Create `src/api.rs`:

```rust
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use utoipa::OpenApi;
use utoipa::ToSchema;
use utoipa_swagger_ui::SwaggerUi;

use crate::config::AppConfig;
use crate::core::{AppState, AppStatus, ShotEvent};

#[derive(Clone)]
pub struct ApiState {
    pub app: AppState,
    pub config: AppConfig,
    test_shot_number: Arc<AtomicU64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionAccepted {
    pub accepted: bool,
}

#[derive(OpenApi)]
#[openapi(
    paths(health, status, config, patch_config, test_shot),
    components(schemas(HealthResponse, ActionAccepted, AppConfig, AppStatus, ShotEvent)),
    tags((name = "connector", description = "Garmin Golf connector API"))
)]
pub struct ApiDoc;

pub fn router(config: AppConfig, app: AppState) -> Router {
    let state = ApiState {
        app,
        config,
        test_shot_number: Arc::new(AtomicU64::new(1)),
    };

    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/config", get(config).patch(patch_config))
        .route("/shots/test", post(test_shot))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}

pub async fn serve(config: AppConfig, app: AppState) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind(config.api_addr())
        .await
        .map_err(|err| format!("API bind failed: {err}"))?;
    let addr = listener
        .local_addr()
        .map_err(|err| format!("API local_addr failed: {err}"))?;
    tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router(config, app)).await {
            tracing::error!("API server failed: {err}");
        }
    });
    Ok(addr)
}

#[utoipa::path(get, path = "/health", responses((status = OK, body = HealthResponse)))]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

#[utoipa::path(get, path = "/status", responses((status = OK, body = AppStatus)))]
async fn status(State(state): State<ApiState>) -> Json<AppStatus> {
    Json(state.app.status().await)
}

#[utoipa::path(get, path = "/config", responses((status = OK, body = AppConfig)))]
async fn config(State(state): State<ApiState>) -> Json<AppConfig> {
    Json(state.config)
}

#[utoipa::path(patch, path = "/config", responses((status = ACCEPTED, body = ActionAccepted)))]
async fn patch_config() -> (StatusCode, Json<ActionAccepted>) {
    (StatusCode::ACCEPTED, Json(ActionAccepted { accepted: true }))
}

#[utoipa::path(post, path = "/shots/test", responses((status = ACCEPTED, body = ActionAccepted)))]
async fn test_shot(State(state): State<ApiState>) -> (StatusCode, Json<ActionAccepted>) {
    let shot_number = state.test_shot_number.fetch_add(1, Ordering::SeqCst);
    state.app.publish_shot(ShotEvent::test_shot(shot_number)).await;
    (StatusCode::ACCEPTED, Json(ActionAccepted { accepted: true }))
}
```

- [ ] **Step 5: Run the API contract**

Run:

```sh
cargo test --test api_contract
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

Run:

```sh
git add src/lib.rs src/api.rs tests/api_contract.rs
git commit -m "feat: add connector openapi server"
```

---

### Task 6: Nova-Style WebSocket Broadcast

**Files:**
- Modify: `src/lib.rs`
- Create: `src/nova.rs`
- Test: `tests/nova_ws_contract.rs`

- [ ] **Step 1: Write the failing Nova WebSocket contract**

Create `tests/nova_ws_contract.rs`:

```rust
use futures_util::StreamExt;
use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::{AppState, ShotEvent};
use garmingolf_connector::nova::{shot_to_nova_json, spawn_server};

#[test]
fn serializes_shot_to_nova_style_json() {
    let json = shot_to_nova_json(&ShotEvent::test_shot(44));
    let value: serde_json::Value = serde_json::from_str(&json).expect("json");

    assert_eq!(value["type"], "shot");
    assert_eq!(value["shot_number"], 44);
    assert_eq!(value["ball_speed_miles_per_hour"], 98.5);
    assert_eq!(value["vertical_launch_angle_degrees"], 13.5);
    assert_eq!(value["horizontal_launch_angle_degrees"], 0.0);
    assert_eq!(value["total_spin_rpm"], 2350.2);
    assert_eq!(value["spin_axis_degrees"], -10.2);
}

#[tokio::test]
async fn websocket_subscriber_receives_published_shot() {
    let config = AppConfig {
        garmin_host: "127.0.0.1".into(),
        garmin_port: 0,
        api_host: "127.0.0.1".into(),
        api_port: 0,
        gspro_enabled: false,
        gspro_host: "127.0.0.1".into(),
        gspro_port: 921,
        nova_ws_enabled: true,
        nova_ws_host: "127.0.0.1".into(),
        nova_ws_port: 0,
    };
    let state = AppState::new(&config);
    let addr = spawn_server(config, state.clone()).await.expect("server");

    let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .expect("websocket");

    state.publish_shot(ShotEvent::test_shot(55)).await;
    let message = socket.next().await.expect("message").expect("ok").into_text().unwrap();

    assert!(message.contains(r#""type":"shot""#));
    assert!(message.contains(r#""shot_number":55"#));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```sh
cargo test --test nova_ws_contract
```

Expected: fails because `src/nova.rs` does not implement the server.

- [ ] **Step 3: Export the Nova module**

Replace `src/lib.rs`:

```rust
pub mod api;
pub mod config;
pub mod core;
pub mod garmin;
pub mod gspro;
pub mod nova;
```

- [ ] **Step 4: Add Nova serialization and WebSocket server**

Create `src/nova.rs`:

```rust
use std::net::SocketAddr;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures_util::SinkExt;
use serde_json::json;
use tokio::net::TcpListener;

use crate::config::AppConfig;
use crate::core::{AppState, ConnectionStatus, ShotEvent};

pub fn shot_to_nova_json(shot: &ShotEvent) -> String {
    json!({
        "type": "shot",
        "shot_number": shot.shot_number,
        "ball_speed_miles_per_hour": shot.ball.ball_speed,
        "vertical_launch_angle_degrees": shot.ball.vla,
        "horizontal_launch_angle_degrees": shot.ball.hla,
        "total_spin_rpm": shot.ball.total_spin,
        "spin_axis_degrees": shot.ball.spin_axis
    })
    .to_string()
}

pub async fn spawn_server(config: AppConfig, state: AppState) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind(config.nova_ws_addr())
        .await
        .map_err(|err| format!("Nova WebSocket bind failed: {err}"))?;
    let addr = listener
        .local_addr()
        .map_err(|err| format!("Nova WebSocket local_addr failed: {err}"))?;
    state
        .update_nova(|status| {
            status.connection_status = ConnectionStatus::Listening;
            status.port = addr.port();
            status.last_error = None;
        })
        .await;

    let router = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state.clone());

    tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router).await {
            tracing::error!("Nova WebSocket server failed: {err}");
        }
    });
    Ok(addr)
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| pump_shots(socket, state))
}

async fn pump_shots(mut socket: WebSocket, state: AppState) {
    let mut shots = state.subscribe_shots();
    while let Ok(shot) = shots.recv().await {
        if socket
            .send(Message::Text(shot_to_nova_json(&shot).into()))
            .await
            .is_err()
        {
            break;
        }
    }
}
```

- [ ] **Step 5: Run the Nova contract**

Run:

```sh
cargo test --test nova_ws_contract
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

Run:

```sh
git add src/lib.rs src/nova.rs tests/nova_ws_contract.rs
git commit -m "feat: add nova websocket shot feed"
```

---

### Task 7: Garmin TCP Runtime

**Files:**
- Modify: `src/garmin/mod.rs`
- Create: `src/garmin/runtime.rs`
- Test: `tests/garmin_runtime_contract.rs`

- [ ] **Step 1: Write the failing Garmin runtime smoke test**

Create `tests/garmin_runtime_contract.rs`:

```rust
use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::AppState;
use garmingolf_connector::garmin::runtime::spawn_listener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::test]
async fn tcp_runtime_responds_to_garmin_messages_and_publishes_shot() {
    let config = AppConfig {
        garmin_host: "127.0.0.1".into(),
        garmin_port: 0,
        api_host: "127.0.0.1".into(),
        api_port: 0,
        gspro_enabled: false,
        gspro_host: "127.0.0.1".into(),
        gspro_port: 921,
        nova_ws_enabled: false,
        nova_ws_host: "127.0.0.1".into(),
        nova_ws_port: 8765,
    };
    let state = AppState::new(&config);
    let mut shots = state.subscribe_shots();
    let addr = spawn_listener(config, state).await.expect("listener");
    let mut client = TcpStream::connect(addr).await.expect("client");

    client.write_all(br#"{"Type":"Handshake"}"#).await.unwrap();
    let mut buf = vec![0; 1024];
    let n = client.read(&mut buf).await.unwrap();
    assert!(String::from_utf8_lossy(&buf[..n]).contains(r#""Type":"Handshake""#));

    client.write_all(br#"{"Type":"SetBallData","BallData":{"BallSpeed":151.58,"SpinAxis":353.3982,"TotalSpin":4721.59,"LaunchDirection":-5.0065,"LaunchAngle":17.7736}}"#).await.unwrap();
    client.write_all(br#"{"Type":"SendShot"}"#).await.unwrap();

    let shot = shots.recv().await.expect("shot");
    assert_eq!(shot.shot_number, 1);
    assert_eq!(shot.ball.ball_speed, 151.58);
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```sh
cargo test --test garmin_runtime_contract
```

Expected: fails because runtime socket handling is missing.

- [ ] **Step 3: Export the Garmin runtime module**

Replace `src/garmin/mod.rs`:

```rust
pub mod protocol;
pub mod runtime;
```

- [ ] **Step 4: Add Garmin TCP runtime**

Create `src/garmin/runtime.rs`:

```rust
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use crate::config::AppConfig;
use crate::core::{AppState, ConnectionStatus};

use super::protocol::{
    ack_json, authentication_json, handshake_json, parse_incoming, GarminIncoming, ShotAssembler,
};

pub async fn spawn_listener(config: AppConfig, state: AppState) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind(config.garmin_addr())
        .await
        .map_err(|err| format!("Garmin bind failed: {err}"))?;
    let addr = listener
        .local_addr()
        .map_err(|err| format!("Garmin local_addr failed: {err}"))?;
    state
        .update_garmin(|status| {
            status.connection_status = ConnectionStatus::Listening;
            status.port = addr.port();
            status.last_error = None;
        })
        .await;

    let next_shot = Arc::new(AtomicU64::new(1));
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    let state = state.clone();
                    let next_shot = next_shot.clone();
                    tokio::spawn(async move {
                        handle_client(stream, peer, state, next_shot).await;
                    });
                }
                Err(err) => {
                    state
                        .update_garmin(|status| {
                            status.connection_status = ConnectionStatus::Error;
                            status.last_error = Some(format!("Garmin accept failed: {err}"));
                        })
                        .await;
                    break;
                }
            }
        }
    });
    Ok(addr)
}

async fn handle_client(
    mut stream: TcpStream,
    peer: SocketAddr,
    state: AppState,
    next_shot: Arc<AtomicU64>,
) {
    state
        .update_garmin(|status| {
            status.connection_status = ConnectionStatus::Connected;
            status.active_client = Some(peer.to_string());
            status.last_error = None;
        })
        .await;

    let mut assembler = ShotAssembler::default();
    let mut buf = vec![0_u8; 8192];

    loop {
        let read = match stream.read(&mut buf).await {
            Ok(0) => break,
            Ok(read) => read,
            Err(err) => {
                state
                    .update_garmin(|status| {
                        status.last_error = Some(format!("Garmin read failed: {err}"));
                    })
                    .await;
                break;
            }
        };

        let text = String::from_utf8_lossy(&buf[..read]);
        let incoming = match parse_incoming(&text) {
            Ok(incoming) => incoming,
            Err(err) => {
                state
                    .update_garmin(|status| {
                        status.malformed_message_count += 1;
                        status.last_error = Some(err);
                    })
                    .await;
                continue;
            }
        };

        match incoming.clone() {
            GarminIncoming::Handshake => write_json(&mut stream, handshake_json()).await,
            GarminIncoming::Challenge => write_json(&mut stream, authentication_json()).await,
            GarminIncoming::SetClubType { .. } => {
                assembler.apply(incoming);
                write_json(&mut stream, ack_json("SetClubType")).await;
            }
            GarminIncoming::SetBallData { .. } => {
                assembler.apply(incoming);
                write_json(&mut stream, ack_json("SetBallData")).await;
            }
            GarminIncoming::SetClubData { .. } => {
                assembler.apply(incoming);
                write_json(&mut stream, ack_json("SetClubData")).await;
            }
            GarminIncoming::SendShot => {
                let shot_number = next_shot.fetch_add(1, Ordering::SeqCst);
                match assembler.build_shot(shot_number) {
                    Ok(shot) => state.publish_shot(shot).await,
                    Err(err) => {
                        state
                            .update_garmin(|status| {
                                status.last_error = Some(err);
                            })
                            .await;
                    }
                }
                write_json(&mut stream, ack_json("SendShot")).await;
            }
            GarminIncoming::Disconnect => break,
            GarminIncoming::Pong | GarminIncoming::Unknown(_) => {}
        }
    }

    state
        .update_garmin(|status| {
            status.connection_status = ConnectionStatus::Listening;
            status.active_client = None;
        })
        .await;
}

async fn write_json(stream: &mut TcpStream, text: String) {
    if let Err(err) = stream.write_all(text.as_bytes()).await {
        tracing::warn!("Garmin write failed: {err}");
    }
}
```

- [ ] **Step 5: Run the Garmin runtime contract**

Run:

```sh
cargo test --test garmin_runtime_contract
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

Run:

```sh
git add src/garmin/mod.rs src/garmin/runtime.rs tests/garmin_runtime_contract.rs
git commit -m "feat: add garmin tcp runtime"
```

---

### Task 8: GSPro Forwarding Runtime

**Files:**
- Modify: `src/gspro/mod.rs`
- Create: `src/gspro/runtime.rs`
- Test: extend `tests/gspro_payload_contract.rs`

- [ ] **Step 1: Add a failing GSPro socket forwarding test**

Append to `tests/gspro_payload_contract.rs`:

```rust

#[tokio::test]
async fn runtime_forwards_published_shot_to_tcp_server() {
    use garmingolf_connector::config::AppConfig;
    use garmingolf_connector::core::{AppState, ShotEvent};
    use garmingolf_connector::gspro::runtime::spawn_forwarder;
    use tokio::io::AsyncReadExt;
    use tokio::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let config = AppConfig {
        garmin_host: "127.0.0.1".into(),
        garmin_port: 0,
        api_host: "127.0.0.1".into(),
        api_port: 0,
        gspro_enabled: true,
        gspro_host: addr.ip().to_string(),
        gspro_port: addr.port(),
        nova_ws_enabled: false,
        nova_ws_host: "127.0.0.1".into(),
        nova_ws_port: 8765,
    };
    let state = AppState::new(&config);
    spawn_forwarder(config, state.clone()).await;

    let (mut socket, _) = listener.accept().await.unwrap();
    state.publish_shot(ShotEvent::test_shot(99)).await;

    let mut buf = vec![0; 2048];
    let n = socket.read(&mut buf).await.unwrap();
    let text = String::from_utf8_lossy(&buf[..n]);
    assert!(text.contains(r#""ShotNumber":99"#));
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run:

```sh
cargo test --test gspro_payload_contract runtime_forwards_published_shot_to_tcp_server
```

Expected: fails because GSPro runtime is missing.

- [ ] **Step 3: Export the GSPro runtime module**

Replace `src/gspro/mod.rs`:

```rust
pub mod payload;
pub mod runtime;
```

- [ ] **Step 4: Add GSPro forwarding runtime**

Create `src/gspro/runtime.rs`:

```rust
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};

use crate::config::AppConfig;
use crate::core::{AppState, ConnectionStatus};

use super::payload::GsProShotPayload;

pub async fn spawn_forwarder(config: AppConfig, state: AppState) {
    if !config.gspro_enabled {
        return;
    }

    tokio::spawn(async move {
        let mut shots = state.subscribe_shots();
        loop {
            let addr = format!("{}:{}", config.gspro_host, config.gspro_port);
            state
                .update_gspro(|status| {
                    status.connection_status = ConnectionStatus::Connecting;
                    status.last_error = None;
                })
                .await;

            match TcpStream::connect(&addr).await {
                Ok(mut socket) => {
                    state
                        .update_gspro(|status| {
                            status.connection_status = ConnectionStatus::Connected;
                            status.last_error = None;
                        })
                        .await;

                    while let Ok(shot) = shots.recv().await {
                        let payload = GsProShotPayload::from_shot(&shot, true);
                        match serde_json::to_vec(&payload) {
                            Ok(bytes) => {
                                if let Err(err) = socket.write_all(&bytes).await {
                                    state
                                        .update_gspro(|status| {
                                            status.connection_status = ConnectionStatus::Error;
                                            status.last_error =
                                                Some(format!("GSPro send failed: {err}"));
                                        })
                                        .await;
                                    break;
                                }
                                state
                                    .update_gspro(|status| {
                                        status.last_shot_number = Some(shot.shot_number);
                                    })
                                    .await;
                            }
                            Err(err) => {
                                state
                                    .update_gspro(|status| {
                                        status.last_error =
                                            Some(format!("GSPro payload serialize failed: {err}"));
                                    })
                                    .await;
                            }
                        }
                    }
                }
                Err(err) => {
                    state
                        .update_gspro(|status| {
                            status.connection_status = ConnectionStatus::Error;
                            status.last_error = Some(format!("GSPro connect failed: {err}"));
                        })
                        .await;
                    sleep(Duration::from_millis(250)).await;
                }
            }
        }
    });
}
```

- [ ] **Step 5: Run the GSPro forwarding test**

Run:

```sh
cargo test --test gspro_payload_contract
```

Expected: all tests pass.

- [ ] **Step 6: Commit**

Run:

```sh
git add src/gspro/mod.rs src/gspro/runtime.rs tests/gspro_payload_contract.rs
git commit -m "feat: add gspro forwarding runtime"
```

---

### Task 9: CLI Runtime Wiring

**Files:**
- Modify: `src/bin/garmingolf-connector.rs`
- Test: `cargo test`

- [ ] **Step 1: Replace CLI entrypoint with runtime wiring**

Replace `src/bin/garmingolf-connector.rs`:

```rust
use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::AppState;

#[tokio::main]
async fn main() -> Result<(), String> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "garmingolf_connector=info,tower_http=info".into()),
        )
        .try_init();

    let config = AppConfig::from_env()?;
    let state = AppState::new(&config);

    let garmin_addr =
        garmingolf_connector::garmin::runtime::spawn_listener(config.clone(), state.clone())
            .await?;
    let api_addr = garmingolf_connector::api::serve(config.clone(), state.clone()).await?;

    if config.gspro_enabled {
        garmingolf_connector::gspro::runtime::spawn_forwarder(config.clone(), state.clone()).await;
    }
    if config.nova_ws_enabled {
        let nova_addr = garmingolf_connector::nova::spawn_server(config.clone(), state.clone()).await?;
        println!("Nova WebSocket listening on ws://{nova_addr}/ws");
    }

    println!("Garmin listener running on {garmin_addr}");
    println!("OpenAPI server running on http://{api_addr}");
    println!("Swagger UI available at http://{api_addr}/swagger-ui");

    tokio::signal::ctrl_c()
        .await
        .map_err(|err| format!("failed waiting for ctrl-c: {err}"))?;
    Ok(())
}
```

- [ ] **Step 2: Run the full Rust test suite**

Run:

```sh
cargo test
```

Expected: all tests pass.

- [ ] **Step 3: Run a compile check for the binary**

Run:

```sh
cargo check --bin garmingolf-connector
```

Expected: check passes.

- [ ] **Step 4: Commit**

Run:

```sh
git add src/bin/garmingolf-connector.rs
git commit -m "feat: wire connector cli runtime"
```

---

### Task 10: README And Final Verification

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Replace README usage with Rust CLI instructions**

Update `README.md` so it contains:

```markdown
# Garmin Golf Connector

Rust CLI/library bridge for Garmin Golf launch monitor data. The connector listens for Garmin Golf's E6 Connect / Play on PC TCP messages, normalizes shot data, and forwards shots to enabled simulator integrations.

## Run

```sh
cargo run --bin garmingolf-connector -- \
  --garmin-host 0.0.0.0 \
  --garmin-port 2483 \
  --api-host 127.0.0.1 \
  --api-port 5178
```

OpenAPI is available at:

```text
http://127.0.0.1:5178/swagger-ui
```

## GSPro

Enable GSPro forwarding:

```sh
cargo run --bin garmingolf-connector -- \
  --enable-gspro \
  --gspro-host 127.0.0.1 \
  --gspro-port 921
```

## Nova-Style WebSocket

Enable the WebSocket shot feed:

```sh
cargo run --bin garmingolf-connector -- \
  --enable-nova-ws \
  --nova-ws-host 127.0.0.1 \
  --nova-ws-port 8765
```

Subscribers connect to:

```text
ws://127.0.0.1:8765/ws
```

Shot messages use this shape:

```json
{
  "type": "shot",
  "shot_number": 1,
  "ball_speed_miles_per_hour": 98.5,
  "vertical_launch_angle_degrees": 13.5,
  "horizontal_launch_angle_degrees": 0.0,
  "total_spin_rpm": 2350.2,
  "spin_axis_degrees": -10.2
}
```

## Environment Variables

Every CLI option has a `GARMINGOLF_` environment equivalent, for example:

- `GARMINGOLF_API_PORT`
- `GARMINGOLF_GARMIN_PORT`
- `GARMINGOLF_ENABLE_GSPRO`
- `GARMINGOLF_ENABLE_NOVA_WS`

CLI flags override environment values.

## Garmin Golf Setup

Open Garmin Golf on the phone, choose E6 Connect / Play on PC mode, and set the PC address and port to the machine running this connector. The default Garmin listener port is `2483`.
```

- [ ] **Step 2: Run final verification**

Run:

```sh
cargo test
cargo check --bin garmingolf-connector
```

Expected: all tests and the binary check pass.

- [ ] **Step 3: Inspect final diff**

Run:

```sh
git status --short
git diff --stat
```

Expected: only README changes are unstaged before the final commit.

- [ ] **Step 4: Commit**

Run:

```sh
git add README.md
git commit -m "docs: document rust cli connector"
```

---

## Self-Review Checklist

- Spec coverage:
  - Rust library crate: Tasks 1-4.
  - CLI daemon: Task 9.
  - Selectable OpenAPI port: Tasks 1 and 5.
  - Optional Nova WebSocket: Task 6.
  - Garmin/E6 TCP bridge behavior: Tasks 3 and 7.
  - GSPro forwarding: Tasks 4 and 8.
  - Contract tests: Tasks 1-8.
- Placeholder scan: no deferred implementation markers are used in this plan.
- Type consistency:
  - `AppConfig`, `AppState`, `ShotEvent`, and `ConnectionStatus` are introduced before dependent modules.
  - Runtime functions consistently use `spawn_listener`, `spawn_forwarder`, `spawn_server`, and `serve`.
  - Test imports match the module names created in earlier tasks.
