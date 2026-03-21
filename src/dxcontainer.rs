use std::{fs, io::Write};

use bytemuck::{Pod, Zeroable, cast_slice, checked, from_bytes};

use tracing::{debug, info};

use crate::{error::Result, patcher::Patcher};

unsafe extern "C" {
    unsafe fn CalculateDXBCChecksum(pData: *const u8, dwSize: u32, dwHash: &mut [u32; 4]) -> bool;
}

pub const DXBC_MAGIC: [u8; 4] = [b'D', b'X', b'B', b'C'];
pub const DX_HEADER_SIZE: usize = size_of::<DXContainerHeader>();

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

impl DXContainerHeader {
    pub fn from_bytes(bytes: &[u8]) -> &Self {
        checked::from_bytes(bytes)
    }
}

#[derive(Debug)]
pub struct DXContainerViewMut<'a> {
    pub raw: &'a mut [u8],
}

impl<'a> DXContainerViewMut<'a> {
    pub fn from_raw(raw: &'a mut [u8]) -> Self {
        // TODO: add checks
        Self { raw }
    }

    pub fn get_header(&self) -> &DXContainerHeader {
        checked::from_bytes(&self.raw[0..DX_HEADER_SIZE])
    }

    pub fn get_header_mut(&mut self) -> &mut DXContainerHeader {
        checked::from_bytes_mut(&mut self.raw[0..DX_HEADER_SIZE])
    }

    pub fn get_data_mut(&mut self) -> &mut [u8] {
        &mut self.raw[DX_HEADER_SIZE..]
    }

    pub fn fix_checksum(&mut self) -> Option<u128> {
        let hash = calculate_checksum(self.raw);

        let dxc_header = self.get_header_mut();
        let stored_digest = dxc_header.digest.to_owned();
        let stored_hash: u128 = *from_bytes(&stored_digest);

        if stored_hash == hash {
            return None;
        }

        for (i, &byte) in hash.to_le_bytes().iter().enumerate() {
            dxc_header.digest[i] = byte;
        }

        Some(stored_hash)
    }

    pub fn patch(&mut self, patcher: &Patcher) -> Result<bool> {
        patcher.patch(self.raw)?;

        if let Some(old_hash) = self.fix_checksum() {
            info!("Patched shader `{:x}`", old_hash);
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub fn patch_recursive(raw: &mut [u8], patcher: &Patcher, recurse: bool) -> Result<(usize, usize)> {
    let mut shaders_patched = 0;
    let mut shaders_found = 0;
    let mut h_start = 0;

    while h_start < raw.len() - DX_HEADER_SIZE {
        if raw[h_start] == DXBC_MAGIC[0]
            && raw[h_start + 1] == DXBC_MAGIC[1]
            && raw[h_start + 2] == DXBC_MAGIC[2]
            && raw[h_start + 3] == DXBC_MAGIC[3]
        {
            let h_end = h_start + DX_HEADER_SIZE;
            let f_size = DXContainerHeader::from_bytes(&raw[h_start..h_end]).file_size as usize;
            let f_end = h_start + f_size;

            let mut shader = DXContainerViewMut::from_raw(&mut raw[h_start..f_end]);

            if recurse {
                let (sub_found, sub_patched) =
                    patch_recursive(shader.get_data_mut(), patcher, false)?;

                if sub_found != 0 {
                    shaders_found += sub_found;

                    if sub_patched != 0 {
                        shader.fix_checksum();
                        shaders_patched += sub_patched + 1;
                    }
                } else {
                    shaders_found += 1;
                    if shader.patch(patcher)? {
                        shaders_patched += 1;
                    }
                }
            } else {
                shaders_found += 1;
                if shader.patch(patcher)? {
                    shaders_patched += 1;
                }
            }

            h_start += f_size;
        } else {
            h_start += 1;
        }
    }

    Ok((shaders_found, shaders_patched))
}

pub fn dump_shaders(bytes: &[u8], only_big: bool) -> Result<usize> {
    fs::create_dir_all("shaders/dumped")?;

    let mut shaders_dumped = 0;
    let mut i = 0;

    if !only_big {
        debug!("Shader part dumping enabled, expect redundant shaders");
    }

    while i < bytes.len() {
        if bytes[i] == DXBC_MAGIC[0]
            && bytes[i + 1] == DXBC_MAGIC[1]
            && bytes[i + 2] == DXBC_MAGIC[2]
            && bytes[i + 3] == DXBC_MAGIC[3]
        {
            let header_size = DX_HEADER_SIZE;
            let header: &DXContainerHeader = from_bytes(&bytes[i..(i + header_size)]);
            let file_size = header.file_size as usize;

            let hash: u128 = *from_bytes(&header.digest.clone());
            info!("Dumping shader with hash `{:X}`", &hash);

            fs::File::create(format!("shaders/dumped/{:X}.dxbc", hash))?
                .write_all(&bytes[i..(i + file_size)])?;

            shaders_dumped += 1;
            i += if only_big { file_size } else { 1 };
        } else {
            i += 1;
        }
    }

    Ok(shaders_dumped)
}

pub fn calculate_checksum(data: &[u8]) -> u128 {
    let mut digest = [0u32; 4];
    unsafe {
        CalculateDXBCChecksum(data.as_ptr(), data.len() as u32, &mut digest);
    }
    let digest: &[u8] = cast_slice(&digest);
    *from_bytes(digest)
}
