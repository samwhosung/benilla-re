//! `HasNewMail` (`0x4afea0`) against the exe: the x87 compare `|x| < epsilon` and the condition bits
//! it leaves.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::frame_methods::fabs_cmp;
use fpkit::next;

#[test]
fn frameapi_fpleaf_fabs_cmp() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // fld [x]; fabs; fcomp [thr]; fnstsw ax — stop AT `test ah,5` (0x4afeb0), read the status word.
    let (st, en, x_a, thr_a) = (0x4afea0u64, 0x4afeb0u64, 0x845eacu64, 0x8029d4u64);
    let thr = emu.read_f32(thr_a); // 0x34800000 = 2^-22 ~ 2.38e-07
    let mut total = 0u32;
    let chk = |emu: &mut Emu, x: f32, total: &mut u32| {
        emu.write_u32(x_a, x.to_bits());
        emu.run(st, en);
        let sw = (emu.get_reg(RegisterX86::EAX) & 0xffff) as u32;
        let bin = (sw & 0x4000 != 0, sw & 0x0400 != 0, sw & 0x0100 != 0); // (C3, C2, C0)
        assert_eq!(
            bin,
            fabs_cmp(x, thr),
            "fabs_cmp x={x} ({:#010x}) thr={thr}: binary C3C2C0={bin:?} vs rust {:?}  sw={sw:#06x}",
            x.to_bits(),
            fabs_cmp(x, thr)
        );
        *total += 1;
    };
    // boundaries around ±thr, zeros, NaN, infinities, subnormals.
    let probes = [
        0.0f32,
        -0.0,
        thr,
        -thr,
        f32::from_bits(thr.to_bits() - 1),
        f32::from_bits(thr.to_bits() + 1),
        -f32::from_bits(thr.to_bits() + 1),
        1.0,
        -1.0,
        1e20,
        -1e20,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
    ];
    for x in probes {
        chk(&mut emu, x, &mut total);
    }
    let mut s = 0x4afe_a000u32;
    for _ in 0..8000 {
        // mix wide draws with values clustered near the epsilon threshold.
        let x = if next(&mut s) & 1 == 0 {
            f32::from_bits(next(&mut s))
        } else {
            f32::from_bits(
                thr.to_bits()
                    .wrapping_add(next(&mut s) & 0x1f)
                    .wrapping_sub(0x10),
            )
        };
        chk(&mut emu, x, &mut total);
    }
    eprintln!("frameapi_fpleaf_fabs_cmp: {total} bit-exact (thr={thr})");
}
