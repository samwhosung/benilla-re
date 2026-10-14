//! The loading bar's progress fraction, `0x406920`.

/// Progress-blend weights (`.rdata`): unphased 0.70 and 0.30 (`0x7ffd7c`, `0x7ffd78`), phased 0.35
/// and 0.15 (`0x7ffd70`, `0x7ffd6c`); 0.5 is `0x7ffa24`.
const W_A_UNPHASED: f32 = f32::from_bits(0x3f33_3333); // 0.70 (input A / 0x882bd0)
const W_B_UNPHASED: f32 = f32::from_bits(0x3e99_999a); // 0.30 (input B / 0x882df4)
const W_A_PHASED: f32 = f32::from_bits(0x3eb3_3333); // 0.35
const W_B_PHASED: f32 = f32::from_bits(0x3e19_999a); // 0.15
const HALF: f32 = f32::from_bits(0x3f00_0000);

/// The loading bar's fill in `[0, 1]` from the three progress inputs, `0x406920`.
///
/// `phase` is `0x882894`, `input_a` `0x882bd0` and `input_b` `0x882df4`; the result goes to
/// `0x882be4`. When `phase != 0`: `base = phase * 0.5` and weights (0.35, 0.15); else `base = 0` and
/// weights (0.70, 0.30). `f = (input_a * wA + base) + input_b * wB`, then `f < 0` gives 0 and
/// anything not below 1 gives 1, NaN included; -0 stays -0 (`0x406993` to `0x4069db`: an ordered
/// compare of 0 with `f`, then of `f` with 1, `test ah,5; jp`). The intermediates are `f64`; the `base` and `f` stores round to `f32`.
pub fn progress_fraction(phase: f32, input_a: f32, input_b: f32) -> f32 {
    let phased = phase != 0.0;
    let base = if phased {
        (phase as f64 * HALF as f64) as f32 // fstp [0x882be4] (f32) then reloaded
    } else {
        0.0
    };
    let (wa, wb) = if phased {
        (W_A_PHASED, W_B_PHASED)
    } else {
        (W_A_UNPHASED, W_B_UNPHASED)
    };
    let t1 = input_a as f64 * wa as f64 + base as f64; // A·wA then + base (fadd order)
    let f = (t1 + input_b as f64 * wb as f64) as f32; // + B·wB, fstp m32
    if f < 0.0 {
        0.0
    } else if f < 1.0 {
        f
    } else {
        1.0
    }
}
