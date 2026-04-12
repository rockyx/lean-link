use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::path::{Path, PathBuf};
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
    pub web: crate::service::web::WebConfig,
    #[cfg(feature = "web")]
    pub jwt: crate::service::web::JwtConfig,
    #[cfg(feature = "web")]
    pub web_socket: crate::service::websocket::WebSocketConfig,
    #[cfg(feature = "modbus")]
    pub modbus_tcp: Vec<crate::service::modbus::ModbusTCPConfig>,
    #[cfg(feature = "modbus")]
    pub modbus_rtu: Vec<crate::service::modbus::ModbusRTUConfig>,
    #[cfg(feature = "serialport")]
    pub serialport: Vec<crate::service::serialport::SerialPortConfig>,
    #[cfg(feature = "mqtt")]
    pub mqtt: Vec<crate::service::mqtt::MqttConfig>,
    #[serde(default)]
    pub sys: Sys,
    #[cfg(feature = "socket")]
    pub socket: Vec<crate::service::socket::SocketConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            database: DatabaseConfig::default(),
            #[cfg(feature = "web")]
            web: crate::service::web::WebConfig::default(),
            #[cfg(feature = "web")]
            jwt: crate::service::web::JwtConfig::default(),
            #[cfg(feature = "web")]
            web_socket: crate::service::websocket::WebSocketConfig::default(),
            #[cfg(feature = "modbus")]
            modbus_tcp: vec![],
            #[cfg(feature = "modbus")]
            modbus_rtu: vec![],
            #[cfg(feature = "serialport")]
            serialport: vec![],
            #[cfg(feature = "mqtt")]
            mqtt: vec![],
            sys: Sys::default(),
            #[cfg(feature = "socket")]
            socket: vec![],
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
    use crate::ServerConfig;
    use crate::config::*;

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
            let current_exe = match std::env::current_exe() {
                Ok(exe) => exe,
                Err(e) => {
                    panic!("Failed to get current exe: {}", e);
                }
            };
            let mut expected_path = match current_exe.parent() {
                Some(parent) => parent.to_path_buf(),
                None => {
                    panic!("Failed to get parent directory of current exe");
                }
            };
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
    #[ignore] // This test requires a real config file
    fn test_load_config() {
        let config: ServerConfig = load_config("leanlink").expect("Failed to load config");
        println!("{:#?}", config);
    }
}
