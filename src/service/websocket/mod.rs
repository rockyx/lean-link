use std::{collections::HashMap, sync::Arc};

use futures::{SinkExt, StreamExt};
use tokio::{
    net::{TcpListener, TcpStream},
    select,
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
    },
};
use tokio_tungstenite::{accept_async, tungstenite::Message};

pub enum WebSocketMessage {
    NewConnected(String),
    Message(Message),
}

#[derive(Clone)]
pub struct WebSocketServer {
    writer_map: Arc<Mutex<HashMap<String, Sender<Message>>>>,
}

impl WebSocketServer {
    pub fn new() -> Self {
        WebSocketServer {
            writer_map: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn start(
        &self,
        addr: &str,
        port: &u16,
    ) -> std::io::Result<Receiver<WebSocketMessage>> {
        let (read_sender, read_recver) = mpsc::channel::<WebSocketMessage>(1024);
        let addr = format!("{}:{}", addr, port);
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("WebSocket server listening on {}", addr);

        let writer_map = self.writer_map.clone();
        tokio::spawn(async move {
            start_listening(listener, writer_map, read_sender).await;
        });

        Ok(read_recver)
    }

    pub async fn broadcast(&self, message: Message) {
        let mut writer_map = self.writer_map.lock().await;
        if writer_map.is_empty() {
            return;
        }

        if writer_map.len() == 1 {
            let (_, writer) = writer_map.iter().next().unwrap();
            let _ = writer.send(message).await;
            return;
        }
        
        let mut send_futures = Vec::new();
        for (_, writer) in writer_map.iter_mut() {
            send_futures.push(writer.send(message.clone()));
        }

        futures::future::join_all(send_futures).await;
    }

    pub async fn send(&self, id: &str, message: Message) {
        let mut writer_map = self.writer_map.lock().await;
        if let Some(writer) = writer_map.get_mut(id) {
            let _ = writer.send(message).await;
        }
    }
}

async fn start_listening(
    listener: TcpListener,
    writer_map: Arc<Mutex<HashMap<String, Sender<Message>>>>,
    read_sender: Sender<WebSocketMessage>,
) {
    while let Ok((stream, _)) = listener.accept().await {
        let writer_map = writer_map.clone();
        tokio::spawn(handle_connection(stream, writer_map, read_sender.clone()));
    }
}

async fn handle_connection(
    raw_stream: TcpStream,
    writer_map: Arc<Mutex<HashMap<String, Sender<Message>>>>,
    read_sender: Sender<WebSocketMessage>,
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
        let mut writer_map = writer_map.lock().await;
        writer_map.insert(peer_addr.clone(), writer_send);
    }

    let (mut writer, mut reader) = ws_stream.split();

    let _ = read_sender
        .send(WebSocketMessage::NewConnected(peer_addr.clone()))
        .await;
    loop {
        select! {
            message = reader.next() => {
                match message {
                    Some(msg) => {
                        match msg {
                            Ok(data) => {
                                let _ = read_sender.send(WebSocketMessage::Message(data)).await;
                            },
                            Err(e) => {
                                tracing::error!("WebSocket Error: {}", e);
                                let mut writer_map = writer_map.lock().await;
                                writer_map.remove(&peer_addr);
                                break;
                            }
                        }
                    },
                    None => {
                        let mut writer_map = writer_map.lock().await;
                        writer_map.remove(&peer_addr);
                        break;
                    },
                }
            },

            send_msg = writer_recv.recv() => {
                match send_msg {
                    Some(msg) => {
                        let _ = writer.send(msg).await;
                    },
                    None => {
                        break;
                    },
                }
            },
            _ = tokio::time::sleep(std::time::Duration::from_secs(30)) => {
                let _ = writer.send(Message::Ping(bytes::Bytes::new())).await;
            }
        }
    }
}
