use aho_corasick::{AhoCorasick, MatchKind};
use anyhow::Result;
use bytemuck::{cast, cast_slice};
use shader_patcher::{BinaryPatcher, error::Error};
use tracing::warn;

//TODO?
//RGB_FULL_G22_NONE_P2020_To_scRGB
//RGB_FULL_G22_NONE_P2020_IA_To_scRGB
//RGB_FULL_G22_NONE_P2020_To_HDR10
//RGB_FULL_G22_NONE_P2020_IA_To_HDR10
//RGB_8BITSTUDIO_G22_NONE_P709_To_scRGB
//RGB_8BITSTUDIO_G22_NONE_P709_IA_To_scRGB
//RGB_10BITSTUDIO_G22_NONE_P2020_To_scRGB
//RGB_10BITSTUDIO_G22_NONE_P2020_IA_To_scRGB
//RGB_10BITSTUDIO_G22_NONE_P2020_To_HDR10
//RGB_10BITSTUDIO_G22_NONE_P2020_IA_To_HDR10
//RGB_FULL_G22EXT_NONE_P709_To_scRGB
//RGB_FULL_G22EXT_NONE_P709_IA_To_scRGB
//RGB_FULL_G22EXT_NONE_P709_To_HDR10
//RGB_FULL_G22EXT_NONE_P709_IA_To_HDR10
//RGB_FULL_G22_NONE_P3_To_scRGB
//RGB_FULL_G22_NONE_P3_IA_To_scRGB
//RGB_FULL_G22_NONE_P3_To_HDR10
//RGB_FULL_G22_NONE_P3_IA_To_HDR10

static SDR2HDR_WHITELIST: [u128; 6] = [
    0x84cd59df5f46a95d22ee9bffed30de4c, // 4cde30edff9bee225da9465fdf59cd84 (RGB_FULL_G22_NONE_P709_To_scRGB, SM 4.0)
    0x6965d7ccd34ad1892d6b815cdea54270, // 7042a5de5c816b2d89d14ad3ccd76569 (RGB_FULL_G22_NONE_P709_IA_To_scRGB, SM 4.0)
    0x85d254ecdbd4d71dcdec559258d1e696, // 96e6d1589255eccd1dd7d4dbec54d285 (RGB_FULL_G22_NONE_P709_To_scRGB, SM 4.0 + Level 9)
    0x3caf9cdde6b655e3ddfba2c137b02621, // 2126b037c1a2fbdde355b6e6dd9caf3c (RGB_FULL_G22_NONE_P709_IA_To_scRGB, SM 4.0 + Level 9)
    0xdbadc38d66727c965df029e2ff26892c, // 2c8926ffe229f05d967c72668dc3addb (RGB_FULL_G22_NONE_P709_To_scRGB, SM 4.0 + Level 9)
    0xf5a79888be546336d9b324afbbbf93f6, // f693bfbbaf24b3d9366354be8898a7f5 (RGB_FULL_G22_NONE_P709_IA_To_scRGB, SM 4.0 + Level 9)
];

static ALPHA_CORRECT_WHITELIST: [u128; 15] = [
    0x758445033aff77cf29cee921baf880fa, // fa80f8ba21e9ce29cf77ff3a03458475 (AlphaCorrectSDR, SM 4.0)
    0xa4c7098ffff49306e2903060139e3302, // 02339e13603090e20693f4ff8f09c7a4 (AlphaCorrectExtendedSDR, SM 4.0)
    0x8409a2019f0244e55454452192a1773e, // 3e77a19221455454e544029f01a20984 (BoostSDRLuminance, SM 4.0)
    0x2f47c64cc12e299e8908a6559a6ab706, // 06b76a9a55a608899e292ec14cc6472f (BoostSDRLuminance_Old, SM 4.0)
    0xbedcfd5b561c32f89e25143a56d57153, // 5371d5563a14259ef8321c565bfddcbe (BoostExtendedSDRLuminance, SM 4.0)
    0x8c6b41ab0271dcc371334ea7dd2d635f, // 5f632ddda74e3371c3dc7102ab416b8c (AlphaCorrectSDR, SM 4.0 + Level 9)
    0x4e7a85dd3ca31d26522d23665e029155, // 5591025e66232d52261da33cdd857a4e (AlphaCorrectSDR, SM 4.0 + Level 9)
    0xdf24f226d9b0aba18adfbd889fa024d4, // d424a09f88bddf8aa1abb0d926f224df (AlphaCorrectExtendedSDR, SM 4.0 + Level 9)
    0xc846a6eff08c0b5c2d944de6653fdd35, // 35dd3f65e64d942d5c0b8cf0efa646c8 (AlphaCorrectExtendedSDR, SM 4.0 + Level 9)
    0xa50ef1a70ad73ea497e3af2755c94d4d, // 4d4dc95527afe397a43ed70aa7f10ea5 (BoostSDRLuminance, SM 4.0 + Level 9)
    0xb50b1a06f22d392aa46b819c2e4d26c5, // c5264d2e9c816ba42a392df2061a0bb5 (BoostSDRLuminance, SM 4.0 + Level 9)
    0x86cb33f844d10c92ccaa46b32242e68e, // 8ee64222b346aacc920cd144f833cb86 (BoostSDRLuminance_Old, SM 4.0 + Level 9)
    0xb3fb6fd115c919e5497e9795957cc013, // 13c07c9595977e49e519c915d16ffbb3 (BoostSDRLuminance_Old, SM 4.0 + Level 9)
    0x74ab0d8e3e3ba4638c3f3ec97ff64d36, // 364df67fc93e3f8c63a43b3e8e0dab74 (BoostExtendedSDRLuminance, SM 4.0 + Level 9)
    0x7737fc9ef8ff2f6711c3b81b49f7de0c, // 0cdef7491bb8c311672ffff89efc3777 (BoostExtendedSDRLuminance, SM 4.0 + Level 9)
];

#[allow(clippy::excessive_precision)]
static ORIGINAL_PATTERNS: [[f32; 4]; 12] = [
    // SDR-to-HDR patterns
    [2.4, 2.4, 2.4, 0.0],
    [0.04045, 0.04045, 0.04045, 0.0],
    [0.055000, 0.055000, 0.055000, 0.0],
    [0.94786733, 0.94786733, 0.94786733, 0.0],
    // AlphaCorrect and BoostSDR patterns
    [-1.77846289, -0.114420228, 0.0, 0.0],
    [1.0, -1.77846289, -0.114420228, 0.0],
    [-1.77846289, -0.114420228, -1.0, 4.0],
    [-1.57869995, 0.0255999994, -0.101999998, 0.833899975],
    [1.0, 0.0255999994, -0.101999998, -1.57869995],
    // AlphaCorrect and BoostSDR reinterpreted patterns
    [-1.77846289, 9.6296497e-35, 1.4694156e-39, 1.4012985e-45],
    [-1.77846289, 1.5407531e-33, 1.4694604e-39, 1.4012985e-45],
    [-1.57869995, 1.5407531e-33, 1.4693932e-39, 1.4012985e-45],
];

pub struct SimplePatcher<'a> {
    aho: &'a AhoCorasick,
    replacements: [[u8; 16]; 12],
    ignore_whitelist: bool,
}

impl<'a> SimplePatcher<'a> {
    pub fn new(
        aho: &'a AhoCorasick,
        mut gamma: f32,
        mut brightness: f32,
        ignore_whitelist: bool,
        no_alpha_fix: bool,
    ) -> Self {
        if gamma <= 0.0 {
            warn!("Gamma must be positive, defaulting to 2.2!");
            gamma = 2.2;
        }

        if brightness <= 0.0 {
            warn!("Brightness must be positive, defaulting to 1.0!");
            brightness = 1.0;
        }

        // Dynamic border opacity correction:
        // In unpatched DWM at standard SDR (lum = 1.0), the polynomial computes:
        //   (lum - 0.5) * -1.778463 = -0.8892315
        // which dims subtle border alpha (e.g. 0.08 -> 0.0181) for natural visual blending.
        // Under brightness boost (lum > 1.0), this became strongly negative, clamping alpha to 0.0.
        // Setting it to 0.0 leaves alpha un-dimmed (0.08), which appears too bright / glaring.
        // By scaling b1 inversely with (brightness - 0.5), (lum - 0.5) * b1 remains exactly -0.8892315!
        // The border maintains its exact original Windows 11 opacity without vanishing or glowing!
        #[allow(clippy::excessive_precision)]
        let (mut b1, mut c1) = (-1.77846289, -1.57869995);
        let scale = brightness.powf(1.0 / gamma);

        if !no_alpha_fix && (brightness > 1.0) {
            b1 /= 2.0 * brightness - 1.0;
            c1 /= brightness;
        }

        #[allow(clippy::excessive_precision)]
        let replacements: [[u8; 16]; 12] = cast([
            [gamma, gamma, gamma, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [scale, scale, scale, 0.0],
            [b1, -0.114420228, 0.0, 0.0],
            [1.0, b1, -0.114420228, 0.0],
            [b1, -0.114420228, -1.0, 4.0],
            [c1, 0.0255999994, -0.101999998, 0.833899975],
            [1.0, 0.0255999994, -0.101999998, c1],
            [b1, 9.6296497e-35, 1.4694156e-39, 1.4012985e-45],
            [b1, 1.5407531e-33, 1.4694604e-39, 1.4012985e-45],
            [c1, 1.5407531e-33, 1.4693932e-39, 1.4012985e-45],
        ]);

        Self {
            aho,
            replacements,
            ignore_whitelist,
        }
    }
}

impl<'a> BinaryPatcher for SimplePatcher<'a> {
    fn patch(&self, data: &mut [u8], checksum: u128) -> Result<bool, Error> {
        if !(self.ignore_whitelist
            || SDR2HDR_WHITELIST.contains(&checksum)
            || ALPHA_CORRECT_WHITELIST.contains(&checksum))
        {
            return Ok(false);
        }

        let patched = self.aho.replace_all_bytes(data, &self.replacements);

        if patched.len() != data.len() {
            return Err(Error::ReplLenChange);
        }

        data.copy_from_slice(&patched);
        Ok(true)
    }
}

pub fn build_aho_corasick() -> Result<AhoCorasick> {
    let patterns: &[[u8; 16]] = cast_slice(&ORIGINAL_PATTERNS);

    Ok(AhoCorasick::builder()
        .match_kind(MatchKind::LeftmostLongest)
        .build(patterns)?)
}
