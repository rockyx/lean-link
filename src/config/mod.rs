use serde::{Deserialize, Serialize};
#[cfg(any(feature = "modbus", feature = "serialport"))]
use serialport::{DataBits, FlowControl, Parity, StopBits};
#[cfg(any(feature = "modbus", feature = "serialport", feature = "web"))]
use std::time::Duration;

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize, Serialize)]
pub struct WebConfig {
    pub host: String,
    pub port: u16,
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize, Serialize)]
pub struct JwtConfig {
    pub secret: String,
    #[serde(with = "duration_seconds")]
    pub expiration_seconds: Duration,
}

#[cfg(feature = "web")]
pub mod duration_seconds {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_secs() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

#[cfg(feature = "modbus")]
#[derive(Debug, Deserialize, Serialize)]
pub struct ModbusTCPConfig {
    pub host: String,
    pub port: u16,
}

#[cfg(feature = "modbus")]
#[derive(Debug, Deserialize, Serialize)]
pub struct ModbusRTUConfig {
    pub path: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
    #[serde(with = "duration_millis")]
    pub timeout: Duration,
}

#[cfg(any(feature = "modbus", feature = "serialport"))]
pub mod duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

/// 串口配置
#[cfg(feature = "serialport")]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SerialPortConfig {
    pub path: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
    #[serde(with = "duration_millis")]
    pub timeout: Duration,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerConfig<UserConfig = ()> {
    pub database: DatabaseConfig,
    #[cfg(feature = "web")]
    pub web: WebConfig,
    #[cfg(feature = "web")]
    pub jwt: JwtConfig,
    #[cfg(feature = "modbus")]
    pub modbus_tcp: Vec<ModbusTCPConfig>,
    #[cfg(feature = "modbus")]
    pub modbus_rtu: Vec<ModbusRTUConfig>,
    #[cfg(feature = "serialport")]
    pub serialport: Vec<SerialPortConfig>,
    // 用户自定义配置部分，默认为空单元类型
    #[serde(flatten)]
    pub user_config: UserConfig,
}
