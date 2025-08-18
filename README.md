# lean-link

High-Performance Industrial Control Backend Library in Rust

[![Rust](https://img.shields.io/badge/rust-1.89%2B-blue.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)

## Overview

lean-link is a high-performance industrial control backend library written in Rust. It provides a robust foundation for building industrial control systems with support for various industrial communication protocols.

The library focuses on:
- High performance and memory safety through Rust
- Support for common industrial communication protocols
- Easy integration with web services and databases
- Modular design with feature flags for customization

## Features

- **Multi-Protocol Support**: Built-in support for various industrial protocols
- **Database Integration**: Seamless integration with SQLite, MySQL, and PostgreSQL
- **Web API Ready**: RESTful API support with Actix-web
- **Modular Design**: Feature-based architecture allowing you to include only what you need
- **High Performance**: Leveraging Rust's async runtime for maximum efficiency
- **Safety First**: Memory safety and thread safety guaranteed by Rust's ownership system

## Supported Protocols

- **Modbus**: RTU and TCP support via `tokio-modbus`
- **MQTT**: Client implementation using `rumqttc`
- **Serial Port Communication**: Cross-platform serial communication via `tokio-serial`
- **WebSocket**: Real-time communication support

## Architecture

lean-link follows a modular architecture with the following key components:

```
┌─────────────────┐
│   Web Service   │
├─────────────────┤
│  MQTT Service   │
├─────────────────┤
│ Serial Service  │
├─────────────────┤
│  Modbus Service │
├─────────────────┤
│   Database      │
└─────────────────┘
```

## Getting Started

### Prerequisites

- Rust 1.89 or higher
- Cargo package manager

### Installation

Add lean-link to your `Cargo.toml`:

```toml
[dependencies]
lean-link = { path = "path/to/lean-link" }
```

### Feature Flags

lean-link provides the following optional features:

| Feature    | Description                          | Default |
|------------|--------------------------------------|---------|
| `web`      | Enable web service support           | ✓       |
| `sqlite`   | Enable SQLite database support       | ✓       |
| `mysql`    | Enable MySQL database support        |         |
| `postgres` | Enable PostgreSQL database support   |         |
| `mqtt`     | Enable MQTT protocol support         |         |
| `modbus`   | Enable Modbus protocol support       |         |
| `serialport` | Enable serial port communication  |         |

### Quick Example

```rust
lean_link::config_db!([
    you_package::m20250814_000002_create_tables::Migration,
]);

#[lean_link::web_main]
async fn main() -> std::io::Result<()> {
    let db_path = ".data";
    lean_link::utils::file::create_paths(db_path)?;
    let db_url = format!("sqlite://{}/data.sqlite?mode=rwc", db_path);
    let db_conn = lean_link::database::init_connection(&db_url).await.unwrap();
    setup_db(&db_conn).await.unwrap();
    
    actix_web::HttpServer::new(|| lean_link::new_actix_app!())
        .bind(("127.0.0.1", 3030))?
        .run()
        .await
}
```

## Project Structure

```
lean-link/
├── lean-link-macros/     # Procedural macros for easier setup
├── src/
│   ├── database/         # Database entities and operations
│   ├── service/          # Protocol implementations
│   │   ├── modbus/
│   │   ├── mqtt/
│   │   ├── serialport/
│   │   └── web/
│   └── utils/            # Utility functions
└── Cargo.toml
```

## Development

### Building

```bash
cargo build
```

### Running Tests

```bash
cargo test
```

### Running Examples

```bash
cargo run --example your_example
```

## License

This project is licensed under either of:

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit your changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

## Contact

Rocky Tsui - rocky@lsfinfo.com

Project Link: [https://github.com/rockyx/lean-link](https://github.com/rockyx/lean-link)