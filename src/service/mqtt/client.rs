use rand::{Rng, distr::Alphanumeric};
use rumqttc::{AsyncClient, ClientError, ConnectionError, Event, EventLoop, MqttOptions, QoS};
use std::time::Duration;

use crate::config::MqttConfig;

pub struct ClientBuilder {
    host: String,
    port: u16,
    username: String,
    password: String,
    client_id: String,
    keep_alive: Duration,
}

fn generate_mqtt_id() -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(12)
        .map(char::from)
        .collect()
}

impl ClientBuilder {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            username: "".to_string(),
            password: "".to_string(),
            client_id: generate_mqtt_id(),
            keep_alive: Duration::from_secs(10),
        }
    }

    pub fn with_config(mut self, config: &MqttConfig) -> Self {
        self.host = config.host.clone();
        self.port = config.port;
        self.username = config.username.clone();
        self.password = config.password.clone();
        self.client_id = config.client_id.clone();
        self.keep_alive = config.keep_alive;
        self
    }

    pub fn with_username(mut self, username: &str) -> Self {
        self.username = username.to_string();
        self
    }

    pub fn with_password(mut self, password: &str) -> Self {
        self.password = password.to_string();
        self
    }

    pub fn with_client_id(mut self, client_id: &str) -> Self {
        self.client_id = client_id.to_string();
        self
    }

    pub fn with_keep_alive(mut self, keep_alive: Duration) -> Self {
        self.keep_alive = keep_alive;
        self
    }

    pub fn build(self) -> (AsyncClient, EventLoop) {
        let mut mqtt_options = MqttOptions::new(self.client_id, self.host, self.port);
        if self.keep_alive.as_secs() > 0 {
            mqtt_options.set_keep_alive(self.keep_alive);
        }
        if self.username != "" && self.password != "" {
            mqtt_options.set_credentials(self.username, self.password);
        }

        AsyncClient::new(mqtt_options, 1024)
    }
}
