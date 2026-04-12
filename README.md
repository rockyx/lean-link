# lean-link

[中文文档](README_CN.md)

High-Performance Industrial Control Backend Library in Rust

[![Rust](https://img.shields.io/badge/rust-1.89%2B-blue.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)

---

lean-link is a high-performance, modular backend library for building industrial control systems in Rust. It emphasizes safety, async performance, and protocol interoperability, with feature flags to include only what you need.

- Memory and thread safety via Rust
- Async-first design for high throughput
- Feature-flag driven architecture
- Integrations with databases, WebSocket, and optional industrial protocols

## Features

- **Web/WebSocket support** (feature: `web`, `socket`)
- **Database** via SeaORM with SQLite/MySQL/PostgreSQL backends
- **MQTT client** (feature: `mqtt`)
- **Modbus** TCP/RTU (feature: `modbus`)
- **Serial port communication** (feature: `serialport`)
- **Industrial camera support** (feature: `industry-camera`)
- **Visual inspection** (feature: `inspection`, in progress)
- **Utilities**: time/I2C helpers, CRC, scheduling, JWT (when `web` enabled)
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

As a git dependency:
```toml
[dependencies]
lean-link = { git = "https://github.com/rockyx/lean-link.git", rev = "<commit-or-tag>" }
```

## Feature Flags

No default features enabled.

### Database Features (mutually exclusive)
- `sqlite` - SQLite backend
- `mysql` - MySQL backend
- `postgres` - PostgreSQL backend

### Communication Features
- `web` - Actix Web framework, JWT authentication, WebSocket runtime
- `mqtt` - MQTT client via rumqttc
- `modbus` - Modbus TCP/RTU (implies `serialport`)
- `serialport` - Serial port communication
- `socket` - Raw WebSocket via tokio-tungstenite

### Hardware Features
- `industry-camera` - Industrial camera support (IMV SDK bindings)
- `inspection` - Visual inspection system (implies `industry-camera`)

### Meta Feature
- `all` - Enables all features (uses `postgres` as database)

```bash
cargo build --features "web sqlite"
cargo build --features "modbus mqtt"
cargo build --features "all"
```

## Configuration

Config path resolved per OS:
- Linux: `/etc/<app_name>/config.yaml`
- Windows: `<exe_dir>/etc/config.yaml`
- macOS: Platform config directory

Example `config.yaml`:
```yaml
database:
  url: "sqlite://./data.sqlite?mode=rwc"

web:
  host: "127.0.0.1"
  port: 8080

jwt:
  secret: "your-jwt-secret"
  expires_in: "15m"

web_socket:
  host: "127.0.0.1"
  port: 9001
  max_connections: 1024
  broadcast_channel_capacity: 128
  heartbeat_interval: "30s"

mqtt:
  - host: "broker.emqx.io"
    port: 1883
    client_id: "leanlink-client"
    keep_alive: "30s"
    topic:
      - topic: "plant/telemetry"
        qos: "AtLeastOnce"

sys:
  sync_time_from_client: false
  sync_time_from_rtc: false
```

## Quick Start

```rust
use lean_link::{tracing, tracing_subscriber};
use lean_link::service::websocket::{WsMessage, WebSocketMessage};
use tokio_tungstenite::tungstenite::Message;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let state = lean_link::AppState::new("leanlink").await?;

    #[cfg(feature = "web")]
    {
        let ws_rx = state.start_web_socket().await?;
        tokio::spawn(async move {
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
    }

    let _db = state.db_conn.clone();
    Ok(())
}
```

## Modbus Service

```rust
use lean_link::service::modbus::{ModbusTCPBuilder, ModbusRTUBuilder};

// Modbus TCP
let mut service = ModbusTCPBuilder::new("192.168.1.100".to_string(), 502)
    .timeout(std::time::Duration::from_secs(1))
    .build();

// Read holding registers
let registers = service.read_holding_registers(0x0001, 10).await?;
```

## Project Structure

```
lean-link/
├── Cargo.toml
├── build.rs
├── src/
│   ├── lib.rs              # AppState, re-exports
│   ├── config/             # Config types (YAML)
│   ├── database/           # SeaORM entities/migrator
│   ├── errors.rs           # Error types
│   ├── ffi/                # FFI bindings (IMV SDK)
│   ├── service/
│   │   ├── camera/         # Industrial camera
│   │   ├── inspection/     # Visual inspection
│   │   ├── modbus/         # Modbus TCP/RTU
│   │   ├── mqtt/           # MQTT client
│   │   ├── serialport/     # Serial port
│   │   ├── web/            # Actix Web + JWT
│   │   └── websocket/      # WebSocket server
│   └── utils/              # BCD, datetime, I2C, CRC
└── lean-link-macros/       # Procedural macros
```

## Development

```bash
cargo build --features "web sqlite"
cargo test
```

## License

MIT or Apache-2.0 at your option.

## Contact

- Author: Rocky Tsui — rocky@lsfinfo.com
- Project: https://github.com/rockyx/lean-link
