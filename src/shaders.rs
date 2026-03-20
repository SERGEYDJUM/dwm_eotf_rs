use bytemuck::{Pod, Zeroable, checked};
use tracing::{debug, warn};

use crate::{error::Result, utils::calculate_checksum};

pub const DXBC_MAGIC: [u8; 4] = [b'D', b'X', b'B', b'C'];
pub const DX_HEADER_SIZE: usize = size_of::<DXContainerHeader>();

pub static SHADER_HASHES: [[u8; 16]; 4] = [
    [
        0x96, 0xe6, 0xd1, 0x58, 0x92, 0x55, 0xec, 0xcd, 0x1d, 0xd7, 0xd4, 0xdb, 0xec, 0x54, 0xd2,
        0x85,
    ],
    [
        0x21, 0x26, 0xb0, 0x37, 0xc1, 0xa2, 0xfb, 0xdd, 0xe3, 0x55, 0xb6, 0xe6, 0xdd, 0x9c, 0xaf,
        0x3c,
    ],
    [
        0x2c, 0x89, 0x26, 0xff, 0xe2, 0x29, 0xf0, 0x5d, 0x96, 0x7c, 0x72, 0x66, 0x8d, 0xc3, 0xad,
        0xdb,
    ],
    [
        0xf6, 0x93, 0xbf, 0xbb, 0xaf, 0x24, 0xb3, 0xd9, 0x36, 0x63, 0x54, 0xbe, 0x88, 0x98, 0xa7,
        0xf5,
    ],
];

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
    pub fn patch(&mut self) -> Result<bool> {
        let (header, _) = self.raw.split_at_mut(size_of::<DXContainerHeader>());
        let dxc_header: &mut DXContainerHeader = checked::from_bytes_mut(header);

        let digest = dxc_header.digest.clone();
        let hash: u128 = *bytemuck::from_bytes(&digest);

        debug!("Found shader with hash `{:x}`", hash);

        if hash != calculate_checksum(self.raw) {
            warn!("Hash mismatch!");
        }

        Ok(false)
    }
}
