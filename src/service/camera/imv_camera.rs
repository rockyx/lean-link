#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use std::ffi::{CStr, CString};
use std::os::raw::c_void;
use std::ptr::null_mut;

use bytes::BufMut;
use tokio::sync::mpsc;

use crate::errors::Error;
use crate::ffi::imv::*;
use crate::service::camera::{
    CameraFrame, CameraInfo, FrameSize, GrabMode, IndustryCamera, PixelFormat,
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

    pub fn build(&self) -> Result<IMVCamera, Error> {
        IMVCamera::get_camera_list()?;
        let mut camera = IMVCamera {
            handle: null_mut(),
            grab_mode: GrabMode::Continuous,
            exposure_auto: false,
            exposure_time: std::time::Duration::from_millis(1000),
            frame_sender: None,
        };

        match self.mode {
            _IMV_ECreateHandleMode_modeByIndex => unsafe {
                let ret =
                    IMV_CreateHandle(&mut camera.handle, self.mode, self.index as *mut c_void);
                if ret != IMV_OK {
                    return Err(Error::Camera(ret));
                }
            },
            _IMV_ECreateHandleMode_modeByCameraKey => {
                let c_string = CString::new(self.camera_key.as_str())?;
                let ptr_c_void = c_string.as_ptr() as *mut c_void;

                unsafe {
                    let ret = IMV_CreateHandle(&mut camera.handle, self.mode, ptr_c_void);
                    if ret != IMV_OK {
                        return Err(Error::Camera(ret));
                    }
                }
            }
            _IMV_ECreateHandleMode_modeByDeviceUserID => {
                let c_string = CString::new(self.device_user_id.as_str())?;
                let ptr_c_void = c_string.as_ptr() as *mut c_void;

                unsafe {
                    let ret = IMV_CreateHandle(&mut camera.handle, self.mode, ptr_c_void);
                    if ret != IMV_OK {
                        return Err(Error::Camera(ret));
                    }
                }
            }
            _IMV_ECreateHandleMode_modeByIPAddress => {
                let c_string = CString::new(self.ip_address.as_str())?;
                let ptr_c_void = c_string.as_ptr() as *mut c_void;

                unsafe {
                    let ret = IMV_CreateHandle(&mut camera.handle, self.mode, ptr_c_void);
                    if ret != IMV_OK {
                        return Err(Error::Camera(ret));
                    }
                }
            }
            _ => {
                return Err(Error::Camera(IMV_ERROR));
            }
        }
        Ok(camera)
    }
}

#[repr(C)]
pub struct IMVCamera {
    handle: IMV_HANDLE,
    pub grab_mode: GrabMode,
    pub exposure_auto: bool,
    pub exposure_time: std::time::Duration,
    pub frame_sender: Option<mpsc::Sender<CameraFrame>>,
}

impl IndustryCamera for IMVCamera {
    fn get_camera_list() -> Result<Vec<CameraInfo>, Error> {
        let mut camera_list = Vec::new();

        unsafe {
            let mut device_info_list = IMV_DeviceList::default();
            let ret = IMV_EnumDevices(
                &mut device_info_list,
                _IMV_EInterfaceType_interfaceTypeAll as u32,
            );

            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }

            for i in 0..device_info_list.nDevNum {
                let device_info = *device_info_list.pDevInfo.add(i as usize);
                camera_list.push(CameraInfo {
                    key: CStr::from_ptr(device_info.cameraKey.as_ptr())
                        .to_string_lossy()
                        .into_owned(),
                    name: CStr::from_ptr(device_info.cameraName.as_ptr())
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
                });
            }
        }

        Ok(camera_list)
    }

    fn open(&self) -> Result<(), Error> {
        if self.is_opened() {
            return Ok(());
        }

        unsafe {
            let ret = IMV_Open(self.handle);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
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
            if ret == 0 {
                return false;
            }

            return true;
        }
    }

    fn is_grabbing(&self) -> bool {
        if !self.is_opened() {
            return false;
        }
        unsafe {
            let ret = IMV_IsGrabbing(self.handle);
            if ret == 0 {
                return false;
            }
        }

        true
    }

    fn stop_grab(&self) -> Result<(), Error> {
        if !self.is_opened() {
            return Ok(());
        }

        unsafe {
            let ret = IMV_IsGrabbing(self.handle);
            if ret == 0 {
                return Ok(());
            }
        }

        unsafe {
            let ret = IMV_StopGrabbing(self.handle);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }

            let _ = IMV_ClearFrameBuffer(self.handle);
        }
        Ok(())
    }

    fn start_grab(&mut self) -> Result<(), Error> {
        if !self.is_opened() {
            return Err(Error::Camera(IMV_ERROR));
        }

        if self.is_grabbing() {
            return Ok(());
        }

        self.sync_grab_mode()?;

        let _ = self.sync_exposure_auto();
        let _ = self.sync_exposure_time();

        if self.grab_mode == GrabMode::Continuous {
            unsafe {
                let ret = IMV_AttachGrabbing(
                    self.handle,
                    Some(frame_callback),
                    self as *mut Self as *mut c_void,
                );
                if ret != IMV_OK {
                    return Err(Error::Camera(ret));
                }
            }
        }

        unsafe {
            let ret = IMV_StartGrabbing(self.handle);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }
        }

        Ok(())
    }

    fn close(&self) -> Result<(), Error> {
        if !self.is_opened() {
            return Ok(());
        }

        unsafe {
            let ret = IMV_Close(self.handle);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }
        }

        Ok(())
    }

    fn frame_size(&self) -> Result<FrameSize, Error> {
        let width = self.get_int_feature_value("Width")? as usize;
        let height = self.get_int_feature_value("Height")? as usize;

        Ok(FrameSize { width, height })
    }

    fn trigger_one_frame(&self) -> Result<(), Error> {
        unsafe {
            // Clear frame buffer
            let ret = IMV_ClearFrameBuffer(self.handle);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }
        }

        // Execute soft trigger once
        self.execute_command_feature("TriggerSoftware")?;

        let mut try_counter = Some(0);

        while let Some(counter) = try_counter {
            if counter > 3 {
                try_counter = None;
            } else {
                try_counter = Some(counter + 1);
            }

            let mut frame = IMV_Frame::default();
            unsafe {
                let ret = IMV_GetFrame(self.handle, &mut frame, 1000);
                if ret != IMV_OK {
                    continue;
                }
            }

            self.handle_frame(frame.into());
        }

        Ok(())
    }

    fn create_frame_channel(&mut self) -> mpsc::Receiver<CameraFrame> {
        let (sender, receiver) = mpsc::channel(1024);
        self.frame_sender = Some(sender);
        receiver
    }

    fn set_grab_mode(&mut self, grab_mode: GrabMode) {
        self.grab_mode = grab_mode;
    }

    fn set_exposure_auto(&mut self, auto: bool) {
        self.exposure_auto = auto;
    }

    fn set_exposure_time(&mut self, time: std::time::Duration) {
        self.exposure_time = time;
    }
}

impl IMVCamera {
    fn set_enum_feature_symbol(&self, feature_name: &str, enum_symbol: &str) -> Result<(), Error> {
        let feature_name_ptr = feature_name.as_ptr() as *const i8;
        let enum_symbol_ptr = enum_symbol.as_ptr() as *const i8;

        unsafe {
            let ret = IMV_SetEnumFeatureSymbol(self.handle, feature_name_ptr, enum_symbol_ptr);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }
        }

        Ok(())
    }

    fn sync_grab_mode(&self) -> Result<(), Error> {
        // Set trigger selector to FrameStart
        self.set_enum_feature_symbol("TriggerSelector", "FrameStart")?;

        match self.grab_mode {
            GrabMode::Continuous => {
                // Set trigger mode to Off
                self.set_enum_feature_symbol("TriggerMode", "Off")?;
            }
            GrabMode::SingleFrame => {
                // Set trigger source to Software
                self.set_enum_feature_symbol("TriggerSource", "Software")?;
                self.set_enum_feature_symbol("TriggerMode", "On")?;
            }
        }

        Ok(())
    }

    fn set_enum_feature_value(&self, feature_name: &str, enum_value: u64) -> Result<(), Error> {
        unsafe {
            let feature_name_ptr = feature_name.as_ptr() as *const i8;
            let ret = IMV_SetEnumFeatureValue(self.handle, feature_name_ptr, enum_value);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }
        }

        Ok(())
    }

    fn sync_exposure_auto(&self) -> Result<(), Error> {
        if self.exposure_auto {
            self.set_enum_feature_value("ExposureAuto", 2)?;
        } else {
            self.set_enum_feature_value("ExposureAuto", 0)?;
        }
        Ok(())
    }

    fn get_double_feature_value(&self, feature_name: &str) -> Result<f64, Error> {
        unsafe {
            let feature_name_ptr = feature_name.as_ptr() as *const i8;
            let mut value = 0.0;
            let ret = IMV_GetDoubleFeatureValue(self.handle, feature_name_ptr, &mut value);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }

            Ok(value)
        }
    }

    fn get_double_feature_min(&self, feature_name: &str) -> Result<f64, Error> {
        unsafe {
            let feature_name_ptr = feature_name.as_ptr() as *const i8;
            let mut value = 0.0;
            let ret = IMV_GetDoubleFeatureMin(self.handle, feature_name_ptr, &mut value);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }

            Ok(value)
        }
    }

    fn get_double_feature_max(&self, feature_name: &str) -> Result<f64, Error> {
        unsafe {
            let feature_name_ptr = feature_name.as_ptr() as *const i8;
            let mut value = 0.0;
            let ret = IMV_GetDoubleFeatureMax(self.handle, feature_name_ptr, &mut value);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }

            Ok(value)
        }
    }

    fn set_double_feature_value(&self, feature_name: &str, double_value: f64) -> Result<(), Error> {
        unsafe {
            let feature_name_ptr = feature_name.as_ptr() as *const i8;
            let ret = IMV_SetDoubleFeatureValue(self.handle, feature_name_ptr, double_value);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }

            Ok(())
        }
    }

    fn sync_exposure_time(&self) -> Result<(), Error> {
        let mut et = self.get_double_feature_value("ExposureTime")?;
        let exposure_min_value = self.get_double_feature_min("ExposureTime")?;
        let exposure_max_value = self.get_double_feature_max("ExposureTime")?;

        tracing::debug!(
            "et: {:?}, exposure_min_value: {:?}, exposure_max_value: {:?}",
            et,
            exposure_min_value,
            exposure_max_value
        );
        
        et = self.exposure_time.as_millis() as f64;
        if et < exposure_min_value {
            et = exposure_min_value;
        } else if et > exposure_max_value {
            et = exposure_max_value;
        }
        self.set_double_feature_value("ExposureTime", et)
    }

    fn handle_frame(&self, frame: CameraFrame) {
        let frame_sender = self.frame_sender.clone();
        if frame_sender.is_none() {
            return;
        }

        let frame_sender = frame_sender.unwrap();
        tokio::spawn(async move {
            let result = frame_sender.send(frame).await;
            if result.is_err() {
                tracing::error!("send frame to channel failed: {:?}", result.unwrap_err());
            }
        });
    }

    fn get_int_feature_value(&self, feature_name: &str) -> Result<i64, Error> {
        unsafe {
            let feature_name_ptr = feature_name.as_ptr() as *const i8;
            let mut value: i64 = 0;
            let ret = IMV_GetIntFeatureValue(self.handle, feature_name_ptr, &mut value);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }

            Ok(value)
        }
    }

    fn execute_command_feature(&self, feature_name: &str) -> Result<(), Error> {
        unsafe {
            let feature_name_ptr = feature_name.as_ptr() as *const i8;
            let ret = IMV_ExecuteCommandFeature(self.handle, feature_name_ptr);
            if ret != IMV_OK {
                return Err(Error::Camera(ret));
            }

            Ok(())
        }
    }
}

impl Drop for IMVCamera {
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

    let camera = unsafe { &mut *(user_ptr as *mut IMVCamera) };

    unsafe {
        let frame = frame_ptr.as_ref();
        if frame.is_none() {
            return;
        }
        let frame = frame.unwrap();
        camera.handle_frame((*frame).into());

        IMV_ReleaseFrame(camera.handle, frame_ptr);
    }
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

impl Into<CameraFrame> for IMV_Frame {
    fn into(self) -> CameraFrame {
        let frame_info = self.frameInfo;
        let mut data = bytes::BytesMut::with_capacity(frame_info.size as usize);
        unsafe {
            for i in 0..frame_info.size as usize {
                data.put_u8(self.pData.add(i) as u8);
            }
        }
        let frame = CameraFrame {
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
        };

        frame
    }
}
