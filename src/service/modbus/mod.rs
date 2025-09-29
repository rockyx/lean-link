use std::time::Duration;
use tokio::select;
use tokio_modbus::{prelude::*, *};

mod inner;

pub struct ModbusRTUBuilder {
    path: String,
    slave: u8,
    baud_rate: u32,
    data_bits: serialport::DataBits,
    parity: serialport::Parity,
    stop_bits: serialport::StopBits,
    flow_control: serialport::FlowControl,
    timeout: Duration,
}

impl ModbusRTUBuilder {
    pub fn new(path: &str, baud_rate: u32) -> Self {
        ModbusRTUBuilder {
            path: path.to_string(),
            slave: 1,
            baud_rate,
            data_bits: serialport::DataBits::Eight,
            parity: serialport::Parity::None,
            stop_bits: serialport::StopBits::One,
            flow_control: serialport::FlowControl::None,
            timeout: Duration::from_millis(0),
        }
    }

    pub fn with_slave(mut self, slave: u8) -> Self {
        self.slave = slave;
        self
    }

    pub fn with_data_bits(mut self, data_bits: serialport::DataBits) -> Self {
        self.data_bits = data_bits;
        self
    }

    pub fn with_parity(mut self, parity: serialport::Parity) -> Self {
        self.parity = parity;
        self
    }

    pub fn with_stop_bits(mut self, stop_bits: serialport::StopBits) -> Self {
        self.stop_bits = stop_bits;
        self
    }

    pub fn with_flow_control(mut self, flow_control: serialport::FlowControl) -> Self {
        self.flow_control = flow_control;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn build(self) -> ModbusService {
        ModbusService {
            inner: Box::new(inner::ModbusRTUContext {
                path: self.path,
                baud_rate: self.baud_rate,
                data_bits: self.data_bits,
                parity: self.parity,
                stop_bits: self.stop_bits,
                flow_control: self.flow_control,
                timeout: self.timeout,
                slave: self.slave,
                ctx: None,
            }),
        }
    }
}

pub struct ModbusTCPBuilder {
    addr: String,
    port: u16,
    timeout: Duration,
}

impl ModbusTCPBuilder {
    pub fn new(addr: String, port: u16) -> Self {
        Self {
            addr,
            port,
            timeout: Duration::from_millis(0),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn build(self) -> ModbusService {
        ModbusService {
            inner: Box::new(inner::ModbusTCPContext {
                addr: self.addr,
                port: self.port,
                timeout: self.timeout,
                ctx: None,
            }),
        }
    }
}

pub struct ModbusService {
    inner: Box<dyn inner::ModbusContext + Send>,
}

impl ModbusService {
    /// Read multiple coils (0x01)
    pub async fn read_coils(&mut self, addr: u16, cnt: u16) -> Result<Vec<bool>> {
        let _ = self.inner.connect().await?;
        let will_timeout = self.inner.will_timeout();
        let timeout = self.inner.timeout();
        let ctx = self.inner.mut_context();
        if !will_timeout {
            match ctx.read_coils(addr, cnt).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    self.inner.close().await;
                    return Err(e);
                }
            }
        }

        select! {
            result = ctx.read_coils(addr, cnt) => {
                match result {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        self.inner.close().await;
                        Err(e)
                    },
                }
            }
            _ = tokio::time::sleep(timeout) => {
                Err(tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "read_coils timed out",
                )))
            }
        }
    }

    /// Read multiple discrete inputs (0x02)
    pub async fn read_discrete_inputs(&mut self, addr: u16, cnt: u16) -> Result<Vec<bool>> {
        let _ = self.inner.connect().await?;
        let will_timeout = self.inner.will_timeout();
        let timeout = self.inner.timeout();
        let ctx = self.inner.mut_context();
        if !will_timeout {
            match ctx.read_discrete_inputs(addr, cnt).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    self.inner.close().await;
                    return Err(e);
                }
            }
        }

        select! {
            result = ctx.read_discrete_inputs(addr, cnt) => {
                match result {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        self.inner.close().await;
                        Err(e)
                    },
                }
            }
            _ = tokio::time::sleep(timeout) => {
                Err(tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "read_discrete_inputs timed out",
                )))
            }
        }
    }

    /// Read multiple holding registers (0x03)
    pub async fn read_holding_registers(&mut self, addr: u16, cnt: u16) -> Result<Vec<u16>> {
        let _ = self.inner.connect().await?;
        let will_timeout = self.inner.will_timeout();
        let timeout = self.inner.timeout();
        let ctx = self.inner.mut_context();
        if !will_timeout {
            match ctx.read_holding_registers(addr, cnt).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    self.inner.close().await;
                    return Err(e);
                }
            }
        }

        select! {
            result = ctx.read_holding_registers(addr, cnt) => {
                match result {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        self.inner.close().await;
                        Err(e)
                    },
                }
            }
            _ = tokio::time::sleep(timeout) => {
                Err(tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "read_holding_registers timed out",
                )))
            }
        }
    }

    /// Read multiple input registers (0x04)
    pub async fn read_input_registers(&mut self, addr: u16, cnt: u16) -> Result<Vec<u16>> {
        let _ = self.inner.connect().await?;
        let will_timeout = self.inner.will_timeout();
        let timeout = self.inner.timeout();
        let ctx = self.inner.mut_context();

        if !will_timeout {
            match ctx.read_input_registers(addr, cnt).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    self.inner.close().await;
                    return Err(e);
                }
            }
        }

        select! {
            result = ctx.read_input_registers(addr, cnt) => {
                match result {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        self.inner.close().await;
                        Err(e)
                    }
                }
            }
            _ = tokio::time::sleep(timeout) => {
                Err(tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "read_input_registers timed out",
                )))
            }
        }
    }

    /// Read and write multiple holding registers (0x17)
    ///
    /// The write operation is performed before the read unlike
    /// the name of the operation might suggest!
    pub async fn read_write_multiple_registers(
        &mut self,
        read_addr: u16,
        read_count: u16,
        write_addr: u16,
        write_data: &[u16],
    ) -> Result<Vec<u16>> {
        let _ = self.inner.connect().await?;
        let will_timeout = self.inner.will_timeout();
        let timeout = self.inner.timeout();
        let ctx = self.inner.mut_context();

        if !will_timeout {
            match ctx
                .read_write_multiple_registers(read_addr, read_count, write_addr, write_data)
                .await
            {
                Ok(res) => return Ok(res),
                Err(e) => {
                    self.inner.close().await;
                    return Err(e);
                }
            }
        }

        select! {
            result = ctx.read_write_multiple_registers(read_addr, read_count, write_addr, write_data) => {
                match result {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        self.inner.close().await;
                        Err(e)
                    }
                }
            }
            _ = tokio::time::sleep(timeout) => {
                Err(tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "read_write_multiple_registers timed out",
                )))
            }
        }
    }

    /// Write a single coil (0x05)
    pub async fn write_single_coil(&mut self, addr: u16, coil: bool) -> Result<()> {
        let _ = self.inner.connect().await?;
        let will_timeout = self.inner.will_timeout();
        let timeout = self.inner.timeout();
        let ctx = self.inner.mut_context();

        if !will_timeout {
            match ctx.write_single_coil(addr, coil).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    self.inner.close().await;
                    return Err(e);
                }
            }
        }

        select! {
            result = ctx.write_single_coil(addr, coil) => {
                match result {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        self.inner.close().await;
                        Err(e)
                    }
                }
            }
            _ = tokio::time::sleep(timeout) => {
                Err(tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "write_single_coil timed out",
                )))
            }
        }
    }

    /// Write a single holding register (0x06)
    pub async fn write_single_register(
        &mut self,
        addr: u16,
        word: u16,
    ) -> Result<()> {
        let _ = self.inner.connect().await?;
        let will_timeout = self.inner.will_timeout();
        let timeout = self.inner.timeout();
        let ctx = self.inner.mut_context();

        if !will_timeout {
            match ctx.write_single_register(addr, word).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    self.inner.close().await;
                    return Err(e);
                }
            }
        }

        select! {
            result = ctx.write_single_register(addr, word) => {
                match result {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        self.inner.close().await;
                        Err(e)
                    }
                }
            }

            _ = tokio::time::sleep(timeout) => {
                Err(tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "write_single_coil timed out",
                )))
            }
        }
    }

    /// Write multiple coils (0x0F)
    pub async fn write_multiple_coils(
        &mut self,
        addr: u16,
        coils: &[bool],
    ) -> Result<()> {
        let _ = self.inner.connect().await?;
        let will_timeout = self.inner.will_timeout();
        let timeout = self.inner.timeout();
        let ctx = self.inner.mut_context();

        if !will_timeout {
            match ctx.write_multiple_coils(addr, coils).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    self.inner.close().await;
                    return Err(e);
                }
            }
        }

        select! {
            result = ctx.write_multiple_coils(addr, coils) => {
                match result {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        self.inner.close().await;
                        Err(e)
                    }
                }
            }
            _ = tokio::time::sleep(timeout) => {
                Err(tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "write_multiple_coils timed out",
                )))
            }
        }
    }

    /// Write multiple holding registers (0x10)
    pub async fn write_multiple_registers(
        &mut self,
        addr: u16,
        words: &[u16],
    ) -> Result<()> {
        let _ = self.inner.connect().await?;
        let will_timeout = self.inner.will_timeout();
        let timeout = self.inner.timeout();
        let ctx = self.inner.mut_context();

        if !will_timeout {
            match ctx.write_multiple_registers(addr, words).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    self.inner.close().await;
                    return Err(e);
                }
            }
        }

        select! {
            result = ctx.write_multiple_registers(addr, words) => {
                match result {
                    Ok(res) => Ok(res),
                    Err(e) => {
                        self.inner.close().await;
                        Err(e)
                    }
                }
            }
            _ = tokio::time::sleep(timeout) => {
                Err(tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "write_multiple_registers timed out",
                )))
            }
        }
    }

    /// Set or clear individual bits of a holding register (0x16)
    pub async fn masked_write_register(
        &mut self,
        addr: u16,
        and_mask: u16,
        or_mask: u16,
    ) -> Result<()> {
        let _ = self.inner.connect().await?;
        let will_timeout = self.inner.will_timeout();
        let timeout = self.inner.timeout();
        let ctx = self.inner.mut_context();

        if !will_timeout {
            match ctx.masked_write_register(addr, and_mask, or_mask).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    self.inner.close().await;
                    return Err(e);
                }
            }
        }

        select! {
            result = ctx.masked_write_register(addr, and_mask, or_mask) => {
                match result {
                    Ok(res) => return Ok(res),
                    Err(e) => {
                        self.inner.close().await;
                        return Err(e);
                    }
                }
            }
            _ = tokio::time::sleep(timeout) => {
                Err(tokio_modbus::Error::Transport(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "masked_write_register timed out",
                )))
            }
        }
    }
}

/// 将两个 u16 寄存器转换为 f32 浮点数
/// reg1: 第一个寄存器值
/// reg2: 第二个寄存器值
/// register_order: 寄存器顺序
///   - 'high_first': reg1 为高16位，reg2 为低16位
///   - 'low_first': reg1 为低16位，reg2 为高16位
/// byte_order: 字节顺序
///   - 'big_endian': 大端序
///   - 'little_endian': 小端序
pub fn registers_to_f32(
    reg1: u16,
    reg2: u16,
    register_order: &str,
    byte_order: &str,
) -> std::result::Result<f32, String> {
    let mut bytes: [u8; 4] = [0; 4];

    // 1. 先以大端存
    match register_order {
        "high_first" => {
            bytes[0] = (reg1 >> 8) as u8;
            bytes[1] = reg1 as u8;
            bytes[2] = (reg2 >> 8) as u8;
            bytes[3] = reg2 as u8;
        }
        "low_first" => {
            bytes[0] = (reg2 >> 8) as u8;
            bytes[1] = reg2 as u8;
            bytes[2] = (reg1 >> 8) as u8;
            bytes[3] = reg1 as u8;
        }
        _ => return Err("Invalid register order".into()),
    };

    // 2. 根据所需的字节序返回
    match byte_order {
        "big_endian" => Ok(f32::from_be_bytes(bytes)), // 已经是目标的大端序
        "little_endian" => Ok(f32::from_le_bytes(bytes)),
        _ => Err("Invalid byte order. Use 'big_endian' or 'little_endian'.".into()),
    }
}

/// 将两个 u16 寄存器转换为 u32 整形
/// reg1: 第一个寄存器值
/// reg2: 第二个寄存器值
/// register_order: 寄存器顺序
///   - 'high_first': reg1 为高16位，reg2 为低16位
///   - 'low_first': reg1 为低16位，reg2 为高16位
/// byte_order: 字节顺序
///   - 'big_endian': 大端序
///   - 'little_endian': 小端序
pub fn registers_to_u32(
    reg1: u16,
    reg2: u16,
    register_order: &str,
    byte_order: &str,
) -> std::result::Result<u32, String> {
    let mut bytes: [u8; 4] = [0; 4];

    // 1. 先以大端存
    match register_order {
        "high_first" => {
            bytes[0] = (reg1 >> 8) as u8;
            bytes[1] = reg1 as u8;
            bytes[2] = (reg2 >> 8) as u8;
            bytes[3] = reg2 as u8;
        }
        "low_first" => {
            bytes[0] = (reg2 >> 8) as u8;
            bytes[1] = reg2 as u8;
            bytes[2] = (reg1 >> 8) as u8;
            bytes[3] = reg1 as u8;
        }
        _ => return Err("Invalid register order".into()),
    };

    // 2. 根据所需的字节序返回
    match byte_order {
        "big_endian" => Ok(u32::from_be_bytes(bytes)), // 已经是目标的大端序
        "little_endian" => Ok(u32::from_le_bytes(bytes)),
        _ => Err("Invalid byte order. Use 'big_endian' or 'little_endian'.".into()),
    }
}

#[tokio::test]
async fn test_modbus() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let builder = ModbusRTUBuilder::new("/dev/tty.usbserial-0001", 9600)
        .with_slave(1)
        .with_timeout(std::time::Duration::from_secs(1));
    let mut service = builder.build();

    loop {
        {
            let result = service.read_coils(0x0001, 1).await;
            tracing::debug!("{:?}", result);
        }

        {
            let result = service.read_discrete_inputs(0x0001, 1).await;
            tracing::debug!("{:?}", result);
        }

        {
            let result = service.read_holding_registers(0x0001, 1).await;
            tracing::debug!("{:?}", result);
        }

        {
            let result = service.read_input_registers(0x0001, 1).await;
            tracing::debug!("{:?}", result);
        }

        {
            let result = service
                .read_write_multiple_registers(0x0001, 1, 0x0002, &[0x0001, 0x0002])
                .await;
            tracing::debug!("{:?}", result);
        }

        {
            let result = service.write_single_coil(0x0001, true).await;
            tracing::debug!("{:?}", result);
        }

        {
            let result = service.write_single_register(0x0001, 0x0001).await;
            tracing::debug!("{:?}", result);
        }

        {
            let result = service
                .write_multiple_coils(0x0001, &[true, false, true])
                .await;
            tracing::debug!("{:?}", result);
        }

        {
            let result = service
                .write_multiple_registers(0x0001, &[0x0001, 0x0002])
                .await;
            tracing::debug!("{:?}", result);
        }

        {
            let result = service.masked_write_register(0x0001, 0x0002, 0x0001).await;
            tracing::debug!("{:?}", result);
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

#[test]
fn test_reigsters_to_f32() {
    // 示例：假设从 Modbus 读取的两个寄存器值
    let mut reg_high = 0x42F1; // 高16位寄存器
    let mut reg_low = 0x0000; // 低16位寄存器

    match registers_to_f32(reg_high, reg_low, "high_first", "big_endian") {
        Ok(value) => {
            println!("转换后的浮点数为: {:.2}", value); // 期望输出 ~120.5
            assert_eq!(value, 120.5);
        }
        Err(e) => eprintln!("转换错误: {}", e),
    }

    reg_high = 0x0000;
    reg_low = 0x42F1;

    match registers_to_f32(reg_high, reg_low, "low_first", "big_endian") {
        Ok(value) => {
            println!("转换后的浮点数为: {:.2}", value); // 期望输出 ~120.5
            assert_eq!(value, 120.5);
        }
        Err(e) => eprintln!("转换错误: {}", e),
    }

    reg_high = 0x0000;
    reg_low = 0xF142;

    match registers_to_f32(reg_high, reg_low, "high_first", "little_endian") {
        Ok(value) => {
            println!("转换后的浮点数为: {:.2}", value); // 期望输出 ~120.5
            assert_eq!(value, 120.5);
        }
        Err(e) => eprintln!("转换错误: {}", e),
    }

    reg_high = 0xF142;
    reg_low = 0x0000;

    match registers_to_f32(reg_high, reg_low, "low_first", "little_endian") {
        Ok(value) => {
            println!("转换后的浮点数为: {:.2}", value); // 期望输出 ~120.5
            assert_eq!(value, 120.5);
        }
        Err(e) => eprintln!("转换错误: {}", e),
    }
}

#[test]
fn test_reigsters_to_u32() {
    // 示例：假设从 Modbus 读取的两个寄存器值
    let mut reg_high = 0x0000; // 高16位寄存器
    let mut reg_low = 0x2710; // 低16位寄存器

    match registers_to_u32(reg_high, reg_low, "high_first", "big_endian") {
        Ok(value) => {
            println!("转换后的浮点数为: {:.2}", value); // 期望输出 ~120.5
            assert_eq!(value, 10000);
        }
        Err(e) => eprintln!("转换错误: {}", e),
    }

    reg_high = 0x2710;
    reg_low = 0x0000;

    match registers_to_u32(reg_high, reg_low, "low_first", "big_endian") {
        Ok(value) => {
            println!("转换后的浮点数为: {:.2}", value); // 期望输出 ~10000
            assert_eq!(value, 10000);
        }
        Err(e) => eprintln!("转换错误: {}", e),
    }

    reg_high = 0x1027;
    reg_low = 0x0000;

    match registers_to_u32(reg_high, reg_low, "high_first", "little_endian") {
        Ok(value) => {
            println!("转换后的浮点数为: {:.2}", value); // 期望输出 ~10000
            assert_eq!(value, 10000);
        }
        Err(e) => eprintln!("转换错误: {}", e),
    }

    reg_high = 0x0000;
    reg_low = 0x1027;

    match registers_to_u32(reg_high, reg_low, "low_first", "little_endian") {
        Ok(value) => {
            println!("转换后的浮点数为: {:.2}", value); // 期望输出 ~10000
            assert_eq!(value, 10000);
        }
        Err(e) => eprintln!("转换错误: {}", e),
    }
}
