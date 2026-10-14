//! The arithmetic of `0x678960` against the exe, each stretch run as the exe's own instructions on
//! a frame laid out in the emulator: the basis from the view columns, the fade gate, the flake
//! position, the alpha word and the corner positions. The fade gate runs the exe's two compares on
//! a relocated copy of `0x678de9..0x678e11` whose skip branches land on a marker, so the outcome
//! is the exe's, not recomputed. Values the exe leaves wide are loaded into an island from a
//! double.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::wx_snowrender::{
    snow_alpha_pack, snow_basis, snow_flake_center, snow_flake_gate, snow_tri_vertices, SnowBasis,
};
use common::next;

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

/// Assemble an island: `fld qword [live]` pushes the value the exe leaves wide, then the copied
/// `body` bytes, then `fstp dword [out]`. Returns the entry, the end and the slot.
fn build_body_island(emu: &mut Emu, live: f64, body: &[u8]) -> (u64, u64, u64) {
    let live_slot = emu.alloc(8);
    emu.write(live_slot, &live.to_bits().to_le_bytes());
    let out = emu.alloc(8);
    let mut code: Vec<u8> = Vec::new();
    code.push(0xdd);
    code.push(0x05);
    code.extend_from_slice(&(live_slot as u32).to_le_bytes());
    code.extend_from_slice(body);
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let island = emu.alloc(code.len() as u64);
    emu.write(island, &code);
    (island, island + code.len() as u64, out)
}

/// `0x678adf` to `0x678cbe`: the view columns identity, signed, degenerate, NaN and infinite, and a
/// fuzz.
#[test]
fn snow_basis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x300) + 0x180;
    let esp_top = frame + 0x100;
    let edi_blk = emu.alloc(0x200);
    let mut s = 0x5342_4153_u32;

    let run = |emu: &mut Emu, r: [f32; 3], u: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0xc, r[0].to_bits());
        emu.write_u32(frame - 0x8, r[1].to_bits());
        emu.write_u32(frame - 0x4, r[2].to_bits());
        emu.write_u32(frame - 0x1c, u[0].to_bits());
        emu.write_u32(frame - 0x18, u[1].to_bits());
        emu.write_u32(frame - 0x14, u[2].to_bits());
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.set_reg(RegisterX86::EDI, edi_blk);
        emu.run(0x678adf, 0x678cbe);
        let binary = SnowBasis {
            a: [
                emu.read_f32(frame - 0x54),
                emu.read_f32(frame - 0x50),
                emu.read_f32(frame - 0x4c),
            ],
            b: [
                emu.read_f32(frame - 0x48),
                emu.read_f32(frame - 0x44),
                emu.read_f32(frame - 0x40),
            ],
            c: [
                emu.read_f32(frame - 0x3c),
                emu.read_f32(frame - 0x38),
                emu.read_f32(frame - 0x34),
            ],
        };
        let ours = snow_basis(r, u);
        for k in 0..3 {
            assert_field(binary.a[k], ours.a[k], tag, &format!("a[{k}]"));
            assert_field(binary.b[k], ours.b[k], tag, &format!("b[{k}]"));
            assert_field(binary.c[k], ours.c[k], tag, &format!("c[{k}]"));
        }
    };

    let probes: [([f32; 3], [f32; 3]); 8] = [
        ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([0.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        ([1.0, 0.0, 0.0], [1.0, 0.0, 0.0]),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([3.0, 4.0, 0.0], [0.0, 0.0, 5.0]),
        ([-0.0, -0.0, -0.0], [1.0, 1.0, 1.0]),
        ([f32::NAN, 1.0, 0.0], [0.0, 1.0, 0.0]),
        ([f32::INFINITY, 0.0, 0.0], [0.0, f32::NEG_INFINITY, 0.0]),
    ];
    for &(r, u) in &probes {
        run(&mut emu, r, u, "probe");
    }
    for _ in 0..400 {
        let r = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let u = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, r, u, "fuzz");
    }
}

/// `0x678de9` to `0x678e2c`: the gate outcome from the exe, the window edges exactly, `t` around
/// `f2 + 0.25` for sums that are not f32 (the exe compares the sum wide), NaN, the age, and a fuzz.
#[test]
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn snow_flake_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x200) + 0x100;
    let mut s = 0x4754_4147_u32;

    let hi_body = emu.read(0x678dfd, 0x678e03 - 0x678dfd);
    assert_eq!(
        hi_body,
        vec![0xd8, 0x05, 0xb0, 0x29, 0x80, 0x00],
        "snow_flake_gate: fadd-0.25 body bytes must match 0x678dfd verbatim"
    );
    let fade_body = emu.read(0x678e2f, 0x678e32 - 0x678e2f);
    assert_eq!(
        fade_body,
        vec![0xd8, 0x65, 0xe0],
        "snow_flake_gate: fade fsub bytes must match 0x678e2f verbatim"
    );
    let fade_fsub = &fade_body[..];

    let flag = emu.alloc(8);
    let slice_len = 0x678e11 - 0x678de9;
    let mut gate_code = emu.read(0x678de9, slice_len);
    let gate_island = emu.alloc(0x60);
    let skip_pad = gate_island + slice_len as u64 + 12;
    for jnp_at in [0x678df4u64 - 0x678de9, 0x678e0b - 0x678de9] {
        let at = jnp_at as usize;
        assert_eq!(
            &gate_code[at..at + 2],
            &[0x0f, 0x8b],
            "jnp rel32 at 0x{:x}",
            0x678de9 + jnp_at
        );
        let rel = (skip_pad as i64 - (gate_island as i64 + jnp_at as i64 + 6)) as i32;
        gate_code[at + 2..at + 6].copy_from_slice(&rel.to_le_bytes());
    }
    let mov_flag = |v: u32| {
        let mut b = vec![0xc7u8, 0x05];
        b.extend_from_slice(&(flag as u32).to_le_bytes());
        b.extend_from_slice(&v.to_le_bytes());
        b
    };
    gate_code.extend(mov_flag(1));
    gate_code.extend([0xeb, 0x0a]);
    gate_code.extend(mov_flag(2));
    let gate_island_end = gate_island + gate_code.len() as u64;
    emu.write(gate_island, &gate_code);

    let run = |emu: &mut Emu, t: f32, f1: f32, f2: f32, tag: &str| {
        let (emit, span_top) = snow_flake_gate(t, f1, f2);

        let (isl, isl_end, out) = build_body_island(emu, f64::from(f2), &hi_body);
        emu.run(isl, isl_end);
        let bin_hi = emu.read_f32(out);
        let our_hi = (f64::from(f2) + f64::from(f32::from_bits(0x3e80_0000))) as f32;
        assert_field(bin_hi, our_hi, tag, "hi");

        let m = if !(t < f2) { f2 } else { t };
        emu.write_u32(frame - 0x20, m.to_bits());
        emu.write_u32(frame - 0x24, f1.to_bits());
        emu.write_u32(frame - 0x2c, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(0x678e23, 0x678e2c);
        let bin_span = emu.read_f32(frame - 0x2c);
        assert_field(bin_span, span_top, tag, "span_top");

        emu.write_u32(frame - 0x20, f2.to_bits());
        let (isl2, isl2_end, out2) = build_body_island(emu, f64::from(t), fade_fsub);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(isl2, isl2_end);
        let bin_fade = emu.read_f32(out2);
        let our_fade = (f64::from(t) - f64::from(f2)) as f32;
        assert_field(bin_fade, our_fade, tag, "fade");

        emu.write_u32(frame - 0x30, t.to_bits());
        emu.write_u32(frame - 0x24, f1.to_bits());
        emu.write_u32(frame - 0x20, f2.to_bits());
        emu.write_u32(flag, 0);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.run(gate_island, gate_island_end);
        assert_eq!(
            emu.read_u32(flag) == 1,
            emit,
            "{tag}: the exe's fade gate disagrees with the transcription (t={t} f1={f1} f2={f2})"
        );
    };

    type GateProbe = (f32, f32, f32);
    let probes: [GateProbe; 12] = [
        (0.5, 0.0, 1.0),
        (0.0, 0.0, 1.0),
        (1.0, 0.0, 1.0),
        (1.25, 0.0, 1.0),
        (1.2500001, 0.0, 1.0),
        (-0.0001, 0.0, 1.0),
        (0.5, 0.5, 0.5),
        (f32::NAN, 0.0, 1.0),
        (0.5, f32::NAN, 1.0),
        (0.5, 0.0, f32::NAN),
        (1.0e30, -1.0e30, 1.0e30),
        (0.333_333_34, 0.1, 0.9),
    ];
    for &(t, f1, f2) in &probes {
        run(&mut emu, t, f1, f2, "probe");
    }
    let mut inexact: Vec<f32> = Vec::new();
    while inexact.len() < 30 {
        let f2 = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 10.0) as f32;
        let wide = f64::from(f2) + 0.25;
        if f64::from(wide as f32) != wide {
            inexact.push(f2);
        }
    }
    for f2 in inexact {
        let wide = f64::from(f2) + 0.25;
        let base = (wide as f32).to_bits() as i32;
        for d in -3..=3 {
            let t = f32::from_bits((base + d) as u32);
            run(&mut emu, t, 0.0, f2, "t around f2 + 0.25");
        }
    }
    for _ in 0..300 {
        let band =
            |s: &mut u32| (f64::from(next(s) >> 8) / f64::from(1u32 << 24) * 2.0 - 0.5) as f32;
        let (t, f1, f2) = if (next(&mut s) >> 8) & 1 == 0 {
            (band(&mut s), band(&mut s), band(&mut s))
        } else {
            (fuzz(&mut s), fuzz(&mut s), fuzz(&mut s))
        };
        run(&mut emu, t, f1, f2, "fuzz");
    }
}

/// `0x678e4f` to `0x678e8e`: positions from an age, a velocity and an origin, and a fuzz.
#[test]
fn snow_flake_center_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let pkt = emu.alloc(0x100) + 0x40;
    let frame = emu.alloc(0x300) + 0x180;
    let esp_top = frame + 0x100;
    let mut s = 0x4354_4e43_u32;

    let run = |emu: &mut Emu, span: f32, size: [f32; 3], origin: [f32; 3], tag: &str| {
        emu.write_u32(frame - 0x2c, span.to_bits());
        emu.write_u32(pkt - 0x8, size[0].to_bits());
        emu.write_u32(pkt - 0x4, size[1].to_bits());
        emu.write_u32(pkt, size[2].to_bits());
        emu.write_u32(pkt - 0x14, origin[0].to_bits());
        emu.write_u32(pkt - 0x10, origin[1].to_bits());
        emu.write_u32(pkt - 0xc, origin[2].to_bits());
        emu.write_u32(frame - 0xc, 0xdead_beef);
        emu.write_u32(frame - 0x8, 0xdead_beef);
        emu.write_u32(frame - 0x4, 0xdead_beef);
        emu.set_reg(RegisterX86::EBX, pkt);
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x678e4f, 0x678e8e);
        let binary = [
            emu.read_f32(frame - 0xc),
            emu.read_f32(frame - 0x8),
            emu.read_f32(frame - 0x4),
        ];
        let ours = snow_flake_center(span, size, origin);
        for k in 0..3 {
            assert_field(binary[k], ours[k], tag, &format!("center[{k}]"));
        }
    };

    let probes: [(f32, [f32; 3], [f32; 3]); 6] = [
        (1.0, [2.0, 3.0, 4.0], [10.0, 20.0, 30.0]),
        (0.0, [2.0, 3.0, 4.0], [10.0, 20.0, 30.0]),
        (0.5, [-1.0, -2.0, -3.0], [0.0, 0.0, 0.0]),
        (f32::NAN, [1.0, 1.0, 1.0], [0.0, 0.0, 0.0]),
        (1.0e20, [1.0e20, 1.0e20, 1.0e20], [0.0, 0.0, 0.0]),
        (-0.0, [1.0, 1.0, 1.0], [-0.0, -0.0, -0.0]),
    ];
    for &(sp, sz, og) in &probes {
        run(&mut emu, sp, sz, og, "probe");
    }
    for _ in 0..400 {
        let sz = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let og = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        run(&mut emu, fuzz(&mut s), sz, og, "fuzz");
    }
}

/// `0x678ecd` to `0x678ee1`: the alpha word for alphas around 0 and 1 and a fuzz.
#[test]
fn snow_alpha_pack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x414c_5041_u32;

    let body = emu.read(0x678ecd, 0x678ee1 - 0x678ecd);
    assert_eq!(
        &body[..6],
        &[0xd8, 0x0d, 0x58, 0xfe, 0x7f, 0x00],
        "snow_alpha_pack: fmul-255 head bytes must match 0x678ecd verbatim"
    );
    assert_eq!(
        &body[body.len() - 6..],
        &[0xd8, 0x05, 0xcc, 0x29, 0x80, 0x00],
        "snow_alpha_pack: fadd-512 tail bytes must match (ends at 0x678edb)"
    );

    let run = |emu: &mut Emu, alpha: f32, tag: &str| {
        let (isl, isl_end, out) = build_body_island(emu, f64::from(alpha), &body);
        emu.run(isl, isl_end);
        let binary = emu.read_f32(out);
        let ours = snow_alpha_pack(alpha);
        assert_field(binary, ours, tag, "packed");
    };

    for &(a, tag) in &[
        (0.0_f32, "z"),
        (1.0, "one"),
        (0.5, "half"),
        (0.25, "q"),
        (f32::NAN, "nan"),
        (f32::INFINITY, "inf"),
        (-1.0, "neg"),
        (0.333_333_34, "subf32"),
    ] {
        run(&mut emu, a, tag);
    }
    for _ in 0..400 {
        let a = if (next(&mut s) >> 8) & 1 == 0 {
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32
        } else {
            fuzz(&mut s)
        };
        run(&mut emu, a, "fuzz");
    }
}

/// `0x678ee4` to `0x678fcf`: the corner positions from a centre, an eye and a basis, and a fuzz.
#[test]
fn snow_tri_vertices_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400) + 0x180;
    let vbuf = emu.alloc(0x400) + 0x80;
    let mut s = 0x5451_5651_u32;

    let run = |emu: &mut Emu, center: [f32; 3], eye: [f32; 3], basis: SnowBasis, tag: &str| {
        emu.write_u32(frame - 0xc, center[0].to_bits());
        emu.write_u32(frame - 0x8, center[1].to_bits());
        emu.write_u32(frame - 0x4, center[2].to_bits());
        emu.write_u32(frame - 0x98, eye[0].to_bits());
        emu.write_u32(frame - 0x94, eye[1].to_bits());
        emu.write_u32(frame - 0x90, eye[2].to_bits());
        emu.write_u32(frame - 0x54, basis.a[0].to_bits());
        emu.write_u32(frame - 0x50, basis.a[1].to_bits());
        emu.write_u32(frame - 0x4c, basis.a[2].to_bits());
        emu.write_u32(frame - 0x48, basis.b[0].to_bits());
        emu.write_u32(frame - 0x44, basis.b[1].to_bits());
        emu.write_u32(frame - 0x40, basis.b[2].to_bits());
        emu.write_u32(frame - 0x3c, basis.c[0].to_bits());
        emu.write_u32(frame - 0x38, basis.c[1].to_bits());
        emu.write_u32(frame - 0x34, basis.c[2].to_bits());
        for off in [0x80u64, 0x7c, 0x78, 0x8c, 0x88, 0x84, 0x68, 0x64, 0x60] {
            emu.write_u32(frame - off, 0xdead_beef);
        }
        emu.set_reg(RegisterX86::EBP, frame);
        emu.set_reg(RegisterX86::ECX, vbuf);
        emu.set_reg(RegisterX86::ESI, vbuf);
        emu.set_reg(RegisterX86::EDI, vbuf);
        emu.run(0x678ee4, 0x678fe2);
        let binary = [
            [
                emu.read_f32(frame - 0x80),
                emu.read_f32(frame - 0x7c),
                emu.read_f32(frame - 0x78),
            ],
            [
                emu.read_f32(frame - 0x8c),
                emu.read_f32(frame - 0x88),
                emu.read_f32(frame - 0x84),
            ],
            [
                emu.read_f32(frame - 0x68),
                emu.read_f32(frame - 0x64),
                emu.read_f32(frame - 0x60),
            ],
        ];
        let ours = snow_tri_vertices(center, eye, basis);
        for corner in 0..3 {
            for k in 0..3 {
                assert_field(
                    binary[corner][k],
                    ours[corner][k],
                    tag,
                    &format!("v[{corner}][{k}]"),
                );
            }
        }
    };

    let basis_of = |s: &mut u32| SnowBasis {
        a: [fuzz(s), fuzz(s), fuzz(s)],
        b: [fuzz(s), fuzz(s), fuzz(s)],
        c: [fuzz(s), fuzz(s), fuzz(s)],
    };

    let zero_b = SnowBasis {
        a: [0.0, 0.0, 0.0],
        b: [0.0, 0.0, 0.0],
        c: [0.0, 0.0, 0.0],
    };
    let unit_b = SnowBasis {
        a: [0.1, 0.0, 0.0],
        b: [0.0, 0.1, 0.0],
        c: [0.0, 0.0, 0.1],
    };
    let probes: [([f32; 3], [f32; 3], SnowBasis); 6] = [
        ([10.0, 20.0, 30.0], [1.0, 2.0, 3.0], unit_b),
        ([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], unit_b),
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0], zero_b),
        ([f32::NAN, 1.0, 2.0], [0.0, 0.0, 0.0], unit_b),
        ([f32::INFINITY, 0.0, 0.0], [0.0, 0.0, 0.0], unit_b),
        ([1.0e30, -1.0e30, 0.0], [0.0, 0.0, 0.0], unit_b),
    ];
    for &(center, eye, basis) in &probes {
        run(&mut emu, center, eye, basis, "probe");
    }
    for _ in 0..400 {
        let center = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let eye = [fuzz(&mut s), fuzz(&mut s), fuzz(&mut s)];
        let basis = basis_of(&mut s);
        run(&mut emu, center, eye, basis, "fuzz");
    }
}
