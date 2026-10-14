//! Helpers the animation tests share: a float compare that accepts any NaN for any NaN, the LCG,
//! random floats, and the selection of the math routine slots.

#![allow(dead_code)]

use benilla_re_difftest::Emu;

/// Asserts two floats have the same bits, or are both NaN.
pub fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// One LCG step. Its low bits repeat quickly, so a choice uses the high bits.
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A random `f32`: in `[-1, 1)`, `[-50, 50)`, `[0, 1e6)` or `[0, 1e-6)`, from the high bits.
pub fn fuzz_f32(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 4;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32, // [-1, 1)
        1 => (m * 100.0 - 50.0) as f32,
        2 => (m * 1.0e6) as f32,
        _ => (m * 1.0e-6) as f32,
    }
}

/// Runs the first-call selection `0x74fb49(1)` on the emulated processor, so the math slots
/// (`0x876504`, `0x876544`, `0x876588`, `0x876594`) hold what a client on that processor holds,
/// and returns the product slot.
///
/// What the run needs from outside the exe is supplied: page 0, where `fs:0` (the exception
/// chain that `0x74fa8a` links) lives; `0x74f951`, the registry read, which finds neither
/// `DisablePSGP` nor `DisableD3DXPSGP` (as on a machine without them); `0x750aae`, the one-time
/// library load whose result no slot depends on; and `IsProcessorFeaturePresent` (the import
/// at `0x7ff344`), which reports SSE (6) and not 3DNow (7). The processor tests run as the exe
/// runs them: `0x74fa8a` executes `cpuid` on the emulator.
pub fn select_math_slots(emu: &mut Emu) -> u32 {
    emu.map(0, 0x1000);
    emu.stub(0x750aae, 0, 0);
    emu.stub(0x74f951, 0, 4);
    // `mov eax,[esp+4]; cmp eax,6; sete al; movzx eax,al; ret 4`
    let api = emu.alloc(0x10);
    emu.write(
        api,
        &[
            0x8b, 0x44, 0x24, 0x04, 0x83, 0xf8, 0x06, 0x0f, 0x94, 0xc0, 0x0f, 0xb6, 0xc0, 0xc2,
            0x04, 0x00,
        ],
    );
    emu.write_u32(0x7ff344, api as u32);
    emu.call(0x74fb49, 0, 0, &[1]);
    emu.read_u32(0x876504)
}
