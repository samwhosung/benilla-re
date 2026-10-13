//! The model cache's noise table (`systems/models/cache.md#noise-table`), both fills run as slices.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::cache::noise_table_randf01;
use common::{assert_f32, next, SLICE_ESP};

/// The table, at `0xcf58f0`.
const NOISE_TABLE: u64 = 0x00cf_58f0;

/// `0x706510` from `0x70680f` to `0x70683b` and `0x7ae010` from `0x7ae13b` to `0x7ae167`: the
/// generator output in `eax`, the slot index 0 at `[ebp-0x4]` and `[ebp-0x8]`; the first entry
/// read back.
#[test]
fn noise_table_randf01_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let run = |e: &mut Emu, word: u32, ctx: &str| {
        for (begin, end, slot) in [(0x70680f, 0x70683b, 0x4), (0x7ae13b, 0x7ae167, 0x8)] {
            e.write_u32(ebp - slot, 0);
            e.write_u32(NOISE_TABLE, 0xdead_beef);
            e.set_reg(RegisterX86::EAX, u64::from(word));
            e.set_reg(RegisterX86::EBP, ebp);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.run(begin, end);
            let got = e.read_f32(NOISE_TABLE);
            assert_f32(
                got,
                noise_table_randf01(word),
                &format!("{ctx} at {begin:#x}"),
            );
        }
    };
    for (w, ctx) in [
        (0x0000_0000u32, "mantissa 0"),
        (0x007f_ffff, "mantissa all ones"),
        (0x0000_0001, "lowest bit"),
        (0x0040_0000, "top mantissa bit"),
        (0xffff_ffff, "all bits"),
        (0x1234_5678, "arbitrary"),
    ] {
        run(&mut e, w, ctx);
    }
    let mut s = 0x7065_1000u32;
    for i in 0..6000u32 {
        let w = next(&mut s);
        run(&mut e, w, &format!("fuzz[{i}]"));
    }
}
