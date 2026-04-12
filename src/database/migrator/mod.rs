pub mod m20250814_000001_create_tables;
pub mod m20260121_000001_modify_t_logs;

#[cfg(feature = "inspection")]
pub mod m20260412_000001_create_tables;

#[cfg(feature = "serialport")]
pub mod m20260412_000002_create_tables;

#[cfg(feature = "modbus")]
pub mod m20260412_000003_create_tables;