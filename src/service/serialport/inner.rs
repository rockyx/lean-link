use super::{
    DataParser, DataParserEvent, FrameHandler, FrameHandlerEvent, FromPortEvent, SerialPortConfig,
    ToPortEvent,
    holder::{HeartbeatCommand, SerialPortHolder, SerialPortHolderBuilder},
};
use std::{
    collections::HashMap,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicI16, Ordering},
    },
    time::Duration,
};
use tokio::{
    select,
    sync::{
        RwLock,
        mpsc::{self, Receiver, Sender},
    },
};

pub type HeartbeatCommandBuilder = Box<dyn Fn() -> HeartbeatCommand + Send + Sync>;

pub struct SerialPortInnerBuilder {
    configs: Vec<SerialPortConfig>,
    data_parser: DataParser,
    frame_handler: FrameHandler,
    heartbeat_command: Option<HeartbeatCommandBuilder>,
    heartbeat_interval: Duration,
}

impl SerialPortInnerBuilder {
    pub fn new(
        configs: Vec<SerialPortConfig>,
        data_parser: DataParser,
        frame_handler: FrameHandler,
    ) -> Self {
        Self {
            configs,
            data_parser,
            frame_handler,
            heartbeat_command: None,
            heartbeat_interval: Duration::from_secs(30),
        }
    }

    pub fn heartbeat(mut self, command: HeartbeatCommandBuilder, interval: Duration) -> Self {
        self.heartbeat_command = Some(command);
        self.heartbeat_interval = interval;
        self
    }

    pub fn build(self) -> SerialPortInner {
        SerialPortInner::new(
            &self.configs,
            self.data_parser,
            self.frame_handler,
            self.heartbeat_command,
            self.heartbeat_interval,
        )
    }
}

fn _start_holder_event(mut from_port_event_rx: Receiver<FromPortEvent>, inner: &SerialPortInner) {
    let inner = inner.clone();
    tokio::spawn(async move {
        loop {
            select! {
                event_result = from_port_event_rx.recv() => {
                    match event_result {
                        Some(event) => {
                            match event {
                                FromPortEvent::Timeout {} => {
                                    tracing::debug!("timeout");
                                },
                                FromPortEvent::Ack {} => {
                                    tracing::debug!("ack");
                                    if inner._add_and_check_ack_finished().await == true {
                                        tracing::debug!("finished");
                                        inner._ack_finished();
                                    }
                                },
                                FromPortEvent::Close {} => {
                                    tracing::debug!("close");
                                },
                                FromPortEvent::HeartbeatAck {} => {
                                    tracing::debug!("heartbeat ack");
                                },
                                FromPortEvent::HeartbeatTimeout {} => {
                                    tracing::debug!("heartbeat timeout");
                                }
                            }
                        }
                        None => {
                            tracing::debug!("from_port_event_rx closed");
                        }
                    }
                }
            }
        }
    });
}

#[derive(Clone)]
pub struct SerialPortInner {
    port_holders: Arc<RwLock<HashMap<String, SerialPortHolder>>>,
    res_counter: Arc<AtomicI16>,
    from_port_event_tx: Sender<FromPortEvent>,
    data_parser: Arc<DataParser>,
    frame_handler: Arc<FrameHandler>,
    heartbeat_command: Arc<Option<HeartbeatCommandBuilder>>,
    heartbeat_interval: Duration,
}

impl SerialPortInner {
    pub fn new(
        configs: &Vec<SerialPortConfig>,
        data_parser: DataParser,
        frame_handler: FrameHandler,
        heartbeat_command: Option<HeartbeatCommandBuilder>,
        heartbeat_interval: Duration,
    ) -> Self {
        let port_holders = HashMap::new();
        let res_counter = Arc::new(AtomicI16::new(-1));
        let (from_port_event_tx, from_port_event_rx) = mpsc::channel::<FromPortEvent>(32);

        let from_port_event_tx = from_port_event_tx.clone();
        let ret = Self {
            port_holders: Arc::new(RwLock::new(port_holders)),
            res_counter,
            from_port_event_tx: from_port_event_tx.clone(),
            data_parser: Arc::new(data_parser),
            frame_handler: Arc::new(frame_handler),
            heartbeat_command: Arc::new(heartbeat_command),
            heartbeat_interval,
        };
        {
            let configs = configs.clone();
            let ret = ret.clone();
            tokio::spawn(async move {
                for config in configs {
                    ret.add_config(&config).await;
                }
            });
        }

        _start_holder_event(from_port_event_rx, &ret);
        ret
    }

    pub async fn add_config(&self, config: &SerialPortConfig) {
        let mut port_holders = self.port_holders.write().await;
        // 检查是否已存在相同路径的配置
        if port_holders.contains_key(&config.path) {
            port_holders.remove(&config.path);
        }

        let (data_parser_tx, data_parser_rx) = mpsc::channel::<DataParserEvent>(32);
        let (frame_handler_tx, frame_handler_rx) = mpsc::channel::<FrameHandlerEvent>(32);

        let mut builder =
            SerialPortHolderBuilder::new(&config.clone(), &self.from_port_event_tx, data_parser_tx);
        let heartbeat_command = self.heartbeat_command.as_ref();
        if heartbeat_command.is_some() {
            let command = heartbeat_command.as_ref().unwrap();
            builder = builder.heartbeat(command(), self.heartbeat_interval);
        }
        let holder = builder.build();
        let to_port_event_tx = holder.to_port_event_tx().clone();
        port_holders.insert(config.path.clone(), holder);

        let data_parser = self.data_parser.clone();
        let frame_handler = self.frame_handler.clone();

        let self_clone = self.clone();

        data_parser(frame_handler_tx, data_parser_rx);
        frame_handler(
            to_port_event_tx,
            frame_handler_rx,
            Box::pin(move |data, need_ack| {
                let self_clone = self_clone.clone();
                Box::pin(async move {
                    if need_ack {
                        self_clone.write_need_ack(&data).await
                    } else {
                        self_clone.write(&data).await
                    }
                })
            }),
        );
    }

    pub async fn remove_config(&self, config: &SerialPortConfig) {
        let mut port_holders = self.port_holders.write().await;
        if port_holders.contains_key(&config.path) {
            port_holders.remove(&config.path);
        }
    }

    fn _ack_finished(&self) {
        self.res_counter.store(-1, Ordering::Release);
    }

    fn _ack_reset(&self) {
        self.res_counter.store(0, Ordering::Release);
    }

    fn _is_waiting_ack(&self) -> bool {
        self.res_counter.load(Ordering::Acquire) >= 0
    }

    async fn _add_and_check_ack_finished(&self) -> bool {
        self.res_counter.store(
            self.res_counter.load(Ordering::Acquire) + 1,
            Ordering::Release,
        );
        let port_holders = self.port_holders.read().await;
        let ret = self.res_counter.load(Ordering::Acquire) >= port_holders.len() as i16;
        tracing::debug!("_add_and_check_ack_finished: {}", ret);
        ret
    }

    pub async fn write(&self, data: &bytes::Bytes) -> std::io::Result<usize> {
        let result = self._write(data, false).await;
        self._ack_finished();
        result
    }

    pub async fn write_need_ack(&self, data: &bytes::Bytes) -> std::io::Result<usize> {
        self._write(data, true).await
    }

    async fn _write(&self, data: &bytes::Bytes, need_ack: bool) -> std::io::Result<usize> {
        let port_holders = self.port_holders.read().await;
        if port_holders.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "No serial port is available",
            ));
        }

        if self._is_waiting_ack() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ResourceBusy,
                "Waiting for ack",
            ));
        }

        self._ack_reset();

        let futures: Vec<_> = port_holders
            .values()
            .map(|holder| {
                let data_clone = data.clone();
                holder.to_port_event_tx().send(ToPortEvent::Write {
                    data: data_clone,
                    need_ack,
                })
            })
            .collect();

        let results = futures::future::join_all(futures).await;
        for result in results {
            match result {
                Ok(_) => {}
                Err(err) => {
                    tracing::error!("Error writing to serial port: {}", err);
                    return Err(std::io::Error::new(std::io::ErrorKind::Other, err));
                }
            }
        }
        Ok(data.len())
    }
}

#[tokio::test]
async fn test_serial_port_inner() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let configs = vec![SerialPortConfig {
        path: "/dev/tty.usbserial-0001".into(),
        baud_rate: 9600,
        data_bits: tokio_serial::DataBits::Eight,
        stop_bits: tokio_serial::StopBits::One,
        parity: tokio_serial::Parity::None,
        timeout: std::time::Duration::from_secs(5),
        flow_control: tokio_serial::FlowControl::None,
    }];

    let inner = SerialPortInnerBuilder::new(
        configs,
        Box::new(move |frame_handler_tx, mut data_parser_rx| {
            tokio::spawn(async move {
                loop {
                    select! {
                        event = data_parser_rx.recv() => {
                            match event {
                                Some(event) => {
                                    match event {
                                        DataParserEvent::Data { data } => {
                                            {
                                                let data = data.clone();
                                                let _ = frame_handler_tx.send(FrameHandlerEvent::Data { data: (data) }).await;
                                            }
                                            tracing::debug!("DataParserEvent::Data {:?}", data);
                                        },
                                        DataParserEvent::Close {} => {
                                            tracing::debug!("DataParserEvent::Close");
                                            let _ = frame_handler_tx.send(FrameHandlerEvent::Close {}).await;
                                            break;
                                        },
                                    }
                                },
                                None => {},
                            }
                        }
                    }
                }
            });
        }),
        Box::new(move |to_port_sender, mut frame_handler_rx, _| {
            tokio::spawn(async move {
                loop {
                    select! {
                        event = frame_handler_rx.recv() => match event {
                            Some(event) => {
                                match event {
                                    FrameHandlerEvent::Data{ data } => {
                                        tracing::debug!("FrameHandlerEvent::Data {:?}", data);
                                        let _ = to_port_sender.send(ToPortEvent::Ack {  }).await;
                                    },
                                    FrameHandlerEvent::Close {} => {
                                        tracing::debug!("FrameHandlerEvent::Close");
                                        break;
                                    }
                                }
                            },
                            None => {},
                        }
                    }
                }
            });
        }),
    ).heartbeat(Box::new(|| { Box::new(||{bytes::Bytes::from("heartbeat")})}), Duration::from_secs(1)).build();
    let inner1 = inner.clone();
    let task1 = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        let e = inner1
            .write_need_ack(&bytes::Bytes::from("hello world"))
            .await;
        tracing::debug!("write_need_ack: {:?}", e);
        assert!(e.is_ok());
    });

    let task2 = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(20)).await;
    });

    tracing::debug!("starting");
    let result = tokio::join!(task1, task2);
    assert!(result.0.is_ok());
    tracing::debug!("ending");
}
