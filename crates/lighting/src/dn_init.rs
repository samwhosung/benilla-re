//! The static track builders the day-night file's initializers run: `BuildSkyGlowTrack`
//! (`0x6ce120`, `systems/lighting/sky.md#sky-colour-tracks`), `BuildCloudEnvelopeTrack`
//! (`0x6ce390`, `systems/lighting/clouds.md#cloud-envelope`), `BuildStarTrack` (`0x6ce4a0`) and
//! `BuildUnusedSkyTrack` (`0x6ce560`) (`systems/lighting/celestial.md#static-tracks`), and
//! `BuildNightTrack` (`0x6ce670`) (`systems/lighting/daynight.md#night-fraction`).
//!
//! Each takes nothing and writes a track's `(time, value)` keys as immediate stores and one
//! operation on `.rdata` constants (`fld`, `fadd` or `fsub` of two `f32`, `fstp dword`), so a
//! function is the list of `(address, bits)` it writes. A sum or difference rounds once at 53 bits
//! and once more to `f32`; a copy keeps the constant's bits.

// The `.rdata` constants the builders load, as `f32` bits.
/// `[0x811510]`, 0.22916667.
const RD_811510: u32 = 0x3e6a_aaab;
/// `[0x811518]`, 0.89583331.
const RD_811518: u32 = 0x3f65_5555;
/// `[0x81151c]`, 0.91666669.
const RD_81151C: u32 = 0x3f6a_aaab;
/// `[0x811528]`, 0.16666667.
const RD_811528: u32 = 0x3e2a_aaab;
/// `[0x8115ac]`, 0.027777778.
const RD_8115AC: u32 = 0x3ce3_8e39;
/// `[0x8115b8]`, 0.10347.
const RD_8115B8: u32 = 0x3dd3_e93f;
/// `[0x8115bc]`, 0.041666668.
const RD_8115BC: u32 = 0x3d2a_aaab;
/// `[0x8115c0]`, 0.10416667.
const RD_8115C0: u32 = 0x3dd5_5555;
/// `[0x802998]`, 0.0625.
const RD_802998: u32 = 0x3d80_0000;
/// `[0x80e004]`, 0.083333336.
const RD_80E004: u32 = 0x3daa_aaab;

/// `fld dword [a]; fsub dword [b]; fstp dword`: the difference at 53 bits, stored as `f32`.
fn fsub(a: u32, b: u32) -> u32 {
    ((f64::from(f32::from_bits(a)) - f64::from(f32::from_bits(b))) as f32).to_bits()
}

/// `fld dword [a]; fadd dword [b]; fstp dword`: the sum at 53 bits, stored as `f32`.
fn fadd(a: u32, b: u32) -> u32 {
    ((f64::from(f32::from_bits(a)) + f64::from(f32::from_bits(b))) as f32).to_bits()
}

/// `0x6ce120`: the sky glow track. Twelve stores at `0xce9b2c` to `0xce9b58`.
pub fn dn_init_6ce120() -> [(u64, u32); 12] {
    [
        (0xce9b2c, fsub(RD_811510, RD_8115C0)), // A - [0x8115c0]
        (0xce9b30, 0x0000_0000),                // immediate 0
        (0xce9b34, fadd(RD_811510, RD_8115BC)), // A + [0x8115bc]
        (0xce9b38, 0x3f80_0000),                // immediate 1.0
        (0xce9b3c, fadd(RD_811510, RD_802998)), // A + [0x802998]
        (0xce9b40, 0x0000_0000),                // immediate 0
        (0xce9b44, fsub(RD_811518, RD_8115BC)), // B - [0x8115bc]
        (0xce9b48, 0x0000_0000),                // immediate 0
        (0xce9b4c, RD_811518),                  // copy of B
        (0xce9b50, 0x3f80_0000),                // immediate 1.0
        (0xce9b54, fadd(RD_811518, RD_8115B8)), // B + [0x8115b8]
        (0xce9b58, 0x0000_0000),                // immediate 0
    ]
}

/// `0x6ce390`: the cloud envelope track, sixteen stores at `0xce9ab8` to `0xce9af4`.
pub fn dn_init_6ce390() -> [(u64, u32); 16] {
    [
        (0xce9ab8, RD_811528),                  // copy of D
        (0xce9abc, 0x3f80_0000),                // immediate 1.0
        (0xce9ac0, fadd(RD_8115AC, RD_811528)), // E + D
        (0xce9ac4, 0x0000_0000),                // immediate 0
        (0xce9ac8, fsub(RD_811510, RD_8115AC)), // A - E
        (0xce9acc, 0x0000_0000),                // immediate 0
        (0xce9ad0, RD_811510),                  // copy of A
        (0xce9ad4, 0x3f80_0000),                // immediate 1.0
        (0xce9ad8, RD_811518),                  // copy of B
        (0xce9adc, 0x3f80_0000),                // immediate 1.0
        (0xce9ae0, fadd(RD_811518, RD_8115AC)), // B + E
        (0xce9ae4, 0x0000_0000),                // immediate 0
        (0xce9ae8, fsub(RD_81151C, RD_8115AC)), // F - E
        (0xce9aec, 0x0000_0000),                // immediate 0
        (0xce9af0, RD_81151C),                  // copy of F
        (0xce9af4, 0x3f80_0000),                // immediate 1.0
    ]
}

/// `0x6ce4a0`: the star track, eight stores at `0xce9a98` to `0xce9ab4`.
pub fn dn_init_6ce4a0() -> [(u64, u32); 8] {
    [
        (0xce9a98, fsub(RD_811510, RD_8115C0)), // A - [0x8115c0]
        (0xce9a9c, 0x3f80_0000),                // immediate 1.0
        (0xce9aa0, fsub(RD_811510, RD_8115BC)), // A - [0x8115bc]
        (0xce9aa4, 0x0000_0000),                // immediate 0
        (0xce9aa8, fadd(RD_811518, RD_8115BC)), // B + [0x8115bc]
        (0xce9aac, 0x0000_0000),                // immediate 0
        (0xce9ab0, fadd(RD_811518, RD_8115C0)), // B + [0x8115c0]
        (0xce9ab4, 0x3f80_0000),                // immediate 1.0
    ]
}

/// `0x6ce560`: the unused track, sixteen stores at `0xce9a58` to `0xce9a94`; its immediates are 2.0
/// and 4.0.
pub fn dn_init_6ce560() -> [(u64, u32); 16] {
    [
        (0xce9a58, fsub(RD_811528, RD_8115BC)), // D - [0x8115bc]
        (0xce9a5c, 0x4000_0000),                // immediate 2.0
        (0xce9a60, RD_811528),                  // copy of D
        (0xce9a64, 0x4080_0000),                // immediate 4.0
        (0xce9a68, RD_811510),                  // copy of A
        (0xce9a6c, 0x4080_0000),                // immediate 4.0
        (0xce9a70, fadd(RD_811510, RD_8115BC)), // A + [0x8115bc]
        (0xce9a74, 0x4000_0000),                // immediate 2.0
        (0xce9a78, fsub(RD_811518, RD_80E004)), // B - [0x80e004]
        (0xce9a7c, 0x4000_0000),                // immediate 2.0
        (0xce9a80, RD_811518),                  // copy of B
        (0xce9a84, 0x4080_0000),                // immediate 4.0
        (0xce9a88, RD_81151C),                  // copy of F
        (0xce9a8c, 0x4080_0000),                // immediate 4.0
        (0xce9a90, fadd(RD_81151C, RD_8115BC)), // F + [0x8115bc]
        (0xce9a94, 0x4000_0000),                // immediate 2.0
    ]
}

/// `0x6ce670`: the night track, eight stores at `0xce9a34` to `0xce9a50`; the only arithmetic is
/// `[0x811518] − [0x8115bc]`, and the rest are immediates.
pub fn dn_init_6ce670() -> [(u64, u32); 8] {
    [
        (0xce9a34, 0x3e80_0000),                // immediate 0.25
        (0xce9a38, 0x3f80_0000),                // immediate 1.0
        (0xce9a3c, 0x3e95_5555),                // immediate 0x3e955555
        (0xce9a40, 0x0000_0000),                // immediate 0
        (0xce9a44, fsub(RD_811518, RD_8115BC)), // B - [0x8115bc]
        (0xce9a48, 0x0000_0000),                // immediate 0
        (0xce9a4c, RD_811518),                  // copy of B
        (0xce9a50, 0x3f80_0000),                // immediate 1.0
    ]
}
