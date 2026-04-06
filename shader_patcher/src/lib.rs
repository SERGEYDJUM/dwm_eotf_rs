pub mod dxcontainer;
pub mod error;
pub mod winapi;

use std::ffi::c_void;
use std::iter::repeat_n;
use std::path::Path;

use windows::Win32::System::Memory::PAGE_READWRITE;
use winsafe::MEMORY_BASIC_INFORMATION;
use winsafe::co::{MEM_STATE, PAGE};
use winsafe::{HPROCESS, co::PROCESS, guard::CloseHandleGuard};

use tracing::{debug, info, warn};

use crate::dxcontainer::{dump_shaders, patch_recursive};
use crate::error::{Error, Result};
use crate::winapi::{
    kill_process_by_name, pid_by_name, resume_process, set_memprotect, suspend_process,
    wait_module_by_name_and_pid, wait_pid_by_name,
};

pub trait BinaryPatcher {
    fn patch(&self, data: &mut [u8], checksum: u128) -> Result<bool>;
}

pub struct ShaderPatcher {
    hprocess: CloseHandleGuard<HPROCESS>,

    module_addr: *mut c_void,
    module_size: u32,

    page_infos: Vec<MEMORY_BASIC_INFORMATION>,
    memory: Vec<u8>,
}

impl Drop for ShaderPatcher {
    fn drop(&mut self) {
        // If something goes horribly wrong
        resume_process(&self.hprocess).ok();
    }
}

impl ShaderPatcher {
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
        let (module_addr, module_size) = wait_module_by_name_and_pid(module_name, pid)?;

        Ok(Self {
            hprocess: HPROCESS::OpenProcess(PROCESS::ALL_ACCESS, false, pid)?,
            module_addr,
            module_size,
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
        suspend_process(&self.hprocess)
    }

    pub fn resume(&self) -> Result<()> {
        debug!("Resuming process...");
        resume_process(&self.hprocess)
    }

    pub fn mempage_info(&self, addr: *mut c_void) -> Result<MEMORY_BASIC_INFORMATION> {
        if addr as usize >= self.module_addr as usize + self.module_size as usize {
            return Err(Error::AddressBeyondModule);
        }
        Ok(self.hprocess.VirtualQueryEx(Some(addr))?)
    }

    pub fn read_ram(&mut self) -> Result<()> {
        let mut mem_offset = 0;

        debug!("Reading process memory...");

        while let Ok(mbi) = self.mempage_info(unsafe { self.module_addr.add(mem_offset) }) {
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

    pub fn patch_shaders<T: BinaryPatcher>(&mut self, patcher: &T) -> Result<usize> {
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
            let old_protect = set_memprotect(&self.hprocess, mbi, PAGE_READWRITE)?;
            self.hprocess
                .WriteProcessMemory(mbi.BaseAddress, &self.memory[mem_offset..next_offset])?;
            set_memprotect(&self.hprocess, mbi, old_protect)?;
            mem_offset = next_offset;
        }

        Ok(())
    }

    pub fn execute_patching<T: BinaryPatcher>(&mut self, patcher: &T) -> Result<usize> {
        self.suspend()?;
        self.read_ram()?;
        let n_patched = self.patch_shaders(patcher)?;

        if n_patched != 0 {
            self.commit_to_ram()?;
        }

        self.resume()?;
        Ok(n_patched)
    }

    pub fn execute_shader_dump(&mut self, path: &Path, only_big: bool) -> Result<usize> {
        self.suspend()?;
        self.read_ram()?;
        let n_shaders = dump_shaders(&self.memory, only_big, path)?;
        self.resume()?;
        Ok(n_shaders)
    }
}
