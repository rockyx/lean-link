use std::sync::Arc;

use crate::config::{Sys, WebSocketConfig};
use bytes::Bytes;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt, stream::SplitSink};
use serde::{Deserialize, Serialize};
use tokio::{
    net::{TcpListener, TcpStream},
    select,
    sync::{broadcast, mpsc},
};
use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WsMessage<T> {
    pub topic: String,
    pub payload: T,
}

impl<T> Into<Message> for WsMessage<T>
where
    T: Serialize,
{
    fn into(self) -> Message {
        let json_data = serde_json::to_value(&self).unwrap();
        tracing::debug!("send message: {}", json_data);

        Message::Text(json_data.to_string().into())
    }
}

#[derive(Debug)]
pub enum WebSocketMessage {
    NewConnected(String),
    Message(String, Message),
}

#[derive(Clone)]
pub struct WebSocketServer {
    writer_map: Arc<DashMap<String, mpsc::Sender<Message>>>,
    websocket_config: WebSocketConfig,
    sys_config: Sys,
    broadcast_sender: broadcast::Sender<Message>,
}

impl WebSocketServer {
    pub fn new(websocket_config: WebSocketConfig, sys_config: Sys) -> Self {
        WebSocketServer {
            writer_map: Arc::new(DashMap::new()),
            websocket_config,
            sys_config,
            broadcast_sender: broadcast::channel(16).0,
        }
    }

    pub async fn start(&self) -> std::io::Result<mpsc::Receiver<WebSocketMessage>> {
        let (read_sender, read_recver) = mpsc::channel::<WebSocketMessage>(1024);
        let addr = format!(
            "{}:{}",
            self.websocket_config.host, self.websocket_config.port
        );
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("WebSocket server listening on {}", addr);

        let writer_map = self.writer_map.clone();
        let websocket_config = self.websocket_config.clone();
        let sys_config = self.sys_config.clone();
        let broadcast_sender = self.broadcast_sender.clone();
        tokio::spawn(async move {
            start_listening(
                listener,
                writer_map,
                read_sender,
                websocket_config,
                sys_config,
                broadcast_sender,
            )
            .await;
        });

        Ok(read_recver)
    }

    pub async fn broadcast(&self, message: Message) {
        let _ = self.broadcast_sender.send(message);
    }

    pub async fn send(&self, id: &str, message: Message) {
        if let Some(writer) = self.writer_map.get_mut(id) {
            let _ = writer.send(message).await;
        }
    }
}

async fn start_listening(
    listener: TcpListener,
    writer_map: Arc<DashMap<String, mpsc::Sender<Message>>>,
    read_sender: mpsc::Sender<WebSocketMessage>,
    websocket_config: WebSocketConfig,
    sys_config: Sys,
    broadcast_sender: broadcast::Sender<Message>,
) {
    while let Ok((stream, _)) = listener.accept().await {
        let writer_map = writer_map.clone();
        tokio::spawn(handle_connection(
            stream,
            writer_map,
            read_sender.clone(),
            websocket_config.clone(),
            sys_config.clone(),
            broadcast_sender.clone(),
        ));
    }
}

async fn handle_websocket_message(
    message: &Result<Message, tokio_tungstenite::tungstenite::Error>,
    writer_map: &Arc<DashMap<String, mpsc::Sender<Message>>>,
    writer: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    read_sender: &mpsc::Sender<WebSocketMessage>,
    peer_addr: &str,
    sys_config: &Sys,
) -> bool {
    match message {
        Ok(data) => match data {
            Message::Ping(_) => {
                let _ = writer.send(Message::Pong(Bytes::new())).await;
                return true;
            }
            Message::Text(msg) => {
                let value = serde_json::from_str::<serde_json::Value>(&msg);
                if value.is_err() {
                    tracing::error!("Invalid JSON message: {}", msg);
                    return true;
                }

                let value = value.unwrap();

                if let Some(topic) = value.get("topic").and_then(|v| v.as_str()) {
                    if topic == "syncSysTime" && sys_config.sync_time_from_client {
                        if let Some(payload) = value.get("payload") {
                            match payload {
                                serde_json::Value::String(s) => {
                                    tracing::info!("syncSysTime payload (string): {}", s);

                                    #[cfg(target_os = "linux")]
                                    {
                                        use std::process::Command;
                                        let disable_ntp_output = Command::new("sudo")
                                            .arg("timedatectl")
                                            .arg("set-ntp")
                                            .arg("false")
                                            .output();

                                        tracing::info!(
                                            "disable_ntp_output command output: {:?}",
                                            disable_ntp_output
                                        );

                                        let output = Command::new("sudo")
                                            .arg("timedatectl")
                                            .arg("set-time")
                                            .arg(s)
                                            .output();
                                        tracing::info!("syncSysTime command output: {:?}", output);

                                        if sys_config.sync_time_from_rtc {
                                            let rtc_i2c_dev = sys_config.rtc_i2c_dev.clone();
                                            let rtc_i2c_addr = sys_config.rtc_i2c_addr;

                                            let bus_result =
                                                crate::utils::i2c::path_to_i2c_bus(&rtc_i2c_dev);

                                            if bus_result.is_err() {
                                                tracing::info!(
                                                    "syncSysTime command output: {:?}",
                                                    bus_result
                                                );
                                            } else {
                                                let bus = bus_result.unwrap();
                                                let output =
                                                crate::utils::datetime::set_ds1307_from_local_time(
                                                    bus,
                                                    rtc_i2c_addr,
                                                );
                                                tracing::info!(
                                                    "syncSysTime command output: {:?}",
                                                    output
                                                );
                                            }
                                        }
                                    }
                                    return true;
                                }
                                _ => {}
                            }
                        }
                    }
                }
                let _ = read_sender
                    .send(WebSocketMessage::Message(peer_addr.into(), data.clone()))
                    .await;
                return true;
            }
            _ => {
                let _ = read_sender
                    .send(WebSocketMessage::Message(peer_addr.into(), data.clone()))
                    .await;
                return true;
            }
        },
        Err(e) => {
            tracing::error!("WebSocket Error: {}", e);
            writer_map.remove(peer_addr);
            return false;
        }
    }
}

async fn handle_message(
    message: &Option<Result<Message, tokio_tungstenite::tungstenite::Error>>,
    writer_map: &Arc<DashMap<String, mpsc::Sender<Message>>>,
    writer: &mut SplitSink<WebSocketStream<TcpStream>, Message>,
    read_sender: &mpsc::Sender<WebSocketMessage>,
    peer_addr: &str,
    sys_config: &Sys,
) -> bool {
    match message {
        Some(msg) => {
            handle_websocket_message(&msg, writer_map, writer, read_sender, peer_addr, sys_config)
                .await
        }
        None => {
            writer_map.remove(peer_addr);
            return false;
        }
    }
}

async fn handle_connection(
    raw_stream: TcpStream,
    writer_map: Arc<DashMap<String, mpsc::Sender<Message>>>,
    read_sender: mpsc::Sender<WebSocketMessage>,
    websocket_config: WebSocketConfig,
    sys_config: Sys,
    broadcast_sender: broadcast::Sender<Message>,
) {
    let ws_stream = accept_async(raw_stream).await;
    if ws_stream.is_err() {
        tracing::error!(
            "Error accepting WebSocket connection: {}",
            ws_stream.err().unwrap()
        );
        return;
    }

    let ws_stream = ws_stream.unwrap();

    let peer_addr = ws_stream.get_ref().peer_addr().unwrap().to_string();

    tracing::info!(
        "New WebSocket connection: {}",
        ws_stream.get_ref().peer_addr().unwrap()
    );

    let (writer_send, mut writer_recv) = mpsc::channel::<Message>(100);
    {
        writer_map.insert(peer_addr.clone(), writer_send);
    }

    let (mut writer, mut reader) = ws_stream.split();

    let _ = read_sender
        .send(WebSocketMessage::NewConnected(peer_addr.clone()))
        .await;

    let mut broadcast_receiver = broadcast_sender.subscribe();
    loop {
        select! {
            message = reader.next() => {
                if !handle_message(&message, &writer_map, &mut writer, &read_sender, &peer_addr, &sys_config).await {
                    break;
                }
            },

            send_msg = writer_recv.recv() => {
                match send_msg {
                    Some(msg) => {
                        let _ = writer.send(msg).await;
                    },
                    None => {},
                }
            },

            broadcast_msg = broadcast_receiver.recv() => {
                match broadcast_msg {
                    Ok(msg) => {
                        let _ = writer.send(msg).await;
                    }
                    Err(e) => {
                        tracing::error!("Error receiving broadcast message: {}", e);
                    }
                }
            }

            _ = tokio::time::sleep(websocket_config.heartbeat_interval) => {
                let _ = writer.send(Message::Ping(bytes::Bytes::new())).await;
            }
        }
    }
}
