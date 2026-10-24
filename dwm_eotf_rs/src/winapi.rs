use anyhow::{Result, anyhow};
use tracing::info;
use windows::Win32::{
    Devices::Display::{
        DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
        DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
        DISPLAYCONFIG_DEVICE_INFO_HEADER, DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO,
        DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SDR_WHITE_LEVEL,
        DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
        QDC_ONLY_ACTIVE_PATHS, QueryDisplayConfig,
    },
    Foundation::ERROR_SUCCESS,
};

fn get_active_displays_paths() -> Result<Vec<DISPLAYCONFIG_PATH_INFO>> {
    let flags = QDC_ONLY_ACTIVE_PATHS;
    let (mut paths_len, mut modes_len) = (0, 0);

    if let status = unsafe { GetDisplayConfigBufferSizes(flags, &mut paths_len, &mut modes_len) }
        && status != ERROR_SUCCESS
    {
        return Err(anyhow!(
            "GetDisplayConfigBufferSizes failed with code {:x}",
            status.0
        ));
    }

    let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); paths_len as usize];
    let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); modes_len as usize];

    if let status = unsafe {
        QueryDisplayConfig(
            flags,
            &mut paths_len,
            paths.as_mut_ptr(),
            &mut modes_len,
            modes.as_mut_ptr(),
            None,
        )
    } && status != ERROR_SUCCESS
    {
        return Err(anyhow!(
            "QueryDisplayConfig failed with code {:x}",
            status.0
        ));
    }

    paths.truncate(paths_len as usize);
    Ok(paths)
}

fn is_display_path_sdr(path: &DISPLAYCONFIG_PATH_INFO) -> Result<bool> {
    let mut packet = DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_ADVANCED_COLOR_INFO,
            size: size_of::<DISPLAYCONFIG_GET_ADVANCED_COLOR_INFO>() as u32,
            adapterId: path.targetInfo.adapterId,
            id: path.targetInfo.id,
        },
        ..Default::default()
    };

    if let status = unsafe { DisplayConfigGetDeviceInfo(&mut packet.header) }
        && status != 0
    {
        return Err(anyhow!(
            "DisplayConfigGetDeviceInfo(GET_ADVANCED_COLOR_INFO) failed with code {:x}",
            status
        ));
    }

    // TODO: somehow include WCG displays
    Ok(packet.bitsPerColorChannel == 8)
}

fn get_display_path_name(path: &DISPLAYCONFIG_PATH_INFO) -> Result<String> {
    let mut packet = DISPLAYCONFIG_TARGET_DEVICE_NAME {
        header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
            r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
            size: size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
            adapterId: path.targetInfo.adapterId,
            id: path.targetInfo.id,
        },
        ..Default::default()
    };

    if let status = unsafe { DisplayConfigGetDeviceInfo(&mut packet.header) }
        && status != 0
    {
        return Err(anyhow!(
            "DisplayConfigGetDeviceInfo(TARGET_DEVICE_NAME) failed with code {:x}",
            status
        ));
    }

    Ok(String::from_utf16_lossy(&packet.monitorFriendlyDeviceName))
}

pub fn get_sdr_white_level() -> Result<Option<f32>> {
    let mut white_level = 0;

    for path in get_active_displays_paths()? {
        if is_display_path_sdr(&path)? {
            continue;
        }

        let mut packet = DISPLAYCONFIG_SDR_WHITE_LEVEL {
            header: DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_SDR_WHITE_LEVEL,
                size: size_of::<DISPLAYCONFIG_SDR_WHITE_LEVEL>() as u32,
                adapterId: path.targetInfo.adapterId,
                id: path.targetInfo.id,
            },
            SDRWhiteLevel: 0,
        };

        if let status = unsafe { DisplayConfigGetDeviceInfo(&mut packet.header) }
            && status != 0
        {
            return Err(anyhow!(
                "DisplayConfigGetDeviceInfo(SDR_WHITE_LEVEL) failed with code {:x}",
                status
            ));
        }

        info!(
            "Found display `{}` with SDRWhiteLevel = {} nits ",
            get_display_path_name(&path)?,
            packet.SDRWhiteLevel as f32 * 0.08
        );

        white_level = white_level.max(packet.SDRWhiteLevel);
    }

    if white_level == 0 {
        return Ok(None);
    }

    // SDRWhiteLevel is stored in units of 1/1000 of 80 nits
    Ok(Some((white_level as f32) * 0.08))
}
