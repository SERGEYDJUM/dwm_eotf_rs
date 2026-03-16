pub mod utils;

use std::ffi::c_void;

use ntapi::ntpsapi::{NtResumeProcess, NtSuspendProcess};
use tracing::debug;
use winsafe::{
    HPROCESS, SysResult,
    co::{ERROR, PROCESS},
    guard::CloseHandleGuard,
};

use crate::utils::{module_by_name_and_pid, pid_by_name};

pub struct DwmProcess {
    hprocess: CloseHandleGuard<HPROCESS>,
    dwmcore_addr: *mut c_void,
    dwmcore_size: u32,
}

impl DwmProcess {
    pub fn open() -> SysResult<Self> {
        let pid = pid_by_name("dwm.exe")?;

        debug!("Found DWM process with PID {}", pid);

        let (dwmcore_addr, dwmcore_size) = module_by_name_and_pid("dwmcore.dll", pid)?;

        debug!(
            "Found DWM Core module at {:x} with size {}",
            dwmcore_addr as usize, dwmcore_size
        );

        Ok(Self {
            hprocess: HPROCESS::OpenProcess(PROCESS::ALL_ACCESS, false, pid)?,
            dwmcore_addr,
            dwmcore_size,
        })
    }

    pub fn kill(&self) -> SysResult<()> {
        self.hprocess.TerminateProcess(0)
    }

    pub fn suspend_process(&self) -> SysResult<()> {
        match unsafe { NtSuspendProcess(self.hprocess.ptr()) } {
            0 => Ok(()),
            _ => Err(ERROR::NOT_FOUND),
        }
    }

    pub fn resume_process(&self) -> SysResult<()> {
        match unsafe { NtResumeProcess(self.hprocess.ptr()) } {
            0 => Ok(()),
            _ => Err(ERROR::NOT_FOUND),
        }
    }
}
