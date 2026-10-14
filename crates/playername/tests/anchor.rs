//! Bit-exact diff for `playername::anchor::anchor_lift` (`0x6c73f0`'s z-1/3 slice) vs `WoW.exe`.

#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_playername::anchor::anchor_lift;

// shared LCG fuzz (high bits only)
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
fn rngf(s: &mut u32, lo: f64, hi: f64) -> f64 {
    lo + f64::from(next(s) >> 8) / f64::from(1u32 << 24) * (hi - lo)
}

// 0x6c743e: fld dword [ebp-4]; fsub dword [0x807a40]=1/3 f32; fstp dword [ebp-4]  (halt after the fstp).
#[test]
fn pn_worldtext_anchor_lift() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let scratch = emu.alloc(0x40);
    let ebp = scratch + 0x20; // room for [ebp-0xc .. ebp+0x14] (the slice reads stack args ebp+0xc/0x10/0x14)
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00; // the slice does `push eax` (arg-marshal for the upcoming 0x6c8570 call)

    // The slice interleaves mov eax/ecx/edx,[ebp+0xc/0x10/0x14] + push eax (marshalling the call args)
    // with the 3 FP ops; only the FP result at [ebp-0x4] is observed. The stack args are
    // uninitialized (irrelevant to the arithmetic). Halt at 0x6c7454 (after the fstp, before push ecx).
    let run = |emu: &mut Emu, z: f32| -> f32 {
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write_u32(ebp - 0x4, z.to_bits());
        emu.run(0x6c743e, 0x6c7454); // fld [ebp-4]; (movs); fsub [1/3]; (movs); push eax; fstp [ebp-4]
        emu.read_f32(ebp - 0x4)
    };

    let mut total = 0u32;
    // deterministic boundary probes incl. the exact 1/3 (z - 1/3 == 0 boundary), signs, 0/-0.
    let probes = [
        0.0f32,
        -0.0,
        f32::from_bits(0x3eaa_aaab), // exactly 1/3 -> 0.0
        1.0,
        -1.0,
        1.0 / 3.0,
        100.0,
        -100.0,
        0.333_333_34,
        1e-7,
        -1e-7,
        1e9,
        -1e9,
    ];
    for &z in &probes {
        assert_eq!(
            run(&mut emu, z).to_bits(),
            anchor_lift(z).to_bits(),
            "anchor_lift z={z}"
        );
        total += 1;
    }
    let mut s = 0x6c73_f000u32;
    for _ in 0..5000 {
        let z = rngf(&mut s, -1000.0, 1000.0) as f32;
        assert_eq!(
            run(&mut emu, z).to_bits(),
            anchor_lift(z).to_bits(),
            "anchor_lift fuzz z={z}"
        );
        total += 1;
    }
    eprintln!("pn_worldtext_anchor_lift: {total} bit-exact (fld; fsub [1/3 f32]; fstp)");
}
