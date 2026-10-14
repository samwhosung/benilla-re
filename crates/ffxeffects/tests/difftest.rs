//! The full-screen effect maths against the exe. Each test runs a slice of a function as an island:
//! the function's own floating-point instructions, copied from their addresses into a scratch buffer
//! between an operand load and a store, each asserted equal to the bytes at its source address. The
//! non-floating-point instructions the exe places between them do not touch `st0`, so the island
//! runs the floating-point ones back to back. The byte extract `(bits >> 14) & 0xff`, the integer
//! downsample and the clamp with `__ftol` are compared with the transcription, not run. The x87
//! runs at 53-bit precision.

// The reference clamps keep the exe's comparison order (`< lo`, then `>= hi`); see `lib.rs`.
#![allow(clippy::manual_clamp)]

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ffxeffects::{
    ffx_compute_rt_dims, ffx_death_pass_pack, ffx_fullscreen_quad_corner, ffx_fullscreen_quad_uv,
    ffx_glow_color_quantize, ffx_glow_death_fade, ffx_glow_lut_s8, ffx_glow_lut_u8, ffx_glow_phase,
    ffx_glow_uv_scroll, ffx_pack_byte, ffx_transform_quad_verts, FfxQuadUv,
};

const FMUL_255: [u8; 6] = [0xd8, 0x0d, 0x58, 0xfe, 0x7f, 0x00]; // fmul [0x7ffe58=255.0]
const FMUL_255_SRC: u64 = 0x6cb0ed;
const FMUL_2_55: [u8; 6] = [0xd8, 0x0d, 0xb0, 0x14, 0x81, 0x00]; // fmul [0x8114b0=2.55]
const FMUL_2_55_SRC: u64 = 0x6cb147;
const FADD_512: [u8; 6] = [0xd8, 0x05, 0xcc, 0x29, 0x80, 0x00]; // fadd [0x8029cc=512.0]
const FADD_512_SRC: u64 = 0x6cb0f5;

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(lcg(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}
fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}
fn assert_f64(binary: f64, ours: f64, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#018x} ours={:#018x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// Builds the pack island `(fld|fild) dword [in]; fmul [scale]; fadd [512]; fstp dword [out]`.
/// Returns a runner mapping the raw input dword (`f32` bits, or `i32`) to the exe's `x * scale + 512` `f32`.
fn pack_island(
    emu: &mut Emu,
    is_int: bool,
    fmul: &[u8; 6],
    fmul_src: u64,
    fadd: &[u8; 6],
    fadd_src: u64,
) -> impl Fn(&mut Emu, u32) -> f32 {
    // The island's instructions are the function's own bytes.
    assert_eq!(
        emu.read(fmul_src, 6),
        fmul,
        "fmul bytes differ @ {fmul_src:#x}"
    );
    assert_eq!(
        emu.read(fadd_src, 6),
        fadd,
        "fadd bytes differ @ {fadd_src:#x}"
    );
    // The pack constants are in the mapped image; they are written again in case a slot is zero.
    emu.write_u32(0x007f_fe58, 255.0_f32.to_bits());
    emu.write_u32(0x0081_14b0, 2.55_f32.to_bits());
    emu.write_u32(0x0080_29cc, 512.0_f32.to_bits());
    let in_slot = emu.alloc(0x10);
    let out_slot = emu.alloc(0x10);
    let isle = emu.alloc(0x40);
    let mut code = vec![if is_int { 0xdb } else { 0xd9 }, 0x05]; // fild/fld dword [in_slot]
    code.extend_from_slice(&(in_slot as u32).to_le_bytes());
    code.extend_from_slice(fmul);
    code.extend_from_slice(fadd);
    code.push(0xd9); // fstp dword [out_slot]
    code.push(0x1d);
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    code.push(0xc3); // ret
    let ret_at = isle + code.len() as u64 - 1;
    emu.write(isle, &code);
    move |emu: &mut Emu, raw: u32| {
        emu.write_u32(in_slot, raw);
        emu.write_u32(out_slot, 0);
        emu.run(isle, ret_at);
        emu.read_f32(out_slot)
    }
}

#[test]
fn ffx_glow_color_quantize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // Pack 1: the scene intensity * 255 (`fld dword`).
    let scene = pack_island(
        &mut emu,
        false,
        &FMUL_255,
        FMUL_255_SRC,
        &FADD_512,
        FADD_512_SRC,
    );
    let mut s = 0x6cb0_2001u32;
    let mut probes = vec![0.0f32, 1.0, 0.5, 0.25, -1.0, 2.0];
    for _ in 0..3000 {
        probes.push(frand(&mut s, -1.0, 2.0));
    }
    for x in probes {
        let bin = scene(&mut emu, x.to_bits());
        assert_f32(
            bin,
            ((f64::from(x) * 255.0) + 512.0) as f32,
            &format!("scene {x}"),
        );
        assert_eq!(
            ((bin.to_bits() >> 14) & 0xff) as u8,
            ffx_glow_color_quantize(x, None).0,
            "scene byte {x}"
        );
    }
    // Pack 2: the player stat percent (`min(stat, 100)`, an integer) * 2.55 (`fild dword`).
    let pct = pack_island(
        &mut emu,
        true,
        &FMUL_2_55,
        FMUL_2_55_SRC,
        &FADD_512,
        FADD_512_SRC,
    );
    for p in 0..=100i32 {
        let bin = pct(&mut emu, p as u32);
        assert_f32(
            bin,
            ((f64::from(p) * f64::from(2.55_f32)) + 512.0) as f32,
            &format!("pct {p}"),
        );
        assert_eq!(
            ((bin.to_bits() >> 14) & 0xff) as u8,
            ffx_glow_color_quantize(0.0, Some(p as u8)).1,
            "pct byte {p}"
        );
    }
    eprintln!(
        "ffx_glow_color_quantize: 3006 scene + 101 pct cases bit-exact against the exe, 0x6cb020"
    );
}

#[test]
fn ffx_death_pass_pack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // The death tint: the scene intensity * 255 (`fld dword`) at `0x6cb9ec` and `0x6cb9f9`, the same
    // instructions as at `0x6cb0ed`.
    let scene = pack_island(&mut emu, false, &FMUL_255, 0x6cb9ec, &FADD_512, 0x6cb9f9);
    let mut s = 0x6cb9_3001u32;
    let mut probes = vec![0.0f32, 1.0, 0.5, -1.0, 2.0];
    for _ in 0..3000 {
        probes.push(frand(&mut s, -1.0, 2.0));
    }
    for x in probes {
        let bin = scene(&mut emu, x.to_bits());
        assert_f32(
            bin,
            ((f64::from(x) * 255.0) + 512.0) as f32,
            &format!("death {x}"),
        );
        let b = ((bin.to_bits() >> 14) & 0xff) as u8;
        assert_eq!(
            u32::from(b),
            ffx_pack_byte(f64::from(x), 255.0).into(),
            "death byte {x}"
        );
        assert_eq!(
            (u32::from(b) << 24) | 0x0053_93a8,
            ffx_death_pass_pack(x),
            "death argb {x}"
        );
    }
    eprintln!("ffx_death_pass_pack: 3005 cases bit-exact against the exe, 0x6cb930");
}

// ffx_transform_quad_verts, `0x6cd750`: the affine UV `uv * scale + offset` with a single `f32` round.
// The register-indirect instructions are copied from the affine loop (`ebx` uv, `edi` scale, `esi` offset).
const FLD_UVX: [u8; 3] = [0xd9, 0x04, 0xcb]; // fld  dword [ebx+8*ecx]      @ 0x6cd7a3
const FMUL_SX: [u8; 2] = [0xd8, 0x0f]; //       fmul dword [edi]            @ 0x6cd7a7
const FLD_UVY: [u8; 4] = [0xd9, 0x44, 0xcb, 0xfc]; // fld dword [ebx+8*ecx-4] @ 0x6cd7af
const FMUL_SY: [u8; 3] = [0xd8, 0x4f, 0x04]; //     fmul dword [edi+4]       @ 0x6cd7b3
const FXCH_1: [u8; 2] = [0xd9, 0xc9]; //            fxch st(1)               @ 0x6cd7b6
const FADD_OX: [u8; 2] = [0xd8, 0x06]; //           fadd dword [esi]         @ 0x6cd7b8
const FADD_OY: [u8; 3] = [0xd8, 0x46, 0x04]; //     fadd dword [esi+4]       @ 0x6cd7c0

#[test]
fn ffx_transform_quad_verts_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(emu.read(0x6cd7a3, 3), FLD_UVX, "fld uv.x");
    assert_eq!(emu.read(0x6cd7a7, 2), FMUL_SX, "fmul scale.x");
    assert_eq!(emu.read(0x6cd7af, 4), FLD_UVY, "fld uv.y");
    assert_eq!(emu.read(0x6cd7b3, 3), FMUL_SY, "fmul scale.y");
    assert_eq!(emu.read(0x6cd7b6, 2), FXCH_1, "fxch");
    assert_eq!(emu.read(0x6cd7b8, 2), FADD_OX, "fadd off.x");
    assert_eq!(emu.read(0x6cd7c0, 3), FADD_OY, "fadd off.y");
    let uv_slot = emu.alloc(0x10);
    let scale_slot = emu.alloc(0x10);
    let offset_slot = emu.alloc(0x10);
    let outx = emu.alloc(0x10);
    let outy = emu.alloc(0x10);
    let isle = emu.alloc(0x80);
    let mut code: Vec<u8> = Vec::new();
    code.extend_from_slice(&FLD_UVX); // ecx = 0: uv.x
    code.push(0x41); // inc ecx: 1
    code.extend_from_slice(&FMUL_SX);
    code.extend_from_slice(&FLD_UVY); // ecx = 1: uv_slot + 4 = uv.y
    code.extend_from_slice(&FMUL_SY);
    code.extend_from_slice(&FXCH_1);
    code.extend_from_slice(&FADD_OX);
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(outx as u32).to_le_bytes());
    code.extend_from_slice(&FADD_OY);
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(outy as u32).to_le_bytes());
    code.push(0xc3);
    let ret_at = isle + code.len() as u64 - 1;
    emu.write(isle, &code);
    let run = |emu: &mut Emu, uv: [f32; 2], scale: [f32; 2], offset: [f32; 2]| -> [f32; 2] {
        emu.write_u32(uv_slot, uv[0].to_bits());
        emu.write_u32(uv_slot + 4, uv[1].to_bits());
        emu.write_u32(scale_slot, scale[0].to_bits());
        emu.write_u32(scale_slot + 4, scale[1].to_bits());
        emu.write_u32(offset_slot, offset[0].to_bits());
        emu.write_u32(offset_slot + 4, offset[1].to_bits());
        emu.set_reg(RegisterX86::EBX, uv_slot);
        emu.set_reg(RegisterX86::EDI, scale_slot);
        emu.set_reg(RegisterX86::ESI, offset_slot);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.run(isle, ret_at);
        [emu.read_f32(outx), emu.read_f32(outy)]
    };
    let mut s = 0x6cd7_5001u32;
    let mut count = 0u32;
    for _ in 0..4000 {
        let uv = [frand(&mut s, -4.0, 4.0), frand(&mut s, -4.0, 4.0)];
        let scale = [frand(&mut s, -8.0, 8.0), frand(&mut s, -8.0, 8.0)];
        let offset = [frand(&mut s, -16.0, 16.0), frand(&mut s, -16.0, 16.0)];
        let bin = run(&mut emu, uv, scale, offset);
        let ours = ffx_transform_quad_verts(uv, scale, offset);
        assert_f32(bin[0], ours[0], &format!("x' {uv:?}*{scale:?}+{offset:?}"));
        assert_f32(bin[1], ours[1], &format!("y' {uv:?}*{scale:?}+{offset:?}"));
        count += 2;
    }
    eprintln!("ffx_transform_quad_verts: {count} coord cases bit-exact against the exe, 0x6cd750");
}

// ffx_fullscreen_quad, `0x6cd580`: the texel-centre UV and the NDC half-pixel corners (the default branch).
const FSUB_HALF: [u8; 6] = [0xd8, 0x25, 0x24, 0xfa, 0x7f, 0x00]; // fsub [0x7ffa24=0.5]
const FADD_HALF: [u8; 6] = [0xd8, 0x05, 0x24, 0xfa, 0x7f, 0x00]; // fadd [0x7ffa24]
const FDIVR_ONE: [u8; 6] = [0xd8, 0x3d, 0xd8, 0xf9, 0x7f, 0x00]; // fdivr [0x7ff9d8=1.0]
const FLD_HALF: [u8; 6] = [0xd9, 0x05, 0x24, 0xfa, 0x7f, 0x00]; //  fld [0x7ffa24]
const FMUL_HALF: [u8; 6] = [0xd8, 0x0d, 0x24, 0xfa, 0x7f, 0x00]; // fmul [0x7ffa24]
const FMUL_S1: [u8; 2] = [0xd8, 0xc9]; // fmul st,st(1)
const FMUL_S2: [u8; 2] = [0xd8, 0xca]; // fmul st,st(2)

fn fild_abs(va: u32) -> Vec<u8> {
    let mut c = vec![0xdb, 0x05];
    c.extend_from_slice(&va.to_le_bytes());
    c
}
fn fld_abs(va: u32) -> Vec<u8> {
    let mut c = vec![0xd9, 0x05];
    c.extend_from_slice(&va.to_le_bytes());
    c
}
fn fstp_abs(va: u32) -> Vec<u8> {
    let mut c = vec![0xd9, 0x1d];
    c.extend_from_slice(&va.to_le_bytes());
    c
}

#[test]
fn ffx_fullscreen_quad_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(0x007f_fa24, 0.5_f32.to_bits());
    emu.write_u32(0x007f_f9d8, 1.0_f32.to_bits());
    for (a, b) in [
        (0x6cd5ae, &FSUB_HALF),
        (0x6cd69a, &FADD_HALF),
        (0x6cd6a3, &FADD_HALF),
        (0x6cd6ab, &FDIVR_ONE),
        (0x6cd6b4, &FDIVR_ONE),
        (0x6cd6bd, &FLD_HALF),
        (0x6cd6cb, &FMUL_HALF),
        (0x6cd5bc, &FADD_HALF),
    ] {
        assert_eq!(emu.read(a, 6), b.as_slice(), "fp bytes @ {a:#x}");
    }
    assert_eq!(emu.read(0x6cd6c3, 2), FMUL_S1, "fmul s1");
    assert_eq!(emu.read(0x6cd6d4, 2), FMUL_S2, "fmul s2");
    assert_eq!(emu.read(0x6cd6dc, 2), FMUL_S1, "fmul s1 v");
    let (sw, sh, dw, dh) = (
        emu.alloc(0x10),
        emu.alloc(0x10),
        emu.alloc(0x10),
        emu.alloc(0x10),
    );
    let inv_h = emu.alloc(0x10);
    let (ou, ov, ohu, ohv) = (
        emu.alloc(0x10),
        emu.alloc(0x10),
        emu.alloc(0x10),
        emu.alloc(0x10),
    );
    let uv_isle = emu.alloc(0x80);
    let mut c = Vec::new();
    c.extend(fild_abs(sw as u32));
    c.extend_from_slice(&FADD_HALF); // src_w + 0.5
    c.extend(fild_abs(sh as u32));
    c.extend_from_slice(&FADD_HALF); // src_h + 0.5
    c.extend(fild_abs(dw as u32));
    c.extend_from_slice(&FDIVR_ONE); // 1/dst_w (f64 in st0)
    c.extend(fild_abs(dh as u32));
    c.extend_from_slice(&FDIVR_ONE); // 1/dst_h
    c.extend(fstp_abs(inv_h as u32)); // spill inv_h as f32
    c.extend_from_slice(&FLD_HALF); // 0.5
    c.extend_from_slice(&FMUL_S1); // 0.5 * inv_w
    c.extend(fstp_abs(ohu as u32));
    c.extend(fld_abs(inv_h as u32)); // reload inv_h(f32)
    c.extend_from_slice(&FMUL_HALF); // inv_h * 0.5
    c.extend(fstp_abs(ohv as u32));
    c.extend_from_slice(&FMUL_S2); // inv_w * a_w
    c.extend(fstp_abs(ou as u32));
    c.extend(fld_abs(inv_h as u32));
    c.extend_from_slice(&FMUL_S1); // inv_h * a_h
    c.extend(fstp_abs(ov as u32));
    c.push(0xc3);
    let uv_ret = uv_isle + c.len() as u64 - 1;
    emu.write(uv_isle, &c);
    // The NDC corner island: the dimensions spilled to `f32`, then offset by 0.5.
    let (da, db, da32, db32) = (
        emu.alloc(0x10),
        emu.alloc(0x10),
        emu.alloc(0x10),
        emu.alloc(0x10),
    );
    let (oyf, oxf) = (emu.alloc(0x10), emu.alloc(0x10));
    let ndc_isle = emu.alloc(0x80);
    let mut n = Vec::new();
    n.extend(fild_abs(da as u32));
    n.extend(fstp_abs(da32 as u32));
    n.extend(fild_abs(db as u32));
    n.extend(fstp_abs(db32 as u32));
    n.extend(fld_abs(da32 as u32));
    n.extend_from_slice(&FSUB_HALF); // dim_a - 0.5
    n.extend(fstp_abs(oyf as u32));
    n.extend(fld_abs(db32 as u32));
    n.extend_from_slice(&FADD_HALF); // dim_b + 0.5
    n.extend(fstp_abs(oxf as u32));
    n.push(0xc3);
    let ndc_ret = ndc_isle + n.len() as u64 - 1;
    emu.write(ndc_isle, &n);
    let mut s = 0x6cd5_8001u32;
    let mut cases: Vec<(i32, i32, i32, i32)> = vec![
        (1, 1, 1, 1),
        (100, 200, 256, 512),
        (1920, 1080, 2048, 1024),
        (640, 480, 640, 480),
        (8, 8, 8, 8),
    ];
    for _ in 0..4000 {
        cases.push((
            frand(&mut s, 1.0, 4096.0) as i32,
            frand(&mut s, 1.0, 4096.0) as i32,
            (frand(&mut s, 1.0, 4096.0)).max(1.0) as i32,
            (frand(&mut s, 1.0, 4096.0)).max(1.0) as i32,
        ));
    }
    for (sw_, sh_, dw_, dh_) in cases {
        emu.write_u32(sw, sw_ as u32);
        emu.write_u32(sh, sh_ as u32);
        emu.write_u32(dw, dw_ as u32);
        emu.write_u32(dh, dh_ as u32);
        for o in [ou, ov, ohu, ohv, inv_h] {
            emu.write_u32(o, 0);
        }
        emu.run(uv_isle, uv_ret);
        let bin = FfxQuadUv {
            u: emu.read_f32(ou),
            v: emu.read_f32(ov),
            half_u: emu.read_f32(ohu),
            half_v: emu.read_f32(ohv),
        };
        let ours = ffx_fullscreen_quad_uv(sw_, sh_, dw_, dh_);
        assert_f32(bin.u, ours.u, &format!("U {sw_}/{dw_}"));
        assert_f32(bin.v, ours.v, &format!("V {sh_}/{dh_}"));
        assert_f32(bin.half_u, ours.half_u, &format!("halfU {dw_}"));
        assert_f32(bin.half_v, ours.half_v, &format!("halfV {dh_}"));
        emu.write_u32(da, sh_ as u32);
        emu.write_u32(db, sw_ as u32);
        for o in [oyf, oxf, da32, db32] {
            emu.write_u32(o, 0);
        }
        emu.run(ndc_isle, ndc_ret);
        let (ox, oy) = ffx_fullscreen_quad_corner(sh_, sw_);
        assert_f32(emu.read_f32(oxf), ox, &format!("x_far {sw_}"));
        assert_f32(emu.read_f32(oyf), oy, &format!("y_far {sh_}"));
    }
    eprintln!("ffx_fullscreen_quad: 4005 UV+corner cases bit-exact against the exe, 0x6cd580 (default branch)");
}

// ffx_compute_rt_dims, `0x6cdb40`: the reciprocals `1 / w`, `1 / h` as an island; the half, quarter and clamp integers compared with the transcription.
const FDIVR_1_SRC: u64 = 0x6cdb90;

fn recip_island(emu: &mut Emu) -> impl Fn(&mut Emu, i32) -> f32 {
    assert_eq!(
        emu.read(FDIVR_1_SRC, 6),
        FDIVR_ONE,
        "fdivr @ {FDIVR_1_SRC:#x}"
    );
    emu.write_u32(0x007f_f9d8, 1.0_f32.to_bits());
    let in_slot = emu.alloc(0x10);
    let out_slot = emu.alloc(0x10);
    let isle = emu.alloc(0x40);
    let mut code = vec![0xdb, 0x05];
    code.extend_from_slice(&(in_slot as u32).to_le_bytes());
    code.extend_from_slice(&FDIVR_ONE);
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    code.push(0xc3);
    let ret_at = isle + code.len() as u64 - 1;
    emu.write(isle, &code);
    move |emu: &mut Emu, n: i32| {
        emu.write_u32(in_slot, n as u32);
        emu.write_u32(out_slot, 0);
        emu.run(isle, ret_at);
        emu.read_f32(out_slot)
    }
}

#[test]
fn ffx_compute_rt_dims_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let recip = recip_island(&mut emu);
    let mut cases: Vec<(i32, i32, i32)> = vec![
        (1024, 768, 2048),
        (800, 600, 1024),
        (640, 480, 256),
        (4, 4, 2048),
        (8, 8, 2048),
        (10, 9, 2048),
        (5000, 4000, 2048),
    ];
    let mut s = 0x6cdb_4001u32;
    for _ in 0..2000 {
        let cap = [256i32, 512, 1024, 2048, 4096][(lcg(&mut s) % 5) as usize];
        cases.push((
            frand(&mut s, 1.0, 4096.0) as i32,
            frand(&mut s, 1.0, 4096.0) as i32,
            cap,
        ));
    }
    for (w, h, cap) in cases {
        // The reciprocals run through the exe's own `fdivr` (island).
        assert_f32(
            recip(&mut emu, w),
            ffx_compute_rt_dims(w, h, cap).inv_w,
            &format!("inv_w {w}"),
        );
        assert_f32(
            recip(&mut emu, h),
            ffx_compute_rt_dims(w, h, cap).inv_h,
            &format!("inv_h {h}"),
        );
        // The integer downsample and clamp are the transcription's own (the `sar` and clamp idioms).
        let clamp = |x: i32| {
            let m = if x < cap { x } else { cap };
            if (m as u32) > 8 {
                m
            } else {
                8
            }
        };
        let (fw, fh) = (clamp(w), clamp(h));
        let half = |x: i32| x.wrapping_sub(x >> 31) >> 1;
        let quarter = |x: i32| x.wrapping_add((x >> 31) & 3) >> 2;
        let d = ffx_compute_rt_dims(w, h, cap);
        assert_eq!(d.full, (fw, fh), "full {w}x{h} cap {cap}");
        assert_eq!(d.half, (half(fw), half(fh)), "half {w}x{h}");
        assert_eq!(d.quarter, (quarter(fw), quarter(fh)), "quarter {w}x{h}");
    }
    eprintln!(
        "ffx_compute_rt_dims: 2007 cases (recip island-diffed; int dims 1:1) vs WoW.exe @ 0x6cdb40"
    );
}

// ffx_glow_pass_render, `0x6cb310`: the frame phases, the UV scroll, the colour packs and the desaturation scale.
const FIDIV_3174: [u8; 6] = [0xda, 0x35, 0xc4, 0x89, 0xce, 0x00]; // fidiv [0xce89c4=3174]
const FIDIV_2805: [u8; 6] = [0xda, 0x35, 0xc8, 0x89, 0xce, 0x00]; // fidiv [0xce89c8=2805]
const FMUL_088: [u8; 6] = [0xd8, 0x0d, 0xc0, 0x89, 0xce, 0x00]; // fmul [0xce89c0=0.88]
const FMUL_1_0: [u8; 6] = [0xd8, 0x0d, 0xbc, 0x89, 0xce, 0x00]; // fmul [0xce89bc=1.0]
const FMUL_INV128: [u8; 6] = [0xd8, 0x0d, 0xf0, 0xa1, 0x80, 0x00]; // fmul [0x80a1f0=1/128]
const FLD_3_0: [u8; 6] = [0xd9, 0x05, 0x7c, 0x14, 0x81, 0x00]; // fld [0x81147c=3.0]
const FMUL_0_25: [u8; 6] = [0xd8, 0x0d, 0xb0, 0x29, 0x80, 0x00]; // fmul [0x8029b0=0.25]

fn phase_island(
    emu: &mut Emu,
    fidiv: &[u8; 6],
    src: u64,
    slot: u64,
    div: u32,
) -> impl Fn(&mut Emu, u32) -> f32 {
    assert_eq!(emu.read(src, 6), fidiv, "fidiv @ {src:#x}");
    emu.write_u32(slot as u32 as u64, div);
    let in_slot = emu.alloc(0x10);
    let out_slot = emu.alloc(0x10);
    let isle = emu.alloc(0x40);
    let mut code = vec![0xdf, 0x2d]; // fild qword [in]
    code.extend_from_slice(&(in_slot as u32).to_le_bytes());
    code.extend_from_slice(fidiv);
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    code.push(0xc3);
    let ret_at = isle + code.len() as u64 - 1;
    emu.write(isle, &code);
    move |emu: &mut Emu, rem: u32| {
        emu.write_u32(in_slot, rem);
        emu.write_u32(in_slot + 4, 0);
        emu.write_u32(out_slot, 0);
        emu.run(isle, ret_at);
        emu.read_f32(out_slot)
    }
}

fn uvscroll_island(
    emu: &mut Emu,
    fa: &[u8; 6],
    fa_src: u64,
    fb_src: u64,
) -> impl Fn(&mut Emu, i32) -> f32 {
    assert_eq!(emu.read(fa_src, 6), fa, "fmul-k @ {fa_src:#x}");
    assert_eq!(emu.read(fb_src, 6), FMUL_INV128, "fmul-1/128 @ {fb_src:#x}");
    emu.write_u32(0x00ce_89c0, 0.88_f32.to_bits());
    emu.write_u32(0x00ce_89bc, 1.0_f32.to_bits());
    emu.write_u32(0x0080_a1f0, (1.0_f32 / 128.0).to_bits());
    let in_slot = emu.alloc(0x10);
    let out_slot = emu.alloc(0x10);
    let isle = emu.alloc(0x40);
    let mut code = vec![0xdb, 0x05];
    code.extend_from_slice(&(in_slot as u32).to_le_bytes());
    code.extend_from_slice(fa);
    code.extend_from_slice(&FMUL_INV128);
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    code.push(0xc3);
    let ret_at = isle + code.len() as u64 - 1;
    emu.write(isle, &code);
    move |emu: &mut Emu, n: i32| {
        emu.write_u32(in_slot, n as u32);
        emu.write_u32(out_slot, 0);
        emu.run(isle, ret_at);
        emu.read_f32(out_slot)
    }
}

fn deathfade_island(emu: &mut Emu) -> impl Fn(&mut Emu, u32) -> f32 {
    assert_eq!(emu.read(0x6cb738, 6), FLD_3_0, "fld 3.0");
    assert_eq!(emu.read(0x6cb73e, 6), FMUL_0_25, "fmul 0.25");
    emu.write_u32(0x0081_147c, 3.0_f32.to_bits());
    emu.write_u32(0x0080_29b0, 0.25_f32.to_bits());
    let in_slot = emu.alloc(0x10);
    let out_slot = emu.alloc(0x10);
    let isle = emu.alloc(0x40);
    let mut code = Vec::new();
    code.extend_from_slice(&FLD_3_0);
    code.extend_from_slice(&[0xd8, 0x0d]); // fmul dword [in]
    code.extend_from_slice(&(in_slot as u32).to_le_bytes());
    code.extend_from_slice(&FMUL_0_25);
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    code.push(0xc3);
    let ret_at = isle + code.len() as u64 - 1;
    emu.write(isle, &code);
    move |emu: &mut Emu, member: u32| {
        emu.write_u32(in_slot, member);
        emu.write_u32(out_slot, 0);
        emu.run(isle, ret_at);
        emu.read_f32(out_slot)
    }
}

#[test]
fn ffx_glow_pass_render_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut s = 0x6cb3_1001u32;
    let mut tms = vec![0u32, 1, 3173, 3174, 6347, 100_000, 0xFFFF_FFFF];
    for _ in 0..3000 {
        tms.push(lcg(&mut s));
    }
    // Frame phases: (ms mod D) / D for D = 3174 and 2805.
    let phase1 = phase_island(&mut emu, &FIDIV_3174, 0x6cb467, 0x00ce_89c4, 3174);
    for &ms in &tms {
        assert_f32(
            phase1(&mut emu, ms % 3174),
            ffx_glow_phase(ms, 3174),
            &format!("phase1 {ms}"),
        );
    }
    let phase2 = phase_island(&mut emu, &FIDIV_2805, 0x6cb47d, 0x00ce_89c8, 2805);
    for &ms in &tms {
        assert_f32(
            phase2(&mut emu, ms % 2805),
            ffx_glow_phase(ms, 2805),
            &format!("phase2 {ms}"),
        );
    }
    // The UV scroll: n * k * (1 / 128), k = 0.88 (U) and 1.0 (V).
    let mut ns: Vec<i32> = vec![0, 1, -1, 128, 1000, -5000, i32::MAX, i32::MIN];
    for _ in 0..3000 {
        ns.push(lcg(&mut s) as i32);
    }
    let uv_u = uvscroll_island(&mut emu, &FMUL_088, 0x6cb49b, 0x6cb4a1);
    for &n in &ns {
        assert_f32(
            uv_u(&mut emu, n),
            ffx_glow_uv_scroll(n, 0.88),
            &format!("uv_u {n}"),
        );
    }
    let uv_v = uvscroll_island(&mut emu, &FMUL_1_0, 0x6cb4b1, 0x6cb4b7);
    for &n in &ns {
        assert_f32(
            uv_v(&mut emu, n),
            ffx_glow_uv_scroll(n, 1.0),
            &format!("uv_v {n}"),
        );
    }
    // The colour packs, through the pack island at this function's pack sites.
    let scene = pack_island(&mut emu, false, &FMUL_255, 0x6cb562, &FADD_512, 0x6cb56d);
    let mut probes = vec![0.0f32, 1.0, 0.5, 0.25, -1.0, 2.0];
    for _ in 0..2000 {
        probes.push(frand(&mut s, -1.0, 2.0));
    }
    for x in probes {
        assert_f32(
            scene(&mut emu, x.to_bits()),
            ((f64::from(x) * 255.0) + 512.0) as f32,
            &format!("glow scene {x}"),
        );
    }
    let pct = pack_island(&mut emu, true, &FMUL_2_55, 0x6cb5ae, &FADD_512, 0x6cb5b4);
    for p in 0..=100i32 {
        assert_f32(
            pct(&mut emu, p as u32),
            ((f64::from(p) * f64::from(2.55_f32)) + 512.0) as f32,
            &format!("glow pct {p}"),
        );
    }
    // The desaturation scale: 3.0 * member * 0.25 (member 1.0 gives 0.75).
    let death = deathfade_island(&mut emu);
    let mut members = vec![1.0f32, 0.0, 0.5, 2.0, -1.0, 0.75, 100.0];
    for _ in 0..2000 {
        members.push(frand(&mut s, -4.0, 4.0));
    }
    for m in members {
        assert_f32(
            death(&mut emu, m.to_bits()),
            ffx_glow_death_fade(m),
            &format!("death-fade {m}"),
        );
    }
    assert_f32(ffx_glow_death_fade(1.0), 0.75, "death-fade const");
    eprintln!("ffx_glow_pass_render: phases+uv-scroll+color-packs+death-fade bit-exact against the exe, 0x6cb310");
}

// ffx_glow_wave_lut, `0x6cbea0`: an entry is `pack(fsin(i * 2π / 128))`. The `fsin` differs between
// CPUs and the emulator's may be the host's, so a comparison of the two is circular and the test
// leaves it out. The multiply chains (`* 128` signed, `(* 0.5 + 0.5) * 255` unsigned) run as islands;
// the clamp (`fcom; fnstsw; test ah,5; jp/jnp`) and `__ftol` are compared with the transcription.
// `sine` is the `fsin` result, drawn over the full range `[-1, 1]`.
const FMUL_128: [u8; 6] = [0xd8, 0x0d, 0x3c, 0x65, 0x80, 0x00]; // fmul [0x80653c=128] @0x6cbf50
const FMUL_05: [u8; 6] = [0xd8, 0x0d, 0x24, 0xfa, 0x7f, 0x00]; //  fmul [0x7ffa24=0.5]  @0x6cc03a
const FADD_05: [u8; 6] = [0xd8, 0x05, 0x24, 0xfa, 0x7f, 0x00]; //  fadd [0x7ffa24=0.5]  @0x6cc040
const FMUL_255_LUT: [u8; 6] = [0xd8, 0x0d, 0x58, 0xfe, 0x7f, 0x00]; // fmul [0x7ffe58=255] @0x6cc046

fn read_f64(emu: &Emu, va: u64) -> f64 {
    f64::from_le_bytes(emu.read(va, 8).try_into().unwrap())
}

/// The island for a glow-wave pack multiply chain: `fld qword [sine]; <ops>; fstp qword [out]`. It
/// returns the exe's value before the clamp (an `f64`, as the clamp and `__ftol` see it). `ops` are the
/// function's own `fmul` and `fadd` instructions, each asserted equal to the bytes at its address.
fn lut_mul_island(emu: &mut Emu, ops: &[(&[u8; 6], u64)]) -> impl Fn(&mut Emu, f64) -> f64 {
    for (bytes, src) in ops {
        assert_eq!(
            emu.read(*src, 6),
            bytes.as_slice(),
            "lut op bytes @ {src:#x}"
        );
    }
    emu.write_u32(0x0080_653c, 128.0_f32.to_bits());
    emu.write_u32(0x007f_fa24, 0.5_f32.to_bits());
    emu.write_u32(0x007f_fe58, 255.0_f32.to_bits());
    let in_slot = emu.alloc(0x10);
    let out_slot = emu.alloc(0x10);
    let isle = emu.alloc(0x60);
    let mut code = vec![0xdd, 0x05]; // fld qword [in_slot]
    code.extend_from_slice(&(in_slot as u32).to_le_bytes());
    for (bytes, _) in ops {
        code.extend_from_slice(*bytes);
    }
    code.push(0xdd); // fstp qword [out_slot]
    code.push(0x1d);
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    code.push(0xc3);
    let ret_at = isle + code.len() as u64 - 1;
    emu.write(isle, &code);
    move |emu: &mut Emu, sine: f64| {
        emu.write(in_slot, &sine.to_le_bytes());
        emu.write(out_slot, &[0u8; 8]);
        emu.run(isle, ret_at);
        read_f64(emu, out_slot)
    }
}

#[test]
fn ffx_glow_wave_lut_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let s8 = lut_mul_island(&mut emu, &[(&FMUL_128, 0x6cbf50)]);
    let u8i = lut_mul_island(
        &mut emu,
        &[
            (&FMUL_05, 0x6cc03a),
            (&FADD_05, 0x6cc040),
            (&FMUL_255_LUT, 0x6cc046),
        ],
    );
    let mut s = 0x6cbe_a001u32;
    // The sine range [-1, 1], its edges and the clamp triggers (`s * 128 >= 127`, the edges of
    // `(s * 0.5 + 0.5) * 255`).
    let mut sines = vec![-1.0f64, 0.0, 1.0, 0.9921875, 0.99219, -0.99219, 0.5, -0.5];
    for _ in 0..4000 {
        sines.push(f64::from(frand(&mut s, -1.0, 1.0)));
    }
    // NaN fails both clamp tests and reaches `__ftol`, whose indefinite has the low dword 0; the
    // infinities and values past the range are clamped before it.
    sines.extend([
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0e300,
        -1.0e300,
    ]);
    for sine in sines {
        let bin_a = s8(&mut emu, sine);
        assert_f64(bin_a, sine * 128.0, &format!("s8 mul {sine}"));
        let byte_a = {
            let c = if bin_a < -128.0 {
                -128.0
            } else if bin_a >= 127.0 {
                127.0
            } else {
                bin_a
            };
            (benilla_re_cmath::x87::ftol(c) & 0xff) as u8
        };
        assert_eq!(byte_a, ffx_glow_lut_s8(sine), "s8 byte {sine}");
        if sine.is_nan() {
            assert_eq!(byte_a, 0, "NaN reaches __ftol and gives the low dword 0");
        }

        let bin_b = u8i(&mut emu, sine);
        assert_f64(
            bin_b,
            ((sine * 0.5) + 0.5) * 255.0,
            &format!("u8 mul {sine}"),
        );
        let byte_b = {
            let c = if bin_b < 0.0 {
                0.0
            } else if bin_b >= 255.0 {
                255.0
            } else {
                bin_b
            };
            (benilla_re_cmath::x87::ftol(c) & 0xff) as u8
        };
        assert_eq!(byte_b, ffx_glow_lut_u8(sine), "u8 byte {sine}");
    }
    eprintln!(
        "ffx_glow_wave_lut: 4013 sine cases, multiply islands bit-exact against the exe (0x6cbea0); \
         clamp and ftol by transcription; fsin not compared"
    );
}
