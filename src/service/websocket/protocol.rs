//! Unified WebSocket binary protocol for efficient transmission
//!
//! This module provides a unified binary format for WebSocket messages
//! containing binary data (e.g., camera frames, inspection results).
//!
//! # Binary Format
//!
//! ```text
//! +----------------+----------------+------------------+------------------+
//! | 4 bytes Magic  | 4 bytes Header | N bytes JSON     | M bytes Binary   |
//! | "LLWS"         | Length (BE u32)| Header           | Data             |
//! +----------------+----------------+------------------+------------------+
//! |  offset 0      |  offset 4      |  offset 8        |  offset 8+N     |
//! +----------------+----------------+------------------+------------------+
//! ```
//!
//! # Header Fields
//!
//! | Field | Type | Description |
//! |-------|------|-------------|
//! | version | u8 | Protocol version |
//! | type | string | Message type ("camera.frame" or "inspection.result") |
//! | topic | string | Routing topic |
//! | encoding | string | Image encoding ("jpeg", "png", "raw") |
//! | timestamp | u64 | Timestamp in microseconds |
//! | width | u32 | Image width |
//! | height | u32 | Image height |
//! | dataLength | usize | Binary data length |
//! | sourceId | string | Source ID (camera/station UUID) |
//! | sourceName | string | Source name |
//! | metadata | object | Type-specific metadata |

use serde::{Deserialize, Serialize};

/// Magic bytes for WebSocket binary protocol identification
pub const WS_BINARY_MAGIC: &[u8] = b"LLWS";

/// Current protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Message types for WebSocket binary payloads
pub const MSG_TYPE_CAMERA_FRAME: &str = "camera.frame";
pub const MSG_TYPE_INSPECTION_RESULT: &str = "inspection.result";

/// Unified header for WebSocket binary messages
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsBinaryHeader {
    /// Protocol version
    pub version: u8,

    /// Message type (e.g., "camera.frame", "inspection.result")
    #[serde(rename = "type")]
    pub msg_type: String,

    /// Routing topic
    pub topic: String,

    /// Image encoding format
    pub encoding: String,

    /// Timestamp in microseconds since epoch
    pub timestamp: u64,

    /// Image width in pixels
    pub width: u32,

    /// Image height in pixels
    pub height: u32,

    /// Length of the binary data following the header
    pub data_length: usize,

    /// Source identifier (camera/station UUID)
    pub source_id: String,

    /// Source display name
    pub source_name: String,

    /// Type-specific metadata
    #[serde(default)]
    pub metadata: serde_json::Value,
}

impl WsBinaryHeader {
    /// Create a new header for camera frame
    pub fn new_camera_frame(
        topic: impl Into<String>,
        encoding: impl Into<String>,
        timestamp: u64,
        width: u32,
        height: u32,
        source_id: impl Into<String>,
        source_name: impl Into<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            msg_type: MSG_TYPE_CAMERA_FRAME.to_string(),
            topic: topic.into(),
            encoding: encoding.into(),
            timestamp,
            width,
            height,
            data_length: 0, // Set by build_binary_payload
            source_id: source_id.into(),
            source_name: source_name.into(),
            metadata,
        }
    }

    /// Create a new header for inspection result
    pub fn new_inspection_result(
        topic: impl Into<String>,
        encoding: impl Into<String>,
        timestamp: u64,
        width: u32,
        height: u32,
        source_id: impl Into<String>,
        source_name: impl Into<String>,
        metadata: serde_json::Value,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            msg_type: MSG_TYPE_INSPECTION_RESULT.to_string(),
            topic: topic.into(),
            encoding: encoding.into(),
            timestamp,
            width,
            height,
            data_length: 0, // Set by build_binary_payload
            source_id: source_id.into(),
            source_name: source_name.into(),
            metadata,
        }
    }
}

/// Build a binary payload with the unified format
///
/// # Arguments
///
/// * `header` - The header to serialize
/// * `data` - The binary data to append
///
/// # Returns
///
/// A Vec<u8> containing the complete binary message
pub fn build_binary_payload(mut header: WsBinaryHeader, data: &[u8]) -> Vec<u8> {
    // Update data_length in header
    header.data_length = data.len();

    let json_bytes = match serde_json::to_vec(&header) {
        Ok(bytes) => bytes,
        Err(e) => {
            tracing::error!("Failed to serialize binary header: {}", e);
            return Vec::new();
        }
    };

    // Total size: 4 (magic) + 4 (header length) + header + data
    let total_size = WS_BINARY_MAGIC.len() + 4 + json_bytes.len() + data.len();
    let mut buffer = Vec::with_capacity(total_size);

    // Magic bytes for identification
    buffer.extend_from_slice(WS_BINARY_MAGIC);

    // Header length (big-endian u32)
    buffer.extend_from_slice(&(json_bytes.len() as u32).to_be_bytes());

    // JSON header
    buffer.extend_from_slice(&json_bytes);

    // Binary data
    buffer.extend_from_slice(data);

    buffer
}

/// Parse result from binary WebSocket message
#[derive(Debug)]
pub struct ParsedBinaryMessage {
    pub header: WsBinaryHeader,
    pub data: Vec<u8>,
}

/// Parse a binary WebSocket message
///
/// # Arguments
///
/// * `data` - The raw binary data from WebSocket
///
/// # Returns
///
/// `Some(ParsedBinaryMessage)` if parsing succeeds, `None` otherwise
pub fn parse_binary_message(data: &[u8]) -> Option<ParsedBinaryMessage> {
    // Minimum size: magic (4) + length (4) = 8
    if data.len() < 8 {
        tracing::warn!("Binary message too short: {} bytes", data.len());
        return None;
    }

    // Check magic bytes
    if &data[0..4] != WS_BINARY_MAGIC {
        tracing::warn!("Invalid magic bytes in binary message");
        return None;
    }

    // Read header length
    let header_length = u32::from_be_bytes([data[4], data[5], data[6], data[7]]) as usize;

    // Check if we have enough data
    if data.len() < 8 + header_length {
        tracing::warn!(
            "Binary message truncated: expected {} bytes header, got {}",
            header_length,
            data.len() - 8
        );
        return None;
    }

    // Parse JSON header
    let header_bytes = &data[8..8 + header_length];
    let header: WsBinaryHeader = match serde_json::from_slice(header_bytes) {
        Ok(h) => h,
        Err(e) => {
            tracing::error!("Failed to parse binary header JSON: {}", e);
            return None;
        }
    };

    // Extract binary data
    let data_start = 8 + header_length;
    let binary_data = data[data_start..].to_vec();

    // Validate data length
    if binary_data.len() != header.data_length {
        tracing::warn!(
            "Data length mismatch: header says {}, actual {}",
            header.data_length,
            binary_data.len()
        );
    }

    Some(ParsedBinaryMessage {
        header,
        data: binary_data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_and_parse() {
        let metadata = serde_json::json!({
            "sequence": 123,
            "pixelFormat": "RGB8",
        });

        let header = WsBinaryHeader::new_camera_frame(
            "camera/stream",
            "jpeg",
            1234567890000,
            1920,
            1080,
            "test-uuid",
            "Test Camera",
            metadata,
        );

        let test_data = vec![0xFF, 0xD8, 0xFF, 0xE0]; // JPEG header bytes
        let payload = build_binary_payload(header, &test_data);

        // Verify magic
        assert_eq!(&payload[0..4], WS_BINARY_MAGIC);

        // Parse and verify
        let parsed = parse_binary_message(&payload).expect("Should parse");
        assert_eq!(parsed.header.msg_type, MSG_TYPE_CAMERA_FRAME);
        assert_eq!(parsed.header.width, 1920);
        assert_eq!(parsed.header.height, 1080);
        assert_eq!(parsed.data, test_data);
    }

    #[test]
    fn test_parse_invalid_magic() {
        let data = b"INVALID_HEADER";
        assert!(parse_binary_message(data).is_none());
    }

    #[test]
    fn test_parse_too_short() {
        let data = b"LLWS";
        assert!(parse_binary_message(data).is_none());
    }
}
