//! The arithmetic of `0x67ae20` against the exe, each stretch run as the exe's own instructions
//! between two addresses on a frame laid out in the emulator. A value the exe leaves wide from a
//! call (the tick value of `0x674600`, the ramp result of `0x67add0`) is loaded from a double by a
//! jump island; a value it leaves wide into a compare (the cell interpolation) is captured with a
//! `fstp qword`.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::wx_mistrender::{
    mistr_cell_lerp, mistr_cell_round, mistr_color_repack, mistr_corner_add, mistr_corner_basis,
    mistr_corner_dist, mistr_drift_clamp, mistr_drift_gate, mistr_fade_mag, mistr_half_scale,
    mistr_hash_count, mistr_life_span, mistr_negate, mistr_node_accum, mistr_phase_advance,
    mistr_phase_wrap, mistr_ramp_product, mistr_spacing_recip, mistr_time_accum, CellRound,
    CornerBasis, CornerDist, NodeAccum,
};
use common::next;

/// Compare two f32 bit for bit, any two NaN equal.
fn af(binary: f32, ours: f32, tag: &str, field: &str) {
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
fn fz(s: &mut u32) -> f32 {
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
fn fz64(s: &mut u32) -> f64 {
    let hi = next(s);
    let lo = next(s);
    match (next(s) >> 8) % 4 {
        0 => f64::from_bits((u64::from(hi) << 32) | u64::from(lo)),
        1 => f64::from(hi) / f64::from(1u32 << 24) * 4.0 - 2.0,
        2 => f64::from(hi as i32) / 1.0e6,
        _ => f64::from(fz(s)),
    }
}

/// Build an island that loads each of `inputs` as a double onto the x87 stack and jumps to
/// `target`; returns its entry.
fn ld_jmp(emu: &mut Emu, inputs: &[f64], target: u64) -> u64 {
    let mut code: Vec<u8> = Vec::new();
    for &v in inputs {
        let slot = emu.alloc(8);
        emu.write(slot, &v.to_bits().to_le_bytes());
        code.push(0xdd);
        code.push(0x05);
        code.extend_from_slice(&(slot as u32).to_le_bytes());
    }
    let island = emu.alloc((code.len() + 5) as u64);
    let jmp_addr = island + code.len() as u64;
    let rel = (target as i64 - (jmp_addr as i64 + 5)) as i32;
    code.push(0xe9);
    code.extend_from_slice(&rel.to_le_bytes());
    emu.write(island, &code);
    island
}

/// Run the copied bytes `src..src+len` then a `fstp qword` into `slot` and return the stored double,
/// which keeps all 53 bits of a value the exe leaves wide.
fn island_f64(emu: &mut Emu, island: u64, slot: u64, src: u64, len: usize) -> f64 {
    let mut code = emu.read(src, len);
    code.push(0xdd);
    code.push(0x1d);
    code.extend_from_slice(&(slot as u32).to_le_bytes());
    emu.write(island, &code);
    emu.run(island, island + code.len() as u64);
    let mut b = [0u8; 8];
    b.copy_from_slice(&emu.read(slot, 8));
    f64::from_bits(u64::from_le_bytes(b))
}

/// `0x67ae66` to `0x67ae96`: the half extent for zero, ordinary, NaN and huge extents and a fuzz.
#[test]
fn mistr_half_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x40);
    let frame = emu.alloc(0x200) + 0x140;
    let mut s = 0x6861_6c66_u32;

    let run = |emu: &mut Emu, extent: f32, tag: &str| {
        emu.write_u32(obj + 0x1c, extent.to_bits());
        emu.write_u32(frame - 0x18, 0xdead_beef);
        emu.set_reg(RegisterX86::EDI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x67ae66, 0x67ae96);
        af(
            emu.read_f32(frame - 0x18),
            mistr_half_scale(extent),
            tag,
            "half",
        );
    };
    run(&mut emu, 2.0, "two");
    run(&mut emu, 0.0, "zero");
    run(&mut emu, f32::NAN, "nan");
    run(&mut emu, 0.333_333_34, "subf32");
    for _ in 0..300 {
        run(&mut emu, fz(&mut s), "fuzz");
    }
}

/// `0x67af1d` to `0x67af7b`: the two matrix rows times the half extent.
#[test]
fn mistr_corner_basis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x140;
    let mut s = 0x6273_6973_u32;

    let run = |emu: &mut Emu, r0: [f32; 3], r1: [f32; 3], half: f32, tag: &str| {
        emu.write_u32(frame - 0xc4, r0[0].to_bits());
        emu.write_u32(frame - 0xb4, r0[1].to_bits());
        emu.write_u32(frame - 0xa4, r0[2].to_bits());
        emu.write_u32(frame - 0xc0, r1[0].to_bits());
        emu.write_u32(frame - 0xb0, r1[1].to_bits());
        emu.write_u32(frame - 0xa0, r1[2].to_bits());
        emu.write_u32(frame - 0x18, half.to_bits());
        for off in [0x54u64, 0x50, 0x4c, 0x40, 0x3c, 0x38] {
            emu.write_u32(frame - off, 0xdead_beef);
        }
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x67af1d, 0x67af7b);
        let binary = CornerBasis {
            a0: emu.read_f32(frame - 0x54),
            a1: emu.read_f32(frame - 0x50),
            a2: emu.read_f32(frame - 0x4c),
            b0: emu.read_f32(frame - 0x40),
            b1: emu.read_f32(frame - 0x3c),
            b2: emu.read_f32(frame - 0x38),
        };
        let ours = mistr_corner_basis(r0, r1, half);
        af(binary.a0, ours.a0, tag, "a0");
        af(binary.a1, ours.a1, tag, "a1");
        af(binary.a2, ours.a2, tag, "a2");
        af(binary.b0, ours.b0, tag, "b0");
        af(binary.b1, ours.b1, tag, "b1");
        af(binary.b2, ours.b2, tag, "b2");
    };
    run(&mut emu, [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 0.5, "ident");
    run(&mut emu, [2.0, 3.0, 4.0], [5.0, 6.0, 7.0], 1.5, "basic");
    run(&mut emu, [f32::NAN, 1.0, 1.0], [1.0, 1.0, 1.0], 0.5, "nan");
    run(
        &mut emu,
        [0.333_333_34, 0.111_111_1, 0.142_857_15],
        [0.7, 0.3, 0.9],
        0.25,
        "subf32",
    );
    for _ in 0..300 {
        run(
            &mut emu,
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            fz(&mut s),
            "fuzz",
        );
    }
}

/// `0x67af7b` to `0x67b098`: the four quad corners, with the negated components; every stored value
/// against the transcription.
#[test]
fn mistr_corner_add_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x140;
    let esp = emu.alloc(0x80) + 0x40;
    let mut s = 0x636f_726e_u32;

    let run0 = |emu: &mut Emu, v: [f32; 6], tag: &str| {
        let [m54, m50, m4c, m40, m3c, m38] = v;
        for (off, val) in [
            (0x54u64, m54),
            (0x50, m50),
            (0x4c, m4c),
            (0x40, m40),
            (0x3c, m3c),
            (0x38, m38),
        ] {
            emu.write_u32(frame - off, val.to_bits());
        }
        for off in [0x34u64, 0x30, 0x24] {
            emu.write_u32(frame - off, 0xdead_beef);
        }
        for d in [0u64, 4, 8] {
            emu.write_u32(esp + d, 0xdead_beef);
        }
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x67af7b, 0x67afa7);
        af(emu.read_f32(frame - 0x34), mistr_negate(m54), tag, "neg54");
        af(emu.read_f32(frame - 0x30), mistr_negate(m50), tag, "neg50");
        af(emu.read_f32(frame - 0x24), mistr_negate(m4c), tag, "neg4c");
        af(
            emu.read_f32(esp + 8),
            mistr_corner_add(mistr_negate(m4c), m38),
            tag,
            "c8",
        );
        af(
            emu.read_f32(esp + 4),
            mistr_corner_add(mistr_negate(m50), m3c),
            tag,
            "c4",
        );
        af(
            emu.read_f32(esp),
            mistr_corner_add(mistr_negate(m54), m40),
            tag,
            "c0",
        );
    };
    run0(&mut emu, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], "c0-basic");
    run0(&mut emu, [0.0; 6], "c0-zero");
    run0(&mut emu, [f32::NAN, 1.0, 1.0, 1.0, 1.0, 1.0], "c0-nan");
    for _ in 0..150 {
        run0(
            &mut emu,
            [
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
            ],
            "c0-fuzz",
        );
    }

    let run1 = |emu: &mut Emu, v: [f32; 6], tag: &str| {
        let [m54, m50, m4c, m40, m3c, m38] = v;
        for (off, val) in [
            (0x54u64, m54),
            (0x50, m50),
            (0x4c, m4c),
            (0x40, m40),
            (0x3c, m3c),
            (0x38, m38),
        ] {
            emu.write_u32(frame - off, val.to_bits());
        }
        for off in [0x14u64, 0x10, 0xc] {
            emu.write_u32(frame - off, 0xdead_beef);
        }
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x67afc7, 0x67affa);
        af(
            emu.read_f32(frame - 0x14),
            mistr_corner_add(m40, m54),
            tag,
            "c14",
        );
        af(
            emu.read_f32(frame - 0x10),
            mistr_corner_add(m3c, m50),
            tag,
            "c10",
        );
        af(
            emu.read_f32(frame - 0xc),
            mistr_corner_add(m38, m4c),
            tag,
            "cc",
        );
    };
    run1(&mut emu, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], "c1-basic");
    run1(&mut emu, [1.0, 1.0, 1.0, f32::NAN, 1.0, 1.0], "c1-nan");
    for _ in 0..150 {
        run1(
            &mut emu,
            [
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
            ],
            "c1-fuzz",
        );
    }

    let run2 = |emu: &mut Emu, v: [f32; 6], tag: &str| {
        let [m54, m50, m4c, m40, m3c, m38] = v;
        for (off, val) in [
            (0x54u64, m54),
            (0x50, m50),
            (0x4c, m4c),
            (0x40, m40),
            (0x3c, m3c),
            (0x38, m38),
        ] {
            emu.write_u32(frame - off, val.to_bits());
        }
        for off in [0x28u64, 0x44, 0x8] {
            emu.write_u32(frame - off, 0xdead_beef);
        }
        for d in [0u64, 4, 8] {
            emu.write_u32(esp + d, 0xdead_beef);
        }
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x67affa, 0x67b02f);
        af(emu.read_f32(frame - 0x28), mistr_negate(m40), tag, "neg40");
        af(emu.read_f32(frame - 0x44), mistr_negate(m3c), tag, "neg3c");
        af(emu.read_f32(frame - 0x8), mistr_negate(m38), tag, "neg38");
        af(
            emu.read_f32(esp + 8),
            mistr_corner_add(mistr_negate(m38), m4c),
            tag,
            "c8",
        );
        af(
            emu.read_f32(esp + 4),
            mistr_corner_add(mistr_negate(m3c), m50),
            tag,
            "c4",
        );
        af(
            emu.read_f32(esp),
            mistr_corner_add(mistr_negate(m40), m54),
            tag,
            "c0",
        );
    };
    run2(&mut emu, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], "c2-basic");
    run2(&mut emu, [1.0, 1.0, 1.0, 1.0, f32::NAN, 1.0], "c2-nan");
    for _ in 0..150 {
        run2(
            &mut emu,
            [
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
                fz(&mut s),
            ],
            "c2-fuzz",
        );
    }

    let run3 = |emu: &mut Emu, m38: f32, m3c: f32, m40: f32, d: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x38, m38.to_bits());
        emu.write_u32(frame - 0x3c, m3c.to_bits());
        emu.write_u32(frame - 0x40, m40.to_bits());
        emu.write_u32(frame - 0xc, d[0].to_bits());
        emu.write_u32(frame - 0x10, d[1].to_bits());
        emu.write_u32(frame - 0x14, d[2].to_bits());
        for dd in [0u64, 4, 8] {
            emu.write_u32(esp + dd, 0xdead_beef);
        }
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x67b075, 0x67b098);
        let sp = emu.get_reg(RegisterX86::ESP);
        af(emu.read_f32(sp + 8), mistr_corner_add(m38, d[0]), tag, "c8");
        af(emu.read_f32(sp + 4), mistr_corner_add(m3c, d[1]), tag, "c4");
        af(emu.read_f32(sp), mistr_corner_add(m40, d[2]), tag, "c0");
    };
    run3(&mut emu, 4.0, 5.0, 6.0, [1.0, 2.0, 3.0], "c3-basic");
    run3(&mut emu, 1.0, 1.0, 1.0, [1.0, 1.0, f32::NAN], "c3-nan");
    for _ in 0..150 {
        run3(
            &mut emu,
            fz(&mut s),
            fz(&mut s),
            fz(&mut s),
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            "c3-fuzz",
        );
    }
}

/// `0x67b11b` to `0x67b138`: `half * half`.
#[test]
fn mistr_fade_mag_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x140;
    let mut s = 0x6661_6465_u32;

    let run = |emu: &mut Emu, half: f32, tag: &str| {
        emu.write_u32(frame - 0x18, half.to_bits());
        emu.write_u32(frame - 0x20, 0);
        emu.write_u32(frame - 0x1c, 0);
        emu.write_u32(frame - 0x74, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x67b11b, 0x67b13b);
        af(emu.read_f32(frame - 0x74), mistr_fade_mag(half), tag, "mag");
    };
    run(&mut emu, 2.0, "two");
    run(&mut emu, 0.0, "zero");
    run(&mut emu, f32::NAN, "nan");
    run(&mut emu, 0.333_333_34, "subf32");
    for _ in 0..300 {
        run(&mut emu, fz(&mut s), "fuzz");
    }
}

const G_DT: u64 = 0x00c6_2510;
const G_C62970: u64 = 0x00c6_2970;
const G_C62974: u64 = 0x00c6_2974;

/// `0x67b141` to `0x67b172`: the phase clock after one frame.
#[test]
fn mistr_time_accum_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x80);
    let frame = emu.alloc(0x200) + 0x140;
    let esp = emu.alloc(0x80) + 0x40;
    let mut s = 0x7469_6d65_u32;

    let run = |emu: &mut Emu, dt: f32, rate: f32, clock: f32, tag: &str| {
        emu.write_u32(G_DT, dt.to_bits());
        emu.write_u32(G_C62970, 0);
        emu.write_u32(G_C62974, 0);
        emu.write_u32(obj + 0x20, rate.to_bits());
        emu.write_u32(obj + 0x2c, clock.to_bits());
        emu.write_u32(obj + 0x48, 0);
        emu.set_reg(RegisterX86::EDI, obj);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x67b141, 0x67b175);
        af(
            emu.read_f32(obj + 0x2c),
            mistr_time_accum(dt, rate, clock),
            tag,
            "accum",
        );
    };
    run(&mut emu, 0.016, 1.0, 0.5, "tick");
    run(&mut emu, 0.0, 1.0, 0.5, "dt0");
    run(&mut emu, f32::NAN, 1.0, 0.5, "nan");
    run(&mut emu, 0.1, 0.333_333_34, 0.7, "subf32");
    for _ in 0..300 {
        run(&mut emu, fz(&mut s), fz(&mut s), fz(&mut s), "fuzz");
    }
}

/// `0x67b261` to `0x67b272`: the clock less 1.0.
#[test]
fn mistr_phase_wrap_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let obj = emu.alloc(0x40);
    let mut s = 0x7772_6170_u32;

    let run = |emu: &mut Emu, clock: f32, tag: &str| {
        emu.write_u32(obj + 0x2c, clock.to_bits());
        emu.set_reg(RegisterX86::EDI, obj);
        emu.run(0x67b261, 0x67b272);
        af(
            emu.read_f32(obj + 0x2c),
            mistr_phase_wrap(clock),
            tag,
            "wrap",
        );
    };
    run(&mut emu, 1.5, "over");
    run(&mut emu, 1.0, "eq");
    run(&mut emu, f32::NAN, "nan");
    run(&mut emu, 0.333_333_34, "subf32");
    for _ in 0..300 {
        run(&mut emu, fz(&mut s), "fuzz");
    }
}

/// `0x67b287` to `0x67b2b5`: the node index for hashes and counts of 0 to 63, exact odd products (a
/// tie that rounds down), a product beyond `i32`, infinite and NaN hashes, the largest count, and a
/// fuzz.
#[test]
fn mistr_hash_count_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x140;
    let mut s = 0x6861_7368_u32;

    let run = |emu: &mut Emu, h: f32, count: i32, tag: &str| {
        emu.write_u32(frame - 0x8, h.to_bits());
        emu.write_u32(frame - 0x60, count as u32);
        emu.write_u32(frame - 0x6c, 0xdead_beef);
        emu.set_reg(RegisterX86::EDX, count as u32 as u64);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x67b287, 0x67b2b5);
        let binary = emu.get_reg(RegisterX86::EAX) as u32 as i32;
        let _ = binary;
        let bin = emu.read_f32(frame - 0x6c).to_bits() as i32;
        assert_eq!(
            bin,
            mistr_hash_count(h, count),
            "{tag}: h={h} count={count} binary={bin} ours={}",
            mistr_hash_count(h, count)
        );
    };
    run(&mut emu, 1.0, 8, "h1");
    run(&mut emu, 1.9999999, 8, "hmax");
    run(&mut emu, 1.5, 16, "h1.5");
    run(&mut emu, 1.25, 4, "round-tie");
    run(&mut emu, f32::NAN, 8, "nan");
    run(&mut emu, 1.0, 0, "count0");
    run(&mut emu, 1.5, 2, "exact odd product: a tie rounding down");
    run(&mut emu, 1.5, 6, "exact odd product, 3");
    run(&mut emu, 1.5, 4, "exact even product");
    run(&mut emu, 1.0e10, 8, "beyond i32");
    run(&mut emu, f32::INFINITY, 8, "infinite");
    run(&mut emu, 1.5, i32::MAX, "largest count");
    run(&mut emu, -5.0, 100, "below 1");
    for _ in 0..300 {
        let h = f32::from_bits(0x3f80_0000 | (next(&mut s) >> 9));
        let count = ((next(&mut s) >> 8) % 64) as i32;
        run(&mut emu, h, count, "fuzz");
    }
}

/// `0x67b2e1` to `0x67b357`: the three accumulators, whose components round at different points.
#[test]
fn mistr_node_accum_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x200);
    let frame = emu.alloc(0x200) + 0x140;
    let mut s = 0x6e6f_6465_u32;

    let run =
        |emu: &mut Emu, n: [f32; 3], half: f32, fade: f32, scal: f32, acc: [f32; 3], tag: &str| {
            emu.write_u32(esi + 0x114, n[0].to_bits());
            emu.write_u32(esi + 0x118, n[1].to_bits());
            emu.write_u32(esi + 0x11c, n[2].to_bits());
            emu.write_u32(esi + 0x128, scal.to_bits());
            emu.write_u32(esi + 0x108, acc[0].to_bits());
            emu.write_u32(esi + 0x10c, acc[1].to_bits());
            emu.write_u32(esi + 0x110, acc[2].to_bits());
            emu.write_u32(frame - 0x18, half.to_bits());
            emu.write_u32(frame - 0x74, fade.to_bits());
            emu.set_reg(RegisterX86::ESI, esi);
            emu.set_reg(RegisterX86::EBP, frame);
            emu.run(0x67b2e1, 0x67b357);
            let binary = NodeAccum {
                acc0: emu.read_f32(esi + 0x108),
                acc1: emu.read_f32(esi + 0x10c),
                acc2: emu.read_f32(esi + 0x110),
            };
            let ours = mistr_node_accum(n, half, fade, scal, acc);
            af(binary.acc0, ours.acc0, tag, "acc0");
            af(binary.acc1, ours.acc1, tag, "acc1");
            af(binary.acc2, ours.acc2, tag, "acc2");
        };
    run(
        &mut emu,
        [1.0, 2.0, 3.0],
        0.5,
        4.0,
        0.25,
        [10.0, 20.0, 30.0],
        "basic",
    );
    run(
        &mut emu,
        [0.0, 0.0, 0.0],
        0.5,
        0.0,
        0.0,
        [0.0, 0.0, 0.0],
        "zero",
    );
    run(
        &mut emu,
        [f32::NAN, 1.0, 1.0],
        0.5,
        1.0,
        1.0,
        [0.0, 0.0, 0.0],
        "nan",
    );
    run(
        &mut emu,
        [0.333_333_34, 0.111_111_1, 0.142_857_15],
        0.25,
        0.7,
        0.9,
        [0.123_456_79, 0.987_654_3, 0.555_555_6],
        "subf32",
    );
    for _ in 0..300 {
        run(
            &mut emu,
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            fz(&mut s),
            fz(&mut s),
            fz(&mut s),
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            "fuzz",
        );
    }
}

/// `0x67b372` to `0x67b386`, with the tick value on the stack: `1 / spacing` and `q / spacing`.
#[test]
fn mistr_spacing_recip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x140;
    let esp = emu.alloc(0x80) + 0x40;
    let mut s = 0x7370_6361_u32;

    let run = |emu: &mut Emu, q: f64, spacing: f32, tag: &str| {
        emu.write_u32(frame - 0x2c, spacing.to_bits());
        emu.write_u32(frame - 0x4, 0xdead_beef);
        emu.write_u32(frame - 0x24, 0xdead_beef);
        emu.write_u32(frame - 0x58, 0);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::EAX, 0);
        let island = ld_jmp(emu, &[q], 0x67b372);
        emu.run(island, 0x67b389);
        let (inv, prod) = mistr_spacing_recip(q, spacing);
        af(emu.read_f32(frame - 0x4), inv, tag, "inv");
        af(emu.read_f32(frame - 0x24), prod, tag, "prod");
    };
    run(&mut emu, 1.0, 4.0, "basic");
    run(&mut emu, 0.0, 4.0, "q0");
    run(&mut emu, 1.0, 0.0, "div0");
    run(&mut emu, f64::NAN, 4.0, "nan-q");
    run(&mut emu, 2.5, 0.333_333_34, "subf32");
    for _ in 0..300 {
        run(&mut emu, fz64(&mut s), fz(&mut s), "fuzz");
    }
}

/// `0x67b38e` to `0x67b3dc` with the tick value on the stack: the clamp at 1.0 (below, equal,
/// above, NaN), the count products and the two words biased by 512.
#[test]
fn mistr_cell_round_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x200);
    let frame = emu.alloc(0x200) + 0x140;
    let mut s = 0x6365_6c6c_u32;

    let run = |emu: &mut Emu, q: f64, arg: f32, count: i32, frac: f32, tag: &str| {
        let stage = |emu: &mut Emu| {
            emu.write_u32(frame - 0x4, arg.to_bits());
            emu.write_u32(esi + 0x104, count as u32);
            emu.write_u32(frame - 0x24, frac.to_bits());
            emu.write_u32(frame - 0x8, 0xdead_beef);
            emu.set_reg(RegisterX86::ESI, esi);
            emu.set_reg(RegisterX86::EBP, frame);
        };
        stage(emu);
        let islanda = ld_jmp(emu, &[q], 0x67b38e);
        emu.run(islanda, 0x67b3c5);
        let bin_n = emu.read_f32(frame - 0x4);
        let bin_ra = emu.read_f32(frame - 0x8);

        stage(emu);
        let islandb = ld_jmp(emu, &[q], 0x67b38e);
        emu.run(islandb, 0x67b3dc);
        let bin_rb = emu.read_f32(frame - 0x8);

        let ours: CellRound = mistr_cell_round(q, arg, count, frac);
        af(bin_n, ours.n_clamped, tag, "n_clamped");
        af(bin_ra, ours.round_a, tag, "round_a");
        af(bin_rb, ours.round_b, tag, "round_b");
    };
    run(&mut emu, 0.5, 0.5, 16, 0.3, "in-range");
    run(&mut emu, 2.0, 1.0, 16, 0.3, "clamp");
    run(&mut emu, 1.0, 1.0, 16, 0.3, "eq1");
    run(&mut emu, 0.9999999, 1.0, 16, 0.3, "below1");
    run(&mut emu, f64::NAN, 1.0, 16, 0.3, "nan-q");
    run(&mut emu, 0.3, 0.7, 8, 0.123_456_79, "subf32");
    for _ in 0..300 {
        let q = fz64(&mut s);
        let arg = fz(&mut s);
        let count = ((next(&mut s) >> 8) % 64) as i32;
        run(&mut emu, q, arg, count, fz(&mut s), "fuzz");
    }
}

/// `0x67b3f5` to `0x67b414`: the interpolated extent, captured as a double, for flat, NaN and
/// fuzzed ramps.
#[test]
fn mistr_cell_lerp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x200);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x200) + 0x140;
    let island = emu.alloc(0x40);
    let slot = emu.alloc(0x10);
    let mut s = 0x6c65_7270_u32;

    let run = |emu: &mut Emu,
               ramp_lo: f32,
               ramp_hi: f32,
               frac_i: i32,
               frac: f32,
               extent: f32,
               tag: &str| {
        emu.write_u32(esi + 0x4, ramp_lo.to_bits());
        emu.write_u32(esi + 0x8, ramp_hi.to_bits());
        emu.write_u32(frame - 0x24, frac_i as u32);
        emu.write_u32(frame - 0x4, frac.to_bits());
        emu.write_u32(edi + 0x1c, extent.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.set_reg(RegisterX86::EAX, 1);
        let binary = island_f64(emu, island, slot, 0x67b3f5, 0x67b414 - 0x67b3f5);
        let ours = mistr_cell_lerp(ramp_lo, ramp_hi, frac_i, frac, extent);
        if !(binary.is_nan() && ours.is_nan()) {
            assert_eq!(
                binary.to_bits(),
                ours.to_bits(),
                "{tag}: lerp binary={binary:e} ours={ours:e}"
            );
        }
    };
    run(&mut emu, 0.0, 1.0, 0, 0.5, 2.0, "basic");
    run(&mut emu, 1.0, 1.0, 0, 0.0, 0.0, "flat");
    run(&mut emu, 0.0, 1.0, 3, 0.25, 0.0, "frac");
    run(&mut emu, f32::NAN, 1.0, 0, 0.5, 1.0, "nan-lo");
    run(&mut emu, 0.1, 0.333_333_34, 2, 0.7, 0.9, "subf32");
    for _ in 0..300 {
        let frac_i = ((next(&mut s) >> 8) % 16) as i32;
        run(
            &mut emu,
            fz(&mut s),
            fz(&mut s),
            frac_i,
            fz(&mut s),
            fz(&mut s),
            "fuzz",
        );
    }
}

/// `0x67b421` to `0x67b433`: the cell phase plus 5/3.
#[test]
fn mistr_phase_advance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x200);
    let mut s = 0x7068_6164_u32;

    let run = |emu: &mut Emu, phase: f32, tag: &str| {
        emu.write_u32(esi + 0x128, phase.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.run(0x67b421, 0x67b433);
        af(
            emu.read_f32(esi + 0x128),
            mistr_phase_advance(phase),
            tag,
            "adv",
        );
    };
    run(&mut emu, 0.0, "zero");
    run(&mut emu, 100.0, "big");
    run(&mut emu, f32::NAN, "nan");
    run(&mut emu, 0.333_333_34, "subf32");
    for _ in 0..300 {
        run(&mut emu, fz(&mut s), "fuzz");
    }
}

/// `0x67b3f5` to `0x67b464` in place: the exe's drift gate (read from whether the cap was stored),
/// the phase, the cap and the new extent, with `old` also at the f32 neighbours of the wide
/// interpolation.
#[test]
fn mistr_drift_clamp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x200);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x200) + 0x140;
    let mut s = 0x6472_6966_u32;

    #[allow(clippy::too_many_arguments)]
    let run = |emu: &mut Emu,
               ramp_lo: f32,
               ramp_hi: f32,
               frac_i: i32,
               frac: f32,
               extent: f32,
               old: f32,
               phase: f32,
               tag: &str| {
        emu.write_u32(esi + 0x4, ramp_lo.to_bits());
        emu.write_u32(esi + 0x8, ramp_hi.to_bits());
        emu.write_u32(esi + 0x110, old.to_bits());
        emu.write_u32(esi + 0x128, phase.to_bits());
        emu.write_u32(frame - 0x24, frac_i as u32);
        emu.write_u32(frame - 0x4, frac.to_bits());
        emu.write_u32(edi + 0x1c, extent.to_bits());
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.set_reg(RegisterX86::EAX, 1);
        emu.run(0x67b3f5, 0x67b464);
        let lerp = mistr_cell_lerp(ramp_lo, ramp_hi, frac_i, frac, extent);
        let gate = mistr_drift_gate(lerp, old);
        let marker = emu.read_u32(frame - 0x4);
        let cap_bits = ((f64::from(extent) * 0.25) as f32).to_bits();
        if cap_bits != frac.to_bits() {
            assert_eq!(
                marker == cap_bits,
                gate,
                "{tag}: the exe's drift gate disagrees with the transcription (lerp={lerp:e} old={old})"
            );
        }
        if gate {
            af(
                emu.read_f32(esi + 0x128),
                mistr_phase_advance(phase),
                tag,
                "phase",
            );
            let cap_ours = {
                let f = f64::from;
                f32::from_bits(
                    ((f(extent) * f64::from(f32::from_bits(0x3e80_0000))) as f32).to_bits(),
                )
            };
            af(emu.read_f32(frame - 0x4), cap_ours, tag, "cap");
            af(
                emu.read_f32(esi + 0x110),
                mistr_drift_clamp(lerp, old, extent),
                tag,
                "new",
            );
        }
    };
    run(&mut emu, 0.0, 1.0, 0, 0.5, 4.0, 0.0, 10.0, "uncl");
    run(&mut emu, 0.0, 100.0, 0, 0.5, 0.4, 0.0, 5.0, "clamp");
    run(&mut emu, 0.0, 1.0, 0, 1.0, 4.0, 2.0, 0.0, "delta-eq-cap");
    run(&mut emu, 0.0, 1.0, 3, 0.25, 8.0, -1.0, 0.0, "frac");
    run(&mut emu, 0.1, 0.333_333_34, 2, 0.7, 0.9, 0.0, 3.0, "subf32");
    for _ in 0..400 {
        let frac_i = ((next(&mut s) >> 8) % 16) as i32;
        run(
            &mut emu,
            fz(&mut s),
            fz(&mut s),
            frac_i,
            fz(&mut s),
            fz(&mut s).abs(),
            -fz(&mut s).abs(),
            fz(&mut s),
            "fuzz",
        );
    }
    for _ in 0..300 {
        let (lo, hi, frac, extent) = (fz(&mut s), fz(&mut s), fz(&mut s), fz(&mut s).abs());
        let frac_i = ((next(&mut s) >> 8) % 16) as i32;
        let wide = mistr_cell_lerp(lo, hi, frac_i, frac, extent);
        if !wide.is_finite() {
            continue;
        }
        let base = (wide as f32).to_bits() as i32;
        for d in -2..=2 {
            let old = f32::from_bits((base + d) as u32);
            run(
                &mut emu,
                lo,
                hi,
                frac_i,
                frac,
                extent,
                old,
                1.0,
                "old around the wide lerp",
            );
        }
    }
}

/// `0x67b46d` to `0x67b488`: `life - lo`.
#[test]
fn mistr_life_span_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let edi = emu.alloc(0x40);
    let frame = emu.alloc(0x200) + 0x140;
    let esp = emu.alloc(0x80) + 0x40;
    let ecx_obj = emu.alloc(0x80);
    let mut s = 0x6c69_6665_u32;

    let run = |emu: &mut Emu, life: f32, lo: f32, tag: &str| {
        emu.write_u32(frame - 0x2c, life.to_bits());
        emu.write_u32(edi + 0x30, lo.to_bits());
        emu.write_u32(frame - 0x5c, ecx_obj as u32);
        emu.write_u32(frame - 0x8, 0);
        emu.write_u32(ecx_obj + 0x70, 0);
        emu.write_u32(esp, 0xdead_beef);
        emu.set_reg(RegisterX86::EDI, edi);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x67b46d, 0x67b488);
        let sp = emu.get_reg(RegisterX86::ESP);
        af(emu.read_f32(sp), mistr_life_span(life, lo), tag, "span");
    };
    run(&mut emu, 5.0, 2.0, "basic");
    run(&mut emu, 2.0, 2.0, "eq");
    run(&mut emu, f32::NAN, 2.0, "nan");
    run(&mut emu, 0.333_333_34, 0.1, "subf32");
    for _ in 0..300 {
        run(&mut emu, fz(&mut s), fz(&mut s), "fuzz");
    }
}

/// `0x67b4ad` with the `0x67add0` result as a double: `e * v`.
#[test]
fn mistr_ramp_product_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x140;
    let island = emu.alloc(0x40);
    let slot = emu.alloc(0x10);
    let mut s = 0x726d_7072_u32;

    let run = |emu: &mut Emu, e: f64, v: f32, tag: &str| {
        emu.write_u32(frame - 0x8, v.to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        let eslot = emu.alloc(8);
        emu.write(eslot, &e.to_bits().to_le_bytes());
        let mut code: Vec<u8> = vec![0xdd, 0x05];
        code.extend_from_slice(&(eslot as u32).to_le_bytes());
        code.extend_from_slice(&emu.read(0x67b4ad, 0x67b4b0 - 0x67b4ad));
        code.push(0xd9);
        code.push(0x1d);
        code.extend_from_slice(&(slot as u32).to_le_bytes());
        emu.write(island, &code);
        emu.run(island, island + code.len() as u64);
        af(emu.read_f32(slot), mistr_ramp_product(e, v), tag, "prod");
    };
    run(&mut emu, 1.0, 1.0, "one");
    run(&mut emu, 0.0, 5.0, "zero");
    run(&mut emu, -2.0, 3.0, "neg");
    run(&mut emu, f64::INFINITY, 0.0, "inf-zero");
    run(&mut emu, f64::NAN, 1.0, "nan");
    run(&mut emu, 0.1, 0.333_333_34, "subf32");
    for _ in 0..300 {
        run(&mut emu, fz64(&mut s), fz(&mut s), "fuzz");
    }
}

const G_CAM0: u64 = 0x00c7_cf20;
const G_CAM1: u64 = 0x00c7_cf24;
const G_CAM2: u64 = 0x00c7_cf28;

/// `0x67b4d0` to `0x67b526`: the camera-relative corner and its distance, where the z square mixes
/// the wide and stored values.
#[test]
fn mistr_corner_dist_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let esi = emu.alloc(0x200);
    let ebx = emu.alloc(0x40) + 0x20;
    let frame = emu.alloc(0x200) + 0x140;
    let esp = emu.alloc(0x80) + 0x40;
    let mut s = 0x6469_7374_u32;

    let run = |emu: &mut Emu, corner: [f32; 3], center: [f32; 3], cam: [f32; 3], tag: &str| {
        emu.write_u32(ebx - 0x8, corner[0].to_bits());
        emu.write_u32(ebx - 0x4, corner[1].to_bits());
        emu.write_u32(ebx, corner[2].to_bits());
        emu.write_u32(esi + 0x108, center[0].to_bits());
        emu.write_u32(esi + 0x10c, center[1].to_bits());
        emu.write_u32(esi + 0x110, center[2].to_bits());
        emu.write_u32(G_CAM0, cam[0].to_bits());
        emu.write_u32(G_CAM1, cam[1].to_bits());
        emu.write_u32(G_CAM2, cam[2].to_bits());
        for off in [0x14u64, 0x10, 0xc] {
            emu.write_u32(frame - off, 0xdead_beef);
        }
        emu.set_reg(RegisterX86::ESI, esi);
        emu.set_reg(RegisterX86::EBX, ebx);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.run(0x67b4d0, 0x67b529);
        let sp = emu.get_reg(RegisterX86::ESP);
        let binary = CornerDist {
            d14: emu.read_f32(frame - 0x14),
            d10: emu.read_f32(frame - 0x10),
            dc: emu.read_f32(frame - 0xc),
            dist: emu.read_f32(sp),
        };
        let ours = mistr_corner_dist(corner, center, cam);
        af(binary.d14, ours.d14, tag, "d14");
        af(binary.d10, ours.d10, tag, "d10");
        af(binary.dc, ours.dc, tag, "dc");
        af(binary.dist, ours.dist, tag, "dist");
    };
    run(
        &mut emu,
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "unit-x",
    );
    run(
        &mut emu,
        [3.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "3-4-5",
    );
    run(
        &mut emu,
        [0.0, 0.0, 0.0],
        [10.0, 20.0, 30.0],
        [10.0, 20.0, 30.0],
        "zero-dist",
    );
    run(
        &mut emu,
        [f32::NAN, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "nan",
    );
    run(
        &mut emu,
        [0.333_333_34, 0.111_111_1, 0.142_857_15],
        [0.7, 0.3, 0.9],
        [0.123_456_79, 0.987_654_3, 0.555_555_6],
        "subf32",
    );
    for _ in 0..400 {
        run(
            &mut emu,
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            "fuzz",
        );
    }
}

/// `0x67b542` to `0x67b551` with the `0x67add0` result as a double: the colour word.
#[test]
fn mistr_color_repack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x140;
    let mut s = 0x636f_6c72_u32;

    let run = |emu: &mut Emu, e: f64, brightness: f32, tag: &str| {
        emu.write_u32(frame - 0x64, brightness.to_bits());
        emu.write_u32(frame - 0x4, 0xdead_beef);
        emu.write_u32(frame - 0x14, 0);
        emu.write_u32(frame - 0x10, 0);
        emu.set_reg(RegisterX86::EBP, frame);
        let island = ld_jmp(emu, &[e], 0x67b542);
        emu.run(island, 0x67b554);
        af(
            emu.read_f32(frame - 0x4),
            mistr_color_repack(e, brightness),
            tag,
            "repack",
        );
    };
    run(&mut emu, 1.0, 1.0, "one");
    run(&mut emu, 0.0, 1.0, "zero");
    run(&mut emu, 0.5, 2.0, "half");
    run(&mut emu, -1.0, 1.0, "neg");
    run(&mut emu, f64::INFINITY, 0.0, "inf-zero");
    run(&mut emu, f64::NAN, 1.0, "nan");
    run(&mut emu, 0.1, 0.333_333_34, "subf32");
    for _ in 0..300 {
        run(&mut emu, fz64(&mut s), fz(&mut s), "fuzz");
    }
}
