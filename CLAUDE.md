# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Common Development Commands

### Building
```bash
# Basic build
cargo build

# Build with specific features
cargo build --features "web sqlite"
cargo build --features "modbus mqtt postgres"

# Build without default features
cargo build --no-default-features --features "mysql web"
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for a specific module
cargo test --test bcd
cargo test config::tests
```

### Running
```bash
# Run with features
cargo run --features "web sqlite"

# Run with all features enabled
cargo run --features "all"
```

## Architecture Overview

### Feature-Flag Driven Design
The library uses Cargo features to enable optional functionality. Key features:
- **Database features**: `sqlite`, `mysql`, `postgres` (only ONE can be enabled at a time)
- **Communication features**: `web` (Actix Web + JWT + WebSocket), `mqtt`, `modbus`, `serialport`, `socket`
- **Hardware features**: `imv-camera`

**Important**: The build.rs enforces that only one database feature can be enabled simultaneously.

### Configuration System
- Configuration is loaded from YAML files at runtime
- Config paths are OS-specific:
  - Linux: `/etc/<app_name>/config.yaml`
  - Windows: `<exe_dir>/etc/config.yaml`
  - macOS: Platform config directory via `directories::ProjectDirs`
- Use `config::load_config(app_name)` to load configuration
- Configuration structures are conditionally compiled based on enabled features

### Core Components

1. **AppState** (`src/lib.rs`): Central application state that holds:
   - Database connection pool
   - WebSocket server (when `web` feature enabled)
   - Configuration

2. **Database Layer** (`src/database/`):
   - SeaORM entities and migrations
   - Supports SQLite (default), MySQL, or PostgreSQL
   - Only one database backend can be active at a time

3. **Service Layer** (`src/service/`):
   - Feature-gated modules for different protocols
   - Each service module is conditionally compiled

4. **Utilities** (`src/utils/`):
   - BCD conversion utilities
   - DateTime helpers with duration parsing
   - File operations
   - I2C communication helpers

### Web Framework Integration
When the `web` feature is enabled:
- Actix Web framework is available
- JWT middleware for authentication
- WebSocket server with heartbeat support
- CORS support via `actix-cors`
- Response models in `src/service/web/response.rs`

### FFI Support
- IMV camera bindings are generated at build time when `imv-camera` feature is enabled
- Requires `MV_VIEWER_DIR` environment variable
- Bindings are generated to `src/ffi/imv.rs`

## Testing Strategy
- Unit tests are embedded within source files
- Test modules use `#[cfg(test)]` attribute
- No separate tests directory - tests live alongside implementation
- Run specific module tests with `cargo test module_name::tests`

## Key Patterns

1. **Conditional Compilation**: Extensive use of `#[cfg(feature = "...")]` throughout the codebase
2. **Re-exports**: The main lib.rs re-exports commonly used dependencies
3. **Error Handling**: Centralized error types in `src/errors.rs`
4. **Async-First**: All I/O operations are async using Tokio
5. **Type Safety**: Leverages Rust's type system for industrial control safety