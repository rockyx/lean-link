# lean-link

[English](README.md)

高性能工业自动化上位机后端库

[![Rust](https://img.shields.io/badge/rust-1.89%2B-blue.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)

---

lean-link 是一个用 Rust 编写的高性能、模块化工业控制系统后端库。它强调安全性、异步性能和协议互操作性，通过特性标志让您只包含所需的功能。

- 通过 Rust 实现内存和线程安全
- 异步优先设计，实现高吞吐量
- 特性标志驱动的架构
- 集成数据库、WebSocket 和可选的工业协议

## 功能特性

- **Web/WebSocket 支持** (特性: `web`, `socket`)
- **数据库** 通过 SeaORM 支持 SQLite/MySQL/PostgreSQL 后端
- **MQTT 客户端** (特性: `mqtt`)
- **Modbus** TCP/RTU (特性: `modbus`)
- **串口通信** (特性: `serialport`)
- **工业相机支持** (特性: `industry-camera`)
- **视觉检测** (特性: `inspection`, 开发中)
- **工具集**: 时间/I2C 辅助工具、CRC、调度、JWT（启用 `web` 时）
- 跨平台配置路径解析

## 环境要求

- Rust 1.89+ (edition 2024)
- Cargo

## 安装

作为工作区/本地依赖：
```toml
# Cargo.toml
[dependencies]
lean-link = { path = "path/to/lean-link" }
```

作为 git 依赖：
```toml
[dependencies]
lean-link = { git = "https://github.com/rockyx/lean-link.git", rev = "<commit-or-tag>" }
```

## 特性标志

默认不启用任何特性。

### 数据库特性（互斥）
- `sqlite` - SQLite 后端
- `mysql` - MySQL 后端
- `postgres` - PostgreSQL 后端

### 通信特性
- `web` - Actix Web 框架、JWT 认证、WebSocket 运行时
- `mqtt` - MQTT 客户端（通过 rumqttc）
- `modbus` - Modbus TCP/RTU（隐含 `serialport`）
- `serialport` - 串口通信
- `socket` - 原始 WebSocket（通过 tokio-tungstenite）

### 硬件特性
- `industry-camera` - 工业相机支持（IMV SDK 绑定）
- `inspection` - 视觉检测系统（隐含 `industry-camera`）

### 元特性
- `all` - 启用所有特性（使用 `postgres` 作为数据库）

```bash
cargo build --features "web sqlite"
cargo build --features "modbus mqtt"
cargo build --features "all"
```

## 配置

配置路径根据操作系统解析：
- Linux: `/etc/<app_name>/config.yaml`
- Windows: `<exe_dir>/etc/config.yaml`
- macOS: 平台配置目录

示例 `config.yaml`:
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

## 快速开始

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
                        tracing::info!("新客户端: {}", id);
                    }
                    WebSocketMessage::Message(id, message) => {
                        tracing::info!("来自 {} => {:?}", id, message);
                    }
                }
            }
        });
    }

    let _db = state.db_conn.clone();
    Ok(())
}
```

## Modbus 服务

```rust
use lean_link::service::modbus::{ModbusTCPBuilder, ModbusRTUBuilder};

// Modbus TCP
let mut service = ModbusTCPBuilder::new("192.168.1.100".to_string(), 502)
    .timeout(std::time::Duration::from_secs(1))
    .build();

// 读取保持寄存器
let registers = service.read_holding_registers(0x0001, 10).await?;
```

## 项目结构

```
lean-link/
├── Cargo.toml
├── build.rs
├── src/
│   ├── lib.rs              # AppState, 重导出
│   ├── config/             # 配置类型 (YAML)
│   ├── database/           # SeaORM 实体/迁移
│   ├── errors.rs           # 错误类型
│   ├── ffi/                # FFI 绑定 (IMV SDK)
│   ├── service/
│   │   ├── camera/         # 工业相机
│   │   ├── inspection/     # 视觉检测
│   │   ├── modbus/         # Modbus TCP/RTU
│   │   ├── mqtt/           # MQTT 客户端
│   │   ├── serialport/     # 串口通信
│   │   ├── web/            # Actix Web + JWT
│   │   └── websocket/      # WebSocket 服务器
│   └── utils/              # BCD、日期时间、I2C、CRC
└── lean-link-macros/       # 过程宏
```

## 开发

```bash
cargo build --features "web sqlite"
cargo test
```

## 许可证

MIT 或 Apache-2.0，任选其一。

## 联系方式

- 作者：Rocky Tsui — rocky@lsfinfo.com
- 项目：https://github.com/rockyx/lean-link
