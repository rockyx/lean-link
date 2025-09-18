use chrono::{DateTime, FixedOffset, Local};
use serde::Serializer;

pub fn to_local_time<S>(dt: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // 1. 将 DateTime<FixedOffset> 转换为系统本地时区的时间
    let local_time: DateTime<Local> = dt.with_timezone(&Local);
    // 2. 将转换后的时间序列化为字符串（例如 RFC3339 格式）
    serializer.serialize_str(&local_time.to_rfc3339())
}

pub fn to_local_time_option<S>(
    dt: &Option<DateTime<FixedOffset>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    if dt.is_none() {
        return serializer.serialize_none();
    }
    let dt = dt.as_ref().unwrap();
    // 1. 将 DateTime<FixedOffset> 转换为系统本地时区的时间
    let local_time: DateTime<Local> = dt.with_timezone(&Local);
    // 2. 将转换后的时间序列化为字符串（例如 RFC3339 格式）
    serializer.serialize_str(&local_time.to_rfc3339())
}

pub mod string_to_duration {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}s", duration.as_secs()))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let s = String::deserialize(deserializer)?;

        // Number is an integer followed (without a space) by a unit of time.​​
        // eg: "30s", "1m", "2h", "500ms", "1.5s"
        let re = regex::Regex::new(r"^\s*(\d+\.?\d*)\s*([a-zA-Z]+)\s*$").unwrap();

        if let Some(caps) = re.captures(&s) {
            let value: f64 = caps[1].parse().map_err(D::Error::custom)?;
            let unit = &caps[2].to_lowercase();

            match unit.as_str() {
                "ms" | "millis" | "millisecond" | "milliseconds" => {
                    Ok(Duration::from_millis(value as u64))
                }
                "s" | "sec" | "second" | "seconds" => Ok(Duration::from_secs(value as u64)),
                "m" | "min" | "minute" | "minutes" => {
                    Ok(Duration::from_secs((value * 60.0) as u64))
                }
                "h" | "hour" | "hours" => Ok(Duration::from_secs((value * 3600.0) as u64)),
                "d" | "day" | "days" => Ok(Duration::from_secs((value * 86400.0) as u64)),
                _ => Err(D::Error::custom(format!("Unknown time unit: {}", unit))),
            }
        } else {
            // If no unit is present, try to parse as a plain number (seconds)
            match s.parse::<u64>() {
                Ok(secs) => Ok(Duration::from_secs(secs)),
                Err(_) => Err(D::Error::custom(format!("Invalid duration format: {}", s))),
            }
        }
    }
}

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
