use std::sync::Arc;

use bytes::{Bytes, BytesMut};
use dashmap::DashMap;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    select,
    sync::{broadcast, mpsc},
};

use crate::config::SocketConfig;

#[derive(Debug)]
pub enum SocketMessage {
    NewConnected(String),
    Message(String, Bytes),
}

#[derive(Clone)]
pub struct SocketServer {
    socket_config: SocketConfig,
    writer_map: Arc<DashMap<String, mpsc::Sender<Bytes>>>,
    broadcast_sender: broadcast::Sender<Bytes>,
}

impl SocketServer {
    pub fn new(socket_config: SocketConfig) -> Self {
        let (tx, _) = broadcast::channel(16);
        SocketServer {
            socket_config,
            writer_map: Arc::new(DashMap::new()),
            broadcast_sender: tx,
        }
    }

    pub async fn start(&self) -> std::io::Result<mpsc::Receiver<SocketMessage>> {
        let (read_sender, read_receiver) = mpsc::channel::<SocketMessage>(1024);
        let addr = format!("{}:{}", self.socket_config.host, self.socket_config.port);
        let listener = TcpListener::bind(&addr).await?;

        tracing::info!("Socket server listening on {}", addr);

        let broadcast_sender = self.broadcast_sender.clone();
        let write_map = self.writer_map.clone();
        tokio::spawn(async move {
            start_listening(listener, broadcast_sender, write_map, read_sender).await;
        });
        Ok(read_receiver)
    }

    pub async fn broadcast(&self, message: Bytes) {
        let _ = self.broadcast_sender.send(message);
    }

    pub async fn send(&self, id: &str, message: Bytes) {
        if let Some(write) = self.writer_map.get_mut(id) {
            let _ = write.send(message).await;
        }
    }
}

async fn start_listening(
    listener: TcpListener,
    broadcast_sender: broadcast::Sender<Bytes>,
    writer_map: Arc<DashMap<String, mpsc::Sender<Bytes>>>,
    read_sender: mpsc::Sender<SocketMessage>,
) {
    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(handle_connection(
            stream,
            broadcast_sender.clone(),
            writer_map.clone(),
            read_sender.clone(),
        ));
    }
}

async fn handle_connection(
    mut raw_stream: TcpStream,
    broadcast_sender: broadcast::Sender<Bytes>,
    writer_map: Arc<DashMap<String, mpsc::Sender<Bytes>>>,
    read_sender: mpsc::Sender<SocketMessage>,
) {
    tracing::info!(
        "New socket connection established: {}",
        raw_stream.peer_addr().unwrap()
    );

    let _ = read_sender
        .send(SocketMessage::NewConnected(
            raw_stream.peer_addr().unwrap().to_string(),
        ))
        .await;

    let mut buffer = BytesMut::with_capacity(1024);
    let mut broadcast_receiver = broadcast_sender.subscribe();
    let (tx, mut rx) = mpsc::channel::<Bytes>(32);
    writer_map.insert(raw_stream.peer_addr().unwrap().to_string(), tx);

    loop {
        select! {
            read_result = raw_stream.read_buf(&mut buffer) => {
                match read_result {
                    Ok(0) => {
                        tracing::info!("Socket connection closed: {}", raw_stream.peer_addr().unwrap());
                        writer_map.remove(&raw_stream.peer_addr().unwrap().to_string());
                        break;
                    }
                    Ok(n) => {
                        tracing::info!("Received {} bytes from {}", n, raw_stream.peer_addr().unwrap());
                        tracing::debug!("Data: {:?}", &buffer[..n]);
                        let _ = read_sender
                            .send(SocketMessage::Message(
                                raw_stream.peer_addr().unwrap().to_string(),
                                buffer.split_to(n).freeze(),
                            ))
                            .await;
                        buffer.clear();
                    }
                    Err(e) => {
                        tracing::error!("Error reading from socket: {}", e);
                    }
                }
            }

            broadcast_msg = broadcast_receiver.recv() => {
                match broadcast_msg {
                    Ok(msg) => {
                        if let Err(e) = raw_stream.write(&msg).await {
                            tracing::error!("Error writing to socket: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::error!("Error receiving broadcast message: {}", e);
                    }
                }
            }

            send_msg = rx.recv() => {
                match send_msg {
                    Some(msg) => {
                        if let Err(e) = raw_stream.write(&msg).await {
                            tracing::error!("Error writing to socket: {}", e);
                        }
                    }
                    None => {
                        tracing::info!("Sender dropped, closing connection: {}", raw_stream.peer_addr().unwrap());
                        writer_map.remove(&raw_stream.peer_addr().unwrap().to_string());
                        break;
                    }
                }
            }
        }
    }
}
