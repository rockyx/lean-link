use directories::ProjectDirs;
#[cfg(feature = "mqtt")]
use rumqttc::QoS;
use serde::{Deserialize, Serialize};
#[cfg(any(feature = "modbus", feature = "serialport"))]
use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::path::{Path, PathBuf};
#[cfg(any(feature = "modbus", feature = "serialport", feature = "web"))]
use std::time::Duration;
use std::fs::File;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        DatabaseConfig {
            url: "sqlite://leanlink.db".to_string(),
        }
    }
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WebConfig {
    pub host: String,
    pub port: u16,
}

#[cfg(feature = "web")]
impl Default for WebConfig {
    fn default() -> Self {
        WebConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WebSocketConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: u32,
    #[serde(with = "crate::utils::datetime::string_to_duration")]
    pub heartbeat_interval: Duration,
}

#[cfg(feature = "web")]
impl Default for WebSocketConfig {
    fn default() -> Self {
        WebSocketConfig {
            host: "127.0.0.1".to_string(),
            port: 8081,
            max_connections: 100,
            heartbeat_interval: Duration::from_secs(30),
        }
    }
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    #[serde(with = "crate::utils::datetime::string_to_duration")]
    pub expires_in: Duration,
}

#[cfg(feature = "web")]
impl Default for JwtConfig {
    fn default() -> Self {
        JwtConfig {
            secret: "secret".to_string(),
            expires_in: Duration::from_secs(3600),
        }
    }
}

#[cfg(feature = "modbus")]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModbusTCPConfig {
    pub host: String,
    pub port: u16,
}

#[cfg(feature = "modbus")]
impl Default for ModbusTCPConfig {
    fn default() -> Self {
        ModbusTCPConfig {
            host: "192.168.1.100".to_string(),
            port: 502,
        }
    }
}

#[cfg(feature = "modbus")]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModbusRTUConfig {
    pub path: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
    #[serde(with = "crate::utils::datetime::string_to_duration")]
    pub timeout: Duration,
}

#[cfg(feature = "modbus")]
impl Default for ModbusRTUConfig {
    fn default() -> Self {
        ModbusRTUConfig {
            path: "/dev/ttyUSB0".to_string(),
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            stop_bits: StopBits::One,
            parity: Parity::None,
            flow_control: FlowControl::None,
            timeout: Duration::from_secs(1),
        }
    }
}

#[cfg(feature = "serialport")]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialPortConfig {
    pub path: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
    #[serde(with = "crate::utils::datetime::string_to_duration")]
    pub timeout: Duration,
}

#[cfg(feature = "serialport")]
impl Default for SerialPortConfig {
    fn default() -> Self {
        SerialPortConfig {
            path: "/dev/ttyUSB0".to_string(),
            baud_rate: 9600,
            data_bits: DataBits::Eight,
            stop_bits: StopBits::One,
            parity: Parity::None,
            flow_control: FlowControl::None,
            timeout: Duration::from_secs(1),
        }
    }
}


#[cfg(feature = "mqtt")]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MqttTopic {
    pub topic: String,
    #[serde(with = "string_to_qos")]
    pub qos: QoS,
}

#[cfg(feature = "mqtt")]
impl Default for MqttTopic {
    fn default() -> Self {
        MqttTopic {
            topic: "leanlink/topic".to_string(),
            qos: QoS::AtLeastOnce,
        }
    }
}

#[cfg(feature = "mqtt")]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MqttConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub client_id: String,
    pub topic: Vec<MqttTopic>,
    #[serde(with = "crate::utils::datetime::string_to_duration")]
    pub keep_alive: Duration,
}

#[cfg(feature = "mqtt")]
impl Default for MqttConfig {
    fn default() -> Self {
        MqttConfig {
            host: "localhost".to_string(),
            port: 1883,
            username: "user".to_string(),
            password: "password".to_string(),
            client_id: "leanlink_client".to_string(),
            topic: vec![MqttTopic::default()],
            keep_alive: Duration::from_secs(60),
        }
    }
}

#[cfg(feature = "mqtt")]
mod string_to_qos {
    use rumqttc::QoS;
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S>(qos: &QoS, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match qos {
            QoS::AtMostOnce => serializer.serialize_str("AtMostOnce"),
            QoS::AtLeastOnce => serializer.serialize_str("AtLeastOnce"),
            QoS::ExactlyOnce => serializer.serialize_str("ExactlyOnce"),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<QoS, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "AtMostOnce" => Ok(QoS::AtMostOnce),
            "AtLeastOnce" => Ok(QoS::AtLeastOnce),
            "ExactlyOnce" => Ok(QoS::ExactlyOnce),
            _ => Err(D::Error::custom(format!("Invalid QoS: {}", s))),
        }
    }
}

#[cfg(feature = "socket")]
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SocketConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: u32,
    #[serde(with = "crate::utils::datetime::string_to_duration")]
    pub heartbeat_interval: Duration,
}

#[cfg(feature = "socket")]
impl Default for SocketConfig {
    fn default() -> Self {
        SocketConfig {
            host: "0.0.0.0".to_string(),
            port: 9000,
            max_connections: 100,
            heartbeat_interval: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Sys {
    #[serde(default)]
    pub sync_time_from_client: bool,
    #[serde(default)]
    pub sync_time_from_rtc: bool,
    #[serde(default)]
    pub rtc_i2c_dev: String,
    #[serde(default)]
    pub rtc_i2c_addr: u16,
}

impl Default for Sys {
    fn default() -> Self {
        Sys {
            sync_time_from_client: false,
            sync_time_from_rtc: false,
            rtc_i2c_dev: "/dev/i2c-1".to_string(),
            rtc_i2c_addr: 0x68,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub database: DatabaseConfig,
    #[cfg(feature = "web")]
    pub web: WebConfig,
    #[cfg(feature = "web")]
    pub jwt: JwtConfig,
    #[cfg(feature = "web")]
    pub web_socket: WebSocketConfig,
    #[cfg(feature = "modbus")]
    pub modbus_tcp: Vec<ModbusTCPConfig>,
    #[cfg(feature = "modbus")]
    pub modbus_rtu: Vec<ModbusRTUConfig>,
    #[cfg(feature = "serialport")]
    pub serialport: Vec<SerialPortConfig>,
    #[cfg(feature = "mqtt")]
    pub mqtt: Vec<MqttConfig>,
    #[serde(default)]
    pub sys: Sys,
    #[cfg(feature = "socket")]
    pub socket: Vec<SocketConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            database: DatabaseConfig::default(),
            #[cfg(feature = "web")]
            web: WebConfig::default(),
            #[cfg(feature = "web")]
            jwt: JwtConfig::default(),
            #[cfg(feature = "web")]
            web_socket: WebSocketConfig::default(),
            #[cfg(feature = "modbus")]
            modbus_tcp: vec![ModbusTCPConfig::default()],
            #[cfg(feature = "modbus")]
            modbus_rtu: vec![ModbusRTUConfig::default()],
            #[cfg(feature = "serialport")]
            serialport: vec![SerialPortConfig::default()],
            #[cfg(feature = "mqtt")]
            mqtt: vec![MqttConfig::default()],
            sys: Sys::default(),
            #[cfg(feature = "socket")]
            socket: vec![SocketConfig::default()],
        }
    }
}

/// Get the cross-platform configuration file path
pub fn get_config_path(app_name: &str) -> Option<PathBuf> {
    // Differentiate operating systems
    if cfg!(target_os = "linux") {
        // Linux: /etc/app-name/config.yaml
        Some(Path::new("/etc").join(app_name).join("config.yaml"))
    } else if cfg!(target_os = "windows") {
        // Windows: Application installation directory etc/config.yaml
        let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
        Some(exe_dir.join("etc").join("config.yaml"))
    } else {
        // Other systems (such as macOS) use standard configuration directories
        ProjectDirs::from("com", "", app_name).map(|dirs| dirs.config_dir().join("config.yaml"))
    }
}

pub fn load_config(app_name: &str) -> std::io::Result<ServerConfig> {
    let config_path = get_config_path(app_name).ok_or(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Could not determine config path",
    ))?;

    tracing::info!("Loading config from {:?}", config_path);

    let normalized_path = normpath::PathExt::normalize(config_path.as_path())?;

    // 打开文件并解析
    let file = File::open(normalized_path.as_path())?;
    let config: ServerConfig = serde_yaml_bw::from_reader(file).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse config: {}", e),
        )
    })?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use crate::config::*;
    use crate::ServerConfig;

    #[test]
    fn test_get_config_path() {
        use directories::BaseDirs;
        // println!("{}", std::env::current_exe().ok().unwrap().parent().unwrap().to_str().unwrap());
        println!("{:?}", std::env::current_exe().ok().unwrap());
        if cfg!(target_os = "linux") {
            let linux_path = get_config_path("my_app");
            let mut expected_path = PathBuf::new();
            expected_path.push("/etc");
            expected_path.push("my_app");
            expected_path.push("config.yaml");
            assert_eq!(linux_path, Some(expected_path));
        }

        if cfg!(target_os = "windows") {
            let windows_path = get_config_path("my_app");
            let mut expected_path = std::env::current_exe()
                .ok()
                .unwrap()
                .parent()
                .unwrap()
                .to_path_buf();
            expected_path.push("etc");
            expected_path.push("config.yaml");
            assert_eq!(windows_path, Some(expected_path));
        }

        if cfg!(target_os = "macos") {
            let macos_path = get_config_path("my_app");
            let mut expected_path = PathBuf::new();
            if let Some(base_dirs) = BaseDirs::new() {
                let home_dir = base_dirs.home_dir();
                expected_path.push(home_dir);
                println!("Home directory: {}", home_dir.display());
            }

            expected_path.push("Library");
            expected_path.push("Application Support");
            expected_path.push("com.my_app");
            expected_path.push("config.yaml");

            assert_eq!(macos_path, Some(expected_path));
        }
    }

    #[test]
    fn test_load_config() {
        let config: ServerConfig = load_config("leanlink").expect("Failed to load config");
        println!("{:#?}", config);
    }
}
