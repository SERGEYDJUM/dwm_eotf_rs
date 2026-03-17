use std::ffi::c_void;

use bytemuck::{Pod, Zeroable};
use winsafe::{
    DisabPriv, HPROCESS, HPROCESSLIST, LUID_AND_ATTRIBUTES, LookupPrivilegeValue, SysResult,
    TOKEN_PRIVILEGES,
    co::{ERROR, SE_PRIV, SE_PRIV_ATTR, TH32CS, TOKEN},
};

// pub static shader_hashes: [[u8; 16]; 4] = [
//     [
//         0x96, 0xe6, 0xd1, 0x58, 0x92, 0x55, 0xec, 0xcd, 0x1d, 0xd7, 0xd4, 0xdb, 0xec, 0x54, 0xd2,
//         0x85,
//     ],
//     [
//         0x21, 0x26, 0xb0, 0x37, 0xc1, 0xa2, 0xfb, 0xdd, 0xe3, 0x55, 0xb6, 0xe6, 0xdd, 0x9c, 0xaf,
//         0x3c,
//     ],
//     [
//         0x2c, 0x89, 0x26, 0xff, 0xe2, 0x29, 0xf0, 0x5d, 0x96, 0x7c, 0x72, 0x66, 0x8d, 0xc3, 0xad,
//         0xdb,
//     ],
//     [
//         0xf6, 0x93, 0xbf, 0xbb, 0xaf, 0x24, 0xb3, 0xd9, 0x36, 0x63, 0x54, 0xbe, 0x88, 0x98, 0xa7,
//         0xf5,
//     ],
// ];

#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct DXContainerHeader {
    pub magic: [u8; 4],
    pub digest: [u8; 16],
    pub major_version: u16,
    pub minor_version: u16,
    pub file_size: u32,
    pub part_count: u32,
}

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
