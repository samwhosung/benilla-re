//! The client's random-number library, the functions of it that are reached
//! (`systems/random.md`): the seed, the advance and the two unit vectors; and the C runtime's
//! `rand` ([`msvc_rand`]).
//!
//! The x87 runs at 53-bit precision. `fsincos` differs by CPU; the host's `f64` sine and cosine
//! stand in for it, and `fsqrt` is exact.

#![allow(clippy::float_cmp)]

/// 2π as `f32` (`0x40c90fdb`), as at `0x80295c`.
pub const TWO_PI_BITS: u32 = 0x40c9_0fdb;
/// `[0x7ff9d8]`, 1.0.
pub const ONE_BITS: u32 = 0x3f80_0000;
/// `[0x801628]`, 2.0.
pub const TWO_F32_BITS: u32 = 0x4000_0000;

/// The table the advance reads, `0x802700`. From any state a lane is at most 251, read as a dword,
/// so 256 bytes cover it; the tests read it from the exe.
pub const TABLE_VA: u32 = 0x0080_2700;

/// `NTempest::CRandom::uint32` (`0x4531e0`, `systems/random.md#state-advance`): each byte of
/// `index_state` is a lane, stepped down by 4, 0xc, 0x18 or 0x1c and given back its modulus 0xbc,
/// 0xd4, 0xec or 0xf4 when it goes negative; the dword of `table` at each lane, rotated left by
/// 1, 2, 3 or 0, is XORed in, and the sum added to `accumulator`. Returns the new accumulator, the
/// new lanes, and `eax`, which is the accumulator.
pub fn crandom_uint32(accumulator: u32, index_state: u32, table: &[u8]) -> (u32, u32, u32) {
    let t = |idx: i32| -> u32 {
        let i = idx as usize;
        u32::from_le_bytes([table[i], table[i + 1], table[i + 2], table[i + 3]])
    };
    let b = |sh: u32| ((index_state >> sh) & 0xff) as i32;
    let (b0, b1, b2, b3) = (b(0), b(8), b(16), b(24));

    let mut i3 = b3 - 0x04;
    if i3 < 0 {
        i3 += 0xbc;
    }
    let mut i2 = b2 - 0x0c;
    if i2 < 0 {
        i2 += 0xd4;
    }
    let mut i1 = b1 - 0x18;
    if i1 < 0 {
        i1 += 0xec;
    }
    let mut i0 = b0 - 0x1c;
    if i0 < 0 {
        i0 += 0xf4;
    }

    let r = t(i3).rotate_left(1) ^ t(i1).rotate_left(3) ^ t(i2).rotate_left(2) ^ t(i0);
    let new_acc = accumulator.wrapping_add(r);
    let new_index = ((i3 as u32) << 24) | ((i2 as u32) << 16) | ((i1 as u32) << 8) | (i0 as u32);
    (new_acc, new_index, new_acc)
}

/// The advance `0x4531e0` on a held state, as the callers that keep it in the globals `[0x882664]`
/// (accumulator) and `[0x882668]` (lanes) run it, and as the weather code inlines it at
/// `0x68e31a`, `0x68e41b` and `0x68e83e`: the same adds of 0xbc, 0xd4, 0xec and 0xf4 as the
/// function.
pub struct Stream<'t> {
    /// The accumulator `[0x882664]`.
    pub acc: u32,
    /// The lanes `[0x882668]`.
    pub idx: u32,
    /// The table at [`TABLE_VA`], at least 256 bytes.
    pub table: &'t [u8],
}

impl Stream<'_> {
    /// One advance; returns the new accumulator.
    pub fn uint32(&mut self) -> u32 {
        let (acc, idx, ret) = crandom_uint32(self.acc, self.idx, self.table);
        self.acc = acc;
        self.idx = idx;
        ret
    }

    /// One advance read as the callers read it: the low 23 bits as the mantissa of a float in
    /// `[1, 2)`.
    pub fn unit12(&mut self) -> f32 {
        f32::from_bits((self.uint32() & 0x007f_ffff) | 0x3f80_0000)
    }
}

/// `NTempest::CRndSeed::SetSeed` (`0x44ea80`, `systems/random.md#state-and-seeding`): the
/// accumulator is the seed, and the lanes are `seed mod 47, 53, 59, 61`, unsigned, packed high
/// byte first and shifted left by 2. `__thiscall`, the seed on the stack, `ret 4`.
pub fn srand_seed_state(seed: u32) -> (u32, u32) {
    let s1 = (((seed % 47) << 24) | ((seed % 53) << 16) | ((seed % 59) << 8) | (seed % 61)) << 2;
    (seed, s1)
}

/// The float part of `RandomUnitVector2` (`0x44fd00`, `0x44fdc8` to `0x44fe17`,
/// `systems/random.md#unit-vectors`): the low 23 bits of `hash` as the mantissa of `m` in
/// `[1, 2)`, `a = (m − 1)·2π` stored as `f32`, and `(cos a, sin a)` from `fsincos`.
pub fn rand2_gradient(hash: u32) -> (f32, f32) {
    let u = f32::from_bits((hash & 0x007f_ffff) | 0x3f80_0000);
    let two_pi = f64::from(f32::from_bits(TWO_PI_BITS));
    let one = f64::from(f32::from_bits(ONE_BITS));
    let angle = ((f64::from(u) - one) * two_pi) as f32; // fld; fsub 1.0; fmul 2π; fstp dword
    let (sin, cos) = f64::from(angle).sin_cos(); // fsincos: the cosine in st0
    (cos as f32, sin as f32)
}

/// `RandomUnitVector3` (`0x44fe20`, `systems/random.md#unit-vectors`): the first advance gives
/// `z = m − 2` for a non-negative accumulator and `2 − m` for a negative one; the second gives the
/// angle `a = (m − 1)·2π` as `f32`; `r = √(1 − z²)` stored as `f32`; the result is
/// `(cos a·r, sin a·r, z)`. `ecx` is the output and `edx` the state. Returns the vector and the
/// new state.
pub fn rand_unit_vector_3d(acc: u32, idx: u32, table: &[u8]) -> ([f32; 3], (u32, u32)) {
    let two = f64::from(f32::from_bits(TWO_F32_BITS));
    let one = f64::from(f32::from_bits(ONE_BITS));
    let two_pi = f64::from(f32::from_bits(TWO_PI_BITS));

    let (acc1, idx1, _) = crandom_uint32(acc, idx, table);
    let u_z = f64::from(f32::from_bits((acc1 & 0x007f_ffff) | 0x3f80_0000));
    let z = if (acc1 as i32) < 0 {
        two - u_z
    } else {
        u_z - two
    } as f32; // fstp dword

    let (acc2, idx2, _) = crandom_uint32(acc1, idx1, table);
    let u_a = f64::from(f32::from_bits((acc2 & 0x007f_ffff) | 0x3f80_0000));
    let angle = ((u_a - one) * two_pi) as f32; // fld; fsub 1.0; fmul 2π; fstp dword

    let r = (one - f64::from(z) * f64::from(z)).sqrt() as f32; // fld z; fmul z; fsubr 1.0; fsqrt; fstp
    let (sin, cos) = f64::from(angle).sin_cos(); // fsincos
    let cos = cos as f32;
    let sin = sin as f32;
    let x = (f64::from(cos) * f64::from(r)) as f32; // fld cos; fmul r; fstp [out]
    let y = (f64::from(sin) * f64::from(r)) as f32; // fld sin; fmul r; fstp [out+4]

    ([x, y, z], (acc2, idx2))
}

pub mod msvc_rand;
