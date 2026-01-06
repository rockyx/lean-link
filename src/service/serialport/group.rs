use super::{FrameAck, SerialPort};
use futures::stream::FuturesUnordered;
use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicI16, Ordering},
    },
};
use tokio::{select, sync::RwLock};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

pub struct SerialPortGroup<T, C> {
    groups: Arc<RwLock<HashMap<String, SerialPort<T, C>>>>,
    ack_counter: Arc<AtomicI16>,
    cancel_token: CancellationToken,
}

impl<T, C> SerialPortGroup<T, C>
where
    T: FrameAck + Clone,
    C: tokio_util::codec::Decoder<Item = T, Error: std::fmt::Debug>
        + tokio_util::codec::Encoder<T, Error = std::io::Error>
        + Unpin
        + Default,
{
    pub fn new() -> Self {
        Self {
            groups: Arc::new(RwLock::new(HashMap::new())),
            ack_counter: Arc::new(AtomicI16::new(-1)),
            cancel_token: CancellationToken::new(),
        }
    }

    pub async fn add_serialport(&mut self, path: &str, port: SerialPort<T, C>) {
        // TODO: maybe compare serial port settings
        self.cancel_token.cancel();
        let mut groups = self.groups.write().await;
        if groups.contains_key(path) {
            groups.remove(path);
            return;
        }
        groups.insert(path.to_string(), port);
    }

    pub async fn remove_serialport(&mut self, path: &str) {
        self.cancel_token.cancel();
        let mut groups = self.groups.write().await;
        groups.remove(path);
    }

    pub async fn send(&self, frame: T) -> std::io::Result<()> {
        if self.ack_counter.load(Ordering::Acquire) >= 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ResourceBusy,
                "Waiting for ack",
            ));
        }
        let mut need_ack_count = 0;
        let mut groups = self.groups.write().await;
        for port in groups.values_mut() {
            if port.will_timeout() {
                need_ack_count += 1;
            }
            port.send(frame.clone()).await?;
        }
        if need_ack_count > 0 {
            self.ack_counter.store(0, Ordering::Release);
        }
        Ok(())
    }

    fn reset_ack_counter(&self) {
        self.ack_counter.store(-1, Ordering::Release);
    }

    pub async fn next(&self) -> std::io::Result<Option<T>> {
        let mut groups = self.groups.write().await;

        let group_len = groups.len();

        if groups.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No serial port is available",
            ));
        }

        if group_len == 1 {
            let port = groups.values_mut().next().unwrap();
            self.reset_ack_counter();
            return port.next().await;
        }

        let mut need_ack_count = 0;
        let mut futures = FuturesUnordered::new();
        for port in groups.values_mut() {
            if port.will_timeout() {
                need_ack_count += 1;
            }
            futures.push(port.next());
        }

        loop {
            select! {
                result = futures.next() => {
                    match result {
                        Some(Ok(data)) => {
                            match data {
                                Some(frame) => {
                                    if frame.is_ack() {
                                        self.ack_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                                        if self.ack_counter.load(Ordering::Acquire) as usize >= need_ack_count {
                                            self.reset_ack_counter();
                                            return Ok(Some(frame));
                                        }
                                    } else {
                                        self.reset_ack_counter();
                                        return Ok(Some(frame));
                                    }
                                }
                                None => {
                                    tracing::debug!("No frame received");
                                    self.reset_ack_counter();
                                    return Ok(None);
                                }
                            }
                        }
                        Some(Err(e)) => {
                            self.reset_ack_counter();
                            return Err(e);
                        }
                        None => {
                            self.reset_ack_counter();
                            return Ok(None);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::service::serialport::{FrameAck, SerialPortBuilder, SerialPortGroup};

    #[tokio::test]
    async fn test_serial_port_group() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();

        #[derive(Clone)]
        struct MyFrame {
            data: bytes::Bytes,
        }
        impl FrameAck for MyFrame {
            fn is_ack(&self) -> bool {
                false
            }
        }

        struct MyCodec {}

        impl Default for MyCodec {
            fn default() -> Self {
                Self {}
            }
        }

        impl tokio_util::codec::Decoder for MyCodec {
            type Item = MyFrame;
            type Error = std::io::Error;

            fn decode(
                &mut self,
                src: &mut bytes::BytesMut,
            ) -> Result<Option<Self::Item>, Self::Error> {
                if src.is_empty() {
                    return Ok(None);
                }
                if src.len() < 11 {
                    return Ok(None);
                }

                let len = src.len();
                let data = src.split_to(len);
                Ok(Some(MyFrame {
                    data: data.freeze(),
                }))
            }
        }

        impl tokio_util::codec::Encoder<MyFrame> for MyCodec {
            type Error = std::io::Error;

            fn encode(
                &mut self,
                item: MyFrame,
                dst: &mut bytes::BytesMut,
            ) -> Result<(), Self::Error> {
                dst.extend_from_slice(&item.data);
                Ok(())
            }
        }

        let serial_port = SerialPortBuilder::new("/dev/tty.usbserial-0001".into(), 9600)
            .with_timeout(std::time::Duration::from_secs(5))
            .build::<MyFrame, MyCodec>();

        let mut serial_port_group = SerialPortGroup::new();
        serial_port_group
            .add_serialport("/dev/tty.usbserial-0001", serial_port)
            .await;

        loop {
            {
                let result = serial_port_group
                    .send(MyFrame {
                        data: bytes::Bytes::from_static(b"value"),
                    })
                    .await;
                if result.is_ok() {
                    tracing::info!("Sended frame");
                } else if result.is_err() {
                    tracing::error!("Error sending frame: {:?}", result.err());
                }
            }
            {
                let result = serial_port_group.next().await;
                if result.is_ok() {
                    tracing::info!("Received frame");
                } else if result.is_err() {
                    tracing::error!("Error receiving frame: {:?}", result.err());
                }
            }

            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }
}
