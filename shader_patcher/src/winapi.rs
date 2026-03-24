use std::ffi::c_void;

use winapi::um::memoryapi::VirtualProtectEx;
use winsafe::{
    DisabPriv, GetLastError, HPROCESS, HPROCESSLIST, LUID_AND_ATTRIBUTES, LookupPrivilegeValue,
    MEMORY_BASIC_INFORMATION, SysResult, TOKEN_PRIVILEGES,
    co::{PAGE, PROCESS, SE_PRIV, SE_PRIV_ATTR, TH32CS, TOKEN},
};

use crate::error::{Error, Result};

pub fn grant_debug_privileges() -> SysResult<()> {
    let this_proc = HPROCESS::GetCurrentProcess();
    let htoken = this_proc.OpenProcessToken(TOKEN::ADJUST_PRIVILEGES | TOKEN::QUERY)?;
    let privelege = LUID_AND_ATTRIBUTES::new(
        LookupPrivilegeValue(None, SE_PRIV::DEBUG_NAME)?,
        SE_PRIV_ATTR::ENABLED,
    );
    let privs = TOKEN_PRIVILEGES::new(&[privelege])?;
    htoken.AdjustTokenPrivileges(DisabPriv::Privs(&privs))
}

pub fn pid_by_name(name: &str) -> Result<u32> {
    for mp in HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPPROCESS, None)?.iter_processes() {
        if let Ok(p) = mp
            && (p.szExeFile() == name)
        {
            return Ok(p.th32ProcessID);
        }
    }

    Err(Error::ProcessNotFound(name.into()))
}

pub fn kill_process_by_name(name: &str) -> Result<u32> {
    let pid = pid_by_name(name)?;
    let process = HPROCESS::OpenProcess(PROCESS::TERMINATE, false, pid)?;
    process.TerminateProcess(0)?;
    Ok(pid)
}

pub fn wait_pid_by_name(name: &str) -> SysResult<u32> {
    loop {
        let mut snap = HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPPROCESS, None)?;
        for mp in snap.iter_processes() {
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
        let snap = HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPMODULE, Some(pid));
        if let Ok(mut snap) = snap {
            for mm in snap.iter_modules() {
                if let Ok(m) = mm
                    && (m.szModule() == name)
                {
                    return Ok((m.modBaseAddr, m.modBaseSize));
                }
            }
        }
    }
}

pub fn module_by_name_and_pid(name: &str, pid: u32) -> Result<(*mut c_void, u32)> {
    let mut snap = HPROCESSLIST::CreateToolhelp32Snapshot(TH32CS::SNAPMODULE, Some(pid))?;
    for mm in snap.iter_modules() {
        if let Ok(m) = mm
            && (m.szModule() == name)
        {
            return Ok((m.modBaseAddr, m.modBaseSize));
        }
    }

    Err(Error::ModuleNotFound(name.into(), pid))
}

pub fn set_memprotect(
    hprocess: &HPROCESS,
    mbi: &MEMORY_BASIC_INFORMATION,
    new_protect: PAGE,
) -> Result<PAGE> {
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
        _ => Ok(unsafe { PAGE::from_raw(*old_protect) }),
    }
}
