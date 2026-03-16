use std::ffi::c_void;

use winsafe::{
    DisabPriv, HPROCESS, HPROCESSLIST, LUID_AND_ATTRIBUTES, LookupPrivilegeValue, SysResult,
    TOKEN_PRIVILEGES,
    co::{ERROR, SE_PRIV, SE_PRIV_ATTR, TH32CS, TOKEN},
};

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
