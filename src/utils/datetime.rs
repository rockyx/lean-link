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

pub fn to_local_time_option<S>(dt: &Option<DateTime<FixedOffset>>, serializer: S) -> Result<S::Ok, S::Error>
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
