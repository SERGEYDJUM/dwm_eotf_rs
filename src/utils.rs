use std::io::Write;
use std::{ffi::c_void, fs};

use bytemuck::checked::from_bytes;
use tracing::debug;
use winapi::um::memoryapi::VirtualProtectEx;
use winsafe::co::{PAGE, PROCESS};
use winsafe::{
    DisabPriv, HPROCESS, HPROCESSLIST, LUID_AND_ATTRIBUTES, LookupPrivilegeValue, SysResult,
    TOKEN_PRIVILEGES,
    co::{ERROR, SE_PRIV, SE_PRIV_ATTR, TH32CS, TOKEN},
};
use winsafe::{GetLastError, MEMORY_BASIC_INFORMATION};

use crate::error::{Error, Result};
use crate::shaders::DXContainerHeader;

pub fn grant_debug_privileges() -> SysResult<()> {
    let htoken =
        HPROCESS::GetCurrentProcess().OpenProcessToken(TOKEN::ADJUST_PRIVILEGES | TOKEN::QUERY)?;

    let privelege = LUID_AND_ATTRIBUTES::new(
        LookupPrivilegeValue(None, SE_PRIV::DEBUG_NAME)?,
        SE_PRIV_ATTR::ENABLED,
    );

    let privs = TOKEN_PRIVILEGES::new(&[privelege])?;
    htoken.AdjustTokenPrivileges(DisabPriv::Privs(&privs))
}

pub fn pid_by_name(name: &str) -> SysResult<u32> {
    for mp in HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPPROCESS, None)?.iter_processes() {
        if let Ok(p) = mp
            && (p.szExeFile() == name)
        {
            return Ok(p.th32ProcessID);
        }
    }

    Err(ERROR::NOT_FOUND)
}

pub fn kill_process_by_name(name: &str) -> SysResult<u32> {
    let pid = pid_by_name(name)?;
    debug!("Killing process with PID {}", pid);
    let process = HPROCESS::OpenProcess(PROCESS::TERMINATE, false, pid)?;
    process.TerminateProcess(0)?;
    Ok(pid)
}

pub fn wait_pid_by_name(name: &str) -> SysResult<u32> {
    loop {
        for mp in
            HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPPROCESS, None)?.iter_processes()
        {
            if let Ok(p) = mp
                && (p.szExeFile() == name)
            {
                return Ok(p.th32ProcessID);
            }
        }
    }
}

pub fn wait_module_by_name_and_pid(name: &str, pid: u32) -> SysResult<(*mut c_void, u32)> {
    loop {
        if let Ok(mut snapshot) =
            HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPMODULE, Some(pid))
        {
            for mm in snapshot.iter_modules() {
                if let Ok(m) = mm
                    && (m.szModule() == name)
                {
                    return Ok((m.modBaseAddr, m.modBaseSize));
                }
            }
        }
    }
}

pub fn module_by_name_and_pid(name: &str, pid: u32) -> SysResult<(*mut c_void, u32)> {
    for mm in HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPMODULE, Some(pid))?.iter_modules()
    {
        if let Ok(m) = mm
            && (m.szModule() == name)
        {
            return Ok((m.modBaseAddr, m.modBaseSize));
        }
    }

    Err(ERROR::NOT_FOUND)
}

pub fn dump_shaders(bytes: &[u8]) -> Result<usize> {
    fs::create_dir_all("shaders/dumped")?;

    let mut shaders_dumped = 0;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'D' && bytes[i + 1] == b'X' && bytes[i + 2] == b'B' && bytes[i + 3] == b'C'
        {
            let header_size = size_of::<DXContainerHeader>();
            let header: &DXContainerHeader = from_bytes(&bytes[i..(i + header_size)]);
            let file_size = header.file_size as usize;

            let hash: u128 = *from_bytes(&header.digest.clone());
            debug!("Dumping shader with hash `{:X}`", &hash);

            fs::File::create(format!("shaders/dumped/{:X}.dxbc", hash))?
                .write_all(&bytes[i..(i + file_size)])?;

            shaders_dumped += 1;
            i += file_size;
        } else {
            i += 1;
        }
    }

    Ok(shaders_dumped)
}

pub fn set_memprotect(
    hprocess: &HPROCESS,
    mbi: &MEMORY_BASIC_INFORMATION,
    new_protect: PAGE,
) -> Result<()> {
    let old_protect = &mut 0;

    let ret_stat = unsafe {
        VirtualProtectEx(
            hprocess.ptr(),
            mbi.BaseAddress,
            mbi.RegionSize,
            new_protect.raw(),
            old_protect,
        )
    };

    match ret_stat {
        0 => Err(Error::WinSafe(GetLastError())),
        _ => Ok(()),
    }
}

pub fn calculate_checksum(data: &[u8]) -> u128 {
    unsafe extern "C" {
        unsafe fn CalculateDXBCChecksum(
            pData: *const u8,
            dwSize: u32,
            dwHash: &mut [u32; 4],
        ) -> bool;
    }

    let mut digest = [0u32; 4];
    unsafe {
        CalculateDXBCChecksum(data.as_ptr(), data.len() as u32, &mut digest);
    }
    let digest: &[u8] = bytemuck::cast_slice(&digest);
    *bytemuck::from_bytes(digest)
}
