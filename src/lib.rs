pub mod error;
pub mod shaders;
pub mod utils;

use std::ffi::c_void;
use std::iter::repeat_n;

use tracing::debug;

use ntapi::ntpsapi::{NtResumeProcess, NtSuspendProcess};
use winsafe::MEMORY_BASIC_INFORMATION;
use winsafe::co::{MEM_STATE, PAGE};
use winsafe::{HPROCESS, co::PROCESS, guard::CloseHandleGuard};

use crate::error::{Error, Result};
use crate::shaders::{DX_HEADER_SIZE, DXContainerHeader, DXContainerViewMut};
use crate::utils::{
    kill_process_by_name, pid_by_name, set_memprotect, wait_module_by_name_and_pid,
    wait_pid_by_name,
};

pub struct DwmProcess {
    hprocess: CloseHandleGuard<HPROCESS>,

    core_addr: *mut c_void,
    core_size: u32,

    page_infos: Vec<MEMORY_BASIC_INFORMATION>,
    memory: Vec<u8>,
}

impl Drop for DwmProcess {
    fn drop(&mut self) {
        // If something goes horribly wrong
        self.resume().unwrap();
    }
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
        Ok(self.hprocess.TerminateProcess(0)?)
    }

    pub fn suspend(&self) -> Result<()> {
        let ntstatus = unsafe { NtSuspendProcess(self.hprocess.ptr()) };
        if ntstatus != 0 {
            return Err(Error::NtApi(ntstatus));
        }
        Ok(())
    }

    pub fn resume(&self) -> Result<()> {
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

        while let Ok(mbi) = self.mempage_info(unsafe { self.core_addr.add(mem_offset) }) {
            let region_size = mbi.RegionSize;

            if mbi.State == MEM_STATE::COMMIT && mbi.Protect == PAGE::READONLY {
                if region_size < 4096 {
                    debug!("Small memory region detected: {}", region_size);
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

    pub fn patch_shaders(&mut self) -> Result<()> {
        let mut shaders_patched = 0;
        let mut shader_count = 0;
        let mut hstart = 0;

        while hstart < self.memory.len() - DX_HEADER_SIZE {
            if self.memory[hstart] == b'D'
                && self.memory[hstart + 1] == b'X'
                && self.memory[hstart + 2] == b'B'
                && self.memory[hstart + 3] == b'C'
            {
                let header_end = hstart + DX_HEADER_SIZE;
                let file_size = DXContainerHeader::from_bytes(&self.memory[hstart..header_end])
                    .file_size as usize;
                let file_end = hstart + file_size;

                let mut shader = DXContainerViewMut {
                    raw: &mut self.memory[hstart..file_end],
                };

                if shader.patch()? {
                    shaders_patched += 1;
                }

                shader_count += 1;
                hstart += file_size;
            } else {
                hstart += 1;
            }
        }

        debug!(
            "{} out of {} shaders were patched",
            shaders_patched, shader_count
        );

        Ok(())
    }

    pub fn commit_to_ram(&self) -> Result<()> {
        let mut mem_offset = 0;

        for mbi in &self.page_infos {
            let next_offset = mem_offset + mbi.RegionSize;
            set_memprotect(&self.hprocess, mbi, PAGE::READWRITE)?;
            self.hprocess
                .WriteProcessMemory(mbi.BaseAddress, &self.memory[mem_offset..next_offset])?;
            set_memprotect(&self.hprocess, mbi, mbi.Protect)?;
            mem_offset = next_offset;
        }

        Ok(())
    }
}
