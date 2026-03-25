use aho_corasick::{AhoCorasick, MatchKind};
use anyhow::{Result, anyhow};
use bytemuck::{cast, cast_slice};
use shader_patcher::{BinaryPatcher, error::Error};

static ORIGINAL_PATTERNS: [[f32; 4]; 4] = [
    [2.4, 2.4, 2.4, 0.0],
    [0.04045, 0.04045, 0.04045, 0.0],
    [0.055000, 0.055000, 0.055000, 0.0],
    [0.94786733, 0.94786733, 0.94786733, 0.0],
];

static HASH_WHITELIST: [u128; 4] = [
    0x85d254ecdbd4d71dcdec559258d1e696,
    0x3caf9cdde6b655e3ddfba2c137b02621,
    0xdbadc38d66727c965df029e2ff26892c,
    0xf5a79888be546336d9b324afbbbf93f6,
];

pub struct HardCodedPatcher {
    aho: AhoCorasick,
    replacements: [[u8; 16]; 4],
}

impl HardCodedPatcher {
    pub fn from_gamma(gamma: f32) -> Result<Self> {
        if gamma <= 0.0 {
            return Err(anyhow!("Gamma must be greater than zero!"));
        }

        let patterns: &[[u8; 16]] = cast_slice(&ORIGINAL_PATTERNS);

        let replacements: [[u8; 16]; 4] = cast([
            [gamma, gamma, gamma, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0, 0.0],
        ]);

        let aho = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .build(patterns)
            .unwrap();

        Ok(Self { aho, replacements })
    }
}

impl BinaryPatcher for HardCodedPatcher {
    fn patch(&self, data: &mut [u8], checksum: u128) -> Result<bool, Error> {
        if !HASH_WHITELIST.contains(&checksum) {
            return Ok(false);
        }

        let patched = self.aho.replace_all_bytes(data, &self.replacements);

        if patched.len() == data.len() {
            data.copy_from_slice(&patched);
            Ok(true)
        } else {
            Err(Error::ReplLenChange)
        }
    }
}
