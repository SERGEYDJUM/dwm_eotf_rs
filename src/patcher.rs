use aho_corasick::{AhoCorasick, MatchKind};
use bytemuck::cast_slice;

use crate::error::{Error, Result};

static ORIGINAL_PATTERNS: [[f32; 3]; 4] = [
    [2.4, 2.4, 2.4],
    [0.04045, 0.04045, 0.04045],
    [0.055000, 0.055000, 0.055000],
    [0.94786733, 0.94786733, 0.94786733],
];

static REPLACEMENT_PATTERNS: [[f32; 3]; 4] = [
    [2.2, 2.2, 2.2],
    [0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0],
    [1.0, 1.0, 1.0],
];

static HASH_WHITELIST: [u128; 4] = [
    0x85d254ecdbd4d71dcdec559258d1e696,
    0x3caf9cdde6b655e3ddfba2c137b02621,
    0xdbadc38d66727c965df029e2ff26892c,
    0xf5a79888be546336d9b324afbbbf93f6,
];

pub trait ShaderPatcher {
    fn patch(&self, data: &mut [u8], checksum: u128) -> Result<bool>;
}

pub struct HardCodedPatcher {
    aho: AhoCorasick,
    repl: Vec<Vec<u8>>,
}

impl Default for HardCodedPatcher {
    fn default() -> Self {
        let orig: Vec<Vec<u8>> = ORIGINAL_PATTERNS
            .iter()
            .map(|p| cast_slice(p).to_vec())
            .collect();

        let repl: Vec<Vec<u8>> = REPLACEMENT_PATTERNS
            .iter()
            .map(|p| cast_slice(p).to_vec())
            .collect();

        let aho = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .build(&orig)
            .unwrap();

        Self { aho, repl }
    }
}

impl ShaderPatcher for HardCodedPatcher {
    fn patch(&self, data: &mut [u8], checksum: u128) -> Result<bool> {
        if !HASH_WHITELIST.contains(&checksum) {
            return Ok(false);
        }

        let new_data = self.aho.replace_all_bytes(data, &self.repl);

        if new_data.len() != data.len() {
            return Err(Error::ReplLenChange);
        }

        data.copy_from_slice(&new_data);

        Ok(true)
    }
}
