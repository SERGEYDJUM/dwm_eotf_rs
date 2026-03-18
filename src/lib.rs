pub mod error;
pub mod utils;

use std::ffi::c_void;
use std::iter::repeat_n;

use ntapi::ntpsapi::{NtResumeProcess, NtSuspendProcess};
use tracing::debug;
use winsafe::MEMORY_BASIC_INFORMATION;
use winsafe::co::{MEM_STATE, PAGE};
use winsafe::{HPROCESS, co::PROCESS, guard::CloseHandleGuard};

use crate::error::{Error, Result};
use crate::utils::{
    kill_process_by_name, pid_by_name, wait_module_by_name_and_pid, wait_pid_by_name,
};

pub struct DwmProcess {
    hprocess: CloseHandleGuard<HPROCESS>,
    dwmcore_addr: *mut c_void,
    dwmcore_size: u32,
}

const DWM_EXE: &str = "dwm.exe";
const DWM_DLL: &str = "dwmcore.dll";

impl DwmProcess {
    pub fn open() -> Result<Self> {
        let pid = pid_by_name(DWM_EXE)?;
        Self::load(pid, DWM_DLL)
    }

    pub fn open_wait() -> Result<Self> {
        let pid = wait_pid_by_name(DWM_EXE)?;
        Self::load(pid, DWM_DLL)
    }

    pub fn open_restarted() -> Result<Self> {
        let killed_pid = kill_process_by_name(DWM_EXE)?;
        let mut new_pid = killed_pid;

        while killed_pid == new_pid {
            new_pid = wait_pid_by_name(DWM_EXE)?;
        }

        Self::load(new_pid, DWM_DLL)
    }

    pub fn load(pid: u32, name: &str) -> Result<Self> {
        let (dwmcore_addr, dwmcore_size) = wait_module_by_name_and_pid(name, pid)?;

        Ok(Self {
            hprocess: HPROCESS::OpenProcess(PROCESS::ALL_ACCESS, false, pid)?,
            dwmcore_addr,
            dwmcore_size,
        })
    }

    pub fn kill(&self) -> Result<()> {
        Ok(self.hprocess.TerminateProcess(0)?)
    }

    pub fn suspend_process(&self) -> Result<()> {
        let ntstatus = unsafe { NtSuspendProcess(self.hprocess.ptr()) };
        if ntstatus != 0 {
            return Err(Error::NtApi(ntstatus));
        }
        Ok(())
    }

    pub fn resume_process(&self) -> Result<()> {
        let ntstatus = unsafe { NtResumeProcess(self.hprocess.ptr()) };
        if ntstatus != 0 {
            return Err(Error::NtApi(ntstatus));
        }
        Ok(())
    }

    pub fn dwmcore_mempage_info(&self, addr: *mut c_void) -> Result<MEMORY_BASIC_INFORMATION> {
        if addr as usize >= self.dwmcore_addr as usize + self.dwmcore_size as usize {
            return Err(Error::AddressBeyondModule);
        }

        Ok(self.hprocess.VirtualQueryEx(Some(addr))?)
    }

    pub fn dwmcore_read_memory(&self) -> Result<Vec<u8>> {
        let mut offset = 0;
        let mut memory = vec![];
        let mut good_pages = 0;
        let mut bad_pages = 0;

        while let Ok(mbi) = self.dwmcore_mempage_info(unsafe { self.dwmcore_addr.add(offset) }) {
            if mbi.State == MEM_STATE::COMMIT && mbi.Protect == PAGE::READONLY {
                if mbi.RegionSize < 4096 {
                    debug!("Small page size: {}", mbi.RegionSize);
                }

                good_pages += 1;
                let i = memory.len();

                memory.extend(repeat_n(0, mbi.RegionSize));

                let bytes_read = self
                    .hprocess
                    .ReadProcessMemory(mbi.BaseAddress, &mut memory[i..(i + mbi.RegionSize)])?;

                if bytes_read != mbi.RegionSize {
                    return Err(Error::PartialMemoryRead(bytes_read, mbi.RegionSize));
                }
            } else {
                bad_pages += 1;
            }

            offset += mbi.RegionSize;
        }

        debug!("{} pages read, {} filtered out", good_pages, bad_pages);

        Ok(memory)
    }
}
