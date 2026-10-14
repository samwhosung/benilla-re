//! The sky drawing band (`dn7e_fp`) against the exe: the glare occlusion tests `0x7e50a0` and
//! `0x7e5220`, the basis `0x7e5670`, the camera-relative deltas of `0x7e57e0` and `0x7e5b90`, the
//! cloud strip colour byte of `0x7e5f80` and the star scale of `0x7e6120`.
//!
//! `0x7e5670` runs as a whole function. The others run the exe's instructions between two addresses
//! on a frame laid out in the emulator, halting before the `call 0x4549a0` that stores the vector
//! (a plain store). The coverage tail of `0x7e5220` leaves its result in st0, so the function's own
//! `ret` is redirected to a `fstp qword` stub that captures it.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_lighting::dn7e_fp::{
    basis_from_normal, dn7e50a0_normalize_half, dn7e5220_bbox_area, dn7e5220_coverage,
    dn7e5220_one_minus_c, star_byte_norm, strip_color_repack, vec3_sub, BASIS_EPS,
};

use common::{assert_f32, next};

/// A mapped stack for the slice runs, whose compiled push/pop and argument stores need real memory.
fn slice_esp(e: &mut Emu) -> u64 {
    let stack = e.alloc(0x1000);
    stack + 0x800
}

/// A fuzzed f32 in `[lo, hi)` from the LCG high bits.
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}
/// A fuzzed f32 over the whole bit space: any sign and magnitude, subnormals, infinities and NaN.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}
/// The constants the slices read, written so a slice does not depend on the loaded image.
const G_ONE: u64 = 0x007f_f9d8;
const G_HALF: u64 = 0x007f_fa24;
const G_BASIS_EPS: u64 = 0x0080_c5c8;
const G_255: u64 = 0x007f_fe58;
const G_512: u64 = 0x0080_29cc;
const G_INV255: u64 = 0x0080_26c8;
const G_005: u64 = 0x0080_fec4;
const G_STRIP_SCALE: u64 = 0x00ce_99d0;

fn install_band_consts(e: &mut Emu) {
    e.write_u32(G_ONE, 0x3f80_0000);
    e.write_u32(G_HALF, 0x3f00_0000);
    e.write_u32(G_BASIS_EPS, 0x3727_c5ac);
    e.write_u32(G_255, 0x437f_0000);
    e.write_u32(G_512, 0x4400_0000);
    e.write_u32(G_INV255, 0x3b80_8081);
    e.write_u32(G_005, 0x3ba3_d70a);
}

/// `0x7e5670` whole function (`ecx` the direction, `edx` the output basis, `0x456280` run real):
/// the fallback arm (axes, near-vertical) and the perpendicular arm (diagonals), a direction where
/// the wide and the f32-stored `Vx` fall on opposite sides of the epsilon, zero and NaN directions,
/// and a fuzz over finite directions.
#[test]
fn d7_basis_from_normal_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    install_band_consts(&mut emu);
    let src = emu.alloc(0x10);
    let dst = emu.alloc(0x40);
    let mut s = 0x7e56_70a0u32;
    let mut seen_perp = false;
    let mut seen_fallback = false;

    let mut case = |emu: &mut Emu, v: [f32; 3], ctx: &str| {
        for (i, &c) in v.iter().enumerate() {
            emu.write_u32(src + 4 * i as u64, c.to_bits());
        }
        for off in [0u64, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28] {
            emu.write_u32(dst + off, 0xdead_beef);
        }
        emu.call(0x7e5670, src as u32, dst as u32, &[]);
        let (ov, ou, ow) = basis_from_normal(v);
        let bv = [
            emu.read_f32(dst),
            emu.read_f32(dst + 4),
            emu.read_f32(dst + 8),
        ];
        let bu = [
            emu.read_f32(dst + 0x10),
            emu.read_f32(dst + 0x14),
            emu.read_f32(dst + 0x18),
        ];
        let bw = [
            emu.read_f32(dst + 0x20),
            emu.read_f32(dst + 0x24),
            emu.read_f32(dst + 0x28),
        ];
        for k in 0..3 {
            assert_f32(bv[k], ov[k], &format!("{ctx} V[{k}]"));
            assert_f32(bu[k], ou[k], &format!("{ctx} U[{k}]"));
            assert_f32(bw[k], ow[k], &format!("{ctx} W[{k}]"));
        }
        let n_len =
            (f64::from(v[0]).powi(2) + f64::from(v[1]).powi(2) + f64::from(v[2]).powi(2)).sqrt();
        let inv = 1.0 / n_len;
        let nvx = (inv * f64::from(v[0])) as f32;
        let nvy = (inv * f64::from(v[1])) as f32;
        let p = (f64::from(nvx) * f64::from(-nvy)).abs();
        if p > f64::from(BASIS_EPS) {
            seen_perp = true;
        } else {
            seen_fallback = true;
        }
    };

    case(&mut emu, [0.0, 0.0, 1.0], "+Z axis -> fallback");
    case(&mut emu, [0.0, 0.0, -1.0], "-Z axis -> fallback");
    case(&mut emu, [1.0, 0.0, 0.0], "+X axis -> p=0 fallback");
    case(&mut emu, [0.0, 1.0, 0.0], "+Y axis -> p=0 fallback");
    case(&mut emu, [1.0, 1.0, 0.0], "XY 45 -> perp");
    case(&mut emu, [1.0, 1.0, 1.0], "diagonal -> perp");
    case(&mut emu, [3.0, 4.0, 0.0], "3-4-0 -> perp");
    case(&mut emu, [0.1, 0.1, 10.0], "near-vertical small XY");
    case(&mut emu, [1.0e-3, 1.0, 0.0], "tiny Vx -> p≈Vx -> near eps");
    case(&mut emu, [3.0e-6, 1.0, 0.0], "p just below eps band");

    {
        let vstr = [
            f32::from_bits(0x3736_dfbf),
            1.0f32,
            f32::from_bits(0x3e99_9cac),
        ];
        let (vx, vy, vz) = (f64::from(vstr[0]), f64::from(vstr[1]), f64::from(vstr[2]));
        let inv = 1.0 / (vx * vx + vy * vy + vz * vz).sqrt();
        let n_vx_live = inv * vx;
        let n_vy = (inv * vy) as f32;
        let p_live = (n_vx_live * f64::from(-n_vy)).abs();
        let p_f32 = (f64::from(n_vx_live as f32) * f64::from(-n_vy)).abs();
        assert!(
            p_live > f64::from(BASIS_EPS) && p_f32 <= f64::from(BASIS_EPS),
            "d7_basis straddle must bracket eps (live→perp / f32-reload→fallback): \
             p_live={p_live:.13e} p_f32={p_f32:.13e}"
        );
        case(
            &mut emu,
            vstr,
            "fst-no-pop straddle p_live>eps>=p_f32 (binary perp; f32-reload→fallback)",
        );
    }

    let nan = f32::NAN;
    let inf = f32::INFINITY;
    case(&mut emu, [0.0, 0.0, 0.0], "zero direction");
    case(&mut emu, [nan, 1.0, 0.0], "NaN x");
    case(&mut emu, [1.0, nan, 0.0], "NaN y");
    case(&mut emu, [1.0, 1.0, nan], "NaN z");
    case(&mut emu, [inf, 1.0, 0.0], "infinite x");
    case(&mut emu, [-0.0, -0.0, -1.0], "negative zero axes");

    for i in 0..6000u32 {
        let v = match i % 5 {
            0 => [
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
            ],
            1 => [0.0, 0.0, frand(&mut s, -10.0, 10.0)],
            2 => [
                frand(&mut s, -1.0e-5, 1.0e-5),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
            ],
            3 => [
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
            ],
            _ => [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)],
        };
        if v.iter().all(|c| c.is_finite()) && v.iter().any(|&c| c != 0.0) {
            case(&mut emu, v, &format!("fuzz{i}"));
        }
    }
    assert!(
        seen_perp && seen_fallback,
        "both basis arms exercised (perp={seen_perp} fallback={seen_fallback})"
    );
}

/// `0x7e50a0` from `0x7e50af` to `0x7e5120` (`edi` the glare position, `eax` the origin state; the
/// three `fstp [esp+N]` results): axis directions that expose the z square, a general case, a fuzz,
/// and NaN and infinite positions.
#[test]
fn d7_50a0_normalize_half_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    install_band_consts(&mut emu);
    let src = emu.alloc(0x40);
    let origin = emu.alloc(0x40);
    let frame = emu.alloc(0x40);
    let ebp = frame + 0x20;
    let esp = slice_esp(&mut emu);
    let mut s = 0x7e50_a0a0u32;

    let case = |emu: &mut Emu, sv: [f32; 3], ov: [f32; 3], ctx: &str| {
        emu.write_u32(src + 0xc, sv[0].to_bits());
        emu.write_u32(src + 0x10, sv[1].to_bits());
        emu.write_u32(src + 0x14, sv[2].to_bits());
        emu.write_u32(origin + 0x18, ov[0].to_bits());
        emu.write_u32(origin + 0x1c, ov[1].to_bits());
        emu.write_u32(origin + 0x20, ov[2].to_bits());
        emu.set_reg(RegisterX86::EDI, src);
        emu.set_reg(RegisterX86::EAX, origin);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x7e50af, 0x7e5120);
        let bin = [
            emu.read_f32(esp - 0xc),
            emu.read_f32(esp - 0x8),
            emu.read_f32(esp - 0x4),
        ];
        let ours = dn7e50a0_normalize_half(sv, ov);
        assert_f32(bin[0], ours[0], &format!("{ctx} x"));
        assert_f32(bin[1], ours[1], &format!("{ctx} y"));
        assert_f32(bin[2], ours[2], &format!("{ctx} z"));
    };

    case(&mut emu, [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], "unit +X");
    case(
        &mut emu,
        [0.0, 0.0, 1.0],
        [0.0, 0.0, 0.0],
        "unit +Z (z trap)",
    );
    case(&mut emu, [3.0, 4.0, 0.0], [0.0, 0.0, 0.0], "3-4-0");
    case(&mut emu, [1.0, 1.0, 1.0], [0.0, 0.0, 0.0], "diagonal");
    case(&mut emu, [10.5, -3.25, 7.75], [1.5, 2.5, -0.5], "general");
    let nan = f32::NAN;
    let inf = f32::INFINITY;
    case(&mut emu, [nan, 1.0, 1.0], [0.0; 3], "NaN x");
    case(&mut emu, [1.0, 1.0, nan], [0.0; 3], "NaN z");
    case(&mut emu, [inf, 1.0, 1.0], [0.0; 3], "infinite x");
    case(
        &mut emu,
        [inf, 1.0, 1.0],
        [inf, 0.0, 0.0],
        "infinity minus infinity",
    );
    case(&mut emu, [1.0, 1.0, 1.0], [1.0, 1.0, 1.0], "zero delta");
    for i in 0..4000u32 {
        let sv = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
        ];
        let ov = match i % 3 {
            0 => [0.0, 0.0, 0.0],
            1 => [
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
            ],
            _ => [
                frand(&mut s, -1.0e4, 1.0e4),
                frand(&mut s, -1.0e4, 1.0e4),
                frand(&mut s, -1.0e4, 1.0e4),
            ],
        };
        if sv.iter().zip(ov.iter()).any(|(a, b)| a != b) {
            case(&mut emu, sv, ov, &format!("fuzz{i}"));
        }
    }
}

/// `0x7e57e0` from `0x7e5943` to `0x7e5966` (`esi` the source, `edi` the origin): the three
/// subtractions that feed `0x4549a0`, over cancelling values and the whole f32 bit space.
#[test]
fn d7_vec3_sub_57e0_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let src = emu.alloc(0x40);
    let origin = emu.alloc(0x40);
    let esp = slice_esp(&mut emu);
    let mut s = 0x57e0_aa01u32;

    let case = |emu: &mut Emu, sv: [f32; 3], ov: [f32; 3], ctx: &str| {
        emu.write_u32(src + 0xc, sv[0].to_bits());
        emu.write_u32(src + 0x10, sv[1].to_bits());
        emu.write_u32(src + 0x14, sv[2].to_bits());
        emu.write_u32(origin + 0x18, ov[0].to_bits());
        emu.write_u32(origin + 0x1c, ov[1].to_bits());
        emu.write_u32(origin + 0x20, ov[2].to_bits());
        emu.set_reg(RegisterX86::ESI, src);
        emu.set_reg(RegisterX86::EDI, origin);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x7e5943, 0x7e5966);
        let bin = [
            emu.read_f32(esp - 0xc),
            emu.read_f32(esp - 0x8),
            emu.read_f32(esp - 0x4),
        ];
        let ours = vec3_sub(sv, ov);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx}[{k}]"));
        }
    };

    case(&mut emu, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "zero");
    case(&mut emu, [1.0, 2.0, 3.0], [0.5, 0.5, 0.5], "small");
    case(
        &mut emu,
        [1.0e7, -1.0e7, 0.0],
        [1.0, 1.0, 1.0],
        "cancellation",
    );
    for i in 0..3000u32 {
        let sv = [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)];
        let ov = [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)];
        case(&mut emu, sv, ov, &format!("fuzz{i}"));
    }
}

/// `0x7e5b90` from `0x7e5d0f` to `0x7e5d31` (`ebx` the source, `esi` the origin): the same three
/// subtractions at this site's offsets.
#[test]
fn d7_vec3_sub_5b90_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let src = emu.alloc(0x40);
    let origin = emu.alloc(0x40);
    let esp = slice_esp(&mut emu);
    let mut s = 0x5b90_bb02u32;

    let case = |emu: &mut Emu, sv: [f32; 3], ov: [f32; 3], ctx: &str| {
        emu.write_u32(src, sv[0].to_bits());
        emu.write_u32(src + 4, sv[1].to_bits());
        emu.write_u32(src + 8, sv[2].to_bits());
        emu.write_u32(origin + 0x18, ov[0].to_bits());
        emu.write_u32(origin + 0x1c, ov[1].to_bits());
        emu.write_u32(origin + 0x20, ov[2].to_bits());
        emu.set_reg(RegisterX86::EBX, src);
        emu.set_reg(RegisterX86::ESI, origin);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x7e5d0f, 0x7e5d31);
        let bin = [
            emu.read_f32(esp - 0xc),
            emu.read_f32(esp - 0x8),
            emu.read_f32(esp - 0x4),
        ];
        let ours = vec3_sub(sv, ov);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx}[{k}]"));
        }
    };

    case(&mut emu, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "zero");
    case(&mut emu, [9.0, -8.0, 7.0], [1.0, 2.0, 3.0], "small");
    for i in 0..3000u32 {
        let sv = [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)];
        let ov = [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)];
        case(&mut emu, sv, ov, &format!("fuzz{i}"));
    }
}

/// `0x7e5220` from `0x7e535f` to `0x7e537d`: the constant fold `1.0 - 0.005`.
#[test]
fn d7_5220_one_minus_c_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    install_band_consts(&mut emu);
    let frame = emu.alloc(0x40);
    let ebp = frame + 0x20;
    let esp = slice_esp(&mut emu);
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESP, esp);
    emu.set_reg(RegisterX86::ECX, 0);
    emu.run(0x7e535f, 0x7e537d);
    let bin = emu.read_f32(esp - 8);
    assert_f32(bin, dn7e5220_one_minus_c(), "1 - 0.005");
}

/// `0x7e5220` from `0x7e55b1` to `0x7e55db`: the box area `|bx - ax| * |by - ay|` from the four
/// stack slots to `[ebp-0x2c]`, with zero extents, an exact 1, a reversed box, a fuzz, NaN corners
/// and infinite extents.
#[test]
fn d7_5220_bbox_area_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let frame = emu.alloc(0x400);
    let ebp = frame + 0x200;
    let esp = slice_esp(&mut emu);
    let mut s = 0x5220_cc03u32;

    let case = |emu: &mut Emu, ax: f32, ay: f32, bx: f32, by: f32, ctx: &str| {
        emu.write_u32(ebp - 0x134, ax.to_bits());
        emu.write_u32(ebp - 0x11c, bx.to_bits());
        emu.write_u32(ebp - 0x138, ay.to_bits());
        emu.write_u32(ebp - 0x12c, by.to_bits());
        emu.write_u32(ebp - 0x2c, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x7e55b1, 0x7e55db);
        let bin = emu.read_f32(ebp - 0x2c);
        assert_f32(bin, dn7e5220_bbox_area(ax, ay, bx, by), ctx);
    };

    case(&mut emu, 0.0, 0.0, 0.0, 0.0, "all-zero -> 0");
    case(&mut emu, 5.0, 7.0, 5.0, 9.0, "zero width -> 0");
    case(&mut emu, 1.0, 1.0, 2.0, 2.0, "1x1 -> 1.0");
    case(&mut emu, 2.0, 3.0, -1.0, -1.0, "negative span (abs)");
    case(&mut emu, f32::NAN, 0.0, 1.0, 1.0, "NaN corner");
    case(
        &mut emu,
        0.0,
        0.0,
        f32::INFINITY,
        0.0,
        "infinite width, zero height",
    );
    case(&mut emu, -0.0, -0.0, 0.0, 0.0, "negative zero corner");
    for _ in 0..3000u32 {
        let ax = frand(&mut s, -1.0e3, 1.0e3);
        let ay = frand(&mut s, -1.0e3, 1.0e3);
        let bx = frand(&mut s, -1.0e3, 1.0e3);
        let by = frand(&mut s, -1.0e3, 1.0e3);
        case(&mut emu, ax, ay, bx, by, "fuzz");
    }
}

/// `0x7e5220` from `0x7e5627`: the coverage of a count over an area, captured from st0 by sending
/// the function's `ret` to a `fstp qword` stub. Entering at `0x7e5627` skips the `[esi+0x38]` gate;
/// the area is at `[ebp-0x2c]`, the count at `[ebp-4]` and `ebx` is the count's zero high dword.
/// Probes: an area of exactly 1 and just under and over it, a ratio of exactly 1 and either side, a
/// zero count, NaN and infinite areas, and a fuzz around the area-1 boundary.
#[test]
fn d7_5220_coverage_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    install_band_consts(&mut emu);
    let frame = emu.alloc(0x80);
    let ebp = frame + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0x800;
    let tramp = emu.alloc(0x10);
    let slot = emu.alloc(0x10);
    let mut code = vec![0xddu8, 0x1d];
    code.extend_from_slice(&(slot as u32).to_le_bytes());
    emu.write(tramp, &code);
    let mut s = 0x5220_ee05u32;
    let mut seen_zero = false;
    let mut seen_keep = false;
    let mut seen_clamp = false;

    let mut case = |emu: &mut Emu, count: u32, area: f32, ctx: &str| {
        emu.write_u32(ebp - 0x2c, area.to_bits());
        emu.write_u32(ebp - 4, count);
        emu.write_u32(ebp, 0);
        emu.write_u32(ebp + 4, tramp as u32);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::EBX, 0);
        emu.run(0x7e5627, tramp + 6);
        let mut b = [0u8; 8];
        b.copy_from_slice(&emu.read(slot, 8));
        let bin = f64::from_bits(u64::from_le_bytes(b));
        let ours = dn7e5220_coverage(count, area);
        assert_eq!(
            bin.to_bits(),
            ours.to_bits(),
            "{ctx}: bin={bin} ours={ours}"
        );
        if area <= 1.0 {
            seen_zero = true;
        } else if f64::from(count) / f64::from(area) > 1.0 {
            seen_clamp = true;
        } else {
            seen_keep = true;
        }
    };

    case(&mut emu, 0, 1.0, "area==1 -> 0");
    case(&mut emu, 10, 1.0, "area==1 -> 0 (count>0)");
    case(&mut emu, 10, 0.9999, "area<1 -> 0");
    case(&mut emu, 1, 2.0, "ratio 0.5 keep");
    case(&mut emu, 2, 2.0, "ratio ==1.0 keep");
    case(&mut emu, 3, 2.0, "ratio 1.5 -> clamp 1");
    case(&mut emu, 1000, 4.0, "ratio 250 -> clamp 1");
    case(&mut emu, 0, 1.5, "count 0 -> 0 ratio keep");
    case(&mut emu, 10, 1.0001, "area just >1 -> clamp 1");

    case(&mut emu, 5, f32::NAN, "NaN area");
    case(&mut emu, 5, f32::INFINITY, "infinite area");
    case(&mut emu, 5, -0.0, "negative zero area");
    case(
        &mut emu,
        5,
        f32::from_bits(0x3f80_0001),
        "area one ulp over 1",
    );
    case(
        &mut emu,
        5,
        f32::from_bits(0x3f7f_ffff),
        "area one ulp under 1",
    );
    case(&mut emu, u32::MAX, 1.5, "largest count");

    for i in 0..1500u32 {
        let count = (next(&mut s) >> 16) % 5000;
        let area = match i % 3 {
            0 => frand(&mut s, 0.0, 2.0),
            1 => frand(&mut s, 1.0001, 50.0),
            _ => frand(&mut s, 0.9, 1.1),
        };
        case(&mut emu, count, area, &format!("fuzz{i}"));
    }
    assert!(
        seen_zero && seen_keep && seen_clamp,
        "all coverage arms exercised (zero={seen_zero} keep={seen_keep} clamp={seen_clamp})"
    );
}

/// `0x7e5f80` from `0x7e602d` to `0x7e6065`: the strip alpha byte from the scale at `[0xce99d0]`,
/// read from the top byte of the packed colour. Probes: the unit range, out-of-range scales, -0,
/// NaN and infinities, and a fuzz.
#[test]
fn d7_strip_color_repack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    install_band_consts(&mut emu);
    let frame = emu.alloc(0x40);
    let ebp = frame + 0x20;
    let this = emu.alloc(0x100);
    let esp = slice_esp(&mut emu);
    let mut s = 0x5f80_dd04u32;

    let case = |emu: &mut Emu, g: f32, ctx: &str| {
        emu.write_u32(G_STRIP_SCALE, g.to_bits());
        emu.write_u32(ebp - 4, 0xdead_beef);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.set_reg(RegisterX86::ESI, this);
        emu.set_reg(RegisterX86::ECX, 0);
        emu.set_reg(RegisterX86::EAX, 0);
        emu.run(0x7e602d, 0x7e6065);
        let packed = emu.read_u32(ebp - 4);
        let bin_byte = ((packed >> 24) & 0xff) as u8;
        assert_eq!(
            bin_byte,
            strip_color_repack(g),
            "{ctx}: packed={packed:#010x}"
        );
    };

    for &g in &[0.0f32, 0.25, 0.5, 0.75, 1.0, -0.1, 1.5, 0.001, 0.999] {
        case(&mut emu, g, &format!("g={g}"));
    }
    for g in [
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fa0_0000),
        f32::from_bits(0x3b80_8081),
    ] {
        case(&mut emu, g, &format!("g={g}"));
    }
    for i in 0..2000u32 {
        let g = match i % 2 {
            0 => frand(&mut s, -0.2, 1.2),
            _ => frand(&mut s, -10.0, 10.0),
        };
        case(&mut emu, g, &format!("fuzz{i}"));
    }
}

/// `0x7e6120` from `0x7e6153` to `0x7e615f`: `byte / 255` for every byte value.
#[test]
fn d7_star_byte_norm_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    install_band_consts(&mut emu);
    let frame = emu.alloc(0x40);
    let ebp = frame + 0x20;
    let esp = slice_esp(&mut emu);
    for b in 0u16..=255 {
        let b = b as u8;
        emu.write_u32(ebp - 4, u32::from(b));
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x7e6153, 0x7e615f);
        let bin = emu.read_f32(esp);
        assert_f32(bin, star_byte_norm(b), &format!("byte={b}"));
    }
}
