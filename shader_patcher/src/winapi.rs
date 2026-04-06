#[link(name = "ntdll")]
unsafe extern "system" {
    pub unsafe fn NtSuspendProcess(ProcessHandle: *mut std::ffi::c_void) -> std::ffi::c_long;
    pub unsafe fn NtResumeProcess(ProcessHandle: *mut std::ffi::c_void) -> std::ffi::c_long;
}

use std::{ffi::c_void, ptr::null_mut};

use windows::Win32::{
    Foundation::HANDLE,
    System::Memory::{PAGE_PROTECTION_FLAGS, VirtualProtectEx},
};
use winsafe::{
    DisabPriv, HPROCESS, HPROCESSLIST, LUID_AND_ATTRIBUTES, LookupPrivilegeValue,
    MEMORY_BASIC_INFORMATION, TOKEN_PRIVILEGES,
    co::{PROCESS, SE_PRIV, SE_PRIV_ATTR, TH32CS, TOKEN},
};

use crate::error::{Error, Result};

pub fn suspend_process(hprocess: &HPROCESS) -> Result<()> {
    match unsafe { NtSuspendProcess(hprocess.ptr()) } {
        0 => Ok(()),
        ntstatus => Err(Error::NtApi(ntstatus)),
    }
}

pub fn resume_process(hprocess: &HPROCESS) -> Result<()> {
    match unsafe { NtResumeProcess(hprocess.ptr()) } {
        0 => Ok(()),
        ntstatus => Err(Error::NtApi(ntstatus)),
    }
}

pub fn obtain_debug_privileges() -> Result<()> {
    let this_proc = HPROCESS::GetCurrentProcess();
    let htoken = this_proc.OpenProcessToken(TOKEN::ADJUST_PRIVILEGES | TOKEN::QUERY)?;
    let privelege = LUID_AND_ATTRIBUTES::new(
        LookupPrivilegeValue(None, SE_PRIV::DEBUG_NAME)?,
        SE_PRIV_ATTR::ENABLED,
    );
    let privs = TOKEN_PRIVILEGES::new(&[privelege])?;
    Ok(htoken.AdjustTokenPrivileges(DisabPriv::Privs(&privs))?)
}

pub fn wait_pid_by_name(name: &str) -> Result<u32> {
    loop {
        match pid_by_name(name) {
            Ok(r) => return Ok(r),
            _ => continue,
        }
    }
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

pub fn wait_module_by_name_and_pid(name: &str, pid: u32) -> Result<(*mut c_void, u32)> {
    loop {
        match module_by_name_and_pid(name, pid) {
            Ok(r) => return Ok(r),
            _ => continue,
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
    new_protect: PAGE_PROTECTION_FLAGS,
) -> Result<PAGE_PROTECTION_FLAGS> {
    let old_protect = null_mut();

    unsafe {
        VirtualProtectEx(
            HANDLE(hprocess.ptr()),
            mbi.BaseAddress,
            mbi.RegionSize,
            new_protect,
            old_protect,
        )
    }?;

    Ok(unsafe { *old_protect })
}
