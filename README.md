# lean-link

High-Performance Industrial Control Backend Library in Rust

[![Rust](https://img.shields.io/badge/rust-1.89%2B-blue.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)

---

lean-link is a high-performance, modular backend library for building industrial control systems in Rust. It emphasizes safety, async performance, and protocol interoperability, with feature flags to include only what you need.

- Memory and thread safety via Rust
- Async-first design for high throughput
- Feature-flag driven architecture
- Integrations with databases, WebSocket, and optional industrial protocols

## Table of Contents

- [Features](#features)
- [Requirements](#requirements)
- [Installation](#installation)
- [Feature Flags](#feature-flags)
- [Configuration](#configuration)
- [Quick Start](#quick-start)
- [WebSocket Usage](#websocket-usage)
- [Database Usage](#database-usage)
- [HTTP/Web Services](#httpweb-services)
- [Response Models](#response-models)
- [Project Structure](#project-structure)
- [Development](#development)
- [Testing](#testing)
- [Contributing](#contributing)
- [License](#license)
- [Contact](#contact)

## Features

From the current implementation and feature set:

- Web/WebSocket support (feature: `web`, `socket`)
- Database via SeaORM with SQLite/MySQL/PostgreSQL backends
- MQTT client (feature: `mqtt`)
- Modbus (TCP/RTU) (feature: `modbus`)
- Serial port communication (feature: `serialport`)
- IMV camera placeholder (feature: `imv-camera`)
- Utilities: time/I2C helpers, CRC, scheduling, JWT (when `web` enabled)
- Cross-platform config path resolution

## Requirements

- Rust 1.89+ (edition 2024)
- Cargo

## Installation

As a workspace/local dependency:
```toml
# Cargo.toml
[dependencies]
lean-link = { path = "path/to/lean-link" }
```

As a git dependency (pin to tag/branch/commit as needed):
```toml
[dependencies]
lean-link = { git = "https://github.com/rockyx/lean-link.git", rev = "<commit-or-tag>" }
```

If published to crates.io in the future:
```toml
[dependencies]
lean-link = "x.y.z"
```

## Feature Flags

Default features: `web`, `sqlite`

Optional features to tailor your build (from Cargo.toml):
- sqlite (default)
- mysql
- postgres
- web (Actix Web utils, JWT, WebSocket runtime integration)
- serialport
- mqtt
- modbus (enables tokio-modbus; also implies serialport)
- socket (tokio-tungstenite only; note web already includes it)
- imv-camera
- all (enables: sqlite, mysql, postgres, web, serialport, mqtt, modbus, socket, imv-camera)

Examples:
```bash
# Enable selected features
cargo build --features "modbus mqtt"

# Disable defaults and choose explicitly
cargo build --no-default-features --features "postgres web"
```

## Configuration

lean-link loads a YAML config at runtime via `config::load_config(app_name)` and constructs `AppState`. The config path is resolved per OS:

- Linux: /etc/<app_name>/config.yaml
- Windows: <exe_dir>/etc/config.yaml
- Others (e.g., macOS): platform config directory via directories::ProjectDirs

Key structures (conditionally compiled by features):
- DatabaseConfig: url
- WebConfig: host, port (feature `web`)
- JwtConfig: secret, expires_in (duration string) (feature `web`)
- WebSocketConfig: host, port, max_connections, heartbeat_interval (duration string) (feature `web`)
- ModbusTCPConfig (feature `modbus`)
- ModbusRTUConfig (feature `modbus`)
- SerialPortConfig (feature `serialport`)
- MqttConfig with topics and QoS mapping (feature `mqtt`)
- Sys: sync_time_from_client, sync_time_from_rtc, rtc_i2c_dev, rtc_i2c_addr

Example config.yaml (enable or remove sections per your enabled features):
```yaml
database:
  url: "sqlite://./data.sqlite?mode=rwc"

web:
  host: "127.0.0.1"
  port: 8080

jwt:
  secret: "your-jwt-secret"
  expires_in: "15m"        # duration string, e.g., "15m", "1h"

web_socket:
  host: "127.0.0.1"
  port: 9001
  max_connections: 1024
  heartbeat_interval: "30s"  # duration string

modbus_tcp:
  - host: "192.168.1.100"
    port: 502

modbus_rtu:
  - path: "/dev/ttyUSB0"
    baud_rate: 9600
    data_bits: "Eight"       # serialport::DataBits enum
    stop_bits: "One"         # serialport::StopBits enum
    parity: "None"           # serialport::Parity enum
    flow_control: "None"     # serialport::FlowControl enum
    timeout: "1s"

serialport:
  - path: "/dev/ttyUSB1"
    baud_rate: 115200
    data_bits: "Eight"
    stop_bits: "One"
    parity: "None"
    flow_control: "None"
    timeout: "1s"

mqtt:
  - host: "broker.emqx.io"
    port: 1883
    username: "user"
    password: "pass"
    client_id: "leanlink-client-01"
    keep_alive: "30s"
    topic:
      - topic: "plant/telemetry"
        qos: "AtLeastOnce"   # QoS: AtMostOnce | AtLeastOnce | ExactlyOnce

sys:
  sync_time_from_client: false
  sync_time_from_rtc: false
  rtc_i2c_dev: "/dev/i2c-1"
  rtc_i2c_addr: 104
```

Notes:
- Duration fields accept string forms parsed by utils (e.g., "30s", "5m", "1h").
- On Linux, `sys.sync_time_from_client` may run `timedatectl` and optionally sync DS1307 RTC when enabled.
- On Windows, config is expected under `<exe_dir>/etc/config.yaml`.

## Quick Start

Minimal bootstrap using `AppState` and WebSocket (when `web` is enabled):

```rust
use lean_link::{tracing, tracing_subscriber};
use lean_link::service::websocket::{WsMessage};
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_env_filter("info").init();

    // Pick an app name used to resolve config path
    let app_name = "leanlink"; // matches your config path rules
    let state = lean_link::AppState::new(app_name).await?;

    // Start WebSocket server (feature "web" required)
    #[cfg(feature = "web")]
    {
        let ws_rx = state.start_web_socket().await?;
        // Example: spawn a task to handle incoming WebSocket messages
        tokio::spawn(async move {
            use lean_link::service::websocket::WebSocketMessage;
            while let Some(msg) = ws_rx.recv().await {
                match msg {
                    WebSocketMessage::NewConnected(id) => {
                        tracing::info!("New client: {}", id);
                    }
                    WebSocketMessage::Message(id, message) => {
                        tracing::info!("From {} => {:?}", id, message);
                    }
                }
            }
        });

        // Example: broadcast a typed message
        let payload = WsMessage { topic: "hello".into(), payload: "world".to_string() };
        state.ws_server.broadcast(Message::from(payload)).await;
    }

    // Use database connection
    let _db = state.db_conn.clone();

    Ok(())
}
```

## WebSocket Usage

Types and APIs:
- `WebSocketServer::new(config, sys)` is constructed internally by `AppState::new`.
- `AppState::start_web_socket()` starts the listener and returns `Receiver<WebSocketMessage>`.
- Broadcast or send:
  - `ws_server.broadcast(Message)` to all
  - `ws_server.send(id, Message)` to a specific connection
- Heartbeats are sent according to `web_socket.heartbeat_interval`.

Special topic:
- When `sys.sync_time_from_client` is true and platform is Linux, a JSON message with:
  ```json
  { "topic": "syncSysTime", "payload": "2025-01-01 12:34:56" }
  ```
  will attempt to set system time (and update DS1307 RTC when `sys.sync_time_from_rtc` is also true).

## Database Usage

- `AppState::new(app_name)` calls `config::load_config(app_name)` and opens a `sea_orm::DatabaseConnection` using `database.url`.
- You can use `state.db_conn` with SeaORM entities/migrations as usual.
- Enable the corresponding database feature: `sqlite` (default), `mysql`, or `postgres`.

## HTTP/Web Services

- Enabling `web` feature pulls in Actix-related deps, JWT support, and WebSocket runtime.
- Route modules are organized under `src/service/web/`. You can build Actix apps in your own binary crate and reuse lean-link utilities, config, and state. (This crate does not currently expose a prebuilt actix `App` factory.)
- JWT middleware utilities reside under `src/service/web/middleware/jwt/` (builder, middleware, inner).

## Response Models

Standard response wrapper for Web endpoints:
```rust
#[derive(Serialize, Deserialize)]
pub enum ErrorCode {
    Success = 0,
    InvalidUsernameOrPassword = 10001,
    Unauthorized = 10002,
    InternalError = 50001,
}

#[derive(Serialize, Deserialize)]
pub struct WebResponse<T> {
    pub code: ErrorCode,     // serialized as u32
    pub success: bool,
    pub timestamp: i64,
    pub result: Option<T>,
    pub message: String,
}
```

Pagination helper:
```rust
#[derive(Serialize, Deserialize)]
pub struct Pagination<D> {
    pub records: Vec<D>,
    pub total: u64,
    pub current: u64,
    pub size: u64,
    pub pages: u64,
}
```

Example JSON:
```json
{
  "code": 0,
  "success": true,
  "timestamp": 1733961600,
  "result": { "id": 1, "name": "Alice" },
  "message": ""
}
```

## Project Structure

```
lean-link/
├── Cargo.toml
├── build.rs
├── src/
│   ├── config/            # Config types + loader (YAML)
│   ├── database/          # SeaORM entities/migrator
│   ├── errors.rs
│   ├── ffi/
│   ├── service/
│   │   ├── camera/
│   │   ├── modbus/
│   │   ├── mqtt/
│   │   ├── serialport/
│   │   ├── web/
│   │   │   ├── middleware/jwt/
│   │   │   └── service/
│   │   └── websocket/
│   ├── storage/
│   └── utils/             # bcd/datetime/file/i2c tools
└── lean-link-macros/      # Procedural macros (internal)
```

## Development

Build:
```bash
cargo build
```

Run with selected features:
```bash
cargo run --features "web sqlite"
```

## Testing

```bash
cargo test
```

## Contributing

Contributions are welcome!

1. Fork the repository
2. Create your feature branch: `git checkout -b feature/AmazingFeature`
3. Commit your changes: `git commit -m "Add some AmazingFeature"`
4. Push to the branch: `git push origin feature/AmazingFeature`
5. Open a Pull Request

Please include tests and doc updates where applicable.

## License

This project is licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## Contact

- Author: Rocky Tsui — rocky@lsfinfo.com
- Project: https://github.com/rockyx/lean-link