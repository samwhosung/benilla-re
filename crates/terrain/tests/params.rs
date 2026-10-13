//! The falloff record and the far clip setter against `0x68a1f0` and `0x6725d0`.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_terrain::params::{falloff_recip, param_setter};
use common::{assert_f32, next};

/// A finite float from the high bits.
fn f32_finite(s: &mut u32) -> f32 {
    loop {
        let f = f32::from_bits(next(s));
        if f.is_finite() {
            return f;
        }
    }
}

/// `InitFalloff` (`0x68a1f0`) called whole: the computed fields and the copies.
#[test]
fn falloff_recip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x80);
    let src = emu.alloc(0x10);
    let mut s = 0x68a1_f000u32;
    let mut checked = 0u32;

    // zeros, tiny and huge radii, powers of two, an infinity
    let mut cases: Vec<(f32, f32)> = vec![
        (1.0, 1.0),
        (2.0, 4.0),
        (10.0, 20.0),
        (0.5, 0.25),
        (0.0, 1.0),
        (-0.0, 1.0),
        (1.0, 0.0),
        (1e-30, 1e30),
        (-3.0, -7.0),
        (f32::INFINITY, 1.0),
        (123.456, 0.001),
    ];
    for _ in 0..600u32 {
        let r1 = if (next(&mut s) >> 8).is_multiple_of(2) {
            (next(&mut s) % 100_000) as f32 * 0.01 - 500.0
        } else {
            f32_finite(&mut s)
        };
        let r2 = if (next(&mut s) >> 8).is_multiple_of(2) {
            (next(&mut s) % 100_000) as f32 * 0.01 - 500.0
        } else {
            f32_finite(&mut s)
        };
        cases.push((r1, r2));
    }

    for (r1, r2) in cases {
        let sv = [next(&mut s), next(&mut s), next(&mut s)];
        for (i, w) in sv.iter().enumerate() {
            emu.write_u32(src + i as u64 * 4, *w);
        }
        let a14 = next(&mut s);
        let a18 = next(&mut s);
        let a1c = next(&mut s);
        for i in 0..0x20u64 {
            emu.write_u32(this + i * 4, 0xdead_beef);
        }
        emu.call(
            0x68a1f0,
            this as u32,
            0,
            &[src as u32, r1.to_bits(), r2.to_bits(), a14, a18, a1c],
        );
        let ours = falloff_recip(r1, r2);
        assert_f32(emu.read_f32(this + 0x18), ours.inv_r1, "1/r1 (this+0x18)");
        assert_f32(emu.read_f32(this + 0x1c), ours.inv_r2, "1/r2 (this+0x1c)");
        assert_f32(emu.read_f32(this + 0x10), ours.neg_r1, "-r1 (this+0x10)");
        // the copies and the zeroed fields
        assert_eq!(emu.read_u32(this + 0x20), sv[0], "src[0] copy");
        assert_eq!(emu.read_u32(this + 0x24), sv[1], "src[1] copy");
        assert_eq!(emu.read_u32(this + 0x28), sv[2], "src[2] copy");
        assert_eq!(emu.read_u32(this + 0x2c), r1.to_bits(), "r1 copy");
        assert_eq!(emu.read_u32(this + 0x30), r2.to_bits(), "r2 copy");
        assert_eq!(emu.read_u32(this + 0x34), a14, "arg14 copy");
        assert_eq!(emu.read_u32(this + 0x38), a18, "arg18 copy");
        assert_eq!(emu.read_u32(this + 0x3c), a1c, "arg1c copy");
        assert_eq!(emu.read_u32(this + 0xc), 0, "this+0xc zero");
        assert_eq!(emu.read_u32(this + 0x14), 0, "this+0x14 zero");
        checked += 1;
    }
    eprintln!("falloff_recip: {checked} cases bit-exact vs WoW.exe @ 0x68a1f0 (range 0x68a1f0..0x68a253; fdiv@0x68a1fc fdiv@0x68a243 fchs@0x68a24c)");
}

/// `CWorld::SetFarClip` (`0x6725d0`) called whole: the stored value, the index and the three flags,
/// or nothing written when the value is the stored one.
#[test]
fn param_setter_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    const STORED: u64 = 0x00c7_b498;
    const IDX_A: u64 = 0x00c7_b2b8;
    const IDX_B: u64 = 0x00c7_b2bc;
    const FLAG_A: u64 = 0x00ca_82b0;
    const FLAG_B: u64 = 0x00c8_9d90;
    const FLAG_C: u64 = 0x00ca_8084;

    let mut s = 0x6725_d000u32;
    let mut checked = 0u32;
    let (mut changed, mut nochange) = (0u32, 0u32);

    let run_case = |emu: &mut Emu, stored: f32, new: f32, ctx: &str| -> bool {
        emu.write_u32(STORED, stored.to_bits());
        emu.write_u32(IDX_A, 0xdead_beef);
        emu.write_u32(IDX_B, 0xdead_beef);
        emu.write_u32(FLAG_A, 0);
        emu.write_u32(FLAG_B, 0);
        emu.write_u32(FLAG_C, 0);
        emu.call(0x6725d0, 0, 0, &[new.to_bits()]);
        let ours = param_setter(stored, new);
        let bstored = emu.read_u32(STORED);
        let bidx_a = emu.read_u32(IDX_A);
        let bidx_b = emu.read_u32(IDX_B);
        let bflag = (
            emu.read_u32(FLAG_A),
            emu.read_u32(FLAG_B),
            emu.read_u32(FLAG_C),
        );
        match ours.changed {
            true => {
                let sb = ours.stored_bits.expect("changed -> stored_bits");
                let idx = ours.index.expect("changed -> index");
                assert_eq!(bstored, sb, "{ctx}: stored value @0xc7b498");
                assert_eq!(bidx_a as i32, idx, "{ctx}: index @0xc7b2b8");
                assert_eq!(bidx_b as i32, idx, "{ctx}: index @0xc7b2bc");
                assert_eq!(bflag, (1, 1, 1), "{ctx}: dirty flags raised");
            }
            false => {
                // nothing written
                assert_eq!(bstored, stored.to_bits(), "{ctx}: stored unchanged");
                assert_eq!(bidx_a, 0xdead_beef, "{ctx}: index A untouched");
                assert_eq!(bidx_b, 0xdead_beef, "{ctx}: index B untouched");
                assert_eq!(bflag, (0, 0, 0), "{ctx}: flags untouched");
            }
        }
        ours.changed
    };

    // equal values write nothing
    for &v in &[
        0.0f32,
        -0.0,
        1.0,
        -0.03,
        100.0,
        -250.5,
        f32::from_bits(0x4048_f5c3),
    ] {
        let c = run_case(&mut emu, v, v, "boundary new==stored");
        assert!(!c, "new==stored must take the no-change path (v={v})");
        nochange += 1;
        checked += 1;
    }
    // −0.0 equals +0.0
    {
        let c = run_case(&mut emu, -0.0, 0.0, "boundary -0==+0");
        assert!(!c, "-0.0 == +0.0 -> no-change");
        nochange += 1;
        checked += 1;
    }

    // signs, truncation towards zero, large values
    for &(st, nw) in &[
        (0.0f32, 1.0),
        (1.0, 2.0),
        (5.0, -5.0),
        (0.0, 33.4),
        (0.0, -33.4),
        (0.0, 1000.0),
        (0.0, -1000.0),
        (0.0, 0.5),
    ] {
        let c = run_case(&mut emu, st, nw, "edge changed");
        assert!(c, "changed-path edge st={st} nw={nw}");
        changed += 1;
        checked += 1;
    }
    // NaN writes
    for &(st, nw) in &[(f32::NAN, 1.0f32), (1.0, f32::NAN), (f32::NAN, f32::NAN)] {
        let c = run_case(&mut emu, st, nw, "edge NaN");
        assert!(c, "NaN -> changed path st={st} nw={nw}");
        changed += 1;
        checked += 1;
    }

    let mut prev_new = 7.0f32;
    for _ in 0..800u32 {
        // an eighth of the cases equal
        let (stored, new) = if (next(&mut s) >> 8).is_multiple_of(8) {
            (prev_new, prev_new)
        } else {
            let st = (next(&mut s) % 200_000) as f32 * 0.005 - 500.0;
            let nw = if (next(&mut s) >> 8).is_multiple_of(2) {
                (next(&mut s) % 200_000) as f32 * 0.005 - 500.0
            } else {
                f32_finite(&mut s)
            };
            (st, nw)
        };
        if run_case(&mut emu, stored, new, "fuzz") {
            changed += 1;
        } else {
            nochange += 1;
        }
        prev_new = new;
        checked += 1;
    }
    assert!(
        changed > 0 && nochange > 0,
        "both branches must fire: changed={changed} nochange={nochange}"
    );
    eprintln!("param_setter: {checked} cases bit-exact vs WoW.exe @ 0x6725d0 (range 0x6725d0..0x672622; fcomp@0x6725d9 fmul@0x6725e9; changed={changed} nochange={nochange})");
}
