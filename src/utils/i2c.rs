use std::process::Command;

pub fn path_to_i2c_bus(path: &str) -> Result<u16, String> {
    path.rsplit_once("i2c-")
        .and_then(|(_, tail)| tail.parse::<u16>().ok())
        .ok_or_else(|| format!("invalid i2c device path: {}", path))
}

// Helper to read one register via i2cget
pub fn i2c_read_reg(bus: u16, addr: u16, reg: u8) -> Result<u8, String> {
    let output = Command::new("i2cget")
        .arg("-y")
        .arg(bus.to_string())
        .arg(format!("0x{:02x}", addr))
        .arg(format!("0x{:02x}", reg))
        .output()
        .map_err(|e| format!("failed to spawn i2cget: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "i2cget failed for reg 0x{:02x}: {}",
            reg,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    // i2cget prints like "0x12"
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let val = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(&s);
    u8::from_str_radix(val, 16)
        .map_err(|_| format!("invalid i2cget output for reg 0x{:02x}: {}", reg, s))
}
