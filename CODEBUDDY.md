# CODEBUDDY.md

This file provides guidance to CodeBuddy Code when working with code in this repository.

## Build Commands

```bash
# Basic build (no default features)
cargo build

# Build with specific features
cargo build --features "web sqlite"
cargo build --features "modbus mqtt"
cargo build --features "postgres web"

# Build all features
cargo build --features "all"

# Build without default features
cargo build --no-default-features --features "mysql web"
```

## Test Commands

```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test --test bcd
cargo test config::tests
cargo test service::modbus::tests
```

## Feature Flags

The library uses feature flags for optional functionality. **Only ONE database feature can be enabled at a time** (enforced by build.rs).

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
- `all` - Enables all features except `sqlite` (uses `postgres`)

## Architecture Overview

### Core Components

**AppState** (`src/lib.rs`): Central application state holding:
- Database connection pool (`db_conn: DatabaseConnection`)
- WebSocket server (when `web` feature enabled)
- Server configuration (`ServerConfig`)

Use `AppState::new(app_name)` to bootstrap, or `AppStateBuilder` for custom configuration.

**Configuration System** (`src/config/mod.rs`):
- YAML config loaded at runtime via `config::load_config(app_name)`
- OS-specific config paths:
  - Linux: `/etc/<app_name>/config.yaml`
  - Windows: `<exe_dir>/etc/config.yaml`
  - macOS: Platform config directory
- Configuration structs are conditionally compiled based on enabled features

**Database Layer** (`src/database/`):
- SeaORM entities in `src/database/entity/`
- Migrations in `src/database/migrator/`
- CRUD operations in `src/database/*.rs` (users, logs, settings, modbus_configs, serialport_configs)

**Service Layer** (`src/service/`):
- Each service module is feature-gated with `#[cfg(feature = "...")]`
- Modules: web, websocket, mqtt, modbus, serialport, socket, camera, inspection

### Web Framework (feature: `web`)

- Actix Web with JWT middleware in `src/service/web/middleware/jwt/`
- REST endpoints in `src/service/web/service/`
- Response wrapper: `WebResponse<T>` with `ErrorCode` enum
- WebSocket server with heartbeat support in `src/service/websocket/`

### Modbus Service (feature: `modbus`)

- `ModbusService` with builder pattern (`ModbusTCPBuilder`, `ModbusRTUBuilder`)
- Supports all standard Modbus functions (read/write coils, registers, etc.)
- Helper functions: `registers_to_f32()`, `registers_to_u32()` for register conversion

### Industrial Camera (feature: `industry-camera`)

- `IndustryCamera` trait in `src/service/camera/mod.rs`
- IMV SDK bindings generated at build time (requires `MV_VIEWER_DIR` env var)
- Camera manager handles multiple camera instances
- Frame encoding: JPEG, PNG, Raw

### Error Handling

- Centralized error type in `src/errors.rs`
- Implements `actix_web::error::ResponseError` for web responses
- Feature-gated error variants

## Key Patterns

1. **Conditional Compilation**: Extensive `#[cfg(feature = "...")]` throughout
2. **Re-exports**: Common crates re-exported from `src/lib.rs` (tokio, sea_orm, tracing, etc.)
3. **Async-First**: All I/O uses Tokio async runtime
4. **Builder Pattern**: Used for `AppState`, `ModbusService`, and camera configuration

## Important Constraints

- **Database features are mutually exclusive** - build.rs will panic if multiple are enabled
- **Camera feature requires Windows** - IMV SDK is Windows-only; requires `MV_VIEWER_DIR` environment variable pointing to the SDK installation
- **RTC sync is Linux-only** - DS1307 RTC operations only compile for Linux targets

## Configuration Example

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
  rtc_i2c_dev: "/dev/i2c-1"
  rtc_i2c_addr: 104
```

## Workspace Structure

```
lean-link/
├── Cargo.toml          # Workspace root
├── build.rs            # Feature validation & FFI binding generation
├── src/
│   ├── lib.rs          # AppState, re-exports
│   ├── config/         # YAML config types
│   ├── database/       # SeaORM entities, migrations
│   ├── errors.rs       # Centralized error types
│   ├── ffi/            # Generated FFI bindings (imv.rs)
│   ├── service/        # Protocol/hardware services
│   ├── storage/        # Storage utilities
│   └── utils/          # BCD, datetime, I2C, file helpers
└── lean-link-macros/   # Procedural macros
```
