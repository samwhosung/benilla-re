//! The frame update pieces against `0x68f9f0`, `0x698390`, `0x68de40` and `0x6bd2a0`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::maptriage::{
    dda_debug_negate3, fade_count_index, fade_time_gate, liquid_lazy_init_scalar, ripple_negate,
};
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

/// `AddRipple` (`0x68f9f0`) from `0x68fa3d` to `0x68fa4e`: the argument at `[ebp+8]` negated in
/// place.
#[test]
fn ripple_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400) + 0x200;
    let mut s = 0x68f9_0000u32;
    let mut checked = 0u32;

    let run_case = |emu: &mut Emu, p: f32| -> f32 {
        emu.write_u32(frame + 8, p.to_bits());
        emu.write_u32(0x00c800e0, 0);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x40);
        emu.set_reg(RegisterX86::EDX, 0);
        emu.run(0x68fa3d, 0x68fa4e);
        emu.read_f32(frame + 8)
    };

    // signs, zeros, infinities, NaN, a subnormal
    let mut cases: Vec<f32> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        3.5,
        -3.5,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x7fc0_0000),
        123.456,
        -0.001,
    ];
    for _ in 0..2000u32 {
        cases.push(f32_finite(&mut s));
    }
    for p in cases {
        let b = run_case(&mut emu, p);
        let o = ripple_negate(p);
        // bits, so a NaN's sign is compared too
        assert_eq!(
            b.to_bits(),
            o.to_bits(),
            "ripple_negate p={p}: binary={:#010x} ours={:#010x}",
            b.to_bits(),
            o.to_bits()
        );
        checked += 1;
    }
    eprintln!(
        "ripple_negate: {checked} cases bit-exact vs WoW.exe @ 0x68f9f0 (slice 0x68fa3d..0x68fa4e; fchs@0x68fa48)"
    );
}

/// `CMap::Update` (`0x698390`) from `0x69841f` to `0x698433`: AH after the compare, read through
/// `test ah,0x41; jne`.
#[test]
fn fade_time_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    const CUR: u64 = 0x00c7_b2a0;
    const LAST: u64 = 0x00ca_04f4;
    const THRESH: u64 = 0x0080_1628;
    assert_eq!(emu.read_f32(THRESH).to_bits(), 0x4000_0000); // 2.0

    let mut s = 0x6983_0000u32;
    let mut checked = 0u32;
    let (mut fire, mut skip) = (0u32, 0u32);

    let run_case = |emu: &mut Emu, cur: f32, last: f32| -> bool {
        emu.write_u32(CUR, cur.to_bits());
        emu.write_u32(LAST, last.to_bits());
        emu.write_u32(THRESH, 0x4000_0000);
        emu.run(0x69841f, 0x698433);
        let ah = (emu.get_reg(RegisterX86::EAX) >> 8) & 0xff;
        // `jne` skips when C0 or C3 is set
        (ah & 0x41) == 0
    };

    // a difference of exactly 2.0 skips
    for &(cur, last) in &[
        (2.0f32, 0.0f32),
        (5.0, 3.0),
        (102.0, 100.0),
        (-1.0, -3.0),
        (2.5, 0.5),
    ] {
        let b = run_case(&mut emu, cur, last);
        let o = fade_time_gate(cur, last);
        assert_eq!(
            b, o,
            "fade_time_gate ==2.0 boundary cur={cur} last={last}: binary_fire={b} ours={o}"
        );
        assert!(!b, "delta==2.0 must NOT fire (C3 -> jne taken -> skip)");
        skip += 1;
        checked += 1;
    }
    // one step above goes on, below skips
    for &(cur, last, want) in &[
        (f32::from_bits((2.0f32).to_bits() + 1) + 0.0, 0.0f32, true),
        (1.9999999f32, 0.0, false),
        (3.0, 0.9999999, true),
    ] {
        let b = run_case(&mut emu, cur, last);
        let o = fade_time_gate(cur, last);
        assert_eq!(b, o, "fade_time_gate near-boundary cur={cur} last={last}");
        assert_eq!(
            b, want,
            "fade_time_gate near-boundary expectation cur={cur}"
        );
        if b {
            fire += 1
        } else {
            skip += 1
        }
        checked += 1;
    }
    // NaN skips
    for &(cur, last) in &[(f32::NAN, 0.0f32), (0.0, f32::NAN), (f32::NAN, f32::NAN)] {
        let b = run_case(&mut emu, cur, last);
        let o = fade_time_gate(cur, last);
        assert_eq!(b, o, "fade_time_gate NaN cur={cur} last={last}");
        assert!(!b, "NaN delta must NOT fire");
        skip += 1;
        checked += 1;
    }

    for _ in 0..2000u32 {
        let cur = (next(&mut s) % 200_000) as f32 * 0.001 - 100.0;
        let last = (next(&mut s) % 200_000) as f32 * 0.001 - 100.0;
        let b = run_case(&mut emu, cur, last);
        let o = fade_time_gate(cur, last);
        assert_eq!(
            b, o,
            "fade_time_gate fuzz cur={cur} last={last}: binary_fire={b} ours={o}"
        );
        if b {
            fire += 1
        } else {
            skip += 1
        }
        checked += 1;
    }
    assert!(
        fire > 0 && skip > 0,
        "both gate outcomes must fire: fire={fire} skip={skip}"
    );
    eprintln!(
        "fade_time_gate: {checked} cases bit-exact vs WoW.exe @ 0x698390 grpA \
         (slice 0x69841f..0x698433; fsub@0x698425; fire={fire} skip={skip})"
    );
}

/// `0x698390` from `0x69844d` to `0x69845d`, the value at `[ebp-0x8]`: eax after `__ftol`.
#[test]
fn fade_count_index_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(emu.read_f32(0x0081061c).to_bits(), 0x3e00_0000); // 0.125

    let frame = emu.alloc(0x400) + 0x300;
    let mut s = 0x6984_4d00u32;
    let mut checked = 0u32;

    let run_case = |emu: &mut Emu, v: i64| -> i32 {
        let lo = (v as u64 & 0xffff_ffff) as u32;
        let hi = ((v as u64 >> 32) & 0xffff_ffff) as u32;
        emu.write_u32(frame - 0x8, lo);
        emu.write_u32(frame - 0x4, hi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x80);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.run(0x69844d, 0x69845d);
        emu.get_reg(RegisterX86::EAX) as u32 as i32
    };

    // the value the client passes (64), truncation towards zero, and values past `i64`
    let mut cases: Vec<i64> = vec![
        0,
        64,
        1,
        7,
        8,
        -8,
        -7,
        100,
        -100,
        1000,
        -1000,
        i64::MAX,
        i64::MIN,
        0x7fff_ffff,
        -0x8000_0000,
    ];
    for _ in 0..2000u32 {
        let mag = (u64::from(next(&mut s)) << 8 | u64::from(next(&mut s) >> 24)) as i64;
        let v = if (next(&mut s) >> 8).is_multiple_of(2) {
            mag % 100_000
        } else {
            mag
        };
        cases.push(v);
    }
    let (mut pos, mut neg, mut zero) = (0u32, 0u32, 0u32);
    for v in cases {
        let b = run_case(&mut emu, v);
        let o = fade_count_index(v);
        assert_eq!(b, o, "fade_count_index v={v}: binary={b} ours={o}");
        match o.cmp(&0) {
            std::cmp::Ordering::Greater => pos += 1,
            std::cmp::Ordering::Less => neg += 1,
            std::cmp::Ordering::Equal => zero += 1,
        }
        checked += 1;
    }
    assert!(
        pos > 0 && neg > 0 && zero > 0,
        "index must span +/-/0: pos={pos} neg={neg} zero={zero}"
    );
    eprintln!(
        "fade_count_index: {checked} cases bit-exact vs WoW.exe @ 0x698390 grpB \
         (slice 0x69844d..0x69845d; fmul@0x698452 + __ftol; pos={pos} neg={neg} zero={zero})"
    );
}

/// `CChunkLiquid::Draw` (`0x68de40`) from `0x68dea6` to `0x68deb7`: eax after `__ftol`; the bytes
/// from `0x68def5` are the same.
#[test]
fn liquid_lazy_init_scalar_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(emu.read_f32(0x0081023c).to_bits(), 0x3e00_0000); // 0.125
    assert_eq!(emu.read_f32(0x00808de4).to_bits(), 0x4280_0000); // 64.0

    // the `fld; fmul` at both sites are the same bytes
    let seq_c1 = emu.read(0x0068dea6, 12);
    let seq_c0 = emu.read(0x0068def5, 12);
    assert_eq!(
        seq_c1, seq_c0,
        "liquid lazy-init case1/case0 fld;fmul bytes must be identical for clone inheritance"
    );
    assert_eq!(
        &seq_c1[6..12],
        &[0xd8, 0x0d, 0xe4, 0x8d, 0x80, 0x00],
        "the fmul opcode must be `fmul dword [0x808de4]`"
    );

    let frame = emu.alloc(0x400) + 0x300;
    emu.set_reg(RegisterX86::ESP, frame - 0x80);
    emu.run(0x0068dea6, 0x0068deb7);
    let b = emu.get_reg(RegisterX86::EAX) as u32 as i32;
    let o = liquid_lazy_init_scalar();
    assert_eq!(b, o, "liquid_lazy_init_scalar: binary={b} ours={o}");
    assert_eq!(o, 8, "0.125 * 64.0 trunc == 8");
    eprintln!(
        "liquid_lazy_init_scalar: 1 slice bit-exact vs WoW.exe @ 0x68de40 \
         (slice 0x68dea6..0x68deb7; fmul@0x68deac == twin fmul@0x68defb by clone inheritance) result={b}"
    );
}

/// `DrawQueryCells` (`0x6bd2a0`) from `0x6bd2c4` to `0x6bd2fa`: the three negated values at
/// `[ebp-0xc]` up.
#[test]
fn dda_debug_negate3_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    const G0: u64 = 0x00c7_cf20;
    const G1: u64 = 0x00c7_cf24;
    const G2: u64 = 0x00c7_cf28;

    let frame = emu.alloc(0x400) + 0x200;
    let mut s = 0x6bd2_0000u32;
    let mut checked = 0u32;

    let run_case = |emu: &mut Emu, g: [f32; 3]| -> [f32; 3] {
        emu.write_u32(G0, g[0].to_bits());
        emu.write_u32(G1, g[1].to_bits());
        emu.write_u32(G2, g[2].to_bits());
        emu.write_u32(frame - 0xc, 0xdead_beef);
        emu.write_u32(frame - 0x8, 0xdead_beef);
        emu.write_u32(frame - 0x4, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, frame - 0x80);
        emu.run(0x006bd2c4, 0x006bd2fa);
        [
            emu.read_f32(frame - 0xc),
            emu.read_f32(frame - 0x8),
            emu.read_f32(frame - 0x4),
        ]
    };

    // signs, zeros, infinities, NaN, a subnormal
    let mut cases: Vec<[f32; 3]> = vec![
        [0.0, -0.0, 1.0],
        [-1.0, 2.5, -3.5],
        [f32::INFINITY, f32::NEG_INFINITY, 0.0],
        [f32::from_bits(0x7fc0_0000), 1.0, -1.0],
        [f32::from_bits(0x0000_0001), -0.0, 123.456],
    ];
    for _ in 0..2000u32 {
        cases.push([f32_finite(&mut s), f32_finite(&mut s), f32_finite(&mut s)]);
    }
    for g in cases {
        let b = run_case(&mut emu, g);
        let o = dda_debug_negate3(g);
        for i in 0..3 {
            assert_eq!(
                b[i].to_bits(),
                o[i].to_bits(),
                "dda_debug_negate3[{i}] g={g:?}: binary={:#010x} ours={:#010x}",
                b[i].to_bits(),
                o[i].to_bits()
            );
        }
        checked += 1;
    }
    eprintln!(
        "dda_debug_negate3: {checked} cases bit-exact vs WoW.exe @ 0x6bd2a0 \
         (slice 0x6bd2c4..0x6bd2fa; fchs@0x6bd2ca fchs@0x6bd2d9 fchs@0x6bd2ee)"
    );
}

/// `DrawQueryCells` (`0x6bd2a0`) from `0x6bd2c4` to `0x6bd2fa`: the camera at `0xc7cf20` written,
/// the negation read from `[ebp-0xc]` up.
#[test]
fn debug_dda_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100); // EBP frame: out at [ebp-0xc]/[ebp-0x8]/[ebp-0x4]; Z slice also
                                // writes [ebp-0x50]/[ebp-0x4c] (in frame)
    let ebp = frame + 0x80;
    const DDA_X: u64 = 0x00c7_cf20;
    const DDA_Y: u64 = 0x00c7_cf24;
    const DDA_Z: u64 = 0x00c7_cf28;
    const SLICE_ESP: u64 = 0x7008_0000;

    // Each leg is its own REAL slice over `fld [DAT]; fchs; fstp dword`. The runtime input slots
    // are the absolute .data VAs; we seed them per-case.
    let run_case = |e: &mut Emu, v: [f32; 3], ctx: &str| {
        e.write_u32(DDA_X, v[0].to_bits());
        e.write_u32(DDA_Y, v[1].to_bits());
        e.write_u32(DDA_Z, v[2].to_bits());
        // junk-seed the out slots.
        e.write_u32(ebp - 0xc, 0xdead_beef);
        e.write_u32(ebp - 0x8, 0xdead_beef);
        e.write_u32(ebp - 0x4, 0xdead_beef);

        // X leg: run(0x6bd2c4, 0x6bd2d2). fld [0xc7cf20]; fchs@6bd2ca; lea eax; fstp
        // [ebp-0xc]@6bd2cf.
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bd2c4, 0x6bd2d2);
        let bin_x = e.read_f32(ebp - 0xc);

        // Y leg: run(0x6bd2d2, 0x6bd2e1). fld [0xc7cf24]; push eax; fchs@6bd2d9; lea ecx; fstp
        // [ebp-0x8]@6bd2de. The `push eax` decrements ESP once — reset ESP first.
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bd2d2, 0x6bd2e1);
        let bin_y = e.read_f32(ebp - 0x8);

        // Z leg: run(0x6bd2e1, 0x6bd2fa). fld [0xc7cf28]; mov [ebp-0x50],imm; fchs@6bd2ee; mov
        // [ebp-0x4c],0; fstp [ebp-0x4]@6bd2f7.
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6bd2e1, 0x6bd2fa);
        let bin_z = e.read_f32(ebp - 0x4);

        let ours = benilla_re_terrain::maptriage::dda_debug_negate3(v);
        assert_f32(bin_x, ours[0], &format!("{ctx} -X"));
        assert_f32(bin_y, ours[1], &format!("{ctx} -Y"));
        assert_f32(bin_z, ours[2], &format!("{ctx} -Z"));
    };

    // boundary probes: ±0 (negate flips sign bit, bit-distinct), ±1, exact magnitudes.
    run_case(&mut e, [0.0, 0.0, 0.0], "dda v=0 → -0 each");
    run_case(&mut e, [-0.0, -0.0, -0.0], "dda v=-0 → +0 each");
    run_case(&mut e, [1.0, 2.0, 3.0], "dda (1,2,3) → (-1,-2,-3)");
    run_case(&mut e, [-1.0, -2.0, -3.0], "dda (-1,-2,-3) → (1,2,3)");
    run_case(&mut e, [12345.678, -9876.5, 0.001], "dda mixed");

    let mut s = 0x6464_6100u32; // "dda\0"
    for i in 0..6000u32 {
        let v = [
            f32::from_bits(next(&mut s)),
            f32::from_bits(next(&mut s)),
            f32::from_bits(next(&mut s)),
        ];
        run_case(&mut e, v, &format!("dda bits[{i}]"));
    }
}
