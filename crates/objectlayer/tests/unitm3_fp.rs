//! The unit auto-move and placement kernels of `src/unitm3_fp.rs` against the exe, each read at
//! the code it covers with the registers, frame and fields it reads set directly (slices) or with
//! its callees stubbed (whole):
//! - `0x60b270`, `[0x60b325, 0x60b332)`: the parity of `fabs(v)` against `2^-20`;
//! - `0x60f600`, `[0x60f798, 0x60f7c2)`: the follow distance gate;
//! - `0x60fcc0`, `[0x60fcdc, 0x60fd1b)`, `[0x60fd2f, 0x60fd5c)` and `[0x60fe37, 0x60fe54)`: the
//!   reach floor, the in-reach test and the plane projection;
//! - `0x610100`, `[0x610161, 0x6101b9)` and `[0x61023b, 0x61025d)`: the aim normalize and the
//!   endpoint add;
//! - `0x610520` whole: the pitch step and the arrived byte;
//! - `0x610e40`, `[0x610f7a, 0x610fb6)` and `[0x610fb6, 0x611003)`: the squared distances and the
//!   goal radius;
//! - `0x611240`, `[0x611336, 0x61134b)` and `[0x6113b9, 0x6113cf)`, and whole for the reach cases;
//! - `0x611400`, `[0x61143b, 0x61146f)` and `[0x611429, 0x611549)`: the ring pulse and corners;
//! - `0x613ef0`, whole for the scale and `[0x614031, 0x614053)` for the bounds sums.
//!
//! Each test adds NaN, the infinities, both zeros and its thresholds, equal and a step either
//! side, to its random inputs.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_objectlayer::unitm3_fp::*;
use common::*;

// ---- shared helpers (the crate float idiom) -------------------------------

#[test]
fn box_dim_pf_even_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let obj = e.alloc(0x80); // ESI; v at +0x38
    let mut s = 0x60b2_7000u32;
    let mut seen_even = false;
    let mut seen_odd = false;

    let mut case = |e: &mut Emu, v: f32, ctx: &str| {
        e.write_u32(obj + 0x38, v.to_bits());
        e.set_reg(RegisterX86::ESI, obj);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60b325, 0x60b332);
        let a = ah(e) & 0x05;
        let bits = (a & 0x01).count_ones() + (a & 0x04).count_ones();
        let bin_even = bits.is_multiple_of(2); // PF set ⟺ even number of set bits in ah&5
        if bin_even {
            seen_even = true;
        } else {
            seen_odd = true;
        }
        assert_eq!(
            bin_even,
            box_dim_pf_even(v),
            "{ctx}: bin_even={bin_even} ah&5={a:#04x} (v={v})"
        );
    };

    let eps = f32::from_bits(0x3580_0000);
    case(&mut e, 0.0, "zero<eps → odd");
    case(&mut e, eps, "==eps → even");
    case(&mut e, eps * 2.0, ">eps → even");
    case(&mut e, eps * 0.5, "<eps → odd");
    case(&mut e, -eps * 0.5, "neg tiny → odd");
    case(&mut e, f32::NAN, "nan → even");
    for i in 0..4000u32 {
        let v = match i % 4 {
            0 => frand(&mut s, -3e-6, 3e-6), // straddle ±eps
            1 => frand(&mut s, -1e3, 1e3),
            2 => frand_bits(&mut s),
            _ => {
                let f = (next(&mut s) >> 8) as f32 / (1u32 << 23) as f32;
                eps * (f - 0.5) * 4.0 // dense near the eps edge, both signs
            }
        };
        case(&mut e, v, &format!("fuzz{i}"));
    }
    // `|v| == 2^-20` exactly and a step either side, then NaN, infinities and zeros.
    for &v in &specials(&[EPS_2EM20]) {
        case(&mut e, v, "special");
    }
    assert!(seen_even, "even-parity (|v|>=eps / NaN) never seen");
    assert!(seen_odd, "odd-parity (|v|<eps) never seen");
}

const FOLLOW_THRESH: u64 = 0xc4d988;

#[test]
fn follow_dist_proceed_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pa = e.alloc(0x20); // EAX = self pos
    let pb = e.alloc(0x20); // EDI = other pos
    let mut s = 0x60f6_0000u32;
    let mut seen_proceed = false;
    let mut seen_abort = false;

    let mut case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], thresh: f32, ctx: &str| {
        for k in 0..3 {
            e.write_u32(pa + 4 * k as u64, a[k].to_bits());
            e.write_u32(pb + 4 * k as u64, b[k].to_bits());
        }
        e.write_u32(FOLLOW_THRESH, thresh.to_bits());
        e.set_reg(RegisterX86::EAX, pa);
        e.set_reg(RegisterX86::EDI, pb);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60f798, 0x60f7c2);
        let m = ah(e) & 0x41;
        let bits = (m & 0x01).count_ones() + (m & 0x40).count_ones();
        let bin_proceed = bits % 2 == 1; // jp not taken ⟺ odd parity ⟺ proceed
        if bin_proceed {
            seen_proceed = true;
        } else {
            seen_abort = true;
        }
        assert_eq!(
            bin_proceed,
            follow_dist_proceed(a, b, thresh),
            "{ctx}: bin_proceed={bin_proceed} ah&0x41={m:#04x}"
        );
    };

    case(&mut e, [0.0; 3], [0.0; 3], 1.0, "dist0 < thresh → proceed");
    case(&mut e, [3.0, 0.0, 0.0], [0.0; 3], 4.0, "9 > 4 → abort"); // wait: dist²=9>4 → abort
    case(&mut e, [2.0, 0.0, 0.0], [0.0; 3], 4.0, "4 == 4 → proceed");
    case(&mut e, [1.0, 0.0, 0.0], [0.0; 3], 4.0, "1 < 4 → proceed");
    case(&mut e, [0.0; 3], [0.0; 3], f32::NAN, "nan thresh → abort");
    for i in 0..5000u32 {
        let scale = match i % 4 {
            0 => 1.0,
            1 => 10.0,
            2 => 100.0,
            _ => 0.0, // forces dist²==0, straddles thresh near 0
        };
        let a = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let b = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        // thresh near plausible dist² magnitudes so both arms fire densely.
        let thr = match i % 3 {
            0 => frand(&mut s, 0.0, 3.0 * scale * scale + 0.01),
            1 => frand(&mut s, 0.0, 1.0),
            _ => frand_bits(&mut s).abs(),
        };
        case(&mut e, a, b, thr, &format!("fuzz{i}"));
    }
    // `dist^2 == thresh` (3-4-5 against 25) and a step either side of each, then NaN, infinities
    // and zeros in the threshold and each coordinate.
    for x in [3.0f32, next_up(3.0), next_down(3.0)] {
        case(&mut e, [x, 4.0, 0.0], [0.0; 3], 25.0, "equal");
    }
    for t in [next_up(25.0), next_down(25.0)] {
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], t, "threshold step");
    }
    for &v in &specials(&[25.0]) {
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], v, "special thresh");
        for k in 0..3 {
            let mut a = [3.0f32, 4.0, 0.0];
            a[k] = v;
            case(&mut e, a, [0.0; 3], 25.0, "special a");
            let mut b = [0.0f32; 3];
            b[k] = v;
            case(&mut e, [3.0, 4.0, 0.0], b, 25.0, "special b");
        }
    }
    assert!(seen_proceed, "proceed (dist²<=thresh) never seen");
    assert!(seen_abort, "abort (dist²>thresh / NaN) never seen");
}

#[test]
fn cast_reach_floor_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let caster = e.alloc(0x200);
    let target = e.alloc(0x200);
    let cfields = e.alloc(0x300);
    let tfields = e.alloc(0x300);
    e.write_u32(caster + 0x110, cfields as u32);
    e.write_u32(target + 0x110, tfields as u32);
    let ebx = e.alloc(0x20); // [ebx+8] = target obj ptr
    e.write_u32(ebx + 8, target as u32);
    let mut s = 0x60fc_c000u32;

    // Start at 0x60fcdc (`mov ecx,[ebx+8]`): EBX→target via [ebx+8], ESI=caster. The body resolves
    // ecx=target, eax=tfields, edx=cfields itself. Halt at 0x60fd1b (the jump target both store arms
    // converge on). The interleaved `push edi` (0x60fcf7) reads ESP (mapped).
    let case = |e: &mut Emu, tr: f32, cr: f32, ctx: &str| {
        e.write_u32(cfields + 0x1f0, cr.to_bits());
        e.write_u32(tfields + 0x1f0, tr.to_bits());
        e.set_reg(RegisterX86::EBX, ebx);
        e.set_reg(RegisterX86::ESI, caster);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60fcdc, 0x60fd1b);
        let bin = e.read_f32(ebp - 4);
        assert_f32(bin, cast_reach_floor(tr, cr), ctx);
    };

    case(&mut e, 1.0, 1.0, "sum 3.33 < 5 → 5");
    case(&mut e, 3.0, 3.0, "sum 7.33 > 5 → keep");
    case(&mut e, 1.833_333_4, 1.833_333_4, "near-5 boundary");
    case(&mut e, f32::NAN, 1.0, "nan → keep nan");
    for i in 0..4000u32 {
        let tr = match i % 3 {
            0 => frand(&mut s, 0.0, 10.0),
            1 => frand(&mut s, 1.0, 2.5), // sum straddles 5
            _ => frand_bits(&mut s),
        };
        let cr = match i % 3 {
            0 => frand(&mut s, 0.0, 10.0),
            1 => frand(&mut s, 1.0, 2.5),
            _ => frand_bits(&mut s),
        };
        case(&mut e, tr, cr, &format!("fuzz{i}"));
    }
    // `tr + cr + 4/3 == 5.0` exactly: 30758228 * 2^-23 plus 2^-23 and 4/3 as f32; and a step
    // either side; then NaN, infinities and zeros.
    let exact = (30758228.0f64 / 8388608.0) as f32;
    let one_ulp = f32::powi(2.0, -23);
    case(&mut e, exact, one_ulp, "reach sum 5");
    case(&mut e, next_up(exact), one_ulp, "reach sum above 5");
    case(&mut e, next_down(exact), one_ulp, "reach sum below 5");
    for &v in &specials(&[3.0]) {
        case(&mut e, v, 1.0, "special target");
        case(&mut e, 1.0, v, "special caster");
    }
}

#[test]
fn cast_in_range_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    let tpos = e.alloc(0x20); // EAX
    let mut s = 0x60fc_c100u32;
    let mut seen_in = false;
    let mut seen_out = false;

    let mut case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], reach: f32, ctx: &str| {
        for (k, v) in a.iter().enumerate() {
            e.write_u32(tpos + 4 * k as u64, v.to_bits());
        }
        e.write_u32(ebp - 0x14, b[0].to_bits());
        e.write_u32(ebp - 0x10, b[1].to_bits());
        e.write_u32(ebp - 0xc, b[2].to_bits());
        e.write_u32(ebp - 4, reach.to_bits());
        e.set_reg(RegisterX86::EAX, tpos);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60fd2f, 0x60fd5c);
        let m = ah(e) & 0x41;
        // fcompp st0=reach² vs st1=dist²; C0=reach²<dist², C3=eq. `test ah,0x41; jne plane-proj`:
        // jne is ZF-based (taken ⟺ (ah&0x41)!=0), not parity. Fall-through (in-reach) ⟺ (ah&0x41)==0.
        let bin_in = m == 0; // fall-through (call 0x6100a0) ⟺ reach²>dist² ordered
        if bin_in {
            seen_in = true;
        } else {
            seen_out = true;
        }
        assert_eq!(
            bin_in,
            cast_in_range(a, b, reach),
            "{ctx}: bin_in={bin_in} ah&0x41={m:#04x}"
        );
    };

    case(
        &mut e,
        [1.0, 0.0, 0.0],
        [0.0; 3],
        2.0,
        "reach²4 > dist1 → in",
    );
    case(
        &mut e,
        [2.0, 0.0, 0.0],
        [0.0; 3],
        2.0,
        "reach²4 == dist4 → out",
    );
    case(
        &mut e,
        [3.0, 0.0, 0.0],
        [0.0; 3],
        2.0,
        "reach²4 < dist9 → out",
    );
    case(&mut e, [0.0; 3], [0.0; 3], f32::NAN, "nan reach → out");
    for i in 0..5000u32 {
        let scale = if i % 2 == 0 { 5.0 } else { 50.0 };
        let a = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let b = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let reach = match i % 3 {
            0 => frand(&mut s, 0.0, scale * 1.8), // reach² straddles dist²
            1 => frand(&mut s, 0.0, 5.0),
            _ => frand_bits(&mut s),
        };
        case(&mut e, a, b, reach, &format!("fuzz{i}"));
    }
    // `reach^2 == dist^2` (3-4-5 against 5) and a step either side, then NaN, infinities and
    // zeros in the reach and each coordinate.
    for x in [3.0f32, next_up(3.0), next_down(3.0)] {
        case(&mut e, [x, 4.0, 0.0], [0.0; 3], 5.0, "equal");
    }
    for r in [next_up(5.0), next_down(5.0)] {
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], r, "reach step");
    }
    for &v in &specials(&[5.0]) {
        case(&mut e, [3.0, 4.0, 0.0], [0.0; 3], v, "special reach");
        for k in 0..3 {
            let mut a = [3.0f32, 4.0, 0.0];
            a[k] = v;
            case(&mut e, a, [0.0; 3], 5.0, "special a");
            let mut b = [0.0f32; 3];
            b[k] = v;
            case(&mut e, [3.0, 4.0, 0.0], b, 5.0, "special b");
        }
    }
    assert!(seen_in, "in-range never seen");
    assert!(seen_out, "out-of-range never seen");
}

const PLANE_GLOBAL: u64 = 0xc4d868;

#[test]
fn cast_plane_z_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let plane = e.alloc(0x20);
    e.write_u32(PLANE_GLOBAL, plane as u32);
    let mut s = 0x60fc_c200u32;

    let case = |e: &mut Emu, dx: f32, dy: f32, p: [f32; 4], ctx: &str| {
        for (k, v) in p.iter().enumerate() {
            e.write_u32(plane + 4 * k as u64, v.to_bits());
        }
        e.write_u32(ebp - 0x14, dx.to_bits());
        e.write_u32(ebp - 0x10, dy.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x60fe37, 0x60fe54);
        let bin = e.read_f32(ebp - 0xc);
        assert_f32(bin, cast_plane_z(dx, dy, p), ctx);
    };

    case(&mut e, 1.0, 2.0, [1.0, 1.0, 1.0, 0.0], "simple");
    case(&mut e, 3.0, 4.0, [0.5, 2.0, 0.25, 1.0], "mixed");
    case(&mut e, 0.0, 0.0, [1.0, 1.0, 1.0, 0.0], "zeros");
    for i in 0..4000u32 {
        let dx = frand(&mut s, -100.0, 100.0);
        let dy = frand(&mut s, -100.0, 100.0);
        let p = if i % 4 == 0 {
            [
                frand_bits(&mut s),
                frand_bits(&mut s),
                frand_bits(&mut s),
                frand_bits(&mut s),
            ]
        } else {
            [
                frand(&mut s, -2.0, 2.0),
                frand(&mut s, -2.0, 2.0),
                // avoid plane[2]==0 most of the time, but allow it (→ ±inf) part of the time
                if i % 7 == 0 {
                    frand(&mut s, -1.0, 1.0)
                } else {
                    let v = frand(&mut s, 0.1, 4.0);
                    if i % 2 == 0 {
                        v
                    } else {
                        -v
                    }
                },
                frand(&mut s, -10.0, 10.0),
            ]
        };
        case(&mut e, dx, dy, p, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, 2.0, [1.0, 2.0, 3.0, 4.0], "special dx");
        case(&mut e, 1.0, v, [1.0, 2.0, 3.0, 4.0], "special dy");
        for k in 0..4 {
            let mut p = [1.0f32, 2.0, 3.0, 4.0];
            p[k] = v;
            case(&mut e, 1.0, 2.0, p, "special plane");
        }
    }
}

#[test]
fn cast_aim_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x60);
    let ebp = frame + 0x40;
    let this = e.alloc(0x200);
    let mvblk = e.alloc(0x80);
    e.write_u32(this + 0x118, mvblk as u32); // [esi+0x118] → movement block; [+0x40] read by the test
    let mut s = 0x6101_0000u32;

    // The slice itself stores [ebp-0x20]=EDX(=v0) at 0x610167 and [ebp-0x18]=ECX(=v2) at 0x61016d
    // (the body loaded v0→edx, v2→ecx, v1→[ebp-0x1c] just before the slice). So set EDX=v0, ECX=v2,
    // and [ebp-0x1c]=v1; the two slots are repopulated inside the slice.
    let case = |e: &mut Emu, v0: f32, v1: f32, v2: f32, ctx: &str| {
        e.write_u32(ebp - 0x1c, v1.to_bits());
        e.set_reg(RegisterX86::EDX, u64::from(v0.to_bits()));
        e.set_reg(RegisterX86::ECX, u64::from(v2.to_bits()));
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x610161, 0x6101b9);
        let bin0 = e.read_f32(ebp - 0x20);
        let bin1 = e.read_f32(ebp - 0x1c);
        let bin2 = e.read_f32(ebp - 0x18);
        let (o0, o1, o2, _okey) = cast_aim_normalize(v0, v1, v2);
        assert_f32(bin0, o0, &format!("{ctx} v0n"));
        assert_f32(bin1, o1, &format!("{ctx} v1n"));
        assert_f32(bin2, o2, &format!("{ctx} v2n"));
    };

    case(&mut e, 1.0, 0.0, 0.0, "unit-x");
    case(&mut e, 0.0, 0.0, 1.0, "unit-z");
    case(&mut e, 3.0, 4.0, 0.0, "3-4-5");
    case(&mut e, 1.0, 1.0, 1.0, "diag");
    case(&mut e, 0.0, 0.0, 0.0, "zero → 1/0 inf");
    for i in 0..3000u32 {
        let (v0, v1, v2) = if i % 4 == 0 {
            (frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s))
        } else {
            (
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
            )
        };
        case(&mut e, v0, v1, v2, &format!("fuzz{i}"));
    }
    // Unit and zero vectors, and NaN, infinities and zeros in each component.
    case(&mut e, 1.0, 0.0, 0.0, "unit x");
    case(&mut e, 0.0, 0.0, 0.0, "zero");
    for &v in &specials(&[1.0]) {
        case(&mut e, v, 1.0, 1.0, "special v0");
        case(&mut e, 1.0, v, 1.0, "special v1");
        case(&mut e, 1.0, 1.0, v, "special v2");
    }
}

// The elevation key `|(v1n + v0n)·0.0 + v2n|` is never stored: it lives on the stack to the branch
// it drives, `fcom SIN60; fnstsw; test ah,0x41; jne 0x6101f5`. The slice `[0x610161, 0x6101c3)` runs
// the normalisation, the key and that compare, and the test checks `key` against the branch: the jump
// is taken for `key <= SIN60` and NaN. Move flag `0x200000`, at `[mvblk+0x40]`, must be set or
// `0x6101b9` jumps to the position add.

#[test]
fn cast_aim_key_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x200);
    let mvblk = e.alloc(0x80);
    e.write_u32(this + 0x118, mvblk as u32);
    e.write_u32(mvblk + 0x40, 0x200000); // force the je fall-through into `fcom SIN60`
    let mut s = 0x6101_5000u32;
    let mut seen = std::collections::BTreeSet::new();

    let sin60 = f32::from_bits(0x3f5d_b37d);
    // Same slice start as cast_aim_normalize: EDX=v0, ECX=v2 (repopulated mid-slice), [ebp-0x1c]=v1.
    let mut case = |e: &mut Emu, v0: f32, v1: f32, v2: f32, ctx: &str| {
        e.write_u32(ebp - 0x1c, v1.to_bits());
        e.set_reg(RegisterX86::EDX, u64::from(v0.to_bits()));
        e.set_reg(RegisterX86::ECX, u64::from(v2.to_bits()));
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x610161, 0x6101c3);
        let m = ah(e) & 0x41; // C0(0x01)=key<SIN60, C3(0x40)=key==SIN60
        seen.insert(m);
        let bin_low = m != 0; // jne taken (low-elevation arm) ⟺ key<SIN60 OR == OR NaN(C0|C3 set)
        let (_v0n, _v1n, _v2n, key) = cast_aim_normalize(v0, v1, v2);
        // jne taken ⟺ (ah&0x41)!=0. NaN key → fcom unordered → C0|C2|C3 → ah&0x41=C0|C3≠0 → jne taken.
        let kd = f64::from(key);
        let sd = f64::from(sin60);
        let our_low = kd.is_nan() || kd < sd || kd == sd;
        assert_eq!(
            bin_low, our_low,
            "{ctx}: bin_low={bin_low} our_low={our_low} key={key} ah&0x41={m:#04x}"
        );
    };

    case(&mut e, 1.0, 0.0, 0.0, "key 0 < sin60 → low");
    case(&mut e, 0.0, 0.0, 1.0, "key 1 > sin60 → high");
    case(&mut e, 1.0, 1.0, 1.0, "diag");
    for i in 0..4000u32 {
        let (v0, v1, v2) = (
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
        );
        case(&mut e, v0, v1, v2, &format!("fuzz{i}"));
    }
    // Unit and zero vectors, and NaN, infinities and zeros in each component.
    case(&mut e, 1.0, 0.0, 0.0, "unit x");
    case(&mut e, 0.0, 0.0, 0.0, "zero");
    for &v in &specials(&[1.0]) {
        case(&mut e, v, 1.0, 1.0, "special v0");
        case(&mut e, 1.0, v, 1.0, "special v1");
        case(&mut e, 1.0, 1.0, v, "special v2");
    }
    assert!(
        seen.len() >= 2,
        "only one key/SIN60 relation seen: {seen:?}"
    );
}

#[test]
fn cast_aim_endpoint_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x60);
    let ebp = frame + 0x40;
    let pos = e.alloc(0x20); // EAX
    let this = e.alloc(0x20); // ESI (read by interleaved mov)
    let mut s = 0x6102_0000u32;

    let case = |e: &mut Emu, vn: [f32; 3], p: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x20, vn[0].to_bits());
        e.write_u32(ebp - 0x1c, vn[1].to_bits());
        e.write_u32(ebp - 0x18, vn[2].to_bits());
        for (k, v) in p.iter().enumerate() {
            e.write_u32(pos + 4 * k as u64, v.to_bits());
        }
        e.set_reg(RegisterX86::EAX, pos);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61023b, 0x61025d);
        // Store order: `fstp [ebp-0x10]`=st0=v0n+pos.x, `fstp [ebp-0xc]`=v1n+pos.y, `fstp [ebp-8]`=
        // v2n+pos.z. So x=[ebp-0x10], y=[ebp-0xc], z=[ebp-8].
        let bin = [
            e.read_f32(ebp - 0x10),
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 8),
        ];
        let ours = cast_aim_endpoint(vn, p);
        for i in 0..3 {
            assert_f32(bin[i], ours[i], &format!("{ctx} [{i}]"));
        }
    };

    case(&mut e, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0], "simple");
    case(&mut e, [0.0; 3], [0.0; 3], "zeros");
    for i in 0..4000u32 {
        let vn = [
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
            frand(&mut s, -1.0, 1.0),
        ];
        let p = if i % 3 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
            ]
        };
        case(&mut e, vn, p, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut vn = [0.5f32, 0.25, 0.125];
            vn[k] = v;
            case(&mut e, vn, [1.0, 2.0, 3.0], "special vn");
            let mut p = [1.0f32, 2.0, 3.0];
            p[k] = v;
            case(&mut e, [0.5, 0.25, 0.125], p, "special pos");
        }
    }
}

const PITCH_BEGIN: u64 = 0x60e860; // begin-movement-update (non-FP); ret 8
const PITCH_APPLY: u64 = 0x60e8a0; // 0x60e8a0(this, arg, stepped): non-FP setter; args[1]=stepped; ret 8

#[test]
fn pitch_interp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000); // current pitch at +0x9c8; this[+0xe0]=scene handle (unused on step path)
    let arrived = e.alloc(0x10);
    e.stub(PITCH_BEGIN, 0, 2);
    e.stub(PITCH_APPLY, 0, 2);
    let mut s = 0x6105_2000u32;
    let mut seen_none = false;
    let mut seen_step = false;
    let mut seen_arr = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, cur: f32, goal: f32, rate: f32, ctx: &str| {
        e.write_u32(this + 0x9c8, cur.to_bits());
        e.write_u32(arrived, 0xdead_beef); // sentinel: detect whether the fn wrote the byte
        e.clear_calls();
        // a=0.0, b=1.0 (a<b) → entry jne taken → MAIN step (skip scene-occlusion). args: a,b,goal,
        // rate, arg(=0), &arrived.
        let al = e.call(
            0x610520,
            this as u32,
            0,
            &[
                0.0f32.to_bits(),
                1.0f32.to_bits(),
                goal.to_bits(),
                rate.to_bits(),
                0,
                arrived as u32,
            ],
        ) & 0xff;
        let ours = pitch_interp(cur, goal, rate);
        match ours {
            None => {
                // dead-band: the early `xor al,al; ret` (0x61060f). No apply call, al==0.
                seen_none = true;
                assert_eq!(
                    al, 0,
                    "{ctx}: expected dead-band (al=0) got al={al} (cur={cur} goal={goal} rate={rate})"
                );
                assert!(
                    e.calls().iter().all(|c| c.addr != PITCH_APPLY),
                    "{ctx}: dead-band must not apply"
                );
            }
            Some((our_step, our_byte)) => {
                seen_step = true;
                // the apply 0x60e8a0(this, arg, stepped): args[1] = the applied stepped value.
                let call = e
                    .calls()
                    .iter()
                    .find(|c| c.addr == PITCH_APPLY)
                    .copied()
                    .unwrap_or_else(|| panic!("{ctx}: apply not called (al={al})"));
                let bin_step = f32::from_bits(call.args[1]);
                assert_f32(bin_step, our_step, &format!("{ctx} step"));
                let byte = e.read_u32(arrived) & 0xff;
                seen_arr.insert(byte as u8);
                assert_eq!(
                    byte == 1,
                    our_byte,
                    "{ctx}: bin out_byte={byte} ours={our_byte} stepped={bin_step} goal={goal}"
                );
            }
        }
    };

    case(&mut e, 0.0, 0.0, 0.1, "at goal → dead-band None");
    case(&mut e, 0.0, 1.0, 0.1, "up, capped → 0.1");
    case(&mut e, 0.0, 0.05, 1.0, "up, reaches goal");
    case(&mut e, 1.0, 0.0, 0.1, "down, capped");
    case(&mut e, 1.0, 0.0, 5.0, "down, reaches goal");
    case(&mut e, 0.0, 0.3, 0.05, "arrived band edge");
    for i in 0..5000u32 {
        let cur = frand(&mut s, -7.0, 7.0);
        let goal = match i % 3 {
            0 => cur + frand(&mut s, -0.002, 0.002), // straddle the eps dead-band
            1 => cur + frand(&mut s, -1.0, 1.0),
            _ => frand(&mut s, -7.0, 7.0),
        };
        let rate = match i % 4 {
            0 => frand(&mut s, 0.0, 0.5),
            1 => frand(&mut s, 0.0, 5.0),
            2 => frand(&mut s, 0.0, 0.4), // near the π/8≈0.3927 arrived band
            _ => frand(&mut s, 0.0001, 0.05),
        };
        case(&mut e, cur, goal, rate, &format!("fuzz{i}"));
    }
    // The dead band `|goal - cur| == 0.001` and a step either side; `|stepped - goal| == pi/8`
    // (a rate of 3600421 * 2^-25 toward a goal of 0.5) and a step either side; then NaN,
    // infinities and zeros in each input.
    let eps = EPS_1EM3;
    for g in [eps, next_up(eps), next_down(eps), -eps] {
        case(&mut e, 0.0, g, 0.1, "dead band");
    }
    let rate = (3600421.0f64 / 33554432.0) as f32;
    for r in [rate, next_up(rate), next_down(rate)] {
        case(&mut e, 0.0, 0.5, r, "arrived edge");
        case(&mut e, 1.0, 0.5, r, "arrived edge down");
    }
    for &v in &specials(&[eps, PITCH_ARRIVED]) {
        case(&mut e, v, 1.0, 0.1, "special cur");
        case(&mut e, 0.5, v, 0.1, "special goal");
        case(&mut e, 0.5, 1.0, v, "special rate");
    }
    assert!(seen_none, "dead-band (|goal-cur|<eps → None) never seen");
    assert!(seen_step, "step path never seen");
    assert!(
        seen_arr.len() >= 2,
        "only one out_byte outcome: {seen_arr:?}"
    );
}

#[test]
fn move_dist_squares_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let tgt = e.alloc(0x20); // ESI
    let slf = e.alloc(0x20); // EAX
    let edi = e.alloc(0x200); // EDI ([+0x118] read)
    e.write_u32(edi + 0x118, edi as u32);
    let mut s = 0x610e_4000u32;

    let case = |e: &mut Emu, t: [f32; 3], p: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(tgt + 4 * k as u64, t[k].to_bits());
            e.write_u32(slf + 4 * k as u64, p[k].to_bits());
        }
        e.set_reg(RegisterX86::ESI, tgt);
        e.set_reg(RegisterX86::EAX, slf);
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x610f7a, 0x610fb6);
        let bin_dist3d = e.read_f32(ebp - 8);
        let dy2 = e.read_f32(ebp + 0xc);
        let dx2 = e.read_f32(ebp + 8);
        let bin_horiz = (f64::from(dy2) + f64::from(dx2)) as f32; // the binary's own f32 squares
        let (our_dist3d, our_horiz) = move_dist_squares(t, p);
        assert_f32(bin_dist3d, our_dist3d, &format!("{ctx} dist3d"));
        assert_f32(bin_horiz, our_horiz, &format!("{ctx} horiz"));
    };

    case(&mut e, [3.0, 4.0, 12.0], [0.0; 3], "3-4-12 → 169");
    case(&mut e, [0.0; 3], [0.0; 3], "zeros");
    case(&mut e, [1.0, 1.0, 1.0], [0.0; 3], "ones");
    for i in 0..4000u32 {
        let scale = match i % 3 {
            0 => 1.0,
            1 => 1000.0,
            _ => 0.001,
        };
        let t = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        let p = [
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
            frand(&mut s, -scale, scale),
        ];
        case(&mut e, t, p, &format!("fuzz{i}"));
    }
    // values whose dz f32-spill differs measurably from full precision (long mantissa diffs).
    for i in 0..1000u32 {
        let t = [0.0, 0.0, frand(&mut s, -1e7, 1e7)];
        let p = [0.0, 0.0, frand(&mut s, -1e7, 1e7)];
        case(&mut e, t, p, &format!("dz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut t = [1.0f32, 2.0, 3.0];
            t[k] = v;
            case(&mut e, t, [0.5, 0.25, 0.125], "special target");
            let mut q = [0.5f32, 0.25, 0.125];
            q[k] = v;
            case(&mut e, [1.0, 2.0, 3.0], q, "special self");
        }
    }
}

const SPEED_LEAF: u64 = 0x7c4c90;
const MOVE_TYPE_GLOBAL: u64 = 0xc4d888;
const RADIUS_CFG_GLOBAL: u64 = 0x80c4c8; // [ebp-4] init source

#[test]
fn move_goal_radius_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // Stub 0x7c4c90 with a code-island returning `speed` in st0: `fld dword[slot]; ret 4` (it is
    // called as `call 0x7c4c90` with a pushed `0` arg on the recompute arm → ret 4; on the first
    // call 0x610fb6 there is NO pushed arg → ret 0. The two call sites differ in args! 0x610fb6:
    // `call 0x7c4c90` (no push). 0x610fde: `push 0; call 0x7c4c90`. So the leaf is cdecl variadic-ish
    // — it pops nothing (caller cleans). Use `e.stub` semantics: a returning float needs a code
    // island, and stack cleanup is the caller's. Build `fld [slot]; ret`.
    let slot = e.alloc(0x10);
    let mut island = vec![0xd9u8, 0x05]; // fld dword[slot]
    island.extend_from_slice(&(slot as u32).to_le_bytes());
    island.push(0xc3); // ret
    let isl = e.alloc(0x10);
    e.write(isl, &island);
    // Redirect the call target: patch the call at 0x610fb6 and 0x610fde to hit the island? We cannot
    // edit .text portably; instead place the island AT 0x7c4c90 via the harness stub mechanism that
    // supports a float return. The harness `stub` returns an integer in EAX; for an st0 return we
    // need code at the real address. Map our island over 0x7c4c90's bytes.
    e.write(SPEED_LEAF, &island);

    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let this = e.alloc(0x2000); // EDI ([+0x118] read on recompute arm)
    e.write_u32(this + 0x118, this as u32);
    let mut s = 0x610e_5000u32;
    let mut seen = std::collections::BTreeSet::new();

    let mut case = |e: &mut Emu, speed: f32, cfg: f32, move_type: u32, ctx: &str| {
        e.write_u32(slot, speed.to_bits());
        e.write_u32(RADIUS_CFG_GLOBAL, cfg.to_bits()); // [ebp-4] is loaded from [0x80c4c8] at entry
        e.write_u32(ebp - 4, cfg.to_bits()); // and we set [ebp-4] directly (slice starts mid-fn)
        e.write_u32(MOVE_TYPE_GLOBAL, move_type);
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // [0x610fb6, 0x611003): speed leaf → /7 → clamp → *cfg → fcom cfg → fnstsw (move-type 5/10).
        e.run(0x610fb6, 0x611003);
        let m = ah(e) & 0x05; // radius vs cfg: C0 = radius<cfg
        seen.insert(m);
        let radius = move_goal_radius(speed, cfg);
        // reconstruct `fcom [ebp-4]=cfg; test ah,5` (ah&5 = C0|C2). For an UNORDERED compare the x87
        // sets C0=C2=C3=1, so C0 is set for NaN too (not just radius<cfg).
        let rd = f64::from(radius);
        let cd = f64::from(cfg);
        let unordered = rd.is_nan() || cd.is_nan();
        let our_c0 = unordered || rd < cd;
        let our_c2 = unordered;
        let our_bits = u32::from(our_c0) + u32::from(our_c2);
        let our_pf_even = our_bits % 2 == 0;
        let bin_bits = (m & 0x01).count_ones() + (m & 0x04).count_ones();
        let bin_pf_even = bin_bits.is_multiple_of(2);
        assert_eq!(
            bin_pf_even, our_pf_even,
            "{ctx}: bin ah&5={m:#04x} radius={radius} cfg={cfg} speed={speed}"
        );
    };

    // move_type 5 and 10 both reach the `fcom [ebp-4]` clamp.
    case(&mut e, 14.0, 3.0, 5, "speed/7=2 >1 → r=6, vs cfg3");
    case(&mut e, 3.5, 3.0, 5, "speed/7=0.5 <1 → r=cfg3, ==");
    case(&mut e, 7.0, 2.0, 10, "speed/7=1 → r=2, ==cfg");
    case(&mut e, 70.0, 0.5, 5, "big speed");
    for i in 0..4000u32 {
        let speed = match i % 4 {
            0 => frand(&mut s, 0.0, 7.0), // speed/7 in [0,1) → clamps to 1.0
            1 => frand(&mut s, 7.0, 70.0),
            2 => frand(&mut s, 6.9, 7.1), // straddle speed/7==1
            _ => frand_bits(&mut s).abs(),
        };
        let cfg = match i % 3 {
            0 => frand(&mut s, 0.0, 10.0),
            1 => frand(&mut s, 0.0, 1.0),
            _ => frand(&mut s, -5.0, 5.0),
        };
        let mt = if i % 2 == 0 { 5 } else { 10 };
        case(&mut e, speed, cfg, mt, &format!("fuzz{i}"));
    }
    assert!(seen.len() >= 2, "only one radius/cfg relation: {seen:?}");
}

const OBJ_LOOKUP: u64 = 0x468460; // returns target obj in EAX
const SQUARE_GLOBAL: u64 = 0xc4d998;
const GATE_TABLE: u64 = 0x860a58; // per-action gate byte at GATE_TABLE + action*0xc
const GATE_TABLE2: u64 = 0x860a61; // the second gate byte (must be 0 for the floor arm)

#[test]
fn cast_goal_reach_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let caster = e.alloc(0x200);
    let cfields = e.alloc(0x300);
    let target = e.alloc(0x200);
    let tfields = e.alloc(0x300);
    let guid = e.alloc(0x10); // targetGuid* (obj-lookup reads [eax],[eax+4])
    let out = e.alloc(0x10);
    e.write_u32(caster + 0x110, cfields as u32);
    e.write_u32(target + 0x110, tfields as u32);
    e.stub(OBJ_LOOKUP, target as u32, 3); // 0x468460(ecx=8,edx=str,3 args) → target ; ret 0xc
                                          // force the reach-floor arm for actions 6 and 10: gate byte nonzero, second gate byte 0.
    for action in [6u64, 10] {
        e.write(GATE_TABLE + action * 0xc, &[1u8]);
        e.write(GATE_TABLE2 + action * 0xc, &[0u8]);
    }
    let mut s = 0x6112_4000u32;

    // __fastcall(ecx=this, edx=action, [esp+4]=guid, [esp+8]=out) ; ret 8.
    let case = |e: &mut Emu, tr: f32, cr: f32, action: u32, ctx: &str| {
        e.write_u32(cfields + 0x1f0, cr.to_bits());
        e.write_u32(tfields + 0x1f0, tr.to_bits());
        e.write_u32(out, 0);
        let _ = e.call(0x611240, caster as u32, action, &[guid as u32, out as u32]);
        let bin = e.read_f32(out);
        assert_f32(
            bin,
            cast_goal_reach(tr, cr, action == 10),
            &format!("{ctx} a{action}"),
        );
    };

    case(&mut e, 1.0, 1.0, 6, "sum<5 → 5");
    case(&mut e, 3.0, 3.0, 6, "sum>5 → keep");
    case(&mut e, 3.0, 3.0, 10, "a10: keep - 4/3");
    case(&mut e, 1.0, 1.0, 10, "a10: 5 - 4/3");
    for i in 0..3000u32 {
        let tr = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 8.0)
        };
        let cr = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 8.0)
        };
        let action = if i % 2 == 0 { 6 } else { 10 };
        case(&mut e, tr, cr, action, &format!("fuzz{i}"));
    }
    // The same floor edge as `cast_reach_floor`, both actions, then the specials.
    let exact = (30758228.0f64 / 8388608.0) as f32;
    let one_ulp = f32::powi(2.0, -23);
    for action in [6u32, 10] {
        for tr in [exact, next_up(exact), next_down(exact)] {
            case(&mut e, tr, one_ulp, action, "reach sum 5");
        }
        for &v in &specials(&[3.0]) {
            case(&mut e, v, 1.0, action, "special target");
            case(&mut e, 1.0, v, action, "special caster");
        }
    }
}

#[test]
fn cast_goal_square_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let out = e.alloc(0x10);
    let mut s = 0x6113_3600u32;

    // Slice [0x611336, 0x61134b): fld [g]; mov edx,[ebp+0xc]; fmul [g]; pop edi; mov al,1; pop esi;
    // fstp [edx=out]. Run THROUGH the `fstp [edx]` at 0x611349 (halt 0x61134b). The pops read ESP
    // (harmless).
    let case = |e: &mut Emu, g: f32, ctx: &str| {
        e.write_u32(SQUARE_GLOBAL, g.to_bits());
        e.write_u32(ebp + 0xc, out as u32);
        e.write_u32(out, 0);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x611336, 0x61134b);
        let bin = e.read_f32(out);
        assert_f32(bin, cast_goal_square(g), ctx);
    };

    case(&mut e, 0.0, "zero");
    case(&mut e, 5.0, "25");
    case(&mut e, -3.0, "9");
    case(&mut e, 1e6, "big");
    for i in 0..3000u32 {
        let g = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, -1e4, 1e4)
        };
        case(&mut e, g, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn cast_goal_range_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let out = e.alloc(0x10);
    let mut s = 0x6113_7d00u32;

    // Slice [0x6113b9, 0x6113cf): fld [ebp+8]=range; fmul [0x804588]=0.9; (mov edx,[ebp+0xc]; pop
    // edi; mov al,1); fld st0; (pop esi); fmul st,st1; fstp [edx=out]. Halt 0x6113cf (after fstp).
    let case = |e: &mut Emu, range: f32, ctx: &str| {
        e.write_u32(ebp + 8, range.to_bits());
        e.write_u32(ebp + 0xc, out as u32);
        e.write_u32(out, 0);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x6113b9, 0x6113cf);
        let bin = e.read_f32(out);
        assert_f32(bin, cast_goal_range(range), ctx);
    };

    case(&mut e, 0.0, "zero");
    case(&mut e, 10.0, "(9)² = 81");
    case(&mut e, 30.0, "range 30");
    case(&mut e, -5.0, "neg");
    for i in 0..3000u32 {
        let r = if i % 3 == 0 {
            frand_bits(&mut s)
        } else {
            frand(&mut s, 0.0, 100.0)
        };
        case(&mut e, r, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        case(&mut e, v, "special");
    }
}

#[test]
fn click_ring_pulse_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x40);
    let ebp = frame + 0x20;
    let mut s = 0x6114_0000u32;

    let case = |e: &mut Emu, tmod: i32, ctx: &str| {
        e.write_u32(ebp - 4, tmod as u32);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x61143b, 0x61146f);
        let bin = e.read_f32(ebp - 4);
        assert_f32(bin, click_ring_pulse(tmod), ctx);
    };

    for t in [0, 1, 125, 250, 375, 499] {
        case(&mut e, t, &format!("t={t}"));
    }
    for i in 0..3000u32 {
        let tmod = ((next(&mut s) >> 8) % 500) as i32; // tmod = tick % 500 ∈ [0,499]
        case(&mut e, tmod, &format!("fuzz{i}"));
    }
    // Every value the clock modulo can take.
    for tmod in 0..500 {
        case(&mut e, tmod, "every tmod");
    }
}

const CENTER_GETTER: u64 = 0x7c4930; // returns the marker-center vec3* in EAX ; ret 8

#[test]
fn click_ring_corner_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80; // need [ebp-0x68 .. ebp-4] mapped
    let this = e.alloc(0x2000); // ESI (only address-computed)
    let center = e.alloc(0x20); // the getter result vec3
    e.stub(CENTER_GETTER, center as u32, 2); // → center block in EAX ; ret 8
    let mut s = 0x6114_2900u32;

    let case = |e: &mut Emu, c: [f32; 3], tmod: i32, ctx: &str| {
        for (k, cv) in c.iter().enumerate() {
            e.write_u32(center + 4 * k as u64, cv.to_bits());
        }
        e.set_reg(RegisterX86::EDX, tmod as u32 as u64); // [ebp-4]=tmod via `mov [ebp-4],edx`
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x611429, 0x611549);
        let pulse = e.read_f32(ebp - 4);
        // validate the pulse (re-derive from tmod) and the six corners.
        assert_f32(pulse, click_ring_pulse(tmod), &format!("{ctx} pulse"));
        let corner_a = [
            e.read_f32(ebp - 0x28),
            e.read_f32(ebp - 0x24),
            e.read_f32(ebp - 0x20),
        ];
        let corner_b = [
            e.read_f32(ebp - 0x1c),
            e.read_f32(ebp - 0x18),
            e.read_f32(ebp - 0x14),
        ];
        for i in 0..3 {
            let (om, op) = click_ring_corner(c[i], pulse);
            assert_f32(corner_a[i], om, &format!("{ctx} cornerA[{i}] (c-pulse)"));
            assert_f32(corner_b[i], op, &format!("{ctx} cornerB[{i}] (c+pulse)"));
        }
    };

    case(&mut e, [10.0, 20.0, 30.0], 250, "simple");
    case(&mut e, [0.0, 0.0, 0.0], 0, "zeros");
    case(&mut e, [-100.0, 5.0, 1e3], 125, "mixed");
    for i in 0..3000u32 {
        let c = if i % 3 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
            ]
        };
        let tmod = ((next(&mut s) >> 8) % 500) as i32;
        case(&mut e, c, tmod, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut c = [1.0f32, 2.0, 3.0];
            c[k] = v;
            case(&mut e, c, 123, "special corner");
        }
    }
}

const RENDER_SCALE_LEAF: u64 = 0x7bdd00; // captures arg0 = (s_a*s_b)*s_z
const RENDER_MAT_A: u64 = 0x7bdc40; // matrix build (non-FP for our arg) — runs real / harmless
const RENDER_MAT_B: u64 = 0x7bdd60;
const SCENE_PUSH: u64 = 0x6717d0; // final push to scene node — stub

#[test]
fn world_model_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x2000);
    let vt = e.alloc(0x100);
    e.write_u32(this, vt as u32);
    e.write_u32(this + 0xe0, 0x1000); // scene handle != 0 (not deref'd before our capture)
    e.write_u32(this + 0xd8, 0); // no model-data → skip the bounds-midpoint allocation path
                                 // vtable getters [+0x14]=GetPosition, [+0x18]=GetFacing: stub to return cleanly (write nothing).
    e.stub(this + 0x100, 0, 0); // placeholder (unused slot)
                                // The body calls [eax+0x14] then 0x7bdc40 then [edx+0x18] then 0x7bdd60 then 0x7bdd00. Stub the
                                // vtable slots and the render leaves so only our captured push matters.
    let vget = e.alloc(0x10);
    let mut isl = vec![0xc2u8, 0x04, 0x00]; // ret 4 (one stack arg, the out ptr)
    e.write(vget, &isl);
    e.write_u32(vt + 0x14, vget as u32);
    isl = vec![0xc2u8, 0x04, 0x00];
    let vget2 = e.alloc(0x10);
    e.write(vget2, &isl);
    e.write_u32(vt + 0x18, vget2 as u32);
    e.stub(RENDER_MAT_A, 0, 2);
    e.stub(RENDER_MAT_B, 0, 1);
    e.stub(RENDER_SCALE_LEAF, 0, 1); // captures arg0
    e.stub(SCENE_PUSH, 0, 3);
    let mut s = 0x613e_f000u32;

    let check = |e: &mut Emu, s_a: f32, s_b: f32, s_z: f32, dc: u32, ctx: &str| {
        e.write_u32(this + 0x94, s_a.to_bits());
        e.write_u32(this + 0x90, s_b.to_bits());
        e.write_u32(this + 0x9c, s_z.to_bits()); // s_z source when dc!=0
        e.write_u32(this + 0x98, s_z.to_bits()); // s_z source when dc==0 (same value → one input)
        e.write_u32(this + 0xdc, dc);
        e.clear_calls();
        let _ = e.call(0x613ef0, this as u32, 0, &[0u32]);
        let call = e
            .calls()
            .iter()
            .find(|c| c.addr == RENDER_SCALE_LEAF)
            .copied()
            .expect("scale leaf called");
        let bin = f32::from_bits(call.args[0]);
        assert_f32(bin, world_model_scale(s_a, s_b, s_z), ctx);
    };

    check(&mut e, 1.0, 1.0, 1.0, 1, "ones dc!=0");
    check(&mut e, 2.0, 3.0, 4.0, 0, "2*3*4 dc==0");
    check(&mut e, 0.5, 0.25, 8.0, 1, "fracs");
    for i in 0..2000u32 {
        check(
            &mut e,
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            if (next(&mut s) >> 8).is_multiple_of(2) {
                0
            } else {
                1
            },
            &format!("fuzz{i}"),
        );
    }
    for &v in &specials(&[]) {
        check(&mut e, v, 2.0, 3.0, 0, "special s_a");
        check(&mut e, 2.0, v, 3.0, 1, "special s_b");
        check(&mut e, 2.0, 3.0, v, 0, "special s_z");
        check(&mut e, 2.0, 3.0, v, 1, "special s_z alt");
    }
}

#[test]
fn world_bounds_sum_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let bounds = e.alloc(0x40); // EAX
    let mut s = 0x6140_3100u32;
    // a fixed ESP into the mapped stack region; the slice `sub esp,0xc` then writes [esp..+8].
    let esp0 = SLICE_ESP;

    let case = |e: &mut Emu, lo: [f32; 3], hi: [f32; 3], ctx: &str| {
        e.write_u32(bounds, lo[0].to_bits());
        e.write_u32(bounds + 4, lo[1].to_bits());
        e.write_u32(bounds + 8, lo[2].to_bits());
        e.write_u32(bounds + 0xc, hi[0].to_bits());
        e.write_u32(bounds + 0x10, hi[1].to_bits());
        e.write_u32(bounds + 0x14, hi[2].to_bits());
        e.set_reg(RegisterX86::EAX, bounds);
        e.set_reg(RegisterX86::ESP, esp0);
        e.run(0x614031, 0x614053);
        // after `sub esp,0xc` the args are at [esp..], [esp+4], [esp+8] (x,y,z).
        let sp = e.get_reg(RegisterX86::ESP);
        let bin = [e.read_f32(sp), e.read_f32(sp + 4), e.read_f32(sp + 8)];
        let ours = world_bounds_sum(lo, hi);
        for i in 0..3 {
            assert_f32(bin[i], ours[i], &format!("{ctx} [{i}]"));
        }
    };

    case(&mut e, [1.0, 2.0, 3.0], [4.0, 5.0, 6.0], "simple");
    case(&mut e, [0.0; 3], [0.0; 3], "zeros");
    case(&mut e, [-1.0, -2.0, -3.0], [1.0, 2.0, 3.0], "symmetric");
    for i in 0..3000u32 {
        let lo = if i % 3 == 0 {
            [frand_bits(&mut s), frand_bits(&mut s), frand_bits(&mut s)]
        } else {
            [
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
                frand(&mut s, -1e4, 1e4),
            ]
        };
        let hi = [
            frand(&mut s, -1e4, 1e4),
            frand(&mut s, -1e4, 1e4),
            frand(&mut s, -1e4, 1e4),
        ];
        case(&mut e, lo, hi, &format!("fuzz{i}"));
    }
    for &v in &specials(&[]) {
        for k in 0..3 {
            let mut lo = [1.0f32, 2.0, 3.0];
            lo[k] = v;
            case(&mut e, lo, [0.5, 0.25, 0.125], "special lo");
            let mut hi = [0.5f32, 0.25, 0.125];
            hi[k] = v;
            case(&mut e, [1.0, 2.0, 3.0], hi, "special hi");
        }
    }
}
