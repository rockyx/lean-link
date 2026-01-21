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

pub mod local_time {
    use super::to_local_time;
    use chrono::{DateTime, FixedOffset};
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S>(dt: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        to_local_time(dt, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&s)
            .map_err(|_| D::Error::custom(format!("Invalid datetime format: {}", s)))
    }
}
pub mod local_time_option {
    use super::to_local_time_option;
    use chrono::{DateTime, FixedOffset};
    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S>(
        dt: &Option<DateTime<FixedOffset>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        to_local_time_option(dt, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<FixedOffset>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let result = DateTime::parse_from_rfc3339(&s)
            .map_err(|_| D::Error::custom(format!("Invalid datetime format: {}", s)))?;
        Ok(Some(result))
    }
}

pub mod string_to_duration {
    use serde::{Deserialize, Deserializer, Serializer, de::Error};
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

#[cfg(target_os = "linux")]
pub fn set_ds1307_from_local_time(bus: u16, addr: u16) -> Result<(), String> {
    use crate::utils::bcd::dec_to_bcd;
    use chrono::{Datelike, Local, Timelike};
    use std::process::Command;

    // Read local time
    let now = Local::now();
    let mut sec = dec_to_bcd(now.second() as u8).map_err(|e| e.to_string())?;
    // Clear CH bit (bit7) in seconds register
    sec &= 0x7F;

    let min = dec_to_bcd(now.minute() as u8).map_err(|e| e.to_string())?;
    let mut hour = dec_to_bcd(now.hour() as u8).map_err(|e| e.to_string())?;
    // 24-hour mode: ensure bit6 = 0
    hour &= 0x3F;

    // DS1307: day of week 1..=7, with 1=Sunday
    let dow = match now.weekday() {
        chrono::Weekday::Sun => 1,
        chrono::Weekday::Mon => 2,
        chrono::Weekday::Tue => 3,
        chrono::Weekday::Wed => 4,
        chrono::Weekday::Thu => 5,
        chrono::Weekday::Fri => 6,
        chrono::Weekday::Sat => 7,
    };
    let dow = dec_to_bcd(dow as u8).map_err(|e| e.to_string())?;

    let dom = dec_to_bcd(now.day() as u8).map_err(|e| e.to_string())?;
    let mon = dec_to_bcd(now.month() as u8).map_err(|e| e.to_string())?;
    let year = {
        let y = (now.year() % 100) as i32;
        let y = if y < 0 { 0 } else { y as u8 };
        dec_to_bcd(y).map_err(|e| e.to_string())?
    };

    // Build i2cset command: write starting at register 0x00 with 7 bytes
    // i2cset -y <bus> <addr> 0x00 sec min hour dow dom mon year i
    let status = Command::new("sudo")
        .arg("i2cset")
        .arg("-y")
        .arg(bus.to_string())
        .arg(format!("0x{:02x}", addr))
        .arg("0x00")
        .arg(format!("0x{:02x}", sec))
        .arg(format!("0x{:02x}", min))
        .arg(format!("0x{:02x}", hour))
        .arg(format!("0x{:02x}", dow))
        .arg(format!("0x{:02x}", dom))
        .arg(format!("0x{:02x}", mon))
        .arg(format!("0x{:02x}", year))
        .arg("i")
        .output()
        .map_err(|e| format!("failed to spawn i2cset: {}", e))?;

    if status.status.success() {
        Ok(())
    } else {
        Err(format!(
            "i2cset failed (code {:?}): {}",
            status.status.code(),
            String::from_utf8_lossy(&status.stderr).trim()
        ))
    }
}

#[cfg(not(target_os = "linux"))]
pub fn set_ds1307_from_local_time(_bus: u16, _addr: u16) -> Result<(), String> {
    Err("set_ds1307_from_local_time is only supported on Linux".into())
}

#[cfg(target_os = "linux")]
pub fn set_local_time_from_ds1307(bus: u16, addr: u16) -> Result<(), String> {
    use crate::utils::{bcd::bcd_to_dec, i2c::i2c_read_reg};
    use std::process::Command;

    // Read DS1307 registers
    let raw_sec = i2c_read_reg(bus, addr, 0x00)?;
    let raw_min = i2c_read_reg(bus, addr, 0x01)?;
    let raw_hour = i2c_read_reg(bus, addr, 0x02)?;
    let _raw_dow = i2c_read_reg(bus, addr, 0x03)?; // not needed for timedatectl
    let raw_dom = i2c_read_reg(bus, addr, 0x04)?;
    let raw_mon = i2c_read_reg(bus, addr, 0x05)?;
    let raw_year = i2c_read_reg(bus, addr, 0x06)?;

    // Decode BCD with proper masks
    let sec = bcd_to_dec(raw_sec & 0x7F).map_err(|e| e.to_string())?; // clear CH bit
    let min = bcd_to_dec(raw_min & 0x7F).map_err(|e| e.to_string())?;

    // Hour: handle 24h or 12h mode
    let hour = if (raw_hour & 0x40) == 0 {
        // 24-hour mode, bits 5..0
        bcd_to_dec(raw_hour & 0x3F).map_err(|e| e.to_string())? as u32
    } else {
        // 12-hour mode
        let pm = (raw_hour & 0x20) != 0;
        let h12 = bcd_to_dec(raw_hour & 0x1F).map_err(|e| e.to_string())? as u32; // 1..12
        let mut h24 = if h12 == 12 { 0 } else { h12 };
        if pm {
            h24 = (h24 + 12) % 24;
        }
        h24
    } as u8;

    let day = bcd_to_dec(raw_dom & 0x3F).map_err(|e| e.to_string())?;
    let month = bcd_to_dec(raw_mon & 0x1F).map_err(|e| e.to_string())?;
    let year2 = bcd_to_dec(raw_year).map_err(|e| e.to_string())?;
    let year = 2000u16 + (year2 as u16);

    // Basic sanity checks
    if !(1..=31).contains(&day) {
        return Err(format!("invalid day from RTC: {}", day));
    }
    if !(1..=12).contains(&month) {
        return Err(format!("invalid month from RTC: {}", month));
    }
    if hour > 23 {
        return Err(format!("invalid hour from RTC: {}", hour));
    }
    if min > 59 {
        return Err(format!("invalid minute from RTC: {}", min));
    }
    if sec > 59 {
        return Err(format!("invalid second from RTC: {}", sec));
    }

    let disable_ntp_output = Command::new("sudo")
        .arg("timedatectl")
        .arg("set-ntp")
        .arg("false")
        .output();

    tracing::info!(
        "disable_ntp_output command output: {:?}",
        disable_ntp_output
    );

    // Format as "YYYY-MM-DD HH:MM:SS"
    let ts = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        year, month, day, hour, min, sec
    );

    // Use sudo timedatectl to set system time
    let output = Command::new("sudo")
        .arg("timedatectl")
        .arg("set-time")
        .arg(&ts)
        .output()
        .map_err(|e| format!("failed to spawn 'sudo timedatectl': {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "timedatectl failed (code {:?}): {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(not(target_os = "linux"))]
pub fn set_local_time_from_ds1307(_bus: u16, _addr: u16) -> Result<(), String> {
    Err("set_local_time_from_ds1307 is only supported on Linux".into())
}

pub fn duration_to_seconds_string(duration: std::time::Duration, decimals: usize) -> String {
    let total_seconds = duration.as_secs_f64();
    format!("{:.*}", decimals, total_seconds)
}
