pub mod dxcontainer;
pub mod error;
pub mod patcher;
pub mod winapi;

use std::ffi::c_void;
use std::iter::repeat_n;

use ntapi::ntpsapi::{NtResumeProcess, NtSuspendProcess};
use winsafe::MEMORY_BASIC_INFORMATION;
use winsafe::co::{MEM_STATE, PAGE};
use winsafe::{HPROCESS, co::PROCESS, guard::CloseHandleGuard};

use tracing::{debug, info, warn};

use crate::dxcontainer::patch_recursive;
use crate::error::{Error, Result};
use crate::patcher::Patcher;
use crate::winapi::{
    kill_process_by_name, pid_by_name, set_memprotect, wait_module_by_name_and_pid,
    wait_pid_by_name,
};

pub struct TargetProcess {
    hprocess: CloseHandleGuard<HPROCESS>,

    core_addr: *mut c_void,
    core_size: u32,

    page_infos: Vec<MEMORY_BASIC_INFORMATION>,
    memory: Vec<u8>,
}

impl Drop for TargetProcess {
    fn drop(&mut self) {
        // If something goes horribly wrong
        unsafe { NtResumeProcess(self.hprocess.ptr()) };
    }
}

impl TargetProcess {
    pub fn open(exe: &str, module: &str) -> Result<Self> {
        let pid = pid_by_name(exe)?;
        Self::load(pid, module)
    }

    pub fn open_wait(exe: &str, module: &str) -> Result<Self> {
        let pid = wait_pid_by_name(exe)?;
        Self::load(pid, module)
    }

    pub fn open_restarted(exe: &str, module: &str) -> Result<Self> {
        let killed_pid = kill_process_by_name(exe)?;
        let mut new_pid = killed_pid;

        info!("Killed `{}` process with PID {}", exe, killed_pid);

        while killed_pid == new_pid {
            new_pid = wait_pid_by_name(exe)?;
        }

        info!("Found `{}` process with PID {}", exe, new_pid);
        Self::load(new_pid, module)
    }

    pub fn load(pid: u32, module_name: &str) -> Result<Self> {
        let (dwmcore_addr, dwmcore_size) = wait_module_by_name_and_pid(module_name, pid)?;

        Ok(Self {
            hprocess: HPROCESS::OpenProcess(PROCESS::ALL_ACCESS, false, pid)?,
            core_addr: dwmcore_addr,
            core_size: dwmcore_size,
            page_infos: vec![],
            memory: vec![],
        })
    }

    pub fn kill(&self) -> Result<()> {
        debug!("Killing the process...");
        Ok(self.hprocess.TerminateProcess(0)?)
    }

    pub fn suspend(&self) -> Result<()> {
        debug!("Suspending process...");
        let ntstatus = unsafe { NtSuspendProcess(self.hprocess.ptr()) };
        if ntstatus != 0 {
            return Err(Error::NtApi(ntstatus));
        }
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
        debug!("Resuming process...");
        let ntstatus = unsafe { NtResumeProcess(self.hprocess.ptr()) };
        if ntstatus != 0 {
            return Err(Error::NtApi(ntstatus));
        }
        Ok(())
    }

    pub fn mempage_info(&self, addr: *mut c_void) -> Result<MEMORY_BASIC_INFORMATION> {
        if addr as usize >= self.core_addr as usize + self.core_size as usize {
            return Err(Error::AddressBeyondModule);
        }
        Ok(self.hprocess.VirtualQueryEx(Some(addr))?)
    }

    pub fn read_ram(&mut self) -> Result<()> {
        let mut mem_offset = 0;

        debug!("Reading process memory...");

        while let Ok(mbi) = self.mempage_info(unsafe { self.core_addr.add(mem_offset) }) {
            let region_size = mbi.RegionSize;

            if mbi.State == MEM_STATE::COMMIT && mbi.Protect == PAGE::READONLY {
                if region_size < 4096 {
                    warn!("Detected small ({} bytes) memory region", region_size);
                }

                let start = self.memory.len();
                self.memory.extend(repeat_n(0, region_size));
                let end = self.memory.len();

                let bytes_read = self
                    .hprocess
                    .ReadProcessMemory(mbi.BaseAddress, &mut self.memory[start..end])?;

                if bytes_read != region_size {
                    return Err(Error::PartialMemoryRead(bytes_read, region_size));
                }

                self.page_infos.push(mbi);
            }

            mem_offset += region_size;
        }

        Ok(())
    }

    pub fn view_memory(&self) -> &[u8] {
        &self.memory
    }

    pub fn patch_shaders(&mut self, patcher: &Patcher) -> Result<usize> {
        debug!("Patching shaders recursively...");
        let (found, patched) = patch_recursive(&mut self.memory, patcher, true)?;
        info!("{} out of {} shaders were patched", patched, found);
        Ok(patched)
    }

    pub fn commit_to_ram(&self) -> Result<()> {
        let mut mem_offset = 0;

        debug!("Writing patched regions back...");

        for mbi in &self.page_infos {
            let next_offset = mem_offset + mbi.RegionSize;
            let old_protect = set_memprotect(&self.hprocess, mbi, PAGE::READWRITE)?;
            self.hprocess
                .WriteProcessMemory(mbi.BaseAddress, &self.memory[mem_offset..next_offset])?;
            set_memprotect(&self.hprocess, mbi, old_protect)?;
            mem_offset = next_offset;
        }

        Ok(())
    }
}
