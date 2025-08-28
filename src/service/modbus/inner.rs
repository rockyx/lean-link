use std::{fmt::Debug, time::Duration};
use tokio_modbus::prelude::*;

#[async_trait::async_trait]
pub trait ModbusContext: Debug {
    async fn connect(&mut self) -> tokio_modbus::Result<()>;
    // fn context(&self) -> &tokio_modbus::client::Context;
    fn mut_context(&mut self) -> &mut tokio_modbus::client::Context;
    fn will_timeout(&self) -> bool;
    fn timeout(&self) -> Duration;
}

#[derive(Debug)]
pub struct ModbusRTUContext {
    pub path: String,
    pub baud_rate: u32,
    pub data_bits: serialport::DataBits,
    pub parity: serialport::Parity,
    pub stop_bits: serialport::StopBits,
    pub flow_control: serialport::FlowControl,
    pub timeout: Duration,
    pub slave: u8,
    pub ctx: Option<tokio_modbus::client::Context>,
}

#[async_trait::async_trait]
impl ModbusContext for ModbusRTUContext {
    async fn connect(&mut self) -> tokio_modbus::Result<()> {
        use tokio_serial::SerialStream;

        if self.ctx.is_some() {
            return Ok(Ok(()));
        }

        let builder = tokio_serial::new(&self.path, self.baud_rate)
            .data_bits(self.data_bits)
            .parity(self.parity)
            .stop_bits(self.stop_bits)
            .flow_control(self.flow_control)
            .timeout(self.timeout);

        let port = SerialStream::open(&builder);
        if port.is_err() {
            return Err(tokio_modbus::Error::Transport(port.err().unwrap().into()));
        }
        let port = port.unwrap();

        let ctx = rtu::attach_slave(port, Slave(self.slave));
        self.ctx = Some(ctx);

        Ok(Ok(()))
    }

    // fn context(&self) -> &tokio_modbus::client::Context {
    //     self.ctx.as_ref().unwrap()
    // }

    fn mut_context(&mut self) -> &mut tokio_modbus::client::Context {
        self.ctx.as_mut().unwrap()
    }

    fn will_timeout(&self) -> bool {
        self.timeout.as_millis() > 0
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[derive(Debug)]
pub struct ModbusTCPContext {
    pub addr: String,
    pub port: u16,
    pub timeout: Duration,
    pub ctx: Option<tokio_modbus::client::Context>,
}

#[async_trait::async_trait]
impl ModbusContext for ModbusTCPContext {
    async fn connect(&mut self) -> tokio_modbus::Result<()> {
        if self.ctx.is_some() {
            return Ok(Ok(()));
        }

        let socket_addr: std::net::SocketAddr = format!("{}:{}", self.addr, self.port)
            .parse()
            .map_err(|_| {
                tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("Invalid socket address: {}:{}", self.addr, self.port),
                ))
            })?;

        let ctx = tcp::connect(socket_addr).await?;
        self.ctx = Some(ctx);
        Ok(Ok(()))
    }

    // fn context(&self) -> &tokio_modbus::client::Context {
    //     self.ctx.as_ref().unwrap()
    // }

    fn mut_context(&mut self) -> &mut tokio_modbus::client::Context {
        self.ctx.as_mut().unwrap()
    }

    fn will_timeout(&self) -> bool {
        self.timeout.as_millis() > 0
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }
}

#[tokio::test]
async fn test_modbus_rtu() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let mut ctx = ModbusRTUContext {
        path: "/dev/tty.usbserial-0001".into(),
        baud_rate: 9600,
        data_bits: serialport::DataBits::Eight,
        parity: serialport::Parity::None,
        stop_bits: serialport::StopBits::One,
        flow_control: serialport::FlowControl::None,
        timeout: Duration::from_millis(100),
        slave: 1,
        ctx: None,
    };

    let result = ctx.connect().await;
    tracing::debug!("Connect result: {:?}", result);

    let res = ctx
        .ctx
        .as_mut()
        .unwrap()
        .read_holding_registers(0x0001, 1)
        .await;
    tracing::debug!("Read holding registers result: {:?}", res);
}
