//! The wheel texture generator `0x78b580` (`systems/ui/colorselect.md#a-missing-wheel-faults`) run
//! from the exe.
//!
//! The function fills a global vector's buffer with the 128 by 128 ARGB wheel. The test points that
//! vector at its own buffer, forces the generation path and halts before the texture is registered
//! (`0x58a980`, `0x449bf0`), so the exe's own polar conversion, HSV to RGB and pack run into the
//! buffer, which is compared dword for dword with `build_wheel_texture`. The vector's resize
//! (`0x526930`) is stubbed to do nothing.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::colorselect_wheel::build_wheel_texture as wheel_lut;

const INIT_FLAG: u64 = 0x00cf_50ac; // one-time-init bitflag (bit0 set -> skip atexit init block)

const VEC_COUNT: u64 = 0x00cf_50b4; // vector element count (must be 0 to ENTER generation)

const VEC_DATA: u64 = 0x00cf_50b8; // vector buffer base (reloaded every texel store)

const ALLOC_FN: u64 = 0x0052_6930; // std::vector resize (boundary) — stubbed no-op

#[test]
fn geo_78b580() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let buf = emu.alloc(0x1_0000); // 128*128 dwords
    let this = emu.alloc(0x400); // the colour-wheel frame object
    let stack = emu.alloc(0x2000); // manual call frame for `run`

    // Pin the global LUT vector to OUR buffer and force the generation path:
    //  - INIT_FLAG bit0 set -> skip the one-time atexit-init block (no 0x409aef call).
    //  - VEC_COUNT == 0      -> ENTER generation (a nonzero count short-circuits to register).
    //  - VEC_DATA -> our scratch buffer (re-read from the global every texel store).
    emu.write_u32(INIT_FLAG, 1);
    emu.write_u32(VEC_COUNT, 0);
    emu.write_u32(VEC_DATA, buf as u32);
    // The std::vector resize is boundary: no-op stub leaves COUNT==0, so the binary's own
    // zero-fill loop runs over OUR buffer, then it sets COUNT=0x4000 and fills.
    emu.stub(ALLOC_FN, 0, 2);

    // [this+0x318] = 0 (the current slot) so the entry's "release previous" vtable call is
    // skipped and arg0 (the new slot) differs from it.
    emu.write_u32(this + 0x318, 0);

    // Manual frame for `run` (entry runs the real prologue `push ebp; mov ebp,esp`): place arg0
    // at [esp+4] (=> [ebp+8]) and this in ECX. Halt before the register tail, so the function's
    // epilogue (and boundary register calls) never execute.
    let esp = stack + 0x1000;
    emu.write_u32(esp, 0); // return-addr placeholder (unused; we halt by `until`)
    emu.write_u32(esp + 4, 1); // arg0: the new texture slot (nonzero, != [this+0x318])
    emu.set_reg(RegisterX86::ESP, esp);
    emu.set_reg(RegisterX86::ECX, this);
    emu.run(0x78b580, 0x78b6f1); // 0x78b6f1 = first instruction of the register tail

    let mut mine = [0u32; 128 * 128];
    wheel_lut(&mut mine);

    let (mut inside, mut outside) = (0u32, 0u32);
    let mut idx = 0usize;
    for y in (-64i32..=63).rev() {
        for x in -64i32..=63 {
            let bin = emu.read_u32(buf + 4 * idx as u64);
            assert_eq!(
                mine[idx], bin,
                "geo_78b580 texel (x={x}, y={y}) idx={idx}: rust {:#010x} vs binary {:#010x}",
                mine[idx], bin
            );
            if x * x + y * y > 0x1000 {
                outside += 1;
            } else {
                inside += 1;
            }
            idx += 1;
        }
    }
    eprintln!(
        "geo_78b580: {} texels bit-exact (HSV colour-wheel LUT; ARGB 0xFFRRGGBB; \
         atan2/sqrt->libm, magic-number f32->byte pack) [disc-inside={inside} outside-zero={outside}]",
        128 * 128
    );
    assert!(
        inside > 0 && outside > 0,
        "geo_78b580: both disc regions must be exercised"
    );
}
