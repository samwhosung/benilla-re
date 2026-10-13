//! The C runtime's `rand` (`0x7400e5`, `systems/random.md#the-c-runtime-rand`), which Lua's
//! `math.random` (`0x7fb310`) calls: `s = s·214013 + 2531011` on the per-thread `_holdrand`
//! (`ptd+0x14`, 1 until `srand`), returning `(s >> 16) & 0x7fff`. Other C runtimes use other
//! generators, so the host's `rand` is not this one.

/// `RAND_MAX`, the mask `& 0x7fff`.
pub const RAND_MAX: i32 = 0x7fff;
/// The multiplier, `imul ecx,ecx,0x343fd`.
pub const MUL: u32 = 214_013;
/// The increment, `add ecx,0x269ec3`.
pub const INC: u32 = 2_531_011;

/// One step from `seed`: returns the new seed and the result, in `[0, 32767]`.
#[must_use]
pub fn rand_step(seed: u32) -> (u32, i32) {
    let new_seed = seed.wrapping_mul(MUL).wrapping_add(INC);
    let result = ((new_seed >> 16) & 0x7fff) as i32;
    (new_seed, result)
}
