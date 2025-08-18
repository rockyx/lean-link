use super::{DataParserEvent, FromPortEvent, SerialPortConfig, ToPortEvent};
use serialport::SerialPort;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::select;
use tokio::sync::mpsc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::{Mutex, Notify};
use tokio::time::timeout;
use tokio_serial::SerialPortBuilderExt;

async fn _start_open(
    is_stop: Arc<AtomicBool>,
    port: Arc<Mutex<Option<tokio_serial::SerialStream>>>,
    config: SerialPortConfig,
) {
    loop {
        if is_stop.load(Ordering::Acquire) {
            break;
        }
        let mut port_guard = port.lock().await;
        if port_guard.is_none() {
            match tokio_serial::new(config.path.clone(), config.baud_rate)
                .data_bits(config.data_bits)
                .parity(config.parity)
                .stop_bits(config.stop_bits)
                .flow_control(config.flow_control)
                .open_native_async()
            {
                Ok(mut port) => {
                    let _ = port.set_timeout(config.timeout);
                    *port_guard = Some(port);
                    tracing::info!("SerialPort opened: {}", config.path);
                }
                Err(e) => {
                    tracing::error!("SerialPort error: {}", e);
                }
            }
        }
        drop(port_guard);
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }
}

fn _start_read_timeout(
    config: SerialPortConfig,
    notify: Arc<Notify>,
    from_port_event_tx: Sender<FromPortEvent>,
) {
    notify.notify_waiters();
    tokio::spawn(async move {
        match timeout(config.timeout, async {
            let _ = notify.notified_owned().await;
        })
        .await
        {
            Ok(_) => {
                let _ = from_port_event_tx.send(FromPortEvent::Ack {}).await;
            }
            Err(_) => {
                let _ = from_port_event_tx.send(FromPortEvent::Timeout {}).await;
            }
        }
    });
}

async fn _start_read_write(
    is_stop: Arc<AtomicBool>,
    port: Arc<Mutex<Option<tokio_serial::SerialStream>>>,
    config: SerialPortConfig,
    data_parser_sender: Sender<DataParserEvent>,
    mut to_port_event_rx: Receiver<ToPortEvent>,
    notify: Arc<Notify>,
    from_port_event_tx: Sender<FromPortEvent>,
) {
    let mut buf = [0u8; 1024]; // 预分配缓冲区
    let mut is_need_ack = false;
    loop {
        let notify = notify.clone();

        if is_stop.load(Ordering::Acquire) {
            break;
        }
        let mut port_guard = port.lock().await;
        if port_guard.is_none() {
            drop(port_guard);
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            continue;
        }
        let port = port_guard.as_mut().unwrap();
        select! {
            read_data = port.read(&mut buf) => {
                match read_data {
                    Ok(0) => {
                        tracing::debug!("serialport read 0 bytes");
                    }
                    Ok(n) => {
                        let data = &buf[..n];
                        let hex_data: Vec<String> = data.iter().map(|b| format!("{:02X}", b)).collect();
                        tracing::info!("串口读取数据({}): [ {} ]", config.path, hex_data.join(" "));
                        let result = data_parser_sender.send(DataParserEvent::Data { data: (bytes::Bytes::copy_from_slice(data)) }).await;
                        tracing::info!("串口读取数据解析({}): {:?}", config.path, result);
                    },
                    Err(e) => {
                        port_guard.take();
                        tracing::error!("Error reading from serial port: {}", e);
                        continue;
                    },
                }
            }

            to_port_event = to_port_event_rx.recv() => {
                match to_port_event {
                    Some(event) => {
                        match event {
                            ToPortEvent::Write { data, need_ack } => {
                                let hex_data: Vec<String> = data.iter().map(|b| format!("{:02X}", b)).collect();
                                tracing::info!("串口写入数据({}): [ {} ]", config.path, hex_data.join(" "));
                                match port.write(&data).await {
                                    Ok(_) => {
                                        tracing::info!("串口写入成功({})", config.path);
                                        is_need_ack = need_ack;
                                        if need_ack {
                                            tracing::info!("等待串口应答...");
                                        }
                                    },
                                    Err(e) => {
                                        tracing::error!("Write error: {}", e);
                                        port_guard.take();
                                        tracing::error!("Error reading from serial port: {}", e);
                                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                                        continue;
                                    },
                                }
                            },
                            ToPortEvent::Ack {} => {
                                tracing::debug!("ToPortEvent::Ack");
                                notify.notify_one();
                            }
                            ToPortEvent::Stop {} => {
                                tracing::debug!("ToPortEvent::Stop");
                                break;
                            },
                        }
                    }
                    None => {
                        tracing::error!("Serial port event channel closed");
                        break;
                    }
                }
            }

            timeout_result = timeout(config.timeout, notify.clone().notified_owned()), if is_need_ack => {
                match timeout_result {
                    Ok(_) => {
                        let _ = from_port_event_tx.send(FromPortEvent::Ack {}).await;
                        is_need_ack = false;
                    },
                    Err(_) => {
                        let _ = from_port_event_tx.send(FromPortEvent::Timeout {}).await;
                        is_need_ack = false;
                    },
                }
            }
        };
    }
    let _ = from_port_event_tx.send(FromPortEvent::Close {}).await;
    let _ = data_parser_sender.send(DataParserEvent::Close {}).await;
    tracing::info!("Serial port closed");
}

pub struct SerialPortHolder {
    is_stop: Arc<AtomicBool>,
    to_port_event_tx: Sender<ToPortEvent>,
}

impl SerialPortHolder {
    pub fn new(
        config: &SerialPortConfig,
        from_port_event_tx: &Sender<FromPortEvent>,
        data_parser_sender: Sender<DataParserEvent>,
    ) -> Self {
        let port = Arc::new(Mutex::new(None));
        let is_stop = Arc::new(AtomicBool::new(false));
        let (to_port_event_tx, to_port_event_rx) = mpsc::channel::<ToPortEvent>(32);
        let notify = Arc::new(Notify::new());

        {
            let is_stop = is_stop.clone();
            let port = port.clone();
            let config = config.clone();
            tokio::spawn(async move {
                _start_open(is_stop, port, config).await;
            });
        }

        {
            let is_stop = is_stop.clone();
            let port = port.clone();
            let config = config.clone();
            let data_parser_sender = data_parser_sender.clone();
            let notify = notify.clone();
            let from_port_event_tx = from_port_event_tx.clone();
            tokio::spawn(async move {
                _start_read_write(
                    is_stop,
                    port,
                    config,
                    data_parser_sender,
                    to_port_event_rx,
                    notify,
                    from_port_event_tx,
                )
                .await;
            });
        }
        Self {
            is_stop,
            to_port_event_tx,
        }
    }

    pub fn stop(&mut self) {
        self.is_stop.store(true, Ordering::Release);
        let to_port_event_tx = self.to_port_event_tx.clone();
        tokio::spawn(async move {
            let _ = to_port_event_tx.send(ToPortEvent::Stop {}).await;
        });
    }

    pub fn to_port_event_tx(&self) -> &mpsc::Sender<ToPortEvent> {
        &self.to_port_event_tx
    }
}

impl Drop for SerialPortHolder {
    fn drop(&mut self) {
        tracing::debug!("Drop for SerialPortHolder");
        self.stop();
    }
}

#[tokio::test]
async fn test_serial_port_holder() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();
    let config = SerialPortConfig {
        path: "/dev/tty.usbserial-0001".into(),
        baud_rate: 9600,
        data_bits: tokio_serial::DataBits::Eight,
        stop_bits: tokio_serial::StopBits::One,
        parity: tokio_serial::Parity::None,
        timeout: std::time::Duration::from_secs(3),
        flow_control: tokio_serial::FlowControl::None,
    };

    let (from_port_event_tx, mut from_port_event_rx) = mpsc::channel::<FromPortEvent>(10);
    let (data_parser_sender, mut data_parser_rx) = mpsc::channel::<DataParserEvent>(10);
    let holder = SerialPortHolder::new(&config, &from_port_event_tx, data_parser_sender.clone());
    let to_port_event_tx = holder.to_port_event_tx();

    let task1_tx = to_port_event_tx.clone();
    let task1 = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(29)).await;
        let _ = task1_tx.send(ToPortEvent::Stop {}).await;
    });

    let data_parser_sender = data_parser_sender.clone();
    let task2 = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(30)).await;
        let _ = data_parser_sender.send(DataParserEvent::Close {}).await;
    });

    let task3_tx = to_port_event_tx.clone();
    let task3 = tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let tx_result = task3_tx
            .send(ToPortEvent::Write {
                data: bytes::Bytes::from("test"),
                need_ack: true,
            })
            .await;
        {
            let tx_result = tx_result.clone();
            if tx_result.is_err() {
                tracing::debug!("tx_result error {}", tx_result.err().unwrap());
            }
        }
        let tx_result = tx_result.clone();
        assert!(tx_result.is_ok());
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let tx_result = task3_tx
            .send(ToPortEvent::Write {
                data: bytes::Bytes::from("test"),
                need_ack: true,
            })
            .await;
        {
            let tx_result = tx_result.clone();
            if tx_result.is_err() {
                tracing::debug!("tx_result error {}", tx_result.err().unwrap());
            }
        }
        let tx_result = tx_result.clone();
        assert!(tx_result.is_ok());
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
        let tx_result = task3_tx
            .send(ToPortEvent::Write {
                data: bytes::Bytes::from("test"),
                need_ack: true,
            })
            .await;
        {
            let tx_result = tx_result.clone();
            if tx_result.is_err() {
                tracing::debug!("tx_result error {}", tx_result.err().unwrap());
            }
        }
        let tx_result = tx_result.clone();
        assert!(tx_result.is_ok());
    });

    let task4 = tokio::spawn(async move {
        loop {
            select! {
                result = from_port_event_rx.recv() => {
                    match result {
                        Some(event) => {
                            tracing::debug!("from_port_event_rx: {:?}", event);
                            match event {
                                FromPortEvent::Close {} => {
                                    break;
                                }
                                FromPortEvent::Timeout {} => {
                                }
                                _ => {},
                            }
                        }
                        None => {
                            tracing::debug!("from_port_event_rx: None");
                        }
                    }
                }
            }
        }
    });

    let task5_tx = to_port_event_tx.clone();
    let task5 = tokio::spawn(async move {
        loop {
            select! {
                result = data_parser_rx.recv() => {
                    match result {
                        Some(event) => {
                            match event {
                                DataParserEvent::Data {data}  => {
                                    tracing::debug!("data_parser_rx: {:?}", data);
                                    if data.is_empty() {
                                        break;
                                    }
                                    let _ = task5_tx.send(ToPortEvent::Ack {  }).await;
                                }
                                DataParserEvent::Close {} => {
                                    break;
                                }
                            }
                        }
                        None => {
                            tracing::debug!("data_parser_rx: None");
                        }
                    }
                }
            }
        }
    });
    let result = tokio::join!(task1, task2, task3, task4, task5);
    assert!(result.0.is_ok());
}
