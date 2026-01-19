use futures_util::sink::SinkExt;
use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::{select, sync::Notify};
use tokio_serial::SerialPortBuilderExt;
use tokio_stream::StreamExt;
use tokio_util::codec::Framed;

pub trait FrameAck {
    fn is_ack(&self) -> bool;
}

pub struct SerialPortBuilder {
    path: String,
    baud_rate: u32,
    data_bits: DataBits,
    flow_control: FlowControl,
    parity: Parity,
    stop_bits: StopBits,
    timeout: Duration,
}

impl SerialPortBuilder {
    pub fn new(path: &str, baud_rate: u32) -> Self {
        Self {
            path: path.to_string(),
            baud_rate,
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(0),
        }
    }

    pub fn with_data_bits(mut self, data_bits: DataBits) -> Self {
        self.data_bits = data_bits;
        self
    }

    pub fn with_flow_control(mut self, flow_control: FlowControl) -> Self {
        self.flow_control = flow_control;
        self
    }

    pub fn with_parity(mut self, parity: Parity) -> Self {
        self.parity = parity;
        self
    }

    pub fn with_stop_bits(mut self, stop_bits: StopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn build<T, C>(self) -> SerialPort<T, C> {
        SerialPort {
            framed: None,
            path: self.path,
            baud_rate: self.baud_rate,
            data_bits: self.data_bits,
            flow_control: self.flow_control,
            parity: self.parity,
            stop_bits: self.stop_bits,
            timeout: self.timeout,
            _marker: std::marker::PhantomData,
            busy: Arc::new(AtomicBool::new(false)),
            send_notify: Arc::new(Notify::new()),
        }
    }
}

pub struct SerialPort<T, C> {
    framed: Option<Framed<tokio_serial::SerialStream, C>>,
    path: String,
    baud_rate: u32,
    data_bits: DataBits,
    flow_control: FlowControl,
    parity: Parity,
    stop_bits: StopBits,
    timeout: Duration,
    _marker: std::marker::PhantomData<T>,
    busy: Arc<AtomicBool>,
    send_notify: Arc<Notify>,
}

impl<T, C> SerialPort<T, C>
where
    C: Default,
{
    fn is_busy(&self) -> bool {
        self.will_timeout() && self.busy.load(Ordering::Acquire)
    }

    pub fn will_timeout(&self) -> bool {
        self.timeout != Duration::from_millis(0)
    }

    fn connect_port(&mut self) -> std::io::Result<()> {
        if self.framed.is_none() {
            tracing::info!("Connecting to serial port {}", self.path);
            let serial_port = tokio_serial::new(&self.path, self.baud_rate)
                .data_bits(self.data_bits)
                .flow_control(self.flow_control)
                .parity(self.parity)
                .stop_bits(self.stop_bits)
                .timeout(self.timeout)
                .open_native_async();
            match serial_port {
                Ok(stream) => {
                    self.framed = Some(Framed::new(stream, C::default()));
                }
                Err(e) => {
                    self.framed = None;
                    return Err(e.into());
                }
            }
        }
        Ok(())
    }
}

impl<T, C> SerialPort<T, C>
where
    T: FrameAck,
    C: tokio_util::codec::Decoder<Item = T, Error: std::fmt::Debug> + Unpin + Default,
{
    fn handle_read_result(
        read: Option<Result<T, C::Error>>,
        notify: &Arc<Notify>,
    ) -> std::io::Result<Option<T>> {
        match read {
            Some(item) => match item {
                Ok(data) => {
                    if data.is_ack() {
                        notify.notify_one();
                    }
                    Ok(Some(data))
                }
                Err(e) => {
                    tracing::error!("Error reading from serial port: {:?}", e);
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Error reading from serial port",
                    ));
                }
            },
            None => {
                tracing::info!("SerialPort disconnected");
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotConnected,
                    "SerialPort disconnected",
                ));
            }
        }
    }

    pub async fn next(&mut self) -> std::io::Result<Option<T>> {
        self.connect_port()?;

        let framed = self.framed.as_mut().unwrap();
        let send_notify = self.send_notify.clone();

        select! {
            read = framed.next() => {
                Self::handle_read_result(read, &send_notify)
            }
        }
    }
}

impl<T, C> SerialPort<T, C>
where
    T: Clone,
    C: tokio_util::codec::Encoder<T, Error = std::io::Error> + Unpin + Default,
{
    pub async fn send(&mut self, frame: T) -> std::io::Result<()> {
        self.connect_port()?;

        if self.is_busy() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ResourceBusy,
                "SerialPort is busy",
            ));
        }

        let framed = self.framed.as_mut().unwrap();
        match framed.send(frame).await {
            Ok(()) => Ok(()),
            Err(e) => {
                self.framed = None;
                Err(e)
            }
        }
    }

    pub async fn send_with_timeout(&mut self, frame: T) -> std::io::Result<()> {
        self.connect_port()?;

        if self.is_busy() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ResourceBusy,
                "SerialPort is busy",
            ));
        }

        if !self.will_timeout() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "SerialPort timeout not set",
            ));
        }

        let framed = self.framed.as_mut().unwrap();
        match framed.send(frame).await {
            Ok(()) => match tokio::time::timeout(self.timeout, self.send_notify.notified()).await {
                Ok(_) => Ok(()),
                Err(e) => Err(std::io::Error::new(std::io::ErrorKind::TimedOut, e)),
            },
            Err(e) => {
                self.framed = None;
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::service::serialport::{FrameAck, SerialPortBuilder};

    #[tokio::test]
    async fn test_serial_port() {
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
        let mut serial_port = SerialPortBuilder::new("/dev/tty.usbserial-0001".into(), 9600)
            .with_timeout(Duration::from_secs(5))
            .build::<MyFrame, MyCodec>();
        loop {
            match serial_port
                .send(MyFrame {
                    data: bytes::Bytes::from_static(b"value"),
                })
                .await
            {
                Ok(()) => {}
                Err(e) => {
                    tracing::error!("Error: {:?}", e);
                }
            }

            match serial_port.next().await {
                Ok(result) => match result {
                    Some(frame) => {
                        println!("Received: {:?}", frame.data);
                    }
                    None => {
                        println!("No data");
                    }
                },
                Err(e) => {
                    tracing::error!("Error: {:?}", e);
                }
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }
}
