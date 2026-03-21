use aho_corasick::{AhoCorasick, MatchKind};
use bytemuck::cast_slice;

use crate::error::{Error, Result};

static ORIGINAL_PATTERNS: [[f32; 4]; 4] = [
    [2.4, 2.4, 2.4, 0.0],
    [0.04045, 0.04045, 0.04045, 0.0],
    [0.055, 0.055, 0.055, 0.0],
    [0.947867, 0.947867, 0.947867, 0.0],
];

static REPLACEMENT_PATTERNS: [[f32; 4]; 4] = [
    [2.2, 2.2, 2.2, 0.0],
    [0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 0.0],
    [1.0, 1.0, 1.0, 0.0],
];

pub struct Patcher {
    aho: AhoCorasick,
    repl: Vec<Vec<u8>>,
}

impl Default for Patcher {
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

impl Patcher {
    pub fn patch(&self, data: &mut [u8]) -> Result<()> {
        let new_data = self.aho.replace_all_bytes(data, &self.repl);
        if new_data.len() != data.len() {
            return Err(Error::ReplLenChange);
        }
        data.copy_from_slice(&new_data);
        Ok(())
    }
}
