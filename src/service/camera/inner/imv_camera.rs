#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::ptr::null_mut;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{RwLock, mpsc};

use crate::ffi::imv::*;
use crate::service::camera::{
    CameraConfig, CameraError, CameraFrame, CameraInfo, CameraSupplier, FrameSize, GrabMode,
    IndustryCamera, PixelFormat,
};

pub struct IMVCameraBuilder {
    mode: IMV_ECreateHandleMode,
    index: u32,
    camera_key: String,
    device_user_id: String,
    ip_address: String,
}

impl IMVCameraBuilder {
    pub fn new() -> Self {
        Self {
            mode: _IMV_ECreateHandleMode_modeByIndex,
            index: 0,
            camera_key: String::from(""),
            device_user_id: String::from(""),
            ip_address: String::from(""),
        }
    }

    pub fn new_with_config(config: &CameraConfig) -> Self {
        tracing::debug!("camera config: {:?}", config);
        let builder = Self::new();
        if let Some(ref key) = config.key {
            return builder
                .with_camera_key(key)
                .with_mode(crate::ffi::imv::_IMV_ECreateHandleMode_modeByCameraKey);
        } else if let Some(ref device_user_id) = config.device_user_id {
            return builder
                .with_device_user_id(device_user_id)
                .with_mode(crate::ffi::imv::_IMV_ECreateHandleMode_modeByDeviceUserID);
        } else if let Some(ref ip_address) = config.ip_address {
            return builder
                .with_ip_address(ip_address)
                .with_mode(crate::ffi::imv::_IMV_ECreateHandleMode_modeByIPAddress);
        }

        return builder;
    }

    pub fn with_mode(mut self, mode: IMV_ECreateHandleMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_camera_key(mut self, camera_key: &str) -> Self {
        self.camera_key = camera_key.into();
        self
    }

    pub fn with_device_user_id(mut self, device_user_id: &str) -> Self {
        self.device_user_id = device_user_id.into();
        self
    }

    pub fn with_ip_address(mut self, ip_address: &str) -> Self {
        self.ip_address = ip_address.into();
        self
    }

    pub fn build(&self) -> Result<IMVCamera, CameraError> {
        let mut handle = null_mut();

        match self.mode {
            _IMV_ECreateHandleMode_modeByIndex => unsafe {
                let ret = IMV_CreateHandle(
                    &mut handle,
                    self.mode,
                    self.index as *mut c_void,
                );
                if ret != IMV_OK {
                    return Err(CameraError::CameraHandler(format!("{:?}", ret)));
                }
            },
            _IMV_ECreateHandleMode_modeByCameraKey => {
                let c_string = CString::new(self.camera_key.as_str())
                    .map_err(|e| CameraError::InvalidKeyName(format!("{:?}", e)))?;
                let ptr_c_void = c_string.as_ptr() as *mut c_void;

                unsafe {
                    let ret = IMV_CreateHandle(&mut handle, self.mode, ptr_c_void);
                    if ret != IMV_OK {
                        return Err(CameraError::CameraHandler(format!("{:?}", ret)).into());
                    }
                }
            }
            _IMV_ECreateHandleMode_modeByDeviceUserID => {
                let c_string = CString::new(self.device_user_id.as_str())
                    .map_err(|e| CameraError::InvalidDeviceUserId(format!("{:?}", e)))?;
                let ptr_c_void = c_string.as_ptr() as *mut c_void;

                unsafe {
                    let ret = IMV_CreateHandle(&mut handle, self.mode, ptr_c_void);
                    if ret != IMV_OK {
                        return Err(CameraError::CameraHandler(format!("{:?}", ret)).into());
                    }
                }
            }
            _IMV_ECreateHandleMode_modeByIPAddress => {
                let c_string = CString::new(self.ip_address.as_str())
                    .map_err(|e| CameraError::InvalidIpAddress(format!("{:?}", e)))?;
                let ptr_c_void = c_string.as_ptr() as *mut c_void;

                unsafe {
                    let ret = IMV_CreateHandle(&mut handle, self.mode, ptr_c_void);
                    if ret != IMV_OK {
                        return Err(CameraError::CameraHandler(format!("{:?}", ret)).into());
                    }
                }
            }
            _ => {
                return Err(CameraError::CameraHandler(format!("{:?}", IMV_ERROR)).into());
            }
        }

        Ok(IMVCamera {
            inner: Arc::new(RwLock::new(CameraHandler::new(handle))),
            grab_mode: GrabMode::Continuous,
            exposure_auto: false,
            exposure_time: std::time::Duration::from_millis(1000),
            frame_sender: None,
        })
    }
}

pub fn get_camera_list() -> Result<Vec<CameraInfo>, CameraError> {
    let mut camera_list = Vec::new();

    unsafe {
        let mut device_info_list = IMV_DeviceList::default();
        let ret = IMV_EnumDevices(
            &mut device_info_list,
            _IMV_EInterfaceType_interfaceTypeAll as u32,
        );

        if ret != IMV_OK {
            return Err(CameraError::EnumerationError(format!("{:?}", ret)));
        }

        for i in 0..device_info_list.nDevNum {
            let device_info = *device_info_list.pDevInfo.add(i as usize);
            let mut camera_info = CameraInfo {
                key: CStr::from_ptr(device_info.cameraKey.as_ptr())
                    .to_string_lossy()
                    .into_owned(),
                device_user_id: CStr::from_ptr(device_info.cameraName.as_ptr())
                    .to_string_lossy()
                    .into_owned(),
                serial_number: CStr::from_ptr(device_info.serialNumber.as_ptr())
                    .to_string_lossy()
                    .into_owned(),
                vendor: CStr::from_ptr(device_info.vendorName.as_ptr())
                    .to_string_lossy()
                    .into_owned(),
                model: CStr::from_ptr(device_info.modelName.as_ptr())
                    .to_string_lossy()
                    .into_owned(),
                manufacture_info: CStr::from_ptr(device_info.manufactureInfo.as_ptr())
                    .to_string_lossy()
                    .into_owned(),
                device_version: CStr::from_ptr(device_info.deviceVersion.as_ptr())
                    .to_string_lossy()
                    .into_owned(),
                ip_address: None,
                mac_address: None,
                camera_supplier: CameraSupplier::IMV,
            };

            if device_info.nCameraType == _IMV_ECameraType_typeGigeCamera {
                camera_info.ip_address = Some(
                    CStr::from_ptr(
                        device_info
                            .InterfaceInfo
                            .gigeInterfaceInfo
                            .ipAddress
                            .as_ptr(),
                    )
                    .to_string_lossy()
                    .into_owned(),
                );
                camera_info.mac_address = Some(
                    CStr::from_ptr(
                        device_info
                            .InterfaceInfo
                            .gigeInterfaceInfo
                            .macAddress
                            .as_ptr(),
                    )
                    .to_string_lossy()
                    .into_owned(),
                );
            }
            camera_list.push(camera_info);
        }
    }

    Ok(camera_list)
}

/// 从 IMV_Frame 拷贝数据到 CameraFrame（不释放帧缓存）
fn convert_imv_frame_to_camera_frame(frame: &IMV_Frame) -> CameraFrame {
    let frame_info = frame.frameInfo;
    let data = unsafe { std::slice::from_raw_parts(frame.pData, frame_info.size as usize) };

    CameraFrame {
        block_id: frame_info.blockId,
        status: frame_info.status,
        frame_size: FrameSize {
            width: frame_info.width as usize,
            height: frame_info.height as usize,
        },
        size: frame_info.size as usize,
        pixel_format: frame_info.pixelFormat.into(),
        timestamp: frame_info.timeStamp,
        chunk_count: frame_info.chunkCount as usize,
        padding_x: frame_info.paddingX as usize,
        padding_y: frame_info.paddingY as usize,
        recv_frame_time: frame_info.recvFrameTime as u64,
        data: data.into(),
    }
}

/// 回调上下文 - 独立于 CameraHandler，用于 C 回调线程安全访问
/// 
/// 使用 Arc<Mutex<...>> 保护，确保：
/// 1. C 回调可以安全地从任意线程访问
/// 2. Rust 侧也可以安全地修改 frame_sender
/// 3. 生命周期由 Arc 引用计数管理
struct GrabCallbackContext {
    handle: IMV_HANDLE,
    frame_sender: Mutex<Option<mpsc::Sender<CameraFrame>>>,
    runtime_handle: Mutex<Option<tokio::runtime::Handle>>,
}

impl GrabCallbackContext {
    fn new(handle: IMV_HANDLE) -> Self {
        Self {
            handle,
            frame_sender: Mutex::new(None),
            runtime_handle: Mutex::new(None),
        }
    }

    fn set_sender(&self, sender: mpsc::Sender<CameraFrame>) {
        let mut guard = self.frame_sender.lock().unwrap();
        *guard = Some(sender);
    }

    fn set_runtime_handle(&self, handle: Option<tokio::runtime::Handle>) {
        let mut guard = self.runtime_handle.lock().unwrap();
        *guard = handle;
    }

    fn clear(&self) {
        let mut sender_guard = self.frame_sender.lock().unwrap();
        *sender_guard = None;
        let mut runtime_guard = self.runtime_handle.lock().unwrap();
        *runtime_guard = None;
    }

    fn handle_frame(&self, frame: &CameraFrame) {
        let sender_guard = self.frame_sender.lock().unwrap();
        if let Some(sender) = sender_guard.as_ref() {
            let runtime_guard = self.runtime_handle.lock().unwrap();
            if let Some(handle) = runtime_guard.as_ref() {
                let sender_clone = sender.clone();
                let frame = frame.clone();
                handle.spawn(async move {
                    if let Err(e) = sender_clone.send(frame).await {
                        tracing::error!("send frame to channel failed: {:?}", e);
                    }
                });
            } else {
                tracing::error!("No tokio runtime available");
            }
        }
    }
}

struct CameraHandler {
    handle: IMV_HANDLE,
    grab_context: Option<Arc<GrabCallbackContext>>,
}

// SAFETY: IMV_HANDLE 是 SDK 提供的句柄，SDK 保证其 API 是线程安全的。
// 所有的 IMV_* 函数都可以从任意线程调用，SDK 内部有适当的同步机制。
// grab_context 使用 Arc<Mutex<...>> 保护，也是线程安全的。
unsafe impl Send for CameraHandler {}
unsafe impl Sync for CameraHandler {}

impl CameraHandler {
    fn new(handle: IMV_HANDLE) -> Self {
        Self {
            handle,
            grab_context: None,
        }
    }

    fn open(&self) -> Result<(), CameraError> {
        unsafe {
            tracing::debug!("imv camera handle: {:?}", self.handle);
            let ret = IMV_Open(self.handle);
            if ret != IMV_OK {
                return Err(CameraError::OpenError(format!("{:?}", ret)));
            }
        }

        Ok(())
    }

    fn is_opened(&self) -> bool {
        if self.handle.is_null() {
            return false;
        }

        unsafe {
            let ret = IMV_IsOpen(self.handle);
            ret != 0
        }
    }

    fn is_grabbing(&self) -> bool {
        unsafe {
            let ret = IMV_IsGrabbing(self.handle);
            ret != 0
        }
    }

    fn stop_grab(&mut self) -> Result<(), CameraError> {
        unsafe {
            let ret = IMV_StopGrabbing(self.handle);
            if ret != IMV_OK {
                return Err(CameraError::GrabError(format!("{:?}", ret)));
            }

            let _ = IMV_ClearFrameBuffer(self.handle);
        }

        // 清理回调上下文
        if let Some(ctx) = &self.grab_context {
            ctx.clear();
        }
        self.grab_context = None;

        Ok(())
    }

    fn start_grab(
        &mut self,
        grab_mode: GrabMode,
        sender: Option<mpsc::Sender<CameraFrame>>,
    ) -> Result<(), CameraError> {
        // 创建独立的回调上下文
        let context = Arc::new(GrabCallbackContext::new(self.handle));
        
        // 设置 sender（如果提供）
        if let Some(s) = sender {
            context.set_sender(s);
        }
        
        // 捕获当前 Tokio 运行时句柄
        context.set_runtime_handle(tokio::runtime::Handle::try_current().ok());

        if grab_mode == GrabMode::Continuous {
            // 传递 Arc 的原始指针给 C 回调
            // 使用 Arc::into_raw 获得 *const T，然后转为 *mut c_void
            let context_ptr = Arc::into_raw(context.clone()) as *mut c_void;
            unsafe {
                let ret = IMV_AttachGrabbing(
                    self.handle,
                    Some(frame_callback),
                    context_ptr,
                );
                if ret != IMV_OK {
                    // 回调注册失败，需要释放 Arc
                    // 从原始指针恢复 Arc 并释放
                    let _ = Arc::from_raw(context_ptr as *const GrabCallbackContext);
                    return Err(CameraError::GrabError(format!("{:?}", ret)));
                }
            }
        }

        unsafe {
            let ret = IMV_StartGrabbing(self.handle);
            if ret != IMV_OK {
                return Err(CameraError::GrabError(format!("{:?}", ret)));
            }
        }
        
        // 保存 context 引用
        self.grab_context = Some(context);
        Ok(())
    }

    fn set_enum_feature_symbol(
        &self,
        feature_name: &str,
        enum_symbol: &str,
    ) -> Result<(), CameraError> {
        unsafe {
            let feature_name_c_str = CString::from_str(feature_name)
                .map_err(|e| CameraError::SystemError(format!("{:?}", e)))?;
            let enum_symbol_c_str = CString::from_str(enum_symbol)
                .map_err(|e| CameraError::SystemError(format!("{:?}", e)))?;
            let feature_name_ptr = feature_name_c_str.as_ptr() as *const i8;
            let enum_symbol_ptr = enum_symbol_c_str.as_ptr() as *const i8;

            let ret = IMV_SetEnumFeatureSymbol(self.handle, feature_name_ptr, enum_symbol_ptr);
            if ret != IMV_OK {
                return Err(CameraError::Config(format!(
                    "设置枚举属性: {} 枚举值: {} {:?}",
                    feature_name, enum_symbol, ret
                )));
            }
        }
        Ok(())
    }

    fn sync_grab_mode(&self, grab_mode: GrabMode) -> Result<(), CameraError> {
        self.set_enum_feature_symbol("TriggerSelector", "FrameStart")?;

        match grab_mode {
            GrabMode::Continuous => {
                self.set_enum_feature_symbol("TriggerMode", "Off")?;
            }
            GrabMode::SingleFrame => {
                self.set_enum_feature_symbol("TriggerSource", "Software")?;
                self.set_enum_feature_symbol("TriggerMode", "On")?;
            }
        }

        Ok(())
    }

    fn set_enum_feature_value(
        &self,
        feature_name: &str,
        enum_value: u64,
    ) -> Result<(), CameraError> {
        unsafe {
            let feature_name_c_str = CString::from_str(feature_name)
                .map_err(|e| CameraError::SystemError(format!("{:?}", e)))?;
            let feature_name_ptr = feature_name_c_str.as_ptr() as *const i8;
            let ret = IMV_SetEnumFeatureValue(self.handle, feature_name_ptr, enum_value);
            if ret != IMV_OK {
                return Err(CameraError::Config(format!(
                    "设置枚举属性值: {} 枚举值: {} {:?}",
                    feature_name, enum_value, ret
                )));
            }
        }

        Ok(())
    }

    fn sync_exposure_auto(&self, exposure_auto: bool) -> Result<(), CameraError> {
        if exposure_auto {
            self.set_enum_feature_value("ExposureAuto", 2)?;
        } else {
            self.set_enum_feature_value("ExposureAuto", 0)?;
        }
        Ok(())
    }

    fn get_double_feature_value(&self, feature_name: &str) -> Result<f64, CameraError> {
        unsafe {
            let feature_name_c_str = CString::from_str(feature_name)
                .map_err(|e| CameraError::SystemError(format!("{:?}", e)))?;
            let feature_name_ptr = feature_name_c_str.as_ptr() as *const i8;
            let mut value = 0.0;
            let ret = IMV_GetDoubleFeatureValue(self.handle, feature_name_ptr, &mut value);
            if ret != IMV_OK {
                return Err(CameraError::Config(format!(
                    "获取浮点属性值: {} {:?}",
                    feature_name, ret
                )));
            }

            Ok(value)
        }
    }

    fn get_double_feature_min(&self, feature_name: &str) -> Result<f64, CameraError> {
        unsafe {
            let feature_name_c_str = CString::from_str(feature_name)
                .map_err(|e| CameraError::SystemError(format!("{:?}", e)))?;
            let feature_name_ptr = feature_name_c_str.as_ptr() as *const i8;
            let mut value = 0.0;
            let ret = IMV_GetDoubleFeatureMin(self.handle, feature_name_ptr, &mut value);
            if ret != IMV_OK {
                return Err(CameraError::Config(format!(
                    "获取浮点属性可设的最小值: {} {:?}",
                    feature_name, ret
                )));
            }

            Ok(value)
        }
    }

    fn get_double_feature_max(&self, feature_name: &str) -> Result<f64, CameraError> {
        unsafe {
            let feature_name_c_str = CString::from_str(feature_name)
                .map_err(|e| CameraError::SystemError(format!("{:?}", e)))?;
            let feature_name_ptr = feature_name_c_str.as_ptr() as *const i8;
            let mut value = 0.0;
            let ret = IMV_GetDoubleFeatureMax(self.handle, feature_name_ptr, &mut value);
            if ret != IMV_OK {
                return Err(CameraError::Config(format!(
                    "获取浮点属性可设的最大值: {} {:?}",
                    feature_name, ret
                )));
            }

            Ok(value)
        }
    }

    fn set_double_feature_value(
        &self,
        feature_name: &str,
        double_value: f64,
    ) -> Result<(), CameraError> {
        unsafe {
            let feature_name_c_ptr = CString::from_str(feature_name)
                .map_err(|e| CameraError::SystemError(format!("{:?}", e)))?;
            let feature_name_ptr = feature_name_c_ptr.as_ptr() as *const i8;
            let ret = IMV_SetDoubleFeatureValue(self.handle, feature_name_ptr, double_value);
            if ret != IMV_OK {
                return Err(CameraError::Config(format!(
                    "设置浮点属性值: {} 值: {} {:?}",
                    feature_name, double_value, ret
                )));
            }

            Ok(())
        }
    }

    fn sync_exposure_time(&self, exposure_time: Duration) -> Result<(), CameraError> {
        let mut et = self.get_double_feature_value("ExposureTime")?;
        let exposure_min_value = self.get_double_feature_min("ExposureTime")?;
        let exposure_max_value = self.get_double_feature_max("ExposureTime")?;

        tracing::debug!(
            "et: {:?}, exposure_min_value: {:?}, exposure_max_value: {:?}",
            et,
            exposure_min_value,
            exposure_max_value
        );

        et = exposure_time.as_millis() as f64;
        if et < exposure_min_value {
            et = exposure_min_value;
        } else if et > exposure_max_value {
            et = exposure_max_value;
        }
        self.set_double_feature_value("ExposureTime", et)
    }

    fn close(&self) -> Result<(), CameraError> {
        unsafe {
            let ret = IMV_Close(self.handle);
            if ret != IMV_OK {
                return Err(CameraError::CloseError(format!("{:?}", ret)));
            }
        }
        Ok(())
    }

    fn clear_frame_buffer(&self) -> Result<(), CameraError> {
        unsafe {
            let ret = IMV_ClearFrameBuffer(self.handle);
            if ret != IMV_OK {
                return Err(CameraError::FrameError(format!("{:?}", ret)));
            }
        }

        Ok(())
    }

    fn execute_command_feature(&self, feature_name: &str) -> Result<(), CameraError> {
        unsafe {
            let feature_name_c_str = CString::from_str(feature_name)
                .map_err(|e| CameraError::SystemError(format!("{:?}", e)))?;
            let feature_name_ptr = feature_name_c_str.as_ptr() as *const i8;
            let ret = IMV_ExecuteCommandFeature(self.handle, feature_name_ptr);
            if ret != IMV_OK {
                return Err(CameraError::Config(format!(
                    "执行命令属性: {} {:?}",
                    feature_name, ret
                )));
            }

            Ok(())
        }
    }

    fn get_frame_and_convert(&self) -> Result<CameraFrame, CameraError> {
        let mut frame = IMV_Frame::default();
        unsafe {
            let ret = IMV_GetFrame(self.handle, &mut frame, 1000);
            if ret != IMV_OK {
                return Err(CameraError::GrabError(format!("{}", ret)));
            }
        }

        // 从 SDK 帧数据拷贝到 Rust 的 Vec
        let camera_frame = convert_imv_frame_to_camera_frame(&frame);

        // 必须释放帧缓存，否则 SDK 内部缓存会耗尽
        // SDK 默认缓存大小通常为 8，超过后 IMV_GetFrame 会失败
        unsafe {
            IMV_ReleaseFrame(self.handle, &mut frame);
        }

        Ok(camera_frame)
    }

    fn get_int_feature_value(&self, feature_name: &str) -> Result<i64, CameraError> {
        unsafe {
            let feature_name_c_str = CString::from_str(feature_name)
                .map_err(|e| CameraError::SystemError(format!("{:?}", e)))?;
            let feature_name_ptr = feature_name_c_str.as_ptr() as *const i8;
            let mut value: i64 = 0;
            let ret = IMV_GetIntFeatureValue(self.handle, feature_name_ptr, &mut value);
            if ret != IMV_OK {
                return Err(CameraError::Config(format!(
                    "获取整数属性值: {} {:?}",
                    feature_name, ret
                )));
            }

            Ok(value)
        }
    }

    fn handle_single_frame(&self, frame: &CameraFrame) {
        // 单帧模式下直接处理（无回调上下文）
        tracing::debug!("Single frame captured: block_id={}", frame.block_id);
    }
}

impl Drop for CameraHandler {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }

        let _ = self.stop_grab();
        let _ = self.close();

        unsafe {
            IMV_DestroyHandle(self.handle);
            self.handle = null_mut();
        }
    }
}

pub struct IMVCamera {
    inner: Arc<RwLock<CameraHandler>>,
    grab_mode: GrabMode,
    exposure_auto: bool,
    exposure_time: std::time::Duration,
    frame_sender: Option<mpsc::Sender<CameraFrame>>,
}

#[async_trait::async_trait]
impl IndustryCamera for IMVCamera {
    async fn open(&self) -> Result<(), CameraError> {
        let inner = self.inner.read().await;
        if inner.is_opened() {
            return Ok(());
        }

        inner.open()
    }

    async fn is_opened(&self) -> bool {
        self.inner.read().await.is_opened()
    }

    async fn is_grabbing(&self) -> bool {
        let inner = self.inner.read().await;
        if !inner.is_opened() {
            return false;
        }
        inner.is_grabbing()
    }

    async fn stop_grab(&mut self) -> Result<(), CameraError> {
        let mut inner = self.inner.write().await;
        if !inner.is_opened() {
            return Ok(());
        }
        if !inner.is_grabbing() {
            return Ok(());
        }
        inner.stop_grab()
    }

    async fn start_grab(&mut self) -> Result<(), CameraError> {
        let mut inner = self.inner.write().await;
        if !inner.is_opened() {
            return Err(CameraError::NotOpened(format!(
                "start_grab {:?}",
                IMV_ERROR
            )));
        }

        if inner.is_grabbing() {
            return Ok(());
        }

        inner.sync_grab_mode(self.grab_mode)?;
        let _ = inner.sync_exposure_auto(self.exposure_auto);
        let _ = inner.sync_exposure_time(self.exposure_time);
        
        // 传递 frame_sender 到 CameraHandler
        let sender = self.frame_sender.clone();
        inner.start_grab(self.grab_mode, sender)
    }

    async fn close(&self) -> Result<(), CameraError> {
        let inner = self.inner.read().await;
        if !inner.is_opened() {
            return Ok(());
        }

        inner.close()
    }

    async fn frame_size(&self) -> Result<FrameSize, CameraError> {
        let inner = self.inner.read().await;
        let width = inner.get_int_feature_value("Width")? as usize;
        let height = inner.get_int_feature_value("Height")? as usize;

        Ok(FrameSize { width, height })
    }

    async fn trigger_one_frame(&self) -> Result<CameraFrame, CameraError> {
        let inner = self.inner.read().await;
        inner.clear_frame_buffer()?;

        inner.execute_command_feature("TriggerSoftware")?;

        let mut try_counter = Some(0);

        while let Some(counter) = try_counter {
            if counter > 3 {
                try_counter = None;
            } else {
                try_counter = Some(counter + 1);
            }

            match inner.get_frame_and_convert() {
                Ok(frame) => {
                    inner.handle_single_frame(&frame);
                    return Ok(frame);
                }
                Err(_) => continue,
            }
        }

        Err(CameraError::GrabError(format!("{}", IMV_ERROR)))
    }

    async fn create_frame_channel(&mut self) -> mpsc::Receiver<CameraFrame> {
        let (sender, receiver) = mpsc::channel(1024);
        self.frame_sender = Some(sender);
        receiver
    }

    async fn set_grab_mode(&mut self, grab_mode: GrabMode) {
        self.grab_mode = grab_mode;
    }

    async fn set_exposure_auto(&mut self, auto: bool) {
        self.exposure_auto = auto;
    }

    async fn set_exposure_time(&mut self, time: std::time::Duration) {
        self.exposure_time = time;
    }
}

/// C 回调函数 - 线程安全
/// 
/// # Safety
/// - user_ptr 是由 Arc::into_raw 生成的 GrabCallbackContext 指针
/// - 回调可能在任意 C 线程中执行
/// - 使用 Arc::from_raw 重建引用，通过 Mutex 安全访问数据
unsafe extern "C" fn frame_callback(
    frame_ptr: *mut IMV_Frame,
    user_ptr: *mut ::std::os::raw::c_void,
) {
    if user_ptr.is_null() {
        tracing::error!("Error: user_ptr is null in frame_callback");
        return;
    }
    if frame_ptr.is_null() {
        tracing::error!("Error: frame_ptr is null in frame_callback");
        return;
    }

    // SAFETY: user_ptr 是由 start_grab 中的 Arc::into_raw 生成的有效指针
    let context = unsafe { Arc::from_raw(user_ptr as *const GrabCallbackContext) };
    let handle = context.handle;

    // SAFETY: frame_ptr 是 SDK 提供的有效帧指针
    unsafe {
        let frame = frame_ptr.as_ref();
        if frame.is_none() {
            // 需要释放我们刚才增加的引用计数
            std::mem::forget(context);
            return;
        }
        let frame = frame.unwrap();
        
        // 拷贝帧数据到 Rust，然后通过 Mutex 安全访问
        let camera_frame = convert_imv_frame_to_camera_frame(frame);
        context.handle_frame(&camera_frame);

        // 释放帧缓存
        IMV_ReleaseFrame(handle, frame_ptr);
    }

    // 释放我们刚才增加的引用计数，但不释放内存
    // 因为 C 库可能还会继续调用回调（Arc 内部仍有一个引用）
    std::mem::forget(context);
}

impl Into<PixelFormat> for IMV_EPixelType {
    fn into(self) -> PixelFormat {
        match self {
            _IMV_EPixelType_gvspPixelTypeUndefined => PixelFormat::Undefined,
            _IMV_EPixelType_gvspPixelMono1p => PixelFormat::Mono1p,
            _IMV_EPixelType_gvspPixelMono2p => PixelFormat::Mono2p,
            _IMV_EPixelType_gvspPixelMono4p => PixelFormat::Mono4p,
            _IMV_EPixelType_gvspPixelMono8 => PixelFormat::Mono8,
            _IMV_EPixelType_gvspPixelMono8S => PixelFormat::Mono8S,
            _IMV_EPixelType_gvspPixelMono10 => PixelFormat::Mono10,
            _IMV_EPixelType_gvspPixelMono10Packed => PixelFormat::Mono10Packed,
            _IMV_EPixelType_gvspPixelMono12 => PixelFormat::Mono12,
            _IMV_EPixelType_gvspPixelMono12Packed => PixelFormat::Mono12Packed,
            _IMV_EPixelType_gvspPixelMono14 => PixelFormat::Mono14,
            _IMV_EPixelType_gvspPixelMono16 => PixelFormat::Mono16,
            _IMV_EPixelType_gvspPixelBayGR8 => PixelFormat::BayGR8,
            _IMV_EPixelType_gvspPixelBayRG8 => PixelFormat::BayRG8,
            _IMV_EPixelType_gvspPixelBayGB8 => PixelFormat::BayGB8,
            _IMV_EPixelType_gvspPixelBayBG8 => PixelFormat::BayBG8,
            _IMV_EPixelType_gvspPixelBayGR10 => PixelFormat::BayGR10,
            _IMV_EPixelType_gvspPixelBayRG10 => PixelFormat::BayRG10,
            _IMV_EPixelType_gvspPixelBayGB10 => PixelFormat::BayGB10,
            _IMV_EPixelType_gvspPixelBayBG10 => PixelFormat::BayBG10,
            _IMV_EPixelType_gvspPixelBayGR12 => PixelFormat::BayGR12,
            _IMV_EPixelType_gvspPixelBayRG12 => PixelFormat::BayRG12,
            _IMV_EPixelType_gvspPixelBayGB12 => PixelFormat::BayGB12,
            _IMV_EPixelType_gvspPixelBayBG12 => PixelFormat::BayBG12,
            _IMV_EPixelType_gvspPixelBayGR10Packed => PixelFormat::BayGR10Packed,
            _IMV_EPixelType_gvspPixelBayRG10Packed => PixelFormat::BayRG10Packed,
            _IMV_EPixelType_gvspPixelBayGB10Packed => PixelFormat::BayGB10Packed,
            _IMV_EPixelType_gvspPixelBayBG10Packed => PixelFormat::BayBG10Packed,
            _IMV_EPixelType_gvspPixelBayGR12Packed => PixelFormat::BayGR12Packed,
            _IMV_EPixelType_gvspPixelBayRG12Packed => PixelFormat::BayRG12Packed,
            _IMV_EPixelType_gvspPixelBayGB12Packed => PixelFormat::BayGB12Packed,
            _IMV_EPixelType_gvspPixelBayBG12Packed => PixelFormat::BayBG12Packed,
            _IMV_EPixelType_gvspPixelBayGR16 => PixelFormat::BayGR16,
            _IMV_EPixelType_gvspPixelBayRG16 => PixelFormat::BayRG16,
            _IMV_EPixelType_gvspPixelBayGB16 => PixelFormat::BayGB16,
            _IMV_EPixelType_gvspPixelBayBG16 => PixelFormat::BayBG16,
            _IMV_EPixelType_gvspPixelRGB8 => PixelFormat::RGB8,
            _IMV_EPixelType_gvspPixelBGR8 => PixelFormat::BGR8,
            _IMV_EPixelType_gvspPixelRGBA8 => PixelFormat::RGBA8,
            _IMV_EPixelType_gvspPixelBGRA8 => PixelFormat::BGRA8,
            _IMV_EPixelType_gvspPixelRGB10 => PixelFormat::RGB10,
            _IMV_EPixelType_gvspPixelBGR10 => PixelFormat::BGR10,
            _IMV_EPixelType_gvspPixelRGB12 => PixelFormat::RGB12,
            _IMV_EPixelType_gvspPixelBGR12 => PixelFormat::BGR12,
            _IMV_EPixelType_gvspPixelRGB16 => PixelFormat::RGB16,
            _IMV_EPixelType_gvspPixelRGB10V1Packed => PixelFormat::RGB10V1Packed,
            _IMV_EPixelType_gvspPixelRGB10P32 => PixelFormat::RGB10P32,
            _IMV_EPixelType_gvspPixelRGB12V1Packed => PixelFormat::RGB12V1Packed,
            _IMV_EPixelType_gvspPixelRGB565P => PixelFormat::RGB565P,
            _IMV_EPixelType_gvspPixelBGR565P => PixelFormat::BGR565P,
            _IMV_EPixelType_gvspPixelYUV411_8_UYYVYY => PixelFormat::YUV4118UYYVYY,
            _IMV_EPixelType_gvspPixelYUV422_8_UYVY => PixelFormat::YUV4228UYVY,
            _IMV_EPixelType_gvspPixelYUV422_8 => PixelFormat::YUV4228,
            _IMV_EPixelType_gvspPixelYUV8_UYV => PixelFormat::YUV8UYV,
            _IMV_EPixelType_gvspPixelYCbCr8CbYCr => PixelFormat::YCbCr8CbYCr,
            _IMV_EPixelType_gvspPixelYCbCr422_8 => PixelFormat::YCbCr4228,
            _IMV_EPixelType_gvspPixelYCbCr422_8_CbYCrY => PixelFormat::YCbCr4228CbYCrY,
            _IMV_EPixelType_gvspPixelYCbCr411_8_CbYYCrYY => PixelFormat::YCbCr4118CbYYCrYY,
            _IMV_EPixelType_gvspPixelYCbCr601_8_CbYCr => PixelFormat::YCbCr6018CbYCr,
            _IMV_EPixelType_gvspPixelYCbCr601_422_8 => PixelFormat::YCbCr6014228,
            _IMV_EPixelType_gvspPixelYCbCr601_422_8_CbYCrY => PixelFormat::YCbCr6014228CbYCrY,
            _IMV_EPixelType_gvspPixelYCbCr601_411_8_CbYYCrYY => PixelFormat::YCbCr6014118CbYYCrYY,
            _IMV_EPixelType_gvspPixelYCbCr709_8_CbYCr => PixelFormat::YCbCr7098CbYCr,
            _IMV_EPixelType_gvspPixelYCbCr709_422_8 => PixelFormat::YCbCr7094228,
            _IMV_EPixelType_gvspPixelYCbCr709_422_8_CbYCrY => PixelFormat::YCbCr7094228CbYCrY,
            _IMV_EPixelType_gvspPixelYCbCr709_411_8_CbYYCrYY => PixelFormat::YCbCr7094118CbYYCrYY,
            _IMV_EPixelType_gvspPixelYUV420SP_NV12 => PixelFormat::YUV420SPNV12,
            _IMV_EPixelType_gvspPixelRGB8Planar => PixelFormat::RGB8Planar,
            _IMV_EPixelType_gvspPixelRGB10Planar => PixelFormat::RGB10Planar,
            _IMV_EPixelType_gvspPixelRGB12Planar => PixelFormat::RGB12Planar,
            _IMV_EPixelType_gvspPixelRGB16Planar => PixelFormat::RGB16Planar,
            _IMV_EPixelType_gvspPixelBayRG10p => PixelFormat::BayRG10p,
            _IMV_EPixelType_gvspPixelBayRG12p => PixelFormat::BayRG12p,
            _IMV_EPixelType_gvspPixelMono1c => PixelFormat::Mono1c,
            _IMV_EPixelType_gvspPixelMono1e => PixelFormat::Mono1e,
            _ => PixelFormat::Undefined,
        }
    }
}

#[cfg(test)]
#[cfg(feature = "industry-camera")]
mod tests {
    use super::get_camera_list;

    #[tokio::test]
    async fn test_enumerate_camera_list() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .init();

        let list = get_camera_list();
        assert!(list.is_ok());
        tracing::info!("list size: {:?}", list);
    }
}
