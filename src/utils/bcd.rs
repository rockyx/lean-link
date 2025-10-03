// Binary Coded Decimal (BCD) utilities.
//// Provides conversions between decimal and single-byte BCD.
//// Convention: single-byte BCD represents 0..=99; high/low nibbles are each within 0..=9.

/// Convert a decimal value (0..=99) to a single-byte BCD.
/// Example: 12 -> 0x12, 45 -> 0x45
/// Returns an error when n is out of range.
pub fn dec_to_bcd(n: u8) -> Result<u8, &'static str> {
    if n > 99 {
        return Err("decimal out of range (expected 0..=99)");
    }
    let hi = n / 10;
    let lo = n % 10;
    Ok((hi << 4) | lo)
}

/// Convert a single-byte BCD to a decimal value (0..=99).
/// Example: 0x12 -> 12, 0x45 -> 45
/// Returns an error if any nibble is not within 0..=9.
pub fn bcd_to_dec(b: u8) -> Result<u8, &'static str> {
    let hi = (b >> 4) & 0x0F;
    let lo = b & 0x0F;
    if hi > 9 || lo > 9 {
        return Err("invalid BCD digit (nibble must be 0..=9)");
    }
    Ok(hi * 10 + lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dec_to_bcd_ok() {
        assert_eq!(dec_to_bcd(0).unwrap(), 0x00);
        assert_eq!(dec_to_bcd(9).unwrap(), 0x09);
        assert_eq!(dec_to_bcd(10).unwrap(), 0x10);
        assert_eq!(dec_to_bcd(12).unwrap(), 0x12);
        assert_eq!(dec_to_bcd(45).unwrap(), 0x45);
        assert_eq!(dec_to_bcd(99).unwrap(), 0x99);
    }

    #[test]
    fn test_dec_to_bcd_err() {
        assert!(dec_to_bcd(100).is_err());
        assert!(dec_to_bcd(u8::MAX).is_err());
    }

    #[test]
    fn test_bcd_to_dec_ok() {
        assert_eq!(bcd_to_dec(0x00).unwrap(), 0);
        assert_eq!(bcd_to_dec(0x09).unwrap(), 9);
        assert_eq!(bcd_to_dec(0x10).unwrap(), 10);
        assert_eq!(bcd_to_dec(0x12).unwrap(), 12);
        assert_eq!(bcd_to_dec(0x45).unwrap(), 45);
        assert_eq!(bcd_to_dec(0x99).unwrap(), 99);
    }

    #[test]
    fn test_bcd_to_dec_err() {
        // Invalid nibble: A(10) is not in 0..=9
        assert!(bcd_to_dec(0x1A).is_err());
        assert!(bcd_to_dec(0xA0).is_err());
        assert!(bcd_to_dec(0xFF).is_err());
    }
}