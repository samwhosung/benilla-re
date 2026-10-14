//! Slices of `DrawBlobShadow` (`0x6d7920`) against the functions of `dn_sky_shadow`: each runs the
//! exe's own instructions between two addresses on a frame laid out in the emulator (EBP near the
//! top of an allocated region so the locals fall inside it, ESP below it for the blocks that store
//! at `[esp+N]`), with the globals a slice reads written directly, and compares the stored `f32`
//! bit for bit.
//!
//! The three gates on `|x − y|` against 2.384e-7 (the two entry gates and the length gate) each
//! have their own test, so every `fsub` and `fabs` of the function is in a slice.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::dn_sky_shadow::{
    shadow_axis_rotate, shadow_color_quantize, shadow_extent_gate, shadow_extent_inv,
    shadow_half_diff, shadow_inv_len, shadow_offset_bias, shadow_quad_corner_transform,
    shadow_span_diff, shadow_vec3_len_scale,
};
use common::{assert_f32, fuzz_f32, next};

/// Sin global.
const G_CEA610: u64 = 0x00ce_a610;
/// Cos global.
const G_CEA60C: u64 = 0x00ce_a60c;

/// The first entry gate, `0x6d7965`: the difference on and either side of the threshold, NaN and a
/// fuzz.
#[test]
fn shadow_extent_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let esp_top = frame + 0x380;
    let mut s = 0x4741_5445_u32;

    let run = |emu: &mut Emu, x: f32, y: f32, tag: &str| {
        emu.write_u32(ebp - 0x18c, x.to_bits());
        emu.write_u32(ebp - 0x198, y.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x6d7965, 0x6d797e);
        let eflags = emu.get_reg(RegisterX86::EFLAGS);
        let binary_proceed = (eflags >> 2) & 1 == 1;
        let ours = shadow_extent_gate(x, y);
        assert_eq!(
            binary_proceed,
            ours,
            "{tag}: x={x} ({:#010x}) y={y} ({:#010x}) eflags={eflags:#x}",
            x.to_bits(),
            y.to_bits()
        );
    };

    let eps = f32::from_bits(0x3480_0000);
    let probes: [(f32, f32); 12] = [
        (eps, 0.0),
        (0.0, eps),
        (f32::from_bits(0x347f_ffff), 0.0),
        (f32::from_bits(0x3480_0001), 0.0),
        (0.0, 0.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (1.0 + eps, 1.0),
        (f32::NAN, 0.0),
        (0.0, f32::NAN),
        (f32::INFINITY, 1.0),
        (f32::INFINITY, f32::INFINITY),
    ];
    for &(x, y) in &probes {
        run(&mut emu, x, y, "probe");
    }
    for _ in 0..400 {
        let (x, y) = if (next(&mut s) >> 8) & 1 == 0 {
            let base = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 1.0e-6) as f32;
            (base, 0.0f32)
        } else {
            (fuzz_f32(&mut s), fuzz_f32(&mut s))
        };
        run(&mut emu, x, y, "fuzz");
    }
}

/// The second entry gate, `0x6d7984`, which differs only in the second operand's address.
#[test]
fn shadow_extent_gate_b_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let esp_top = frame + 0x380;
    let mut s = 0x4742_4254_u32;

    let run = |emu: &mut Emu, x: f32, y: f32, tag: &str| {
        emu.write_u32(ebp - 0x190, x.to_bits());
        emu.write_u32(ebp - 0x19c, y.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x6d7984, 0x6d799d);
        let eflags = emu.get_reg(RegisterX86::EFLAGS);
        let binary_proceed = (eflags >> 2) & 1 == 1;
        let ours = shadow_extent_gate(x, y);
        assert_eq!(
            binary_proceed,
            ours,
            "{tag}: x={x} ({:#010x}) y={y} ({:#010x}) eflags={eflags:#x}",
            x.to_bits(),
            y.to_bits()
        );
    };

    let eps = f32::from_bits(0x3480_0000);
    let probes: [(f32, f32); 12] = [
        (eps, 0.0),
        (0.0, eps),
        (f32::from_bits(0x347f_ffff), 0.0),
        (f32::from_bits(0x3480_0001), 0.0),
        (0.0, 0.0),
        (1.0, 0.0),
        (1.0, 1.0),
        (-0.0, 0.0),
        (f32::NAN, 0.0),
        (0.0, f32::NAN),
        (f32::INFINITY, 1.0),
        (f32::INFINITY, f32::INFINITY),
    ];
    for &(x, y) in &probes {
        run(&mut emu, x, y, "probe");
    }
    for _ in 0..400 {
        let (x, y) = if (next(&mut s) >> 8) & 1 == 0 {
            let base = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 1.0e-6) as f32;
            (base, 0.0f32)
        } else {
            (fuzz_f32(&mut s), fuzz_f32(&mut s))
        };
        run(&mut emu, x, y, "fuzz");
    }
}

/// The gate on `|len − 1|` at `0x6d7ad9`.
#[test]
fn shadow_len_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let esp_top = frame + 0x380;
    let mut s = 0x4c47_4e54_u32;

    let run = |emu: &mut Emu, len: f32, tag: &str| {
        emu.write_u32(ebp - 0x50, len.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x6d7ad9, 0x6d7aff);
        let eflags = emu.get_reg(RegisterX86::EFLAGS);
        let binary_proceed = (eflags >> 2) & 1 == 1;
        let ours = shadow_extent_gate(len, 1.0);
        assert_eq!(
            binary_proceed,
            ours,
            "{tag}: len={len} ({:#010x}) eflags={eflags:#x}",
            len.to_bits()
        );
    };

    let eps = f32::from_bits(0x3480_0000);
    let probes: [f32; 12] = [
        1.0,
        1.0 + eps,
        1.0 - eps,
        f32::from_bits(0x3f80_0001),
        f32::from_bits(0x3f7f_ffff),
        0.0,
        2.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.000_000_1,
    ];
    for &len in &probes {
        run(&mut emu, len, "probe");
    }
    for _ in 0..400 {
        let len = if (next(&mut s) >> 8) & 1 == 0 {
            let d = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 2.0e-6 - 1.0e-6) as f32;
            1.0f32 + d
        } else {
            fuzz_f32(&mut s)
        };
        run(&mut emu, len, "fuzz");
    }
}

/// The four corner transforms, `0x6d7b1f` to `0x6d7ca2`.
#[test]
fn shadow_quad_corner_transform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let esp_top = frame + 0x380;
    let mut s = 0x5368_4457_u32;

    let blocks: [(u64, u64, u64, u64); 4] = [
        (0x6d7b1f, 0x6d7b7c, 0x40, 0x3c),
        (0x6d7b81, 0x6d7bde, 0x40, 0x48),
        (0x6d7be3, 0x6d7c40, 0x4c, 0x48),
        (0x6d7c45, 0x6d7ca2, 0x4c, 0x3c),
    ];

    let m_off = [0x20u64, 0x24, 0x28, 0x2c, 0x30, 0x34];
    let t_off = [0x1cu64, 0x18, 0x14];

    let run = |emu: &mut Emu,
               blk: (u64, u64, u64, u64),
               p: f32,
               q: f32,
               m: [f32; 6],
               t: [f32; 3],
               tag: &str| {
        let (start, end, p_off, q_off) = blk;
        emu.write_u32(ebp - p_off, p.to_bits());
        emu.write_u32(ebp - q_off, q.to_bits());
        for (k, &off) in m_off.iter().enumerate() {
            emu.write_u32(ebp - off, m[k].to_bits());
        }
        for (k, &off) in t_off.iter().enumerate() {
            emu.write_u32(ebp - off, t[k].to_bits());
        }
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(start, end);
        let bx = emu.read_f32(esp_top - 0xc);
        let by = emu.read_f32(esp_top - 0x8);
        let bz = emu.read_f32(esp_top - 0x4);
        let (ox, oy, oz) = shadow_quad_corner_transform(
            p, q, m[0], m[1], m[2], m[3], m[4], m[5], t[0], t[1], t[2],
        );
        assert_f32(bx, ox, &format!("{tag}: out_x"));
        assert_f32(by, oy, &format!("{tag}: out_y"));
        assert_f32(bz, oz, &format!("{tag}: out_z"));
    };

    let probes: [(f32, f32); 6] = [
        (0.0, 0.0),
        (-0.0, 0.0),
        (1.0, 1.0),
        (1.0, -1.0),
        (f32::NAN, 1.0),
        (f32::INFINITY, 2.0),
    ];
    let m_id = [1.0f32, 0.0, 0.0, 0.0, 1.0, 0.0];
    let t_nz = [7.0f32, -3.0, 11.0];
    for (bi, &blk) in blocks.iter().enumerate() {
        for &(p, q) in &probes {
            run(&mut emu, blk, p, q, m_id, t_nz, &format!("blk{bi}/probe"));
        }
    }

    for _ in 0..300 {
        let p = fuzz_f32(&mut s);
        let q = fuzz_f32(&mut s);
        let m = [
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
        ];
        let t = [fuzz_f32(&mut s), fuzz_f32(&mut s), fuzz_f32(&mut s)];
        let bi = (next(&mut s) >> 8) as usize % 4;
        run(&mut emu, blocks[bi], p, q, m, t, "fuzz");
    }
}

/// `0x6d7d8d` to `0x6d7e0f`.
#[test]
fn shadow_extent_inv_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let esp_top = frame + 0x380;
    let mut s = 0x4558_5421_u32;

    let run = |emu: &mut Emu, a_hi: f32, a_lo: f32, b_hi: f32, b_lo: f32, tag: &str| {
        emu.write_u32(ebp - 0x40, a_hi.to_bits());
        emu.write_u32(ebp - 0x4c, a_lo.to_bits());
        emu.write_u32(ebp - 0x3c, b_hi.to_bits());
        emu.write_u32(ebp - 0x48, b_lo.to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x6d7d8d, 0x6d7e0f);
        let inv_b = emu.read_f32(esp_top - 0xc);
        let inv_a = emu.read_f32(esp_top - 0x8);
        let (ob, oa) = shadow_extent_inv(a_hi, a_lo, b_hi, b_lo);
        assert_f32(inv_b, ob, &format!("{tag}: inv_b"));
        assert_f32(inv_a, oa, &format!("{tag}: inv_a"));
    };

    let probes: [(f32, f32, f32, f32); 7] = [
        (10.0, -10.0, 20.0, -20.0),
        (5.0, 5.0, 3.0, 3.0),
        (1.0, 2.0, 4.0, 1.0),
        (0.0, 0.0, 1.0, -1.0),
        (f32::NAN, 0.0, 2.0, -2.0),
        (f32::INFINITY, 0.0, 1.0, 0.0),
        (1.0e30, -1.0e30, 1.0e-30, -1.0e-30),
    ];
    for &(ah, al, bh, bl) in &probes {
        run(&mut emu, ah, al, bh, bl, "probe");
    }
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            "fuzz",
        );
    }
}

/// `0x6d7d23` to `0x6d7d47`, with the sine and cosine globals written.
#[test]
fn shadow_axis_rotate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let mut s = 0x4158_4953_u32;

    let run = |emu: &mut Emu, sin: f32, cos: f32, sc: f32, pivot: f32, tag: &str| {
        emu.write_u32(G_CEA610, sin.to_bits());
        emu.write_u32(G_CEA60C, cos.to_bits());
        emu.write_u32(ebp - 0x4, sc.to_bits());
        emu.write_u32(ebp - 0xc4, pivot.to_bits());
        emu.write_u32(ebp - 0x60, 0xdead_beef);
        emu.write_u32(ebp - 0x54, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.run(0x6d7d23, 0x6d7d47);
        let out0 = emu.read_f32(ebp - 0x60);
        let out1 = emu.read_f32(ebp - 0x54);
        let (o0, o1) = shadow_axis_rotate(sin, cos, sc, pivot);
        assert_f32(out0, o0, &format!("{tag}: out0"));
        assert_f32(out1, o1, &format!("{tag}: out1"));
    };

    let probes: [(f32, f32, f32, f32); 7] = [
        (0.5, 0.866_025_4, 2.0, 10.0),
        (0.0, 1.0, 5.0, -3.0),
        (1.0, 0.0, 0.0, 7.0),
        (-1.0, 0.0, 3.0, 0.0),
        (f32::NAN, 1.0, 2.0, 4.0),
        (f32::INFINITY, 1.0, 1.0, 0.0),
        (1.0e20, 1.0e20, 1.0e20, -1.0e20),
    ];
    for &(si, co, sc, pv) in &probes {
        run(&mut emu, si, co, sc, pv, "probe");
    }
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            "fuzz",
        );
    }
}

/// `0x6d7a31` to `0x6d7a68`, including the squared-length leaf `0x4549f0`.
#[test]
fn shadow_vec3_len_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let esp_top = frame + 0x380;
    let mut s = 0x4c45_4e33_u32;

    let run = |emu: &mut Emu, x: f32, y: f32, z: f32, tag: &str| {
        emu.write_u32(ebp - 0xfc, x.to_bits());
        emu.write_u32(ebp - 0xf8, y.to_bits());
        emu.write_u32(ebp - 0xf4, z.to_bits());
        emu.write_u32(ebp - 0x50, 0xdead_beef);
        emu.write_u32(ebp - 0x4, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x6d7a31, 0x6d7a68);
        let len = emu.read_f32(ebp - 0x50);
        let scaled = emu.read_f32(ebp - 0x4);
        let (ol, osc) = shadow_vec3_len_scale(x, y, z);
        assert_f32(len, ol, &format!("{tag}: len"));
        assert_f32(scaled, osc, &format!("{tag}: scaled"));
    };

    let probes: [(f32, f32, f32); 8] = [
        (0.0, 0.0, 0.0),
        (3.0, 4.0, 0.0),
        (1.0, 2.0, 2.0),
        (-3.0, -4.0, 0.0),
        (f32::NAN, 1.0, 1.0),
        (f32::INFINITY, 0.0, 0.0),
        (1.0e19, 1.0e19, 1.0e19),
        (1.0e-20, 1.0e-20, 1.0e-20),
    ];
    for &(x, y, z) in &probes {
        run(&mut emu, x, y, z, "probe");
    }
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            "fuzz",
        );
    }
}

/// `0x6d7a7a` to `0x6d7abf`, including the scalar leaf `0x6d6e50`.
#[test]
fn shadow_half_diff_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let esp_top = frame + 0x380;
    let mut s = 0x4844_4946_u32;

    let run = |emu: &mut Emu, hi: f32, lo: f32, tag: &str| {
        emu.write_u32(ebp - 0x38, hi.to_bits());
        emu.write_u32(ebp - 0x44, lo.to_bits());
        emu.write_u32(ebp - 0x4, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x6d7a7a, 0x6d7abf);
        let binary = emu.read_f32(ebp - 0x4);
        let ours = shadow_half_diff(hi, lo);
        assert_f32(binary, ours, tag);
    };

    let probes: [(f32, f32); 7] = [
        (12.0, 0.0),
        (5.0, 5.0),
        (-6.0, 6.0),
        (1.0, 2.0),
        (f32::NAN, 0.0),
        (f32::INFINITY, 0.0),
        (1.0e30, -1.0e30),
    ];
    for &(hi, lo) in &probes {
        run(&mut emu, hi, lo, "probe");
    }
    for _ in 0..400 {
        run(&mut emu, fuzz_f32(&mut s), fuzz_f32(&mut s), "fuzz");
    }
}

/// `0x6d7b01` to `0x6d7b1a`.
#[test]
fn shadow_inv_len_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let esp_top = frame + 0x380;
    let mut s = 0x494e_564c_u32;

    let run = |emu: &mut Emu, len: f32, tag: &str| {
        emu.write_u32(ebp - 0x50, len.to_bits());
        emu.write_u32(ebp - 0x10, 0xdead_beef);
        emu.write_u32(ebp - 0xc, 0xdead_beef);
        emu.write_u32(ebp - 0x8, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x6d7b01, 0x6d7b1a);
        let r0 = emu.read_f32(ebp - 0x10);
        let r1 = emu.read_f32(ebp - 0xc);
        let r2 = emu.read_f32(ebp - 0x8);
        let ours = shadow_inv_len(len);
        assert_f32(r0, ours, &format!("{tag}: [-0x10]"));
        assert_f32(r1, ours, &format!("{tag}: [-0xc] (same 1/len)"));
        assert_f32(r2, ours, &format!("{tag}: [-0x8] (same 1/len)"));
    };

    let probes: [f32; 9] = [
        1.0,
        2.0,
        0.5,
        0.0,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        1.0e30,
        1.0e-30,
    ];
    for &len in &probes {
        run(&mut emu, len, "probe");
    }
    for _ in 0..400 {
        run(&mut emu, fuzz_f32(&mut s), "fuzz");
    }
}

/// `0x6d7cb7` to `0x6d7d23`.
#[test]
fn shadow_offset_bias_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let mut s = 0x4f42_4953_u32;

    let run = |emu: &mut Emu, ox: f32, oy: f32, a: [f32; 4], tag: &str| {
        emu.write_u32(ebp - 0xcc, ox.to_bits());
        emu.write_u32(ebp - 0xc8, oy.to_bits());
        emu.write_u32(ebp - 0x68, a[0].to_bits());
        emu.write_u32(ebp - 0x64, a[1].to_bits());
        emu.write_u32(ebp - 0x5c, a[2].to_bits());
        emu.write_u32(ebp - 0x58, a[3].to_bits());
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.run(0x6d7cb7, 0x6d7d23);
        let r0 = emu.read_f32(ebp - 0x68);
        let r1 = emu.read_f32(ebp - 0x64);
        let r2 = emu.read_f32(ebp - 0x5c);
        let r3 = emu.read_f32(ebp - 0x58);
        let (o0, o1, o2, o3) = shadow_offset_bias(ox, oy, a[0], a[1], a[2], a[3]);
        assert_f32(r0, o0, &format!("{tag}: a0"));
        assert_f32(r1, o1, &format!("{tag}: a1"));
        assert_f32(r2, o2, &format!("{tag}: a2"));
        assert_f32(r3, o3, &format!("{tag}: a3"));
    };

    let probes: [(f32, f32, [f32; 4]); 5] = [
        (1.0, 2.0, [10.0, 20.0, 30.0, 40.0]),
        (0.0, -0.0, [0.0, 0.0, 0.0, 0.0]),
        (-5.0, 5.0, [5.0, -5.0, 5.0, -5.0]),
        (f32::NAN, 1.0, [1.0, 1.0, 1.0, 1.0]),
        (f32::INFINITY, f32::NEG_INFINITY, [0.0, 0.0, 0.0, 0.0]),
    ];
    for &(ox, oy, a) in &probes {
        run(&mut emu, ox, oy, a, "probe");
    }
    for _ in 0..300 {
        let a = [
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
        ];
        run(&mut emu, fuzz_f32(&mut s), fuzz_f32(&mut s), a, "fuzz");
    }
}

/// `0x6d7d47` to `0x6d7d88`.
#[test]
fn shadow_span_diff_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x300;
    let esp_top = frame + 0x380;
    let mut s = 0x5350_414e_u32;

    let run = |emu: &mut Emu, a3: f32, a1: f32, a2: f32, a0: f32, tag: &str| {
        emu.write_u32(ebp - 0x58, a3.to_bits());
        emu.write_u32(ebp - 0x64, a1.to_bits());
        emu.write_u32(ebp - 0x5c, a2.to_bits());
        emu.write_u32(ebp - 0x68, a0.to_bits());
        emu.write_u32(ebp - 0x10, 0xdead_beef);
        emu.write_u32(ebp - 0xc, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x6d7d47, 0x6d7d88);
        let d0 = emu.read_f32(ebp - 0x10);
        let d1 = emu.read_f32(ebp - 0xc);
        let (o0, o1) = shadow_span_diff(a3, a1, a2, a0);
        assert_f32(d0, o0, &format!("{tag}: d0"));
        assert_f32(d1, o1, &format!("{tag}: d1"));
    };

    let probes: [(f32, f32, f32, f32); 6] = [
        (10.0, 4.0, 8.0, 3.0),
        (5.0, 5.0, 7.0, 7.0),
        (-1.0, 2.0, -3.0, 4.0),
        (f32::NAN, 1.0, 2.0, 3.0),
        (f32::INFINITY, f32::INFINITY, 1.0, 2.0),
        (1.0e30, -1.0e30, 1.0e-30, -1.0e-30),
    ];
    for &(a3, a1, a2, a0) in &probes {
        run(&mut emu, a3, a1, a2, a0, "probe");
    }
    for _ in 0..400 {
        run(
            &mut emu,
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            "fuzz",
        );
    }
}

/// `0x6d7fd4` to `0x6d8027`, with the getter `0x710ca0` supplying `x`: below 0, 0, between, 1,
/// above 1, NaN and the infinities.
#[test]
fn shadow_color_quantize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200);
    let esp_top = emu.alloc(0x200) + 0x100;
    let mut s = 0x434c_5251_u32;

    let run = |emu: &mut Emu, x: f32, tag: &str| {
        emu.write_u32(this + 0x180, x.to_bits());
        emu.set_reg(RegisterX86::EBX, this);
        emu.set_reg(RegisterX86::ESP, esp_top);
        emu.run(0x6d7fd4, 0x6d8027);
        let binary = emu.get_reg(RegisterX86::EAX) as u32 as i32;
        let ours = shadow_color_quantize(x);
        assert_eq!(
            binary,
            ours,
            "{tag}: x={x} ({:#010x}) binary={binary} ours={ours}",
            x.to_bits()
        );
    };

    let probes: [f32; 18] = [
        0.0,
        1.0,
        -0.0,
        f32::from_bits(0x3f80_0001),
        f32::from_bits(0x3f7f_ffff),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x0000_0001),
        0.5,
        0.25,
        0.999,
        0.001,
        2.0,
        -1.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x3b00_0000),
        0.998_039_2,
    ];
    for &x in &probes {
        run(&mut emu, x, "probe");
    }
    for _ in 0..400 {
        let x = if (next(&mut s) >> 8) & 1 == 0 {
            (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32
        } else {
            fuzz_f32(&mut s)
        };
        run(&mut emu, x, "fuzz");
    }
}
