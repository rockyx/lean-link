use rumqttc::QoS;

pub use rumqttc::*;
pub mod client;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct MqttTopic {
    pub topic: String,
    #[serde(with = "string_to_qos")]
    pub qos: QoS,
}

impl Default for MqttTopic {
    fn default() -> Self {
        MqttTopic {
            topic: "leanlink/topic".to_string(),
            qos: QoS::AtLeastOnce,
        }
    }
}

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