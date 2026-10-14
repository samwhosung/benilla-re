//! The arithmetic of `0x6769d0` and `0x677ff0` against the exe, each stretch run as the exe's own
//! instructions between two addresses on a frame laid out in the emulator. Three shapes: in place
//! (the slice starts clean and stores a slot that is read back); load then body (a value the exe
//! leaves wide on the stack from a call or a clamp is loaded from a double, the slice's bytes are
//! copied after it and a `fstp` is appended, a `qword` where the result is wide); and a copy of a
//! slice that makes a call, with the call patched to its original callee.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::wx_snow::{
    snow_exp2_arg, snow_exp2_blend, snow_field_lerp, snow_final_recip, snow_min_select_scale,
    snow_normalize_scale, snow_record_distance, snow_scroll_matrix, snow_wind_deflect, SnowScroll,
};
use common::next;

const G_C7CF20: u64 = 0x00c7_cf20;
const G_C7B550: u64 = 0x00c7_b550;
const G_C7B56C: u64 = 0x00c7_b56c;
const G_C62510: u64 = 0x00c6_2510;

/// Compare two f32 bit for bit, any two NaN equal.
fn assert_field(binary: f32, ours: f32, tag: &str, field: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{tag}.{field}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// A fuzzed f32: random bits, and finite values of several scales and signs.
fn fuzz(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let bits = next(s);
    match pick {
        0 => f32::from_bits(bits),
        1 => (f64::from(bits >> 4) / f64::from(1u32 << 20) - 128.0) as f32,
        2 => (f64::from(bits >> 8) / f64::from(1u32 << 23) * 2000.0 - 1000.0) as f32,
        3 => (f64::from(bits >> 8) / f64::from(1u32 << 23) * 2.0 - 1.0) as f32,
        4 => f32::from_bits(0x3f80_0000 | (bits >> 9)),
        _ => -f32::from_bits(0x3f00_0000 | (bits >> 9)),
    }
}

/// A fuzzed f64: random bits, and small finite values.
fn fuzz_f64(s: &mut u32) -> f64 {
    let hi = next(s);
    let lo = next(s);
    match (next(s) >> 8) % 4 {
        0 => f64::from_bits((u64::from(hi) << 32) | u64::from(lo)),
        1 => f64::from(hi) / f64::from(1u32 << 24) * 4.0 - 2.0,
        2 => f64::from(hi as i32) / 1.0e6,
        _ => f64::from(fuzz(s)),
    }
}

/// Assemble an island: `fld qword [live]` pushes the value the exe leaves wide, then the copied
/// `body` bytes (position independent), then `fstp dword [out]` (`qword` when asked). Returns the
/// entry, the end and the slot.
fn build_body_island(emu: &mut Emu, live: f64, body: &[u8], qword: bool) -> (u64, u64, u64) {
    let live_slot = emu.alloc(8);
    emu.write(live_slot, &live.to_bits().to_le_bytes());
    let out = emu.alloc(8);
    let mut code: Vec<u8> = Vec::new();
    code.push(0xdd);
    code.push(0x05);
    code.extend_from_slice(&(live_slot as u32).to_le_bytes());
    code.extend_from_slice(body);
    code.push(if qword { 0xdd } else { 0xd9 });
    code.push(0x1d);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let island = emu.alloc(code.len() as u64);
    emu.write(island, &code);
    (island, island + code.len() as u64, out)
}

/// `0x6769d0` from `0x676c37` to `0x676cb6`: exact, signed, zero, NaN and infinite rows, and a
/// fuzz.
#[test]
fn rain_patter_matrix_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x200);
    let frame = emu.alloc(0x200) + 0x100;
    let esp_top = frame + 0x80;
    let mut s = 0x534e_4f57_u32;

    let run = |emu: &mut Emu, a: [f32; 3], b: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x5c, a[0].to_bits());
        emu.write_u32(frame - 0x4c, a[1].to_bits());
        emu.write_u32(frame - 0x3c, a[2].to_bits());
        emu.write_u32(frame - 0x58, b[0].to_bits());
        emu.write_u32(frame - 0x48, b[1].to_bits());
        emu.write_u32(frame - 0x38, b[2].to_bits());
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x676c37, 0x676cb6);
        let binary = SnowScroll {
            m12: [
                emu.read_f32(frame - 0x1c),
                emu.read_f32(frame - 0x18),
                emu.read_f32(frame - 0x14),
            ],
            m6: [
                emu.read_f32(frame - 0x68),
                emu.read_f32(frame - 0x64),
                emu.read_f32(frame - 0x60),
            ],
            neg12: [
                emu.read_f32(frame - 0xc),
                emu.read_f32(frame - 8),
                emu.read_f32(frame - 4),
            ],
        };
        let ours = snow_scroll_matrix(a, b);
        for k in 0..3 {
            assert_field(binary.m12[k], ours.m12[k], tag, &format!("m12[{k}]"));
            assert_field(binary.m6[k], ours.m6[k], tag, &format!("m6[{k}]"));
            assert_field(binary.neg12[k], ours.neg12[k], tag, &format!("neg12[{k}]"));
        }
    };

    let probes: [([f32; 3], [f32; 3]); 6] = [
        ([12.0, 6.0, 3.0], [6.0, 3.0, 1.0]),
        ([0.0, -0.0, 1.0], [0.0, -0.0, 1.0]),
        ([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]),
        ([f32::NAN, 1.0, 2.0], [1.0, 1.0, 1.0]),
        ([f32::INFINITY, f32::NEG_INFINITY, 0.0], [0.0, 0.0, 0.0]),
        ([0.333_333_34, 0.1, 7.0], [0.333_333_34, 0.1, 7.0]),
    ];
    for &(a, b) in &probes {
        run(&mut emu, a, b, "probe");
    }
    for _ in 0..400 {
        let a = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let b = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, a, b, "fuzz");
    }
}

/// `0x677ff0` from `0x6780d8` to `0x678105`, with the call to `0x4549f0` relocated: 3-4-5, zero,
/// NaN, infinite, huge and tiny positions, and a fuzz.
#[test]
fn snow_record_distance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rec_blk = emu.alloc(0x31000);
    let pos_blk = emu.alloc(0x20);
    let frame = emu.alloc(0x100) + 0x80;
    let esp_top = frame + 0x40;
    let mut s = 0x4449_5354_u32;

    let call_off = 0x6780fe - 0x6780d8;
    let slice = emu.read(0x6780d8, 0x678105 - 0x6780d8);
    assert_eq!(
        slice[call_off], 0xe8,
        "snow_record_distance: expected e8 call at slice-offset 38"
    );
    let out = emu.alloc(8);
    let mut code = slice;
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let island = emu.alloc(code.len() as u64);
    let call_site = island + call_off as u64;
    let rel = (0x4549f0_i64 - (call_site as i64 + 5)) as i32;
    code[call_off + 1..call_off + 5].copy_from_slice(&rel.to_le_bytes());
    emu.write(island, &code);
    let island_end = island + code.len() as u64;

    let run = |emu: &mut Emu, rec: [f32; 3], pos: [f32; 3], tag: &str| {
        emu.write_u32(rec_blk + 0x3001c, rec[0].to_bits());
        emu.write_u32(rec_blk + 0x30020, rec[1].to_bits());
        emu.write_u32(rec_blk + 0x30024, rec[2].to_bits());
        emu.write_u32(pos_blk, pos[0].to_bits());
        emu.write_u32(pos_blk + 4, pos[1].to_bits());
        emu.write_u32(pos_blk + 8, pos[2].to_bits());
        emu.write_u32(out, 0xdead_beef);
        emu.set_reg(RegisterX86::EAX, pos_blk);
        emu.set_reg(RegisterX86::EBX, rec_blk);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(island, island_end);
        let binary = emu.read_f32(out);
        let ours = snow_record_distance(rec, pos);
        assert_field(binary, ours, tag, "len");
    };

    let probes: [([f32; 3], [f32; 3]); 7] = [
        ([3.0, 4.0, 0.0], [0.0, 0.0, 0.0]),
        ([1.0, 2.0, 3.0], [1.0, 2.0, 3.0]),
        ([1.0, 2.0, 2.0], [0.0, 0.0, 0.0]),
        ([f32::NAN, 1.0, 0.0], [0.0, 0.0, 0.0]),
        ([f32::INFINITY, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([1.0e19, 1.0e19, 1.0e19], [0.0, 0.0, 0.0]),
        ([1.0e-20, 1.0e-20, 1.0e-20], [0.0, 0.0, 0.0]),
    ];
    for &(rec, pos) in &probes {
        run(&mut emu, rec, pos, "probe");
    }
    for _ in 0..400 {
        let rec = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let pos = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, rec, pos, "fuzz");
    }
}

/// `0x677ff0` from `0x678270` to `0x678317`: the gate at 0.001 exactly, zero, below 0, between 0
/// and 1 and above 1 after the blend, NaN wind, NaN endpoints, and a fuzz.
#[test]
fn snow_wind_deflect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mgr = emu.alloc(0x100);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x5749_4e44_u32;

    let run =
        |emu: &mut Emu, m: f32, e10: f32, ec: f32, glob: [f32; 3], rec: [f32; 3], tag: &str| {
            emu.write_u32(mgr + 0x7c, m.to_bits());
            emu.write_u32(mgr + 0x68, rec[0].to_bits());
            emu.write_u32(mgr + 0x6c, rec[1].to_bits());
            emu.write_u32(mgr + 0x70, rec[2].to_bits());
            emu.write_u32(G_C7CF20, glob[0].to_bits());
            emu.write_u32(G_C7CF20 + 4, glob[1].to_bits());
            emu.write_u32(G_C7CF20 + 8, glob[2].to_bits());
            emu.write_u32(frame - 0x10, e10.to_bits());
            emu.write_u32(frame - 0xc, ec.to_bits());
            emu.set_reg(RegisterX86::ECX, mgr);
            emu.set_reg(RegisterX86::EBP, frame);
            emu.run(0x678270, 0x678317);
            let binary = emu.read_f32(frame - 0xc);
            let ours = snow_wind_deflect(m, e10, ec, glob, rec);
            assert_field(binary, ours, tag, "v");
        };

    let eps = f32::from_bits(0x3a83_126f);
    type WindProbe = (f32, f32, f32, [f32; 3], [f32; 3]);
    let probes: [WindProbe; 9] = [
        (eps, 2350.0, 2350.0, [0.0; 3], [0.0; 3]),
        (0.0, 0.0, 1.0, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0]),
        (0.5, 1000.0, 5000.0, [0.0; 3], [0.0; 3]),
        (0.5, -2350.0, 2350.0, [0.0; 3], [0.0; 3]),
        (0.5, -4350.0, 2350.0, [0.0; 3], [0.0; 3]),
        (0.002, 1.0, 1.0, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0]),
        (f32::NAN, 1.0, 1.0, [0.0; 3], [0.0; 3]),
        (0.5, f32::NAN, 1.0, [0.0; 3], [0.0; 3]),
        (0.0001, 0.0, f32::NAN, [1.0, 1.0, 1.0], [0.0; 3]),
    ];
    for &(m, e10, ec, g, r) in &probes {
        run(&mut emu, m, e10, ec, g, r, "probe");
    }
    for _ in 0..500 {
        let m = if (next(&mut s) >> 8) & 1 == 0 {
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 0.004 - 0.002) as f32
        } else {
            fuzz(&mut s)
        };
        let g = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let r = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, m, fuzz(&mut s), fuzz(&mut s), g, r, "fuzz");
    }
}

/// `0x677ff0` from `0x67833d` to `0x6783a8`: zero, 3-4-0, lengths below and above 2⁻²², NaN,
/// infinite, tiny, huge and -0 vectors, and a fuzz.
#[test]
fn snow_normalize_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let esp_top = frame + 0x40;
    let mut s = 0x4e4f_524d_u32;

    let run = |emu: &mut Emu, v: [f32; 3], m: f32, tag: &str| {
        emu.write_u32(frame - 0x1c, v[0].to_bits());
        emu.write_u32(frame - 0x18, v[1].to_bits());
        emu.write_u32(frame - 0x14, v[2].to_bits());
        emu.write_u32(frame - 0x4, m.to_bits());
        emu.write_u32(frame - 0x34, 0xdead_beef);
        emu.write_u32(frame - 0x30, 0xdead_beef);
        emu.write_u32(frame - 0x2c, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x67833d, 0x6783a8);
        let binary = [
            emu.read_f32(frame - 0x34),
            emu.read_f32(frame - 0x30),
            emu.read_f32(frame - 0x2c),
        ];
        let ours = snow_normalize_scale(v, m);
        for k in 0..3 {
            assert_field(binary[k], ours[k], tag, &format!("scaled[{k}]"));
        }
    };

    let probes: [([f32; 3], f32); 8] = [
        ([0.0, 0.0, 0.0], 2.0),
        ([3.0, 4.0, 0.0], 2.0),
        ([1.0e-5, 0.0, 0.0], 3.0),
        ([f32::NAN, 1.0, 1.0], 1.0),
        ([f32::INFINITY, 0.0, 0.0], 1.0),
        ([1.0e-9, 1.0e-9, 1.0e-9], 1.0),
        ([-0.0, -0.0, -0.0], 5.0),
        ([1.0e20, 1.0e20, 1.0e20], 1.0),
    ];
    for &(v, m) in &probes {
        run(&mut emu, v, m, "probe");
    }
    for _ in 0..400 {
        let v = if (next(&mut s) >> 8) & 1 == 0 {
            let sc = 1.0e-7;
            [
                (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * sc) as f32,
                (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * sc) as f32,
                (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * sc) as f32,
            ]
        } else {
            [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)]
        };
        run(&mut emu, v, fuzz(&mut s), "fuzz");
    }
}

/// `0x677ff0` from `0x678405` to `0x67841a`: the argument stored as a double, for zero, signed,
/// large, NaN, infinite and sub-f32 inputs, and a fuzz.
#[test]
fn snow_exp2_arg_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x100) + 0x80;
    let espbase = emu.alloc(0x40) + 0x20;
    let mut s = 0x4558_3241_u32;

    let run = |emu: &mut Emu, w: f32, tag: &str| {
        emu.write_u32(frame - 0x14, w.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, espbase);
        emu.run(0x678405, 0x67841a);
        let lo = emu.read_u32(espbase - 8);
        let hi = emu.read_u32(espbase - 4);
        let binary = f64::from_bits((u64::from(hi) << 32) | u64::from(lo));
        let ours = snow_exp2_arg(w);
        if binary.is_nan() && ours.is_nan() {
        } else {
            assert_eq!(
                binary.to_bits(),
                ours.to_bits(),
                "{tag}: binary={:#018x} ours={:#018x} (w={w})",
                binary.to_bits(),
                ours.to_bits()
            );
        }
    };

    for &(w, tag) in &[
        (0.0_f32, "z"),
        (1.0, "one"),
        (-1.0, "neg"),
        (10.0, "ten"),
        (f32::NAN, "nan"),
        (f32::INFINITY, "inf"),
        (0.333_333_34, "subf32"),
    ] {
        run(&mut emu, w, tag);
    }
    for _ in 0..400 {
        run(&mut emu, fuzz(&mut s), "fuzz");
    }
}

/// `0x677ff0` from `0x67841f` to `0x678433` (the bytes are checked against the image): the wide
/// quotient, compared as a double, for NaN and infinite `e` and an empty span, and a fuzz.
#[test]
fn snow_exp2_blend_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x4558_3242_u32;

    let body = emu.read(0x67841f, 0x678433 - 0x67841f);
    assert_eq!(
        body,
        vec![
            0xd8, 0x25, 0x50, 0xb5, 0xc7, 0x00, 0xd9, 0x05, 0x6c, 0xb5, 0xc7, 0x00, 0xd8, 0x25,
            0x50, 0xb5, 0xc7, 0x00, 0xde, 0xf9,
        ],
        "snow_exp2_blend: body bytes must match 0x67841f verbatim"
    );

    let run = |emu: &mut Emu, e: f64, lo: f32, hi: f32, tag: &str| {
        emu.write_u32(G_C7B550, lo.to_bits());
        emu.write_u32(G_C7B56C, hi.to_bits());
        let (island, island_end, out) = build_body_island(emu, e, &body, true);
        emu.run(island, island_end);
        let mut b = [0u8; 8];
        b.copy_from_slice(&emu.read(out, 8));
        let binary = f64::from_bits(u64::from_le_bytes(b));
        let ours = snow_exp2_blend(e, lo, hi);
        if !(binary.is_nan() && ours.is_nan()) {
            assert_eq!(
                binary.to_bits(),
                ours.to_bits(),
                "{tag}: binary={binary:e} ours={ours:e} (e={e} lo={lo} hi={hi})"
            );
        }
    };

    let probes: [(f64, f32, f32); 7] = [
        (1.0, 0.0, 2.0),
        (0.0, 0.0, 1.0),
        (1.0, 1.0, 1.0),
        (3.0, 1.0, 5.0),
        (f64::NAN, 0.0, 1.0),
        (f64::INFINITY, 0.0, 1.0),
        (-2.0, -1.0, 1.0),
    ];
    for &(e, lo, hi) in &probes {
        run(&mut emu, e, lo, hi, "probe");
    }
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz_f64(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            "fuzz",
        );
    }
}

/// `0x677ff0` from `0x67845f` to `0x67846a` (the bytes are checked against the image): a wide `t`
/// (NaN included) against the eye heights, and a fuzz.
#[test]
fn snow_field_lerp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x100);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x464c_5250_u32;

    let body = emu.read(0x67845f, 0x67846a - 0x67845f);
    assert_eq!(
        body,
        vec![0xd9, 0x46, 0x18, 0xd8, 0x66, 0x0c, 0xd8, 0xc9, 0xd8, 0x6d, 0xe0,],
        "snow_field_lerp: body bytes must match 0x67845f verbatim"
    );

    let run = |emu: &mut Emu, t: f64, e18: f32, ec: f32, e20: f32, tag: &str| {
        emu.write_u32(obj + 0x18, e18.to_bits());
        emu.write_u32(obj + 0xc, ec.to_bits());
        emu.write_u32(frame - 0x20, e20.to_bits());
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        let (island, island_end, out) = build_body_island(emu, t, &body, false);
        emu.run(island, island_end);
        let binary = emu.read_f32(out);
        let ours = snow_field_lerp(t, e18, ec, e20);
        assert_field(binary, ours, tag, "lerp");
    };

    let probes: [(f64, f32, f32, f32); 7] = [
        (0.0, 5.0, 1.0, 10.0),
        (1.0, 5.0, 1.0, 10.0),
        (0.5, 8.0, 0.0, 0.0),
        (f64::NAN, 5.0, 1.0, 10.0),
        (0.5, f32::INFINITY, 0.0, 0.0),
        (0.333_333_34, 1.0, 0.5, 2.0),
        (0.333_333_333_333_333_3, 1.0, 0.5, 2.0),
    ];
    for &(t, e18, ec, e20) in &probes {
        run(&mut emu, t, e18, ec, e20, "probe");
    }
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz_f64(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            fuzz(&mut s),
            "fuzz",
        );
    }
}

/// `0x677ff0` from `0x67846f` to `0x678496`: the frame time at the cap, either side, zero, negative
/// and NaN.
#[test]
fn snow_min_select_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x100);
    let frame = emu.alloc(0x100) + 0x80;
    let mut s = 0x4d53_454c_u32;

    let run = |emu: &mut Emu, g: f32, ea0: f32, tag: &str| {
        emu.write_u32(G_C62510, g.to_bits());
        emu.write_u32(obj + 0xa0, ea0.to_bits());
        emu.write_u32(frame - 0x10, 0xdead_beef);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x67846f, 0x678499);
        let binary = emu.read_f32(frame - 0x10);
        let ours = snow_min_select_scale(g, ea0);
        assert_field(binary, ours, tag, "ms");
    };

    let cst = f32::from_bits(0x3c88_893b);
    let probes: [(f32, f32); 8] = [
        (cst, 100.0),
        (f32::from_bits(0x3c88_893a), 100.0),
        (f32::from_bits(0x3c88_893c), 100.0),
        (0.0, 50.0),
        (1.0, 2.0),
        (f32::NAN, 1.0),
        (-1.0, 3.0),
        (f32::INFINITY, 0.0),
    ];
    for &(g, ea0) in &probes {
        run(&mut emu, g, ea0, "probe");
    }
    for _ in 0..400 {
        let g = if (next(&mut s) >> 8) & 1 == 0 {
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 0.04 - 0.01) as f32
        } else {
            fuzz(&mut s)
        };
        run(&mut emu, g, fuzz(&mut s), "fuzz");
    }
}

/// `0x677ff0` from `0x67854e`: `6144 / ea0` for positive, zero and infinite values and a fuzz.
#[test]
fn snow_final_recip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x100);
    let edi_blk = emu.alloc(0x40);
    let esp_top = emu.alloc(0x100) + 0x80;
    let mut s = 0x5245_4350_u32;

    let run = |emu: &mut Emu, ea0: f32, tag: &str| {
        emu.write_u32(obj + 0xa0, ea0.to_bits());
        emu.write_u32(edi_blk + 0xc, 1);
        emu.write_u32(edi_blk + 0x10, 1);
        emu.set_reg(RegisterX86::ESI, obj);
        emu.set_reg(RegisterX86::EDI, edi_blk);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x67854e, 0x678572);
        let binary = emu.read_f32(esp_top - 4);
        let ours = snow_final_recip(ea0);
        assert_field(binary, ours, tag, "recip");
    };

    let probes: [f32; 9] = [
        6144.0,
        1.0,
        2.0,
        0.0,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        1.0e30,
        1.0e-30,
    ];
    for &ea0 in &probes {
        run(&mut emu, ea0, "probe");
    }
    for _ in 0..400 {
        run(&mut emu, fuzz(&mut s), "fuzz");
    }
}
