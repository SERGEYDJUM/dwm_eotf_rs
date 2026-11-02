use windows::Win32::{
    Devices::Display::{DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_DEVICE_INFO_TYPE},
    Graphics::Gdi::DISPLAYCONFIG_COLOR_ENCODING,
};

#[repr(C)]
#[allow(nonstandard_style, unused)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
/// Source: wingdi.h
pub enum DISPLAYCONFIG_ADVANCED_COLOR_MODE {
    #[default]
    /// RGB888 composition, display-referred color, display-referred luminance
    Sdr,
    /// Advanced color (FP16 scRGB composition), scene-referred color, display-referred luminance
    Wcg,
    /// Advanced color (FP16 scRGB composition), scene-referred color, scene-referred luminance
    Hdr,
}

#[repr(C)]
#[allow(nonstandard_style)]
#[derive(Clone, Copy, Debug, Default)]
/// Source: wingdi.h
pub struct DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO_2 {
    pub header: DISPLAYCONFIG_DEVICE_INFO_HEADER,
    pub flags: u32,
    pub colorEncoding: DISPLAYCONFIG_COLOR_ENCODING,
    pub bitsPerColorChannel: u32,
    pub activeColorMode: DISPLAYCONFIG_ADVANCED_COLOR_MODE,
}

/// Source: variant of DISPLAYCONFIG_DEVICE_INFO_TYPE enum in wingdi.h
pub const DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO_2: DISPLAYCONFIG_DEVICE_INFO_TYPE =
    DISPLAYCONFIG_DEVICE_INFO_TYPE(15);
