use directories::ProjectDirs;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
#[cfg(any(feature = "modbus", feature = "serialport"))]
use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::path::{Path, PathBuf};
#[cfg(any(feature = "modbus", feature = "serialport", feature = "web"))]
use std::{fs::File, time::Duration};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DatabaseConfig {
    pub url: String,
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WebConfig {
    pub host: String,
    pub port: u16,
}

#[cfg(feature = "web")]
#[derive(Debug, Deserialize, Serialize, Clone)]
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
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ModbusTCPConfig {
    pub host: String,
    pub port: u16,
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

#[derive(Debug, Deserialize, Serialize, Clone)]
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

/// 获取跨平台配置文件路径
pub fn get_config_path(app_name: &str) -> Option<PathBuf> {
    // 区分操作系统
    if cfg!(target_os = "linux") {
        // Linux: /etc/app-name/config.yaml
        Some(Path::new("/etc").join(app_name).join("config.yaml"))
    } else if cfg!(target_os = "windows") {
        // Windows: 应用安装目录下的 etc/config.yaml
        let exe_dir = std::env::current_exe().ok()?.parent()?.to_path_buf();
        Some(exe_dir.join("etc").join("config.yaml"))
    } else {
        // 其他系统（如 macOS）使用标准配置目录
        ProjectDirs::from("com", "", app_name).map(|dirs| dirs.config_dir().join("config.yaml"))
    }
}

pub fn load_config<UserConfig>(app_name: &str) -> std::io::Result<ServerConfig<UserConfig>>
where
    UserConfig: DeserializeOwned + Serialize + Clone,
{
    let config_path = get_config_path(app_name).ok_or(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "Could not determine config path",
    ))?;

    tracing::info!("Loading config from {:?}", config_path);

    let normalized_path = normpath::PathExt::normalize(config_path.as_path())?;

    // 打开文件并解析
    let file = File::open(normalized_path.as_path())?;
    let config: ServerConfig<UserConfig> = serde_yaml::from_reader(file).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Failed to parse config: {}", e),
        )
    })?;
    Ok(config)
}

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
    #[derive(Debug, Deserialize, Serialize, Clone)]
    struct MyUserConfig {
        pub custom_field: String,
    }
    let config: ServerConfig<MyUserConfig> =
        load_config("leanlink").expect("Failed to load config");
    println!("{:#?}", config);
}
