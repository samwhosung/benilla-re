//! The model cache's noise table (`systems/models/cache.md#noise-table`), filled by
//! `M2CacheInitialize` (`0x706510`) and `ParticleRegistryGet` (`0x7ae010`).

/// One entry from a generator output `u`: the bits `(u & 0x7fffff) | 0x3f800000`, a float in
/// `[1, 2)`, less 1, stored as `f32` (`0x70680f` to `0x70683b`, `0x7ae13b` to `0x7ae167`).
pub fn noise_table_randf01(rng_word: u32) -> f32 {
    f32::from_bits((rng_word & 0x007f_ffff) | 0x3f80_0000) - 1.0
}
