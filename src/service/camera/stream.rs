//! Camera stream service for real-time frame distribution via WebSocket
//!
//! This module provides functionality to stream camera frames to WebSocket clients.
//! It supports:
//! - Multiple camera streams
//! - Frame rate control
//! - JPEG encoding for efficient transmission
//! - Compatibility with detection pipeline

use std::{
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use std::io::Cursor;

use base64::{Engine, engine::general_purpose::STANDARD};
use image::{ImageBuffer, Luma, Rgb, codecs::jpeg::JpegEncoder};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};

use crate::service::camera::{CameraConfig, CameraFrame, FrameEncoding, PixelFormat};

/// Camera stream configuration
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraStreamConfig {
    /// Target frame rate (frames per second)
    #[serde(default = "default_fps")]
    pub fps: u32,

    /// Frame encoding format
    #[serde(default)]
    pub encoding: FrameEncoding,

    /// JPEG quality (1-100, only for JPEG encoding)
    #[serde(default = "default_jpeg_quality")]
    pub jpeg_quality: u8,

    /// Maximum frame width (0 = no resize)
    #[serde(default)]
    pub max_width: u32,

    /// Maximum frame height (0 = no resize)
    #[serde(default)]
    pub max_height: u32,
}

fn default_fps() -> u32 {
    10
}

fn default_jpeg_quality() -> u8 {
    75
}

impl Default for CameraStreamConfig {
    fn default() -> Self {
        Self {
            fps: default_fps(),
            encoding: FrameEncoding::Jpeg,
            jpeg_quality: default_jpeg_quality(),
            max_width: 0,
            max_height: 0,
        }
    }
}

/// Camera information included in stream messages
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraStreamInfo {
    /// Camera id
    pub id: uuid::Uuid,

    /// Camera name
    pub name: String,

    /// Frame width
    pub width: usize,

    /// Frame height
    pub height: usize,

    /// Pixel format name
    pub pixel_format: PixelFormat,

    /// Frame timestamp (microseconds since epoch)
    pub timestamp: u64,

    /// Frame sequence number
    pub sequence: u64,
}

/// Camera frame payload for WebSocket transmission
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CameraFramePayload {
    /// Camera information
    pub camera: CameraStreamInfo,

    /// Encoded frame data
    pub data: String,

    /// Data encoding type
    pub encoding: FrameEncoding,

    /// Frame size in bytes (original)
    pub size: usize,
}

/// Control message for camera stream
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "camelCase")]
pub enum CameraControlMessage {
    /// Start streaming from camera
    StartStream {
        #[serde(rename = "cameraIndex")]
        camera_id: uuid::Uuid,
        #[serde(default)]
        config: CameraStreamConfig,
    },

    /// Stop streaming from camera
    StopStream {
        #[serde(rename = "cameraIndex")]
        camera_id: uuid::Uuid,
    },

    /// Update stream configuration
    UpdateConfig {
        #[serde(rename = "cameraIndex")]
        camera_id: uuid::Uuid,
        config: CameraStreamConfig,
    },
}

pub enum StreamEvent {
    Frame(CameraFrame),
    Stop,
}

/// Active stream state
pub struct ActiveStream {
    /// Stop signal
    event_tx: mpsc::Sender<StreamEvent>,

    /// payload broadcast
    payload_tx: broadcast::Sender<CameraFramePayload>,

    subscriber_count: Arc<AtomicU32>,
}

impl ActiveStream {
    pub fn new(id: uuid::Uuid, config: CameraStreamConfig, camera_config: CameraConfig) -> Self {
        let (event_tx, event_rx) = mpsc::channel::<StreamEvent>(1);
        let (payload_tx, _) = broadcast::channel::<CameraFramePayload>(128);
        let sequence = Arc::new(AtomicU64::new(0));
        let subscriber_count = Arc::new(AtomicU32::new(0));

        let obj = Self {
            event_tx,
            payload_tx: payload_tx.clone(),
            subscriber_count: subscriber_count.clone(),
        };

        tokio::spawn(async move {
            Self::stream_loop(
                id,
                config,
                camera_config,
                sequence,
                subscriber_count,
                event_rx,
                payload_tx,
            )
            .await;
        });

        obj
    }

    async fn stream_loop(
        id: uuid::Uuid,
        config: CameraStreamConfig,
        camera_config: CameraConfig,
        sequence: Arc<AtomicU64>,
        subscriber_count: Arc<AtomicU32>,
        mut event_rx: mpsc::Receiver<StreamEvent>,
        payload_tx: broadcast::Sender<CameraFramePayload>,
    ) {
        let frame_interval = Duration::from_millis(1000 / config.fps as u64);
        let mut last_frame_time = Instant::now();
        let mut frame_buffer: Option<CameraFrame> = None;

        tracing::info!(
            "Camera {} stream loop started (interval: {:?})",
            id,
            frame_interval
        );

        loop {
            tokio::select! {
                event = event_rx.recv() => {
                    match event {
                        Some(e) => {
                            match e {
                                StreamEvent::Frame(p) => {
                                    frame_buffer = Some(p)
                                },
                                StreamEvent::Stop => {
                                    tracing::info!("Camera {} stream received stop signal", id);
                                    break;
                                }
                            }
                        }
                        None => {
                            break;
                        }
                    }
                }

                // Send frame at target rate
                _ = tokio::time::sleep(frame_interval.saturating_sub(last_frame_time.elapsed())) => {
                    if let Some(frame) = frame_buffer.take() {
                        // Only send if there are subscribers
                        if subscriber_count.load(Ordering::SeqCst) > 0 {
                            let seq = sequence.fetch_add(1, Ordering::SeqCst);

                            let payload = Self::encode_frame(
                                &id,
                                &camera_config.name(),
                                &frame,
                                seq,
                                &config,
                            );

                            if payload_tx.send(payload).is_err() {
                                tracing::trace!("No subscribers for camera {}", id);
                            };
                        }

                        last_frame_time = Instant::now();
                    }
                }
            }
        }
    }

    fn encode_frame(
        camera_id: &uuid::Uuid,
        camera_name: &str,
        frame: &CameraFrame,
        sequence: u64,
        config: &CameraStreamConfig,
    ) -> CameraFramePayload {
        let camera_info = CameraStreamInfo {
            id: camera_id.clone(),
            name: camera_name.into(),
            width: frame.frame_size.width,
            height: frame.frame_size.height,
            pixel_format: frame.pixel_format,
            timestamp: frame.timestamp,
            sequence,
        };

        let (data, encoding) = match config.encoding {
            FrameEncoding::Jpeg => {
                // Encode as JPEG and convert to base64
                let jpeg_data = Self::encode_jpeg(frame, config.jpeg_quality);
                let base64_data = Self::base64_encode(&jpeg_data);
                (base64_data, FrameEncoding::Jpeg)
            }
            FrameEncoding::Png => {
                // Encode as PNG and convert to base64
                let png_data = Self::encode_png(frame);
                let base64_data = Self::base64_encode(&png_data);
                (base64_data, FrameEncoding::Png)
            }
            FrameEncoding::Raw => {
                let base64_data = Self::base64_encode(&frame.data);
                (base64_data, FrameEncoding::Raw)
            }
        };

        CameraFramePayload {
            camera: camera_info,
            data,
            encoding,
            size: frame.size,
        }
    }

    fn base64_encode(data: &[u8]) -> String {
        STANDARD.encode(data)
    }

    fn encode_jpeg(frame: &CameraFrame, quality: u8) -> Vec<u8> {
        let width = frame.frame_size.width;
        let height = frame.frame_size.height;

        let jpeg_quality = quality.clamp(1, 100);

        match frame.pixel_format {
            PixelFormat::Mono8 => {
                // Grayscale image
                let img: ImageBuffer<Luma<u8>, Vec<u8>> =
                    match ImageBuffer::from_raw(width as u32, height as u32, frame.data.to_vec()) {
                        Some(img) => img,
                        None => {
                            tracing::warn!("Failed to create grayscale image buffer");
                            return frame.data.to_vec();
                        }
                    };

                let mut buffer = Vec::new();
                let encoder = JpegEncoder::new_with_quality(&mut buffer, jpeg_quality);

                match img.write_with_encoder(encoder) {
                    Ok(()) => buffer,
                    Err(e) => {
                        tracing::warn!("JPEG encoding failed: {}", e);
                        frame.data.to_vec()
                    }
                }
            }
            PixelFormat::RGB8 => {
                // RGB image
                let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                    match ImageBuffer::from_raw(width as u32, height as u32, frame.data.to_vec()) {
                        Some(img) => img,
                        None => {
                            tracing::warn!("Failed to create RGB image buffer");
                            return frame.data.to_vec();
                        }
                    };

                let mut buffer = Vec::new();
                let encoder = JpegEncoder::new_with_quality(&mut buffer, jpeg_quality);

                match img.write_with_encoder(encoder) {
                    Ok(()) => buffer,
                    Err(e) => {
                        tracing::warn!("JPEG encoding failed: {}", e);
                        frame.data.to_vec()
                    }
                }
            }
            PixelFormat::BGR8 => {
                // BGR to RGB conversion
                let mut rgb_data = Vec::with_capacity(frame.data.len());
                for chunk in frame.data.chunks(3) {
                    if chunk.len() == 3 {
                        rgb_data.push(chunk[2]); // R
                        rgb_data.push(chunk[1]); // G
                        rgb_data.push(chunk[0]); // B
                    }
                }

                let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                    match ImageBuffer::from_raw(width as u32, height as u32, rgb_data) {
                        Some(img) => img,
                        None => {
                            tracing::warn!("Failed to create RGB image buffer from BGR");
                            return frame.data.to_vec();
                        }
                    };

                let mut buffer = Vec::new();
                let encoder = JpegEncoder::new_with_quality(&mut buffer, jpeg_quality);

                match img.write_with_encoder(encoder) {
                    Ok(()) => buffer,
                    Err(e) => {
                        tracing::warn!("JPEG encoding failed: {}", e);
                        frame.data.to_vec()
                    }
                }
            }
            PixelFormat::RGBA8 => {
                // RGBA to RGB (drop alpha)
                let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
                for chunk in frame.data.chunks(4) {
                    if chunk.len() == 4 {
                        rgb_data.push(chunk[0]); // R
                        rgb_data.push(chunk[1]); // G
                        rgb_data.push(chunk[2]); // B
                        // Skip chunk[3] (Alpha)
                    }
                }

                let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                    match ImageBuffer::from_raw(width as u32, height as u32, rgb_data) {
                        Some(img) => img,
                        None => {
                            tracing::warn!("Failed to create RGB image buffer from RGBA");
                            return frame.data.to_vec();
                        }
                    };

                let mut buffer = Vec::new();
                let encoder = JpegEncoder::new_with_quality(&mut buffer, jpeg_quality);

                match img.write_with_encoder(encoder) {
                    Ok(()) => buffer,
                    Err(e) => {
                        tracing::warn!("JPEG encoding failed: {}", e);
                        frame.data.to_vec()
                    }
                }
            }
            PixelFormat::BGRA8 => {
                // BGRA to RGB conversion
                let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
                for chunk in frame.data.chunks(4) {
                    if chunk.len() == 4 {
                        rgb_data.push(chunk[2]); // R
                        rgb_data.push(chunk[1]); // G
                        rgb_data.push(chunk[0]); // B
                        // Skip chunk[3] (Alpha)
                    }
                }

                let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                    match ImageBuffer::from_raw(width as u32, height as u32, rgb_data) {
                        Some(img) => img,
                        None => {
                            tracing::warn!("Failed to create RGB image buffer from BGRA");
                            return frame.data.to_vec();
                        }
                    };

                let mut buffer = Vec::new();
                let encoder = JpegEncoder::new_with_quality(&mut buffer, jpeg_quality);

                match img.write_with_encoder(encoder) {
                    Ok(()) => buffer,
                    Err(e) => {
                        tracing::warn!("JPEG encoding failed: {}", e);
                        frame.data.to_vec()
                    }
                }
            }
            _ => {
                // Unsupported pixel format, return raw data
                tracing::debug!(
                    "Unsupported pixel format {:?} for JPEG encoding, returning raw data",
                    frame.pixel_format
                );
                frame.data.to_vec()
            }
        }
    }

    fn encode_png(frame: &CameraFrame) -> Vec<u8> {
        let width = frame.frame_size.width;
        let height = frame.frame_size.height;

        match frame.pixel_format {
            PixelFormat::Mono8 => {
                // Grayscale image
                let img: ImageBuffer<Luma<u8>, Vec<u8>> =
                    match ImageBuffer::from_raw(width as u32, height as u32, frame.data.to_vec()) {
                        Some(img) => img,
                        None => {
                            tracing::warn!("Failed to create grayscale image buffer");
                            return frame.data.to_vec();
                        }
                    };

                let mut buffer = Vec::new();
                let mut cursor = Cursor::new(&mut buffer);

                // Encode as PNG
                match img.write_to(&mut cursor, image::ImageFormat::Png) {
                    Ok(()) => buffer,
                    Err(e) => {
                        tracing::warn!("PNG encoding failed: {}", e);
                        frame.data.to_vec()
                    }
                }
            }
            PixelFormat::RGB8 => {
                // RGB image
                let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                    match ImageBuffer::from_raw(width as u32, height as u32, frame.data.to_vec()) {
                        Some(img) => img,
                        None => {
                            tracing::warn!("Failed to create RGB image buffer");
                            return frame.data.to_vec();
                        }
                    };

                let mut buffer = Vec::new();
                let mut cursor = Cursor::new(&mut buffer);

                match img.write_to(&mut cursor, image::ImageFormat::Png) {
                    Ok(()) => buffer,
                    Err(e) => {
                        tracing::warn!("PNG encoding failed: {}", e);
                        frame.data.to_vec()
                    }
                }
            }
            PixelFormat::BGR8 => {
                // BGR to RGB conversion
                let mut rgb_data = Vec::with_capacity(frame.data.len());
                for chunk in frame.data.chunks(3) {
                    if chunk.len() == 3 {
                        rgb_data.push(chunk[2]); // R
                        rgb_data.push(chunk[1]); // G
                        rgb_data.push(chunk[0]); // B
                    }
                }

                let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                    match ImageBuffer::from_raw(width as u32, height as u32, rgb_data) {
                        Some(img) => img,
                        None => {
                            tracing::warn!("Failed to create RGB image buffer from BGR");
                            return frame.data.to_vec();
                        }
                    };

                let mut buffer = Vec::new();
                let mut cursor = Cursor::new(&mut buffer);

                match img.write_to(&mut cursor, image::ImageFormat::Png) {
                    Ok(()) => buffer,
                    Err(e) => {
                        tracing::warn!("PNG encoding failed: {}", e);
                        frame.data.to_vec()
                    }
                }
            }
            PixelFormat::RGBA8 => {
                // RGBA to RGB (drop alpha)
                let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
                for chunk in frame.data.chunks(4) {
                    if chunk.len() == 4 {
                        rgb_data.push(chunk[0]); // R
                        rgb_data.push(chunk[1]); // G
                        rgb_data.push(chunk[2]); // B
                        // Skip chunk[3] (Alpha)
                    }
                }

                let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                    match ImageBuffer::from_raw(width as u32, height as u32, rgb_data) {
                        Some(img) => img,
                        None => {
                            tracing::warn!("Failed to create RGB image buffer from RGBA");
                            return frame.data.to_vec();
                        }
                    };

                let mut buffer = Vec::new();
                let mut cursor = Cursor::new(&mut buffer);

                match img.write_to(&mut cursor, image::ImageFormat::Png) {
                    Ok(()) => buffer,
                    Err(e) => {
                        tracing::warn!("PNG encoding failed: {}", e);
                        frame.data.to_vec()
                    }
                }
            }
            PixelFormat::BGRA8 => {
                // BGRA to RGB conversion
                let mut rgb_data = Vec::with_capacity((width * height * 3) as usize);
                for chunk in frame.data.chunks(4) {
                    if chunk.len() == 4 {
                        rgb_data.push(chunk[2]); // R
                        rgb_data.push(chunk[1]); // G
                        rgb_data.push(chunk[0]); // B
                        // Skip chunk[3] (Alpha)
                    }
                }

                let img: ImageBuffer<Rgb<u8>, Vec<u8>> =
                    match ImageBuffer::from_raw(width as u32, height as u32, rgb_data) {
                        Some(img) => img,
                        None => {
                            tracing::warn!("Failed to create RGB image buffer from BGRA");
                            return frame.data.to_vec();
                        }
                    };

                let mut buffer = Vec::new();
                let mut cursor = Cursor::new(&mut buffer);

                match img.write_to(&mut cursor, image::ImageFormat::Png) {
                    Ok(()) => buffer,
                    Err(e) => {
                        tracing::warn!("PNG encoding failed: {}", e);
                        frame.data.to_vec()
                    }
                }
            }
            _ => {
                // Unsupported pixel format, return raw data
                tracing::debug!(
                    "Unsupported pixel format {:?} for PNG encoding, returning raw data",
                    frame.pixel_format
                );
                frame.data.to_vec()
            }
        }
    }

    /// Subscribe to a camera stream
    pub async fn subscribe(&self) -> broadcast::Receiver<CameraFramePayload> {
        // Increment subscriber count
        self.subscriber_count.fetch_add(1, Ordering::SeqCst);
        self.payload_tx.subscribe()
    }

    /// Unsubscribe from a camera stream
    pub async fn unsubscribe(&self) {
        self.subscriber_count.fetch_sub(1, Ordering::SeqCst);
    }

    pub async fn stop_stream(&self) {
        let _ = self.event_tx.send(StreamEvent::Stop).await;
    }

    pub async fn trigger_frame(&self, frame: &CameraFrame) {
        let _ = self.event_tx.send(StreamEvent::Frame(frame.clone()));
    }
}
