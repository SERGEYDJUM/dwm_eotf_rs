pub mod error;
pub mod utils;

use std::ffi::c_void;
use std::fs::{File, create_dir_all};
use std::io::Write;
use std::iter::repeat_n;

use bytemuck::from_bytes;
use ntapi::ntpsapi::{NtResumeProcess, NtSuspendProcess};
use tracing::debug;
use winsafe::MEMORY_BASIC_INFORMATION;
use winsafe::co::{MEM_STATE, PAGE};
use winsafe::{HPROCESS, co::PROCESS, guard::CloseHandleGuard};

use crate::error::{Error, Result};
use crate::utils::{DXContainerHeader, module_by_name_and_pid, pid_by_name};

pub struct DwmProcess {
    hprocess: CloseHandleGuard<HPROCESS>,
    dwmcore_addr: *mut c_void,
    dwmcore_size: u32,
}

impl Drop for DwmProcess {
    fn drop(&mut self) {
        unsafe { NtResumeProcess(self.hprocess.ptr()) };
    }
}

impl DwmProcess {
    pub fn open() -> Result<Self> {
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
            if mbi.RegionSize > 4096
                && mbi.State == MEM_STATE::COMMIT
                && mbi.Protect == PAGE::READONLY
            {
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

    pub fn dump_shaders(&self) -> Result<usize> {
        let buffer = self.dwmcore_read_memory()?;
        debug!("Read {} bytes from dwmcore's memory", buffer.len());

        create_dir_all("dumped_shaders")?;

        let mut shaders_dumped = 0;

        for i in 0..(buffer.len() - 3) {
            if buffer[i] == b'D'
                && buffer[i + 1] == b'X'
                && buffer[i + 2] == b'B'
                && buffer[i + 3] == b'C'
            {
                let header_size = size_of::<DXContainerHeader>();
                let header: &DXContainerHeader = from_bytes(&buffer[i..(i + header_size)]);

                let hash = header
                    .digest
                    .iter()
                    .fold(String::new(), |s, &b| s + &format!("{:X}", b));

                debug!("Dumping shader with hash `{}`", &hash);

                File::create(format!("dumped_shaders/{}.dxbc", hash))?
                    .write_all(&buffer[i..(i + (header.file_size as usize))])?;

                shaders_dumped += 1;
            }
        }

        Ok(shaders_dumped)
    }
}
