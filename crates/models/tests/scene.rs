//! A scene's collection and ray queries (`systems/models/scene.md`): stretches of `0x707680`,
//! `0x7089c0` and `0x7099e0` run as slices, or as islands of the exe's own bytes, checked, with a
//! store appended.

#![allow(clippy::type_complexity)] // the cases are tuples of the fields they write
#![allow(clippy::needless_range_loop)] // index loops follow the `+off + 4·k` layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::scene::{
    aabb_corner_sum, alpha_scale, half_scale, hit_sphere, mat_x_dir, negate, plane_dist, radius,
    ray_setup, scale_mul, scale_t_dot, segment_sphere, skin_accum, skin_first, sphere_ahead,
    sphere_meets, sphere_near, sphere_within, sum_of_squares, tbest_init, tri_invray,
    tri_predicate, tri_t_world, tworld_recover, vec_add, view_rebase, world_distance,
};
use common::{assert_f32, frand, frand3, next, read_f64, SLICE_ESP};

/// The LCG with the draws the collection tests make.
struct DrawRng(u32);

impl DrawRng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A finite `f32` over six ranges.
    fn f(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 6;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,
            1 => (m * 200.0 - 100.0) as f32,
            2 => (m * 2.0) as f32,
            3 => (m * 0.01) as f32,
            4 => (m * 20.0 - 10.0) as f32,
            _ => (m * 4.0 - 2.0) as f32,
        }
    }
    /// A non-negative `f32` over four ranges.
    fn f_pos(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 4;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 1.0) as f32,
            1 => (m * 100.0) as f32,
            2 => (m * 10000.0) as f32,
            _ => (m * 0.0001) as f32,
        }
    }
}

/// 9.9999997e-6 (`0x80c5c8`).
const EPS_BITS: u32 = 0x3727_c5ac;

/// `0x707680` from `0x7076cb` to `0x707752`: the view at `ebx+0x9c`, the origin through `[ebp+8]`;
/// the translation read from `+0xcc`. Products whose sum depends on the order of the adds probe
/// each row.
#[test]
fn view_rebase_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // EBX = this; basis [ebx+0x9c..], translation [ebx+0xcc..]
    let mat = e.alloc(0x40); // arg matrix; [ebp+8] → eax; in0/in1/in2 = [eax+0/4/8]

    let run_case = |e: &mut Emu, basis: &[(u64, f32)], tr: [f32; 3], inv: [f32; 3], ctx: &str| {
        for &(off, v) in basis {
            e.write_u32(this + off, v.to_bits());
        }
        e.write_u32(this + 0xcc, tr[0].to_bits());
        e.write_u32(this + 0xd0, tr[1].to_bits());
        e.write_u32(this + 0xd4, tr[2].to_bits());
        e.write_u32(mat, inv[0].to_bits());
        e.write_u32(mat + 4, inv[1].to_bits());
        e.write_u32(mat + 8, inv[2].to_bits());
        e.write_u32(ebp + 0x8, mat as u32); // arg matrix pointer
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::EDI, this + 0x9c); // EDI=[ebx+0x9c] (the call 0x58b0b0 sets this; we skip it)
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7076cb, 0x707752);

        // The view's twelve entries from `+0x9c`, the last of each row unused.
        let mut m = [0f32; 12];
        for &(off, v) in basis {
            m[((off - 0x9c) / 4) as usize] = v;
        }
        let [r0, r1, r2] = view_rebase(&m, tr, inv);
        assert_f32(e.read_f32(this + 0xcc), r0, &format!("{ctx} row0"));
        assert_f32(e.read_f32(this + 0xd0), r1, &format!("{ctx} row1"));
        assert_f32(e.read_f32(this + 0xd4), r2, &format!("{ctx} row2"));
    };

    let basis_offs = [0x9c, 0xa0, 0xa4, 0xac, 0xb0, 0xb4, 0xbc, 0xc0, 0xc4];

    // identity-ish + zeros
    let z: Vec<(u64, f32)> = basis_offs.iter().map(|&o| (o, 0.0f32)).collect();
    run_case(&mut e, &z, [0.0; 3], [0.0; 3], "all-zero");
    let ones: Vec<(u64, f32)> = basis_offs.iter().map(|&o| (o, 1.0f32)).collect();
    run_case(&mut e, &ones, [10.0, 20.0, 30.0], [2.0, 3.0, 4.0], "ones");
    // Products 1, −1 and 2^-60 whose sum depends on the order of the adds, in each row.
    let tiny = f32::from_bits(0x2180_0000);
    for (row, ctx) in [(0u64, "row 0"), (4, "row 1"), (8, "row 2")] {
        let mut basis: Vec<(u64, f32)> = basis_offs.iter().map(|&o| (o, 0.0f32)).collect();
        for (off, v) in [(0x9c + row, -1.0f32), (0xac + row, 1.0), (0xbc + row, tiny)] {
            basis.iter_mut().find(|(o, _)| *o == off).unwrap().1 = v;
        }
        run_case(&mut e, &basis, [0.0; 3], [1.0; 3], ctx);
    }

    let mut rng = DrawRng(0x7076_cb00);
    for i in 0..4000u32 {
        let basis: Vec<(u64, f32)> = basis_offs.iter().map(|&o| (o, rng.f())).collect();
        let tr = [rng.f(), rng.f(), rng.f()];
        let inv = [rng.f(), rng.f(), rng.f()];
        run_case(&mut e, &basis, tr, inv, &format!("view fuzz[{i}]"));
    }
}

/// `0x707680` from `0x7078e5` to `0x707915`.
#[test]
fn sum_of_squares_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // ESI; [esi+0xfc/0x100/0x104]

    let run_case = |e: &mut Emu, x: f32, y: f32, z: f32, ctx: &str| {
        e.write_u32(this + 0xfc, x.to_bits());
        e.write_u32(this + 0x100, y.to_bits());
        e.write_u32(this + 0x104, z.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7078e5, 0x707915);
        assert_f32(e.read_f32(ebp - 0x34), sum_of_squares(x, y, z), ctx);
    };

    run_case(&mut e, 0.0, 0.0, 0.0, "sumsq zero");
    run_case(&mut e, 3.0, 4.0, 0.0, "sumsq 3-4 (→25)");
    run_case(&mut e, 1.0, 1.0, 1.0, "sumsq ones (→3)");

    let mut rng = DrawRng(0x7078_e500);
    for i in 0..4000u32 {
        run_case(
            &mut e,
            rng.f(),
            rng.f(),
            rng.f(),
            &format!("sumsq fuzz[{i}]"),
        );
    }
}

/// `0x707680` from `0x707915` to `0x707944`: the three sums pushed for the transform.
#[test]
fn vec_add_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x400); // EBX

    // store dests are [esp+8/+4/+0] with the slice's ESP; the slice has no `sub esp`, so [esp..]
    // overlaps the stack at SLICE_ESP. Read them back at SLICE_ESP+8/+4/+0.
    let run_case = |e: &mut Emu, a: [f32; 3], b: [f32; 3], ctx: &str| {
        e.write_u32(this + 0xc8, a[0].to_bits());
        e.write_u32(this + 0xc4, a[1].to_bits());
        e.write_u32(this + 0xc0, a[2].to_bits());
        e.write_u32(this + 0xbc, b[0].to_bits());
        e.write_u32(this + 0xb8, b[1].to_bits());
        e.write_u32(this + 0xb4, b[2].to_bits());
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x707915, 0x707944);
        // first add → [esp+8] = a[0]+b[0]; second → [esp+4] = a[1]+b[1]; third → [esp] = a[2]+b[2].
        assert_f32(
            e.read_f32(SLICE_ESP + 8),
            vec_add(a[0], b[0]),
            &format!("{ctx} s0"),
        );
        assert_f32(
            e.read_f32(SLICE_ESP + 4),
            vec_add(a[1], b[1]),
            &format!("{ctx} s1"),
        );
        assert_f32(
            e.read_f32(SLICE_ESP),
            vec_add(a[2], b[2]),
            &format!("{ctx} s2"),
        );
    };

    run_case(&mut e, [0.0; 3], [0.0; 3], "vecadd zero");
    run_case(
        &mut e,
        [1.0, 2.0, 3.0],
        [-1.0, -2.0, -3.0],
        "vecadd cancel (→0)",
    );

    let mut rng = DrawRng(0x7079_1500);
    for i in 0..4000u32 {
        let a = [rng.f(), rng.f(), rng.f()];
        let b = [rng.f(), rng.f(), rng.f()];
        run_case(&mut e, a, b, &format!("vecadd fuzz[{i}]"));
    }
}

/// `0x707680` from `0x707959` to `0x707971`.
#[test]
fn radius_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // EBX; [ebx+0xcc]

    let run_case = |e: &mut Emu, sumsq: f32, scale: f32, ctx: &str| {
        e.write_u32(ebp - 0x34, sumsq.to_bits());
        e.write_u32(this + 0xcc, scale.to_bits());
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x707959, 0x707971);
        assert_f32(e.read_f32(ebp - 0x8), radius(sumsq, scale), ctx);
    };

    // ==threshold probes: sumsq==0 (r=0); sumsq==1 (sqrt=1); a perfect square.
    run_case(&mut e, 0.0, 1.0, "radius sumsq==0");
    run_case(&mut e, 1.0, 1.0, "radius sumsq==1");
    run_case(&mut e, 25.0, 2.0, "radius sqrt(25)*2 (→10)");
    run_case(&mut e, 2.0, 0.0, "radius scale==0 (→0)");

    let mut rng = DrawRng(0x7079_5900);
    for i in 0..4000u32 {
        run_case(&mut e, rng.f_pos(), rng.f(), &format!("radius fuzz[{i}]"));
    }
}

/// `0x707680` from `0x707981` to `0x7079bc` in an island with `fstp qword` appended.
#[test]
fn plane_dist_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const ISL_VA: u64 = 0x707981;
    const ISL_LEN: usize = 0x7079bc - 0x707981;
    let bytes = e.read(ISL_VA, ISL_LEN);

    let out = e.alloc(0x10);
    let isl = e.alloc(0x40);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let plane = e.alloc(0x400); // [ebp-0x58] → plane base; p = [+0x1a0/0x1a4/0x1a8/0x1ac]
    let dummy = e.alloc(0x40); // [ebp-0x4] → a readable ptr (its [+4] is loaded but unused for the dot)
    let mut code = bytes;
    code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out]  (DD /3, disp32 absolute)
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + code.len() as u64;
    e.write(isl, &code);

    #[allow(clippy::too_many_arguments)]
    let run_case = |e: &mut Emu, tv: [f32; 3], p: [f32; 4], ctx: &str| {
        e.write_u32(ebp - 0x68, tv[2].to_bits()); // tv_z (read from memory)
        e.write_u32(ebp - 0x58, plane as u32); // plane base
        e.write_u32(ebp - 0x4, dummy as u32); // [ebp-0x4] → readable
        e.write_u32(plane + 0x1a0, p[0].to_bits());
        e.write_u32(plane + 0x1a4, p[1].to_bits());
        e.write_u32(plane + 0x1a8, p[2].to_bits());
        e.write_u32(plane + 0x1ac, p[3].to_bits());
        // The island writes [ebp-0x6c]←EDX (tv_y) and [ebp-0x70]←ECX (tv_x) itself, then reloads.
        e.set_reg(RegisterX86::ECX, u64::from(tv[0].to_bits())); // tv_x
        e.set_reg(RegisterX86::EDX, u64::from(tv[1].to_bits())); // tv_y
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let bin = read_f64(e, out);
        let ours = plane_dist(tv[0], tv[1], tv[2], p[0], p[1], p[2], p[3]);
        assert_eq!(
            bin.to_bits(),
            ours.to_bits(),
            "{ctx}: binary={:#018x} ours={:#018x}",
            bin.to_bits(),
            ours.to_bits()
        );
    };

    run_case(&mut e, [0.0; 3], [0.0; 4], "plane zero");
    run_case(
        &mut e,
        [1.0, 2.0, 3.0],
        [1.0, 0.0, 0.0, -5.0],
        "plane x-only (→1-5=-4)",
    );
    run_case(
        &mut e,
        [1.0, 1.0, 1.0],
        [1.0, 1.0, 1.0, 0.0],
        "plane sum (→3)",
    );

    let mut rng = DrawRng(0x7079_8100);
    for i in 0..3000u32 {
        let tv = [rng.f(), rng.f(), rng.f()];
        let p = [rng.f(), rng.f(), rng.f(), rng.f()];
        run_case(&mut e, tv, p, &format!("plane fuzz[{i}]"));
    }
}

/// `0x707680` from `0x7079c3` to `0x7079c8` in an island with `fstp qword` appended.
#[test]
fn negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const ISL_VA: u64 = 0x7079c3;
    const ISL_LEN: usize = 0x7079c8 - 0x7079c3; // 5: fld [ebp-0x8]; fchs
    let bytes = e.read(ISL_VA, ISL_LEN);

    let out = e.alloc(0x10);
    let isl = e.alloc(0x40);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let mut code = bytes;
    code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + code.len() as u64;
    e.write(isl, &code);

    let run_case = |e: &mut Emu, r: f32, ctx: &str| {
        e.write_u32(ebp - 0x8, r.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let bin = read_f64(e, out);
        let ours = negate(r);
        assert_eq!(
            bin.to_bits(),
            ours.to_bits(),
            "{ctx}: binary={:#018x} ours={:#018x}",
            bin.to_bits(),
            ours.to_bits()
        );
    };

    run_case(&mut e, 0.0, "negate 0 (→-0.0)");
    run_case(&mut e, 5.0, "negate 5 (→-5)");
    run_case(&mut e, -3.5, "negate -3.5 (→3.5)");

    let mut rng = DrawRng(0x7079_c300);
    for i in 0..3000u32 {
        run_case(&mut e, rng.f(), &format!("negate fuzz[{i}]"));
    }
}

/// `0x707680` from `0x7080e5` to `0x7080ee`.
#[test]
fn scale_mul_f_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let buf = e.alloc(0x80); // EAX; factor at [eax+0x3c]

    let run_case = |e: &mut Emu, acc: f32, factor: f32, ctx: &str| {
        e.write_u32(ebp - 0x10, acc.to_bits());
        e.write_u32(buf + 0x3c, factor.to_bits());
        e.set_reg(RegisterX86::EAX, buf);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7080e5, 0x7080ee);
        assert_f32(e.read_f32(ebp - 0x10), scale_mul(acc, factor), ctx);
    };

    run_case(&mut e, 0.0, 5.0, "scaleF acc==0");
    run_case(&mut e, 1.0, 1.0, "scaleF ones");
    run_case(&mut e, 2.5, 0.0, "scaleF factor==0");

    let mut rng = DrawRng(0x7080_e500);
    for i in 0..4000u32 {
        run_case(&mut e, rng.f(), rng.f(), &format!("scaleF fuzz[{i}]"));
    }
}

/// `0x707680` from `0x707afc` to `0x707b12`: both alpha products.
#[test]
fn scale_mul_d1_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x400); // ESI; [esi+0xa0] → factor table base
    let tbl = e.alloc(0x80); // factor table; factor at [tbl+0x3c]

    let run_case = |e: &mut Emu, acc: f32, factor: f32, ctx: &str| {
        e.write_u32(ebp - 0x2c, acc.to_bits());
        e.write_u32(this + 0xa0, tbl as u32);
        e.write_u32(tbl + 0x3c, factor.to_bits());
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EAX, 0); // lea eax,[eax+4*eax]; shl eax,4 → 0
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x707afc, 0x707b12);
        assert_f32(e.read_f32(ebp - 0x2c), scale_mul(acc, factor), ctx);
    };

    run_case(&mut e, 0.0, 5.0, "scaleD1 acc==0");
    run_case(&mut e, 1.0, 1.0, "scaleD1 ones");
    run_case(&mut e, 3.0, 0.5, "scaleD1 3*0.5");

    let mut rng = DrawRng(0x707a_fc00);
    for i in 0..4000u32 {
        run_case(&mut e, rng.f(), rng.f(), &format!("scaleD1 fuzz[{i}]"));
    }
}

/// `0x707680` from `0x707d85` to `0x707da6`.
#[test]
fn sum_of_squares_e_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x400); // ESI; out [esi+0x18]
    let vec = e.alloc(0x40); // EAX; x/y/z = [eax+0/4/8]

    let run_case = |e: &mut Emu, x: f32, y: f32, z: f32, ctx: &str| {
        e.write_u32(vec, x.to_bits());
        e.write_u32(vec + 4, y.to_bits());
        e.write_u32(vec + 8, z.to_bits());
        e.set_reg(RegisterX86::EAX, vec);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x707d85, 0x707da6);
        assert_f32(e.read_f32(this + 0x18), sum_of_squares(x, y, z), ctx);
    };

    run_case(&mut e, 0.0, 0.0, 0.0, "sumsqE zero");
    run_case(&mut e, 3.0, 4.0, 0.0, "sumsqE 3-4 (→25)");

    let mut rng = DrawRng(0x707d_8500);
    for i in 0..4000u32 {
        run_case(
            &mut e,
            rng.f(),
            rng.f(),
            rng.f(),
            &format!("sumsqE fuzz[{i}]"),
        );
    }
}

/// `0x7099e0` from `0x709a38` to `0x709ad7`: the length, its inverse and the direction read from
/// the frame.
#[test]
fn segment_ray_setup_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let p0 = e.alloc(0x20);
    let p1 = e.alloc(0x20);
    let this = e.alloc(0x200); // EBX; `mov eax,[ebx+0x130]`@0x709a75 runs on every path (list head, null)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, a0: [f32; 3], a1: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(p0 + 4 * k as u64, a0[k].to_bits());
            e.write_u32(p1 + 4 * k as u64, a1[k].to_bits());
        }
        e.write_u32(this + 0x130, 0); // null list head (the bail loop is skipped on the normalize path)
        e.write_u32(ebp + 8, p0 as u32); // [ebp+8]  = p0 ptr (ecx)
        e.write_u32(ebp + 0xc, p1 as u32); // [ebp+0xc] = p1 ptr (eax)
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x709a38, 0x709ad7);
        let bin_len = e.read_f32(ebp - 0x18);
        let bin_inv = e.read_f32(ebp - 0x30);
        let bin_ndir = [
            e.read_f32(ebp - 0x28),
            e.read_f32(ebp - 0x24),
            e.read_f32(ebp - 0x20),
        ];
        let (len, inv, ndir) = ray_setup(a0, a1);
        assert_f32(bin_len, len, &format!("{ctx} len"));
        assert_f32(bin_inv, inv, &format!("{ctx} inv"));
        for k in 0..3 {
            assert_f32(bin_ndir[k], ndir[k], &format!("{ctx} ndir[{k}]"));
        }
    };

    // boundary / exact probes: unit-x (len=1), 3-4-0 (len=5 exact), len exactly eps (the ≥
    // boundary).
    run_case(&mut e, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], "setup unit-x");
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        [3.0, 4.0, 0.0],
        "setup 3-4-0 (len=5)",
    );
    run_case(&mut e, [-1.0, -2.0, -3.0], [4.0, 5.0, 6.0], "setup mid");
    // **`len == eps` boundary (deterministic)**: dir = (eps, 0, 0) gives len exactly eps. The eps
    // gate's `jp 0x709aad` (the `len ≥ eps` side) normalizes at exactly-eps — verified at the
    // bytes: `len < eps` bails, `len == eps` takes the normalize path the slice covers. (Probing
    // the gate with `(10+eps)−10` does NOT work: `10+eps` rounds to `10` in f32, giving len=0 → the
    // bail path.)
    let eps = f32::from_bits(EPS_BITS);
    run_case(&mut e, [0.0, 0.0, 0.0], [eps, 0.0, 0.0], "setup len==eps");

    let mut s = 0x709a_3838u32;
    for i in 0..4000u32 {
        let a0 = frand3(&mut s, -1000.0, 1000.0);
        let a1 = frand3(&mut s, -1000.0, 1000.0);
        run_case(&mut e, a0, a1, &format!("setup fuzz[{i}]"));
    }
}

/// `0x7099e0` from `0x709c22` to `0x709c4f` and its copy from `0x709c95`.
#[test]
fn half_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vec = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, begin: u64, until: u64, v: [f32; 3], ctx: &str| {
        for (k, c) in v.iter().enumerate() {
            e.write_u32(vec + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::EAX, vec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(begin, until);
        // post-run ESP = SLICE_ESP − 0xc (the in-slice `sub esp,0xc`); results at +0/+4/+8 (x,y,z).
        let esp = e.get_reg(RegisterX86::ESP);
        let bin = [e.read_f32(esp), e.read_f32(esp + 4), e.read_f32(esp + 8)];
        let ours = half_scale(v);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} half[{k}]"));
        }
    };

    // probes: zero, exact halves, odd values (0.5· keeps exactness for f32-representable doubles).
    for &(begin, until, tag) in &[(0x709c22u64, 0x709c50u64, "A"), (0x709c95, 0x709cc0, "B")] {
        run_case(
            &mut e,
            begin,
            until,
            [0.0, 0.0, 0.0],
            &format!("half{tag} zero"),
        );
        run_case(
            &mut e,
            begin,
            until,
            [2.0, 4.0, 6.0],
            &format!("half{tag} even"),
        );
        run_case(
            &mut e,
            begin,
            until,
            [1.0, 3.0, -5.0],
            &format!("half{tag} odd"),
        );
        let mut s = 0x709c_2222u32 ^ (begin as u32);
        for i in 0..2000u32 {
            let v = frand3(&mut s, -2000.0, 2000.0);
            run_case(&mut e, begin, until, v, &format!("half{tag} fuzz[{i}]"));
        }
    }
}

/// `0x7099e0` from `0x709cf2` to `0x709de8` on spheres the segment meets: the far and near
/// distances read from `[ebp-0x2c]` and `[ebp-0x4]`.
#[test]
fn segment_sphere_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let center = e.alloc(0x20);
    let origin = e.alloc(0x20);
    let extents = e.alloc(0x20);
    let this = e.alloc(0x500); // EBX; [ebx+0x134] read at 0x709ddf
    let hitrec = e.alloc(0x40);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    e.write_u32(this + 0x134, hitrec as u32);

    let run_case = |e: &mut Emu,
                    c: [f32; 3],
                    o: [f32; 3],
                    ext: [f32; 3],
                    scale: f32,
                    ndir: [f32; 3],
                    ctx: &str| {
        for k in 0..3 {
            e.write_u32(center + 4 * k as u64, c[k].to_bits());
            e.write_u32(origin + 4 * k as u64, o[k].to_bits());
            e.write_u32(extents + 4 * k as u64, ext[k].to_bits());
        }
        e.write_u32(ebp - 0x34, scale.to_bits());
        e.write_u32(ebp - 0x28, ndir[0].to_bits());
        e.write_u32(ebp - 0x24, ndir[1].to_bits());
        e.write_u32(ebp - 0x20, ndir[2].to_bits());
        // The length at `[ebp-0x18]` +∞, so every hit reaches the store of `t_near`.
        e.write_u32(ebp - 0x18, f32::INFINITY.to_bits());
        e.write_u32(ebp + 8, origin as u32); // [ebp+8] = origin ptr (ecx)
        e.set_reg(RegisterX86::EAX, center);
        e.set_reg(RegisterX86::EDI, extents);
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x709cf2, 0x709de8);
        let bin_t_far = e.read_f32(ebp - 0x2c);
        let bin_t_near = e.read_f32(ebp - 0x4);
        let h = segment_sphere(c, o, ext, scale, ndir);
        assert!(h.hit, "{ctx}: expected a HIT-path case");
        assert_f32(bin_t_far, h.t_far, &format!("{ctx} t_far"));
        assert_f32(bin_t_near, h.t_near, &format!("{ctx} t_near"));
    };

    // Construct hit cases: origin at the world origin, ndir a unit axis, center along the ray so
    // the sphere is intersected. extents·scale gives radius. proj = oc·ndir. direct on-axis hit:
    // center=(10,0,0), origin=0, ndir=+x, radius from extents.
    run_case(
        &mut e,
        [10.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
        1.0,
        [1.0, 0.0, 0.0],
        "sphere on-axis hit",
    );
    // tangent: perp² exactly == radius². center=(10, r, 0), ndir=+x → perp.y = r exactly, radius=r.
    // pick r=4 (extents=(4,0,0), scale=1) and center y = 4 → perp²=16==radius²=16.
    run_case(
        &mut e,
        [10.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        1.0,
        [1.0, 0.0, 0.0],
        "sphere tangent perp²==r²",
    );
    // t_far == 0 boundary: proj + √disc == 0 → with proj negative and √disc = −proj. center behind
    // the origin so proj<0; pick center=(-3,0,0), ndir=+x → proj=-3, radius=3 → disc=9, √disc=3,
    // t_far=0.
    run_case(
        &mut e,
        [-3.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
        1.0,
        [1.0, 0.0, 0.0],
        "sphere t_far==0",
    );
    // scale ≠ 1 (radius = extents·scale): extents=(2,0,0), scale=1.5 → radius=3.
    run_case(
        &mut e,
        [8.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        1.5,
        [1.0, 0.0, 0.0],
        "sphere scaled radius",
    );

    // Fuzz: random ray + sphere, keep on the HIT path by construction (center on the ray within a
    // generous radius). origin random, ndir a random unit, center = origin + proj·ndir + perp·t
    // with
    // |perp| < radius·0.9 so perp² < radius².
    let mut s = 0x709c_f2f2u32;
    let mut done = 0u32;
    let mut tries = 0u32;
    while done < 4000 && tries < 200_000 {
        tries += 1;
        let o = frand3(&mut s, -500.0, 500.0);
        // random unit ndir
        let mut nd = frand3(&mut s, -1.0, 1.0);
        let nl =
            (f64::from(nd[0]).powi(2) + f64::from(nd[1]).powi(2) + f64::from(nd[2]).powi(2)).sqrt();
        if nl < 1e-3 {
            continue;
        }
        nd = [
            (f64::from(nd[0]) / nl) as f32,
            (f64::from(nd[1]) / nl) as f32,
            (f64::from(nd[2]) / nl) as f32,
        ];
        let radius = frand(&mut s, 1.0, 200.0);
        let proj = frand(&mut s, 0.5, 400.0); // proj>0 so t_far>0 comfortably
                                              // a perpendicular offset (cross ndir with an axis)
                                              // scaled below radius
        let ax = if nd[0].abs() < 0.9 {
            [1.0f32, 0.0, 0.0]
        } else {
            [0.0f32, 1.0, 0.0]
        };
        let perp_dir = [
            nd[1] * ax[2] - nd[2] * ax[1],
            nd[2] * ax[0] - nd[0] * ax[2],
            nd[0] * ax[1] - nd[1] * ax[0],
        ];
        let pl = (f64::from(perp_dir[0]).powi(2)
            + f64::from(perp_dir[1]).powi(2)
            + f64::from(perp_dir[2]).powi(2))
        .sqrt();
        if pl < 1e-3 {
            continue;
        }
        let poff = frand(&mut s, 0.0, radius * 0.85);
        let scl = poff / pl as f32;
        let center = [
            o[0] + proj * nd[0] + perp_dir[0] * scl,
            o[1] + proj * nd[1] + perp_dir[1] * scl,
            o[2] + proj * nd[2] + perp_dir[2] * scl,
        ];
        let ext = [radius, 0.0, 0.0];
        let h = segment_sphere(center, o, ext, 1.0, nd);
        if !h.hit || h.t_far < 0.0 {
            continue;
        }
        run_case(
            &mut e,
            center,
            o,
            ext,
            1.0,
            nd,
            &format!("sphere fuzz[{done}]"),
        );
        done += 1;
    }
    assert!(
        done >= 3000,
        "sphere fuzz produced too few HIT cases: {done}"
    );
}

/// `0x7099e0` from `0x709e7e` to `0x709e86`.
#[test]
fn segment_tbest_init_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let edi = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, len: f32, param: f32, ctx: &str| {
        e.write_u32(ebp - 0x18, len.to_bits());
        e.write_u32(edi, param.to_bits());
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x709e7e, 0x709e86);
        let bin = e.read_f32(ebp - 0x4);
        let ours = tbest_init(len, param);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 5.0, 0.0, "tbest len=5 p=0");
    run_case(&mut e, 5.0, 1.0, "tbest len=5 p=1");
    run_case(&mut e, 100.0, 0.5, "tbest len=100 p=0.5");
    let mut s = 0x709e_7e7eu32;
    for i in 0..3000u32 {
        let len = frand(&mut s, 0.0, 5000.0);
        let p = frand(&mut s, -2.0, 2.0);
        run_case(&mut e, len, p, &format!("tbest fuzz[{i}]"));
    }
}

/// `0x7099e0` from `0x709f1a` to `0x709fa8`.
#[test]
fn tri_invray_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let geom = e.alloc(0x200); // EDI; `mov eax,[edi+0xf0]`/`mov ecx,[edi+0xec]` (geometry ptrs, not FP)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, tp0: [f32; 3], tp1: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x28, tp0[0].to_bits());
        e.write_u32(ebp - 0x24, tp0[1].to_bits());
        e.write_u32(ebp - 0x20, tp0[2].to_bits());
        e.write_u32(ebp - 0x4c, tp1[0].to_bits());
        e.write_u32(ebp - 0x48, tp1[1].to_bits());
        e.write_u32(ebp - 0x44, tp1[2].to_bits());
        e.set_reg(RegisterX86::EDI, geom);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x709f1a, 0x709fa8);
        let bin_inv = e.read_f32(ebp - 0x78);
        let bin_ndir = [
            e.read_f32(ebp - 0x58),
            e.read_f32(ebp - 0x54),
            e.read_f32(ebp - 0x50),
        ];
        let bin_d = [
            e.read_f32(ebp - 0x10),
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
        ];
        let (d, inv_len, ndir) = tri_invray(tp0, tp1);
        assert_f32(bin_inv, inv_len, &format!("{ctx} inv_len"));
        for k in 0..3 {
            assert_f32(bin_d[k], d[k], &format!("{ctx} d[{k}]"));
            assert_f32(bin_ndir[k], ndir[k], &format!("{ctx} ndir[{k}]"));
        }
    };

    // probes: unit, 3-4-0 (len=5), len==0 (inv=+Inf, ndir=NaN), exact.
    run_case(&mut e, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], "inv unit-x");
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        [3.0, 4.0, 0.0],
        "inv 3-4-0 (len=5)",
    );
    run_case(
        &mut e,
        [5.0, 5.0, 5.0],
        [5.0, 5.0, 5.0],
        "inv len=0 (inv=Inf)",
    );
    run_case(&mut e, [-1.0, -2.0, -3.0], [4.0, 5.0, 6.0], "inv mid");

    let mut s = 0x709f_1a1au32;
    for i in 0..4000u32 {
        let tp0 = frand3(&mut s, -1000.0, 1000.0);
        let tp1 = frand3(&mut s, -1000.0, 1000.0);
        run_case(&mut e, tp0, tp1, &format!("inv fuzz[{i}]"));
    }
}

/// `0x7099e0` from `0x709fde` to `0x709fe7` in an island with `fstp qword` appended.
#[test]
fn tri_t_world_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    const ISL_VA: u64 = 0x709fde;
    const ISL_LEN: usize = 0x709fe7 - 0x709fde; // 9
    let bytes = e.read(ISL_VA, ISL_LEN);

    let out = e.alloc(0x10);
    let isl = e.alloc(0x40);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let mut code = bytes;
    code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out]  (DD /3, disp32 absolute)
    code.extend_from_slice(&(out as u32).to_le_bytes());
    let end = isl + code.len() as u64;
    e.write(isl, &code);

    let run_case = |e: &mut Emu, inv: f32, t_local: f32, len: f32, ctx: &str| {
        e.write_u32(ebp - 0x78, inv.to_bits()); // [ebp-0x78] = inv_len_local
        e.write_u32(ebp - 0x14, t_local.to_bits()); // [ebp-0x14] = t_local
        e.write_u32(ebp - 0x18, len.to_bits()); // [ebp-0x18] = len_world
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let bin = read_f64(e, out);
        let ours = tri_t_world(inv, t_local, len);
        // f64 bit-exact (the products are held in f64; the difftest observes the un-stored LIVE
        // st0).
        assert_eq!(
            bin.to_bits(),
            ours.to_bits(),
            "{ctx}: binary={:#018x} ours={:#018x}",
            bin.to_bits(),
            ours.to_bits()
        );
    };

    run_case(&mut e, 0.0, 5.0, 10.0, "tworld inv=0");
    run_case(&mut e, 1.0, 1.0, 1.0, "tworld ones");
    run_case(&mut e, 0.2, 3.0, 50.0, "tworld typical");
    let mut s = 0x709f_dedeu32;
    for i in 0..3000u32 {
        let inv = frand(&mut s, -2.0, 2.0);
        let t = frand(&mut s, -100.0, 100.0);
        let len = frand(&mut s, 0.0, 5000.0);
        run_case(&mut e, inv, t, len, &format!("tworld fuzz[{i}]"));
    }
}

/// `0x7099e0` from `0x70a037` to `0x70a044`.
#[test]
fn segment_world_distance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, inv: f32, t_best: f32, ctx: &str| {
        e.write_u32(ebp - 0x30, inv.to_bits());
        e.write_u32(ebp - 0x4, t_best.to_bits());
        e.write_u32(ebp + 0x10, out as u32); // [ebp+0x10] → out ptr (ecx)
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // halt at 0x70a044 (after `fstp [ecx]`@0x70a042 stores out); the `until` is EXCLUSIVE.
        e.run(0x70a037, 0x70a044);
        let bin = e.read_f32(out);
        let ours = world_distance(inv, t_best);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0.0, 5.0, "final inv=0");
    run_case(&mut e, 1.0, 1.0, "final ones");
    run_case(&mut e, 0.2, 3.0, "final typical");
    let mut s = 0x70a0_3737u32;
    for i in 0..3000u32 {
        let inv = frand(&mut s, -2.0, 2.0);
        let t = frand(&mut s, -100.0, 100.0);
        run_case(&mut e, inv, t, &format!("final fuzz[{i}]"));
    }
}

/// `0x7089c0` from `0x708a17` to `0x708ab6`.
#[test]
fn hit_ray_setup_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let p0 = e.alloc(0x20);
    let p1 = e.alloc(0x20);
    let this = e.alloc(0x400); // EDI; `mov eax,[edi+0x130]`@0x708a54 runs on the body path (list head, null)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, a0: [f32; 3], a1: [f32; 3], ctx: &str| {
        for k in 0..3 {
            e.write_u32(p0 + 4 * k as u64, a0[k].to_bits());
            e.write_u32(p1 + 4 * k as u64, a1[k].to_bits());
        }
        e.write_u32(this + 0x130, 0); // null list head (the bail loop is skipped on the normalize path)
        e.write_u32(this + 0x13c, 0); // `cmp esi,[edi+0x13c]`@0x708ac3 follows but is past our halt
        e.write_u32(ebp + 8, p0 as u32); // [ebp+8]  = p0 ptr (ecx)
        e.write_u32(ebp + 0xc, p1 as u32); // [ebp+0xc] = p1 ptr (eax)
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x708a17, 0x708ab6);
        let bin_len = e.read_f32(ebp - 0x50);
        let bin_inv = e.read_f32(ebp - 0x60);
        let bin_ndir = [
            e.read_f32(ebp - 0x44),
            e.read_f32(ebp - 0x40),
            e.read_f32(ebp - 0x3c),
        ];
        let (len, inv, ndir) = ray_setup(a0, a1);
        assert_f32(bin_len, len, &format!("{ctx} len"));
        assert_f32(bin_inv, inv, &format!("{ctx} inv"));
        for k in 0..3 {
            assert_f32(bin_ndir[k], ndir[k], &format!("{ctx} ndir[{k}]"));
        }
    };

    run_case(&mut e, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], "setup unit-x");
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        [3.0, 4.0, 0.0],
        "setup 3-4-0 (len=5)",
    );
    run_case(&mut e, [-1.0, -2.0, -3.0], [4.0, 5.0, 6.0], "setup mid");
    // **`len == eps` boundary (deterministic)**: dir = (eps, 0, 0) gives len exactly eps; the `jp`
    // takes the normalize path the slice covers (`len < eps` bails).
    let eps = f32::from_bits(EPS_BITS);
    run_case(&mut e, [0.0, 0.0, 0.0], [eps, 0.0, 0.0], "setup len==eps");

    let mut s = 0x708a_1717u32;
    for i in 0..4000u32 {
        let a0 = frand3(&mut s, -1000.0, 1000.0);
        let a1 = frand3(&mut s, -1000.0, 1000.0);
        run_case(&mut e, a0, a1, &format!("setup fuzz[{i}]"));
    }
}

/// `0x7089c0` from `0x708c4f` to `0x708c74` and from `0x708cc0` to `0x708cf8`.
#[test]
fn aabb_corner_sum_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let boxbuf = e.alloc(0x40);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    // Block C: reads [eax+0x14],[eax+8] (z); [eax+0x10],[eax+4] (y); [eax],[eax+0xc] (x).
    let run_c = |e: &mut Emu, b: [f32; 6], ctx: &str| {
        for (k, c) in b.iter().enumerate() {
            e.write_u32(boxbuf + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::EAX, boxbuf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x708c4f, 0x708c74);
        let esp = e.get_reg(RegisterX86::ESP); // SLICE_ESP − 0xc
        let bin = [
            e.read_f32(esp + 8), // z
            e.read_f32(esp + 4), // y
            e.read_f32(esp),     // x
        ];
        // hi/lo as the binary pairs them: z=(box[0x14],box[8]), y=(box[0x10],box[4]),
        // x=(box[0],box[0xc]).
        let hi = [b[5], b[4], b[0]]; // box[0x14], box[0x10], box[0]
        let lo = [b[2], b[1], b[3]]; // box[0x8],  box[0x4],  box[0xc]
        let ours = aabb_corner_sum(hi, lo);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} sum[{k}]"));
        }
    };

    // Block D: reads [ebx+0xc8],[ebx+0xbc] (z); [ebx+0xc4],[ebx+0xb8] (y); [ebx+0xb4],[ebx+0xc0]
    // (x).
    let dbuf = e.alloc(0x200);
    let run_d = |e: &mut Emu, dz: (f32, f32), dy: (f32, f32), dx: (f32, f32), ctx: &str| {
        e.write_u32(dbuf + 0xc8, dz.0.to_bits());
        e.write_u32(dbuf + 0xbc, dz.1.to_bits());
        e.write_u32(dbuf + 0xc4, dy.0.to_bits());
        e.write_u32(dbuf + 0xb8, dy.1.to_bits());
        e.write_u32(dbuf + 0xb4, dx.0.to_bits());
        e.write_u32(dbuf + 0xc0, dx.1.to_bits());
        e.set_reg(RegisterX86::EBX, dbuf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x708cc0, 0x708cf8);
        let esp = e.get_reg(RegisterX86::ESP);
        let bin = [e.read_f32(esp + 8), e.read_f32(esp + 4), e.read_f32(esp)];
        let ours = aabb_corner_sum([dz.0, dy.0, dx.0], [dz.1, dy.1, dx.1]);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} D sum[{k}]"));
        }
    };

    run_c(&mut e, [1.0, 2.0, 3.0, 4.0, 5.0, 6.0], "corner C basic");
    run_c(&mut e, [0.0, 0.0, 0.0, 0.0, 0.0, 0.0], "corner C zero");
    run_d(
        &mut e,
        (1.0, -1.0),
        (2.5, -3.5),
        (10.0, 20.0),
        "corner D basic",
    );

    let mut s = 0x708c_4f4fu32;
    for i in 0..2000u32 {
        let b = [
            frand(&mut s, -1e4, 1e4),
            frand(&mut s, -1e4, 1e4),
            frand(&mut s, -1e4, 1e4),
            frand(&mut s, -1e4, 1e4),
            frand(&mut s, -1e4, 1e4),
            frand(&mut s, -1e4, 1e4),
        ];
        run_c(&mut e, b, &format!("corner C fuzz[{i}]"));
        run_d(
            &mut e,
            (frand(&mut s, -1e4, 1e4), frand(&mut s, -1e4, 1e4)),
            (frand(&mut s, -1e4, 1e4), frand(&mut s, -1e4, 1e4)),
            (frand(&mut s, -1e4, 1e4), frand(&mut s, -1e4, 1e4)),
            &format!("corner D fuzz[{i}]"),
        );
    }
}

/// `0x7089c0` from `0x708d4c` to `0x708e2d` in an island with `fstp qword` appended, on spheres the
/// ray meets: the far distance read from `[ebp-0x64]`, the near from the island.
#[test]
fn hit_sphere_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let center = e.alloc(0x20);
    let origin = e.alloc(0x20);
    let extents = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    // `t_near` is never stored before its compare, so the stretch from `0x708d4c` to `0x708e2d`
    // runs in an island with `fstp qword` appended; on these hits its branches are not taken.
    let tnear_out = e.alloc(0x10);
    let isl = e.alloc(0x200);
    const ISL_VA: u64 = 0x708d4c;
    const ISL_LEN: usize = 0x708e2d - 0x708d4c;
    let isl_bytes = e.read(ISL_VA, ISL_LEN);
    let mut isl_code = isl_bytes;
    isl_code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [abs32]
    isl_code.extend_from_slice(&(tnear_out as u32).to_le_bytes());
    let isl_end = isl + isl_code.len() as u64;
    e.write(isl, &isl_code);

    let run_case = |e: &mut Emu,
                    c: [f32; 3],
                    o: [f32; 3],
                    ext: [f32; 3],
                    scale: f32,
                    ndir: [f32; 3],
                    ctx: &str| {
        for k in 0..3 {
            e.write_u32(center + 4 * k as u64, c[k].to_bits());
            e.write_u32(origin + 4 * k as u64, o[k].to_bits());
            e.write_u32(extents + 4 * k as u64, ext[k].to_bits());
        }
        e.write_u32(ebp - 0x68, scale.to_bits());
        e.write_u32(ebp - 0x44, ndir[0].to_bits());
        e.write_u32(ebp - 0x40, ndir[1].to_bits());
        e.write_u32(ebp - 0x3c, ndir[2].to_bits());
        e.write_u32(ebp + 8, origin as u32); // [ebp+8] = origin ptr (ecx)
        e.set_reg(RegisterX86::EAX, center);
        e.set_reg(RegisterX86::EBX, extents);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // run the island (the whole sphere math + the capturing `fstp qword`). t_far lands in
        // [ebp-0x64]; t_near (LIVE st0 after the `fsubr`) lands in tnear_out.
        e.run(isl, isl_end);
        let bin_t_far = e.read_f32(ebp - 0x64);
        let bin_t_near = read_f64(e, tnear_out) as f32;

        let h = hit_sphere(c, o, ext, scale, ndir);
        assert!(h.hit, "{ctx}: expected a HIT-path case");
        assert_f32(bin_t_far, h.t_far, &format!("{ctx} t_far"));
        assert_f32(bin_t_near, h.t_near, &format!("{ctx} t_near"));
    };

    // direct on-axis hit: center=(10,0,0), origin=0, ndir=+x, radius from extents.
    run_case(
        &mut e,
        [10.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
        1.0,
        [1.0, 0.0, 0.0],
        "sphere on-axis hit",
    );
    // tangent: perp² exactly == radius². center=(10,4,0), ndir=+x, extents=(4,0,0) → perp.y=4,
    // radius=4.
    run_case(
        &mut e,
        [10.0, 4.0, 0.0],
        [0.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        1.0,
        [1.0, 0.0, 0.0],
        "sphere tangent perp²==r²",
    );
    // t_far == 0: center=(-3,0,0), ndir=+x → proj=-3, radius=3 → √disc=3, t_far=0.
    run_case(
        &mut e,
        [-3.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [3.0, 0.0, 0.0],
        1.0,
        [1.0, 0.0, 0.0],
        "sphere t_far==0",
    );
    // scale ≠ 1 (radius = |ext|·scale): extents=(2,0,0), scale=1.5 → radius=3.
    run_case(
        &mut e,
        [8.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [2.0, 0.0, 0.0],
        1.5,
        [1.0, 0.0, 0.0],
        "sphere scaled radius",
    );

    // Fuzz: random ray + sphere, kept on the HIT path by construction (center on the ray within
    // radius).
    let mut s = 0x708d_4c4cu32;
    let mut done = 0u32;
    let mut tries = 0u32;
    while done < 4000 && tries < 200_000 {
        tries += 1;
        let o = frand3(&mut s, -500.0, 500.0);
        let mut nd = frand3(&mut s, -1.0, 1.0);
        let nl =
            (f64::from(nd[0]).powi(2) + f64::from(nd[1]).powi(2) + f64::from(nd[2]).powi(2)).sqrt();
        if nl < 1e-3 {
            continue;
        }
        nd = [
            (f64::from(nd[0]) / nl) as f32,
            (f64::from(nd[1]) / nl) as f32,
            (f64::from(nd[2]) / nl) as f32,
        ];
        let radius = frand(&mut s, 1.0, 200.0);
        let proj = frand(&mut s, 0.5, 400.0);
        let ax = if nd[0].abs() < 0.9 {
            [1.0f32, 0.0, 0.0]
        } else {
            [0.0f32, 1.0, 0.0]
        };
        let perp_dir = [
            nd[1] * ax[2] - nd[2] * ax[1],
            nd[2] * ax[0] - nd[0] * ax[2],
            nd[0] * ax[1] - nd[1] * ax[0],
        ];
        let pl = (f64::from(perp_dir[0]).powi(2)
            + f64::from(perp_dir[1]).powi(2)
            + f64::from(perp_dir[2]).powi(2))
        .sqrt();
        if pl < 1e-3 {
            continue;
        }
        let poff = frand(&mut s, 0.0, radius * 0.85);
        let scl = poff / pl as f32;
        let center = [
            o[0] + proj * nd[0] + perp_dir[0] * scl,
            o[1] + proj * nd[1] + perp_dir[1] * scl,
            o[2] + proj * nd[2] + perp_dir[2] * scl,
        ];
        let ext = [radius, 0.0, 0.0];
        let h = hit_sphere(center, o, ext, 1.0, nd);
        if !h.hit || h.t_far < 0.0 {
            continue;
        }
        run_case(
            &mut e,
            center,
            o,
            ext,
            1.0,
            nd,
            &format!("sphere fuzz[{done}]"),
        );
        done += 1;
    }
    assert!(
        done >= 3000,
        "sphere fuzz produced too few HIT cases: {done}"
    );
}

/// `0x7089c0` from `0x708eba` to `0x708f1e`.
#[test]
fn scale_t_dot_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let p0 = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, ndir: [f32; 3], p: [f32; 3], ctx: &str| {
        e.write_u32(ebp - 0x44, ndir[0].to_bits());
        e.write_u32(ebp - 0x40, ndir[1].to_bits());
        e.write_u32(ebp - 0x3c, ndir[2].to_bits());
        for (k, c) in p.iter().enumerate() {
            e.write_u32(p0 + 4 * k as u64, c.to_bits());
        }
        e.write_u32(ebp + 8, p0 as u32); // [ebp+8] = p0 ptr (eax)
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // halt at 0x708f1e (after `fstp [ebp-0xd4]`@0x708f18 stores the dot; the intervening movs
        // are inert).
        e.run(0x708eba, 0x708f1e);
        let bin = e.read_f32(ebp - 0xd4);
        let ours = scale_t_dot(ndir, p);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, [1.0, 0.0, 0.0], [5.0, 0.0, 0.0], "dot unit-x");
    run_case(&mut e, [0.0, 1.0, 0.0], [0.0, -3.0, 0.0], "dot unit-y");
    let mut s = 0x708e_baba_u32;
    for i in 0..3000u32 {
        let ndir = frand3(&mut s, -1.0, 1.0);
        let p = frand3(&mut s, -1000.0, 1000.0);
        run_case(&mut e, ndir, p, &format!("dot fuzz[{i}]"));
    }
}

/// `0x7089c0` from `0x708f5f` to `0x708f67`.
#[test]
fn hit_tbest_init_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rp = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, len: f32, param: f32, ctx: &str| {
        e.write_u32(ebp - 0x50, len.to_bits());
        e.write_u32(rp, param.to_bits());
        e.set_reg(RegisterX86::ECX, rp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // halt at 0x708f67 (after `fstp [ebp+0xc]`@0x708f64 stores the seed).
        e.run(0x708f5f, 0x708f67);
        let bin = e.read_f32(ebp + 0xc);
        let ours = tbest_init(len, param);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 5.0, 0.0, "tbest len=5 p=0");
    run_case(&mut e, 5.0, 0.5, "tbest len=5 p=0.5");
    run_case(&mut e, 100.0, 0.25, "tbest len=100 p=0.25");
    let mut s = 0x708f_5f5fu32;
    for i in 0..3000u32 {
        let len = frand(&mut s, 0.0, 5000.0);
        let p = frand(&mut s, -2.0, 2.0);
        run_case(&mut e, len, p, &format!("tbest fuzz[{i}]"));
    }
}

/// `0x7089c0` at `0x709099` and `0x7090be`, each after `fld dword` with `fstp qword` appended.
#[test]
fn alpha_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // The two `fmul [reg+reg+disp8]` are 4 bytes each: 0x709099 = d8 4c 3a 3c, 0x7090be = d8 4c 10
    // 0c. We neutralize the base/index registers to 0 by pointing the disp at our `modifier` slot —
    // i.e. set edx=edi=0 (resp. eax=edx=0) and write modifier at absolute address disp8. To keep it
    // simple we instead pre-load base, then run the actual fmul bytes with the two index regs aimed
    // at a buffer.
    let out = e.alloc(0x10);
    let buf = e.alloc(0x100); // the [reg+reg+disp8] target buffer
    let isl = e.alloc(0x40);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_one = |e: &mut Emu, va: u64, base: f32, modifier: f32, ctx: &str| {
        // read the actual fmul bytes (4) and decode the disp8 (byte[3]); place `modifier` at
        // buf+disp8 with the two index regs = 0 so the effective address is buf+disp8.
        let fmul = e.read(va, 4);
        let disp = fmul[3] as u64;
        e.write_u32(buf + disp, modifier.to_bits());
        // code: fld dword [base_abs] (d9 05 + abs32); <fmul bytes>; fstp qword [out] (dd 1d +
        // abs32).
        let base_slot = ebp - 0x4; // a scratch slot for base
        e.write_u32(base_slot, base.to_bits());
        let mut code = vec![0xd9, 0x05];
        code.extend_from_slice(&(base_slot as u32).to_le_bytes());
        code.extend_from_slice(&fmul);
        code.extend_from_slice(&[0xdd, 0x1d]);
        code.extend_from_slice(&(out as u32).to_le_bytes());
        let end = isl + code.len() as u64;
        e.write(isl, &code);
        // the fmul uses [base+index+disp8]; set both to buf so addr = buf + disp8 (base=buf,
        // index=0). 0x709099 is [edx+edi+0x3c]: edx=buf, edi=0. 0x7090be is [eax+edx+0xc]: eax=buf,
        // edx=0.
        e.set_reg(RegisterX86::EAX, buf);
        e.set_reg(RegisterX86::EDX, if va == 0x709099 { buf } else { 0 });
        e.set_reg(RegisterX86::EDI, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let bin = read_f64(e, out);
        let ours = alpha_scale(base, modifier);
        assert_eq!(
            bin.to_bits(),
            ours.to_bits(),
            "{ctx}: binary={:#018x} ours={:#018x}",
            bin.to_bits(),
            ours.to_bits()
        );
    };

    for &va in &[0x709099u64, 0x7090be] {
        run_one(&mut e, va, 1.0, 1.0, &format!("alpha {va:#x} ones"));
        run_one(&mut e, va, 0.5, 0.25, &format!("alpha {va:#x} half"));
        run_one(&mut e, va, 0.0, 7.0, &format!("alpha {va:#x} zero-base"));
        let mut s = 0x7090_9999u32 ^ (va as u32);
        for i in 0..2000u32 {
            let base = frand(&mut s, 0.0, 1.0);
            let modifier = frand(&mut s, 0.0, 4.0);
            run_one(
                &mut e,
                va,
                base,
                modifier,
                &format!("alpha {va:#x} fuzz[{i}]"),
            );
        }
    }
}

/// `0x7089c0` from `0x709224` to `0x7092ad`: the twelve entries read from `[ebp-0xb8]`.
#[test]
fn skin_first_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mat = e.alloc(0x80);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    // the 12 matrix-element offsets in store order (block layout) and their output slots.
    let offs: [u64; 12] = [
        0, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28, 0x30, 0x34, 0x38,
    ];
    let outs: [i64; 12] = [
        -0xb8, -0xb4, -0xb0, -0xa8, -0xa4, -0xa0, -0x98, -0x94, -0x90, -0x88, -0x84, -0x80,
    ];

    let run_case = |e: &mut Emu, wbyte: u8, m: [f32; 12], ctx: &str| {
        e.write_u32(ebp - 0x1c, u32::from(wbyte)); // [ebp-0x1c] = the byte weight (fild reads it)
        for k in 0..12 {
            e.write_u32(mat + offs[k], m[k].to_bits());
        }
        e.set_reg(RegisterX86::EAX, mat);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // halt at 0x7092ad (after `fstp [ebp-0x80]`@0x7092aa stores the 12th element).
        e.run(0x709224, 0x7092ad);
        let ours = skin_first(wbyte, m);
        for k in 0..12 {
            let bin = e.read_f32((ebp as i64 + outs[k]) as u64);
            assert_f32(bin, ours[k], &format!("{ctx} m[{k}]"));
        }
    };

    let ident = {
        let mut m = [0.0f32; 12];
        m[0] = 1.0;
        m[4] = 1.0;
        m[8] = 1.0;
        m
    };
    run_case(&mut e, 255, ident, "skin1 w=255 ident");
    run_case(&mut e, 0, ident, "skin1 w=0");
    run_case(&mut e, 128, ident, "skin1 w=128");
    let mut s = 0x7092_2424u32;
    for i in 0..3000u32 {
        let wbyte = (next(&mut s) >> 16) as u8;
        let mut m = [0.0f32; 12];
        for c in m.iter_mut() {
            *c = frand(&mut s, -100.0, 100.0);
        }
        run_case(&mut e, wbyte, m, &format!("skin1 fuzz[{i}]"));
    }
}

/// `0x7089c0` from `0x7092d0` to `0x7093a6`.
#[test]
fn skin_accum_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mat = e.alloc(0x80);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let offs: [u64; 12] = [
        0, 4, 8, 0x10, 0x14, 0x18, 0x20, 0x24, 0x28, 0x30, 0x34, 0x38,
    ];
    let outs: [i64; 12] = [
        -0xb8, -0xb4, -0xb0, -0xa8, -0xa4, -0xa0, -0x98, -0x94, -0x90, -0x88, -0x84, -0x80,
    ];

    let run_case = |e: &mut Emu, wbyte: u8, m: [f32; 12], acc: [f32; 12], ctx: &str| {
        e.write_u32(ebp - 0x1c, u32::from(wbyte));
        for k in 0..12 {
            e.write_u32(mat + offs[k], m[k].to_bits());
            e.write_u32((ebp as i64 + outs[k]) as u64, acc[k].to_bits());
        }
        e.set_reg(RegisterX86::EAX, mat);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // halt at 0x7093a6 (the loop-back `jb`) — covers exactly one accumulate iteration's 25 FP
        // ops.
        e.run(0x7092d0, 0x7093a6);
        let ours = skin_accum(wbyte, m, acc);
        for k in 0..12 {
            let bin = e.read_f32((ebp as i64 + outs[k]) as u64);
            assert_f32(bin, ours[k], &format!("{ctx} acc[{k}]"));
        }
    };

    let acc0 = [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
    ];
    run_case(&mut e, 255, [0.0; 12], acc0, "skinA w=255 m=0");
    run_case(&mut e, 128, [1.0; 12], acc0, "skinA w=128 m=1");
    let mut s = 0x7092_d0d0u32;
    for i in 0..3000u32 {
        let wbyte = (next(&mut s) >> 16) as u8;
        let mut m = [0.0f32; 12];
        let mut acc = [0.0f32; 12];
        for k in 0..12 {
            m[k] = frand(&mut s, -100.0, 100.0);
            acc[k] = frand(&mut s, -1000.0, 1000.0);
        }
        run_case(&mut e, wbyte, m, acc, &format!("skinA fuzz[{i}]"));
    }
}

/// `0x7089c0` from `0x7093c8` to `0x709445`.
#[test]
fn mat_x_dir_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let geom = e.alloc(0x40); // ESI; reads [esi+0x14/0x18/0x1c]
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    // matrix slots in the binary's fmul order: row0 = [-0x98(·vz), -0xa8(·vy), -0xb8(·vx)],
    // row1 = [-0x94, -0xa4, -0xb4], row2 = [-0x90, -0xa0, -0xb0].
    let mslots: [i64; 9] = [
        -0x98, -0xa8, -0xb8, -0x94, -0xa4, -0xb4, -0x90, -0xa0, -0xb0,
    ];

    let run_case = |e: &mut Emu, m: [f32; 9], v: [f32; 3], acc: [f32; 3], ctx: &str| {
        for k in 0..9 {
            e.write_u32((ebp as i64 + mslots[k]) as u64, m[k].to_bits());
        }
        e.write_u32(geom + 0x14, v[0].to_bits());
        e.write_u32(geom + 0x18, v[1].to_bits());
        e.write_u32(geom + 0x1c, v[2].to_bits());
        e.write_u32(ebp - 0x10, acc[0].to_bits());
        e.write_u32(ebp - 0xc, acc[1].to_bits());
        e.write_u32(ebp - 0x8, acc[2].to_bits());
        e.set_reg(RegisterX86::ESI, geom);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // halt at 0x709445 (after `fstp [ebp-0x8]`@0x709442 stores acc.z).
        e.run(0x7093c8, 0x709445);
        let bin = [
            e.read_f32(ebp - 0x10),
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
        ];
        let ours = mat_x_dir(m, v, acc);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} acc[{k}]"));
        }
    };

    run_case(
        &mut e,
        [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0],
        [2.0, 3.0, 4.0],
        [10.0, 20.0, 30.0],
        "mat ident",
    );
    let mut s = 0x7093_c8c8u32;
    for i in 0..3000u32 {
        let mut m = [0.0f32; 9];
        for c in m.iter_mut() {
            *c = frand(&mut s, -2.0, 2.0);
        }
        let v = frand3(&mut s, -100.0, 100.0);
        let acc = frand3(&mut s, -1000.0, 1000.0);
        run_case(&mut e, m, v, acc, &format!("mat fuzz[{i}]"));
    }
}

/// `0x7089c0` from `0x709445` to `0x7094a8` and from `0x709777` to `0x7097dd`.
#[test]
fn tworld_recover_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x400); // [ebp-0x28] = this; `mov eax,[edx+0x140]` reads a ptr (output sink)
    let sink = e.alloc(0x40);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    e.write_u32(this + 0x140, sink as u32);

    // free vec3s for ndir and the accumulator; A and B place them in their own slots.
    let run_a = |e: &mut Emu, ndir: [f32; 3], acc: [f32; 3], dot: f32, ctx: &str| {
        e.write_u32(ebp - 0x44, ndir[0].to_bits());
        e.write_u32(ebp - 0x40, ndir[1].to_bits());
        e.write_u32(ebp - 0x3c, ndir[2].to_bits());
        e.write_u32(ebp - 0x10, acc[0].to_bits());
        e.write_u32(ebp - 0xc, acc[1].to_bits());
        e.write_u32(ebp - 0x8, acc[2].to_bits());
        e.write_u32(ebp - 0xd4, dot.to_bits());
        e.write_u32(ebp - 0x28, this as u32); // [ebp-0x28] = this (edx ← it for the [edx+0x140] sink)
        e.write_u32(ebp - 0x14, 0); // [ebp-0x14] (the `add ecx,0xc` index) — inert for FP
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // halt at 0x7094a8 (after `fstp [ebp-0xd8]`@0x7094a2 stores the 3rd component).
        e.run(0x709445, 0x7094a8);
        let bin = [
            e.read_f32(ebp - 0xe0),
            e.read_f32(ebp - 0xdc),
            e.read_f32(ebp - 0xd8),
        ];
        // A: dot order (nx,ax),(ny,ay),(nz,az); org (nx,ax),(ny,ay).
        let ours = tworld_recover(
            [(ndir[0], acc[0]), (ndir[1], acc[1]), (ndir[2], acc[2])],
            [(ndir[0], acc[0]), (ndir[1], acc[1])],
            dot,
        );
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} A out[{k}]"));
        }
    };

    let bsink = e.alloc(0x40);
    let this2 = e.alloc(0x400);
    e.write_u32(this2 + 0x140, bsink as u32);
    let run_b = |e: &mut Emu, ndir: [f32; 3], acc: [f32; 3], dot: f32, ctx: &str| {
        // ndir slots are the SAME [-0x44]/-0x40/-0x3c; acc (the B transform output) is at
        // [-0xbc]/-0xc4/-0xc0.
        e.write_u32(ebp - 0x44, ndir[0].to_bits());
        e.write_u32(ebp - 0x40, ndir[1].to_bits());
        e.write_u32(ebp - 0x3c, ndir[2].to_bits());
        e.write_u32(ebp - 0xbc, acc[0].to_bits()); // paired with ndir[2] in the dot's pair0
        e.write_u32(ebp - 0xc4, acc[1].to_bits()); // paired with ndir[0] in the dot's pair1 + org0
        e.write_u32(ebp - 0xc0, acc[2].to_bits()); // paired with ndir[1] in the dot's pair2 + org1
        e.write_u32(ebp - 0xd4, dot.to_bits());
        e.write_u32(ebp - 0x28, this2 as u32);
        e.set_reg(RegisterX86::EDI, this2);
        e.set_reg(RegisterX86::ESI, 0); // `add edx,esi`@0x70978c → edx = [edi+0x140] (the sink), esi=0
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // halt at 0x7097dd (after `fstp [ebp-0xc8]`@0x7097d7 stores the 3rd component).
        e.run(0x709777, 0x7097dd);
        let bin = [
            e.read_f32(ebp - 0xd0),
            e.read_f32(ebp - 0xcc),
            e.read_f32(ebp - 0xc8),
        ];
        // B: dot order (n_3c,a_bc),(n_44,a_c4),(n_40,a_c0); org (n_44,a_c4),(n_40,a_c0).
        let ours = tworld_recover(
            [(ndir[2], acc[0]), (ndir[0], acc[1]), (ndir[1], acc[2])],
            [(ndir[0], acc[1]), (ndir[1], acc[2])],
            dot,
        );
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} B out[{k}]"));
        }
    };

    run_a(
        &mut e,
        [1.0, 0.0, 0.0],
        [5.0, 6.0, 7.0],
        2.0,
        "recover basic",
    );
    run_b(
        &mut e,
        [1.0, 0.0, 0.0],
        [5.0, 6.0, 7.0],
        2.0,
        "recover basic",
    );
    let mut s = 0x7094_4545u32;
    for i in 0..3000u32 {
        let nd = frand3(&mut s, -1.0, 1.0);
        let acc = frand3(&mut s, -500.0, 500.0);
        let dot = frand(&mut s, -500.0, 500.0);
        run_a(&mut e, nd, acc, dot, &format!("recover A fuzz[{i}]"));
        run_b(&mut e, nd, acc, dot, &format!("recover B fuzz[{i}]"));
    }
}

/// `0x7089c0` from `0x70951f` to `0x7095ff` and from `0x70983e` to `0x70991e` on triangles that
/// contain the point: `u`, `v` and `t` read from the frame.
#[test]
fn tri_predicate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let v0 = e.alloc(0x20);
    let v1 = e.alloc(0x20);
    let v2 = e.alloc(0x20);
    let p0 = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let write_tri = |e: &mut Emu, a: [f32; 3], b: [f32; 3], c: [f32; 3], pp: [f32; 3]| {
        for k in 0..3 {
            e.write_u32(v0 + 4 * k as u64, a[k].to_bits());
            e.write_u32(v1 + 4 * k as u64, b[k].to_bits());
            e.write_u32(v2 + 4 * k as u64, c[k].to_bits());
            e.write_u32(p0 + 4 * k as u64, pp[k].to_bits());
        }
        e.write_u32(ebp + 8, p0 as u32);
    };

    // A: u=[ebp-0x1c], v=[ebp-0x14], t=[ebp-0x4]. Halt at 0x7095ff (after `fst [ebp-0x4]`).
    let run_a = |e: &mut Emu, a: [f32; 3], b: [f32; 3], c: [f32; 3], pp: [f32; 3], ctx: &str| {
        write_tri(e, a, b, c, pp);
        e.set_reg(RegisterX86::ECX, v0);
        e.set_reg(RegisterX86::EDX, v1);
        e.set_reg(RegisterX86::ESI, v2);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x70951f, 0x7095ff);
        let ours = tri_predicate(a, b, c, pp);
        assert!(ours.accept, "{ctx}: expected an ACCEPT-path case");
        assert_f32(e.read_f32(ebp - 0x1c), ours.u, &format!("{ctx} A u"));
        assert_f32(e.read_f32(ebp - 0x14), ours.v, &format!("{ctx} A v"));
        assert_f32(e.read_f32(ebp - 0x4), ours.t, &format!("{ctx} A t"));
    };
    // B: u=[ebp-0x24], v=[ebp-0x18], t=[ebp-0x4]. Halt at 0x70991b (after `fst [ebp-0x4]`).
    let run_b = |e: &mut Emu, a: [f32; 3], b: [f32; 3], c: [f32; 3], pp: [f32; 3], ctx: &str| {
        write_tri(e, a, b, c, pp);
        e.set_reg(RegisterX86::ECX, v0);
        e.set_reg(RegisterX86::EDX, v1);
        e.set_reg(RegisterX86::ESI, v2);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // halt at 0x70991e (after `fst [ebp-0x4]`@0x70991b stores t).
        e.run(0x70983e, 0x70991e);
        let ours = tri_predicate(a, b, c, pp);
        assert!(ours.accept, "{ctx}: expected an ACCEPT-path case");
        assert_f32(e.read_f32(ebp - 0x24), ours.u, &format!("{ctx} B u"));
        assert_f32(e.read_f32(ebp - 0x18), ours.v, &format!("{ctx} B v"));
        assert_f32(e.read_f32(ebp - 0x4), ours.t, &format!("{ctx} B t"));
    };

    // A canonical accept-path triangle: p0 inside the (0,0)-(1,0)-(0,1) triangle, all z positive.
    let (a, b, c) = ([0.0, 0.0, 1.0], [4.0, 0.0, 2.0], [0.0, 4.0, 3.0]);
    let pp = [1.0, 1.0, 0.0];
    run_a(&mut e, a, b, c, pp, "tri accept");
    run_b(&mut e, a, b, c, pp, "tri accept");

    // A determinant of exactly 9.9999997e-6 is tested; [`tri_predicate_paths_difftest`] runs
    // the misses.
    let eps = f32::from_bits(EPS_BITS);
    let deg_at = ([0.0f32, 0.0, 1.0], [eps, 0.0, 1.0], [0.0f32, 1.0, 1.0]);
    let pp_in = [eps * 0.25, 0.25f32, 0.0f32];
    run_a(&mut e, deg_at.0, deg_at.1, deg_at.2, pp_in, "tri det==eps");
    run_b(&mut e, deg_at.0, deg_at.1, deg_at.2, pp_in, "tri det==eps");

    // Fuzz accept cases: random triangle, p0 a random interior point (barycentric weights >= 0),
    // z>0.
    let mut s = 0x7095_1f1fu32;
    let mut done = 0u32;
    let mut tries = 0u32;
    while done < 4000 && tries < 400_000 {
        tries += 1;
        let a = [
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, 0.5, 100.0),
        ];
        let b = [
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, 0.5, 100.0),
        ];
        let c = [
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, -100.0, 100.0),
            frand(&mut s, 0.5, 100.0),
        ];
        // interior barycentric weights (all > 0): bw normalized.
        let mut bw = [
            frand(&mut s, 0.05, 1.0),
            frand(&mut s, 0.05, 1.0),
            frand(&mut s, 0.05, 1.0),
        ];
        let sum = bw[0] + bw[1] + bw[2];
        for w in &mut bw {
            *w /= sum;
        }
        let pp = [
            bw[0] * a[0] + bw[1] * b[0] + bw[2] * c[0],
            bw[0] * a[1] + bw[1] * b[1] + bw[2] * c[1],
            0.0,
        ];
        let h = tri_predicate(a, b, c, pp);
        if !h.accept {
            continue;
        }
        run_a(&mut e, a, b, c, pp, &format!("tri A fuzz[{done}]"));
        run_b(&mut e, a, b, c, pp, &format!("tri B fuzz[{done}]"));
        done += 1;
    }
    assert!(
        done >= 3000,
        "tri fuzz produced too few ACCEPT cases: {done}"
    );
}

/// `0x7089c0` from `0x7099c4` to `0x7099d0`.
#[test]
fn hit_world_distance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let out = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, inv: f32, t_best: f32, ctx: &str| {
        e.write_u32(ebp - 0x60, inv.to_bits());
        e.write_u32(ebp + 0xc, t_best.to_bits());
        e.write_u32(ebp + 0x10, out as u32); // [ebp+0x10] → out ptr (ecx)
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // halt at 0x7099d0 (after `fstp [ecx]`@0x7099ce; the `pop edi`@0x7099cd is inert here).
        e.run(0x7099c4, 0x7099d0);
        let bin = e.read_f32(out);
        let ours = world_distance(inv, t_best);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0.0, 5.0, "final inv=0");
    run_case(&mut e, 1.0, 1.0, "final ones");
    run_case(&mut e, 0.2, 3.0, "final typical");
    let mut s = 0x7099_c4c4u32;
    for i in 0..3000u32 {
        let inv = frand(&mut s, -2.0, 2.0);
        let t = frand(&mut s, -100.0, 100.0);
        run_case(&mut e, inv, t, &format!("final fuzz[{i}]"));
    }
}

/// One sphere's inputs.
#[derive(Clone, Copy)]
struct SphereIn {
    center: [f32; 3],
    origin: [f32; 3],
    extents: [f32; 3],
    scale: f32,
    ndir: [f32; 3],
    len: f32,
}

/// Probes of the sphere compares, the ray along +x from the origin: a tangent, a far point exactly
/// at the start and behind it, a near point exactly at the end and past it, a miss, and NaN.
fn sphere_probes() -> Vec<(SphereIn, &'static str)> {
    let at = |c: [f32; 3], r: f32, len: f32| SphereIn {
        center: c,
        origin: [0.0; 3],
        extents: [r, 0.0, 0.0],
        scale: 1.0,
        ndir: [1.0, 0.0, 0.0],
        len,
    };
    vec![
        (at([10.0, 4.0, 0.0], 4.0, 100.0), "tangent"),
        (at([10.0, 4.5, 0.0], 4.0, 100.0), "miss"),
        (at([-3.0, 0.0, 0.0], 3.0, 100.0), "far point at the start"),
        (at([-5.0, 0.0, 0.0], 3.0, 100.0), "behind"),
        (at([10.0, 0.0, 0.0], 3.0, 7.0), "near point at the end"),
        (
            at([10.0, 0.0, 0.0], 3.0, f32::from_bits(0x40df_ffff)),
            "near point past the end",
        ),
        (at([f32::NAN, 0.0, 0.0], 3.0, 100.0), "NaN centre"),
        (at([10.0, 0.0, 0.0], f32::NAN, 100.0), "NaN radius"),
        (at([10.0, 0.0, 0.0], 3.0, f32::NAN), "NaN length"),
    ]
}

/// Random spheres round a random ray, most meeting it.
fn sphere_fuzz(s: &mut u32) -> SphereIn {
    let o = frand3(s, -50.0, 50.0);
    let d = frand3(s, -1.0, 1.0);
    let l = (d.iter().map(|c| f64::from(*c).powi(2)).sum::<f64>())
        .sqrt()
        .max(1e-3);
    let ndir = d.map(|c| (f64::from(c) / l) as f32);
    let t = frand(s, -20.0, 80.0);
    let off = frand3(s, -5.0, 5.0);
    SphereIn {
        center: [0, 1, 2].map(|k| o[k] + ndir[k] * t + off[k]),
        origin: o,
        extents: frand3(s, 0.0, 4.0),
        scale: frand(s, 0.25, 2.0),
        ndir,
        len: frand(s, 1.0, 100.0),
    }
}

/// Which sphere function and where its stretch starts and joins the node loop.
struct SphereSite {
    begin: u64,
    join: u64,
    /// The register the extents are read through (`edi` or `ebx`) and the one the scene is.
    extents_reg: RegisterX86,
    scene_reg: RegisterX86,
    scale: u64,
    ndir: u64,
    len: u64,
    slot: u64,
    index: u64,
}

/// Runs a sphere stretch to its join with the hit list wired, and compares whether a candidate was
/// recorded, and its near and far distances, with [`sphere_meets`], [`sphere_ahead`],
/// [`sphere_within`] and [`sphere_near`].
fn check_sphere_site(
    e: &mut Emu,
    site: &SphereSite,
    model: fn([f32; 3], [f32; 3], [f32; 3], f32, [f32; 3]) -> benilla_re_models::scene::SphereHit,
) {
    let center = e.alloc(0x20);
    let origin = e.alloc(0x20);
    let extents = e.alloc(0x20);
    let scene = e.alloc(0x200);
    let node = e.alloc(0x500);
    let list = e.alloc(0x40);
    let rec = e.alloc(0x40);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    e.write_u32(scene + 0x134, rec as u32);
    e.write_u32(scene + 0x138, list as u32);
    let run = |e: &mut Emu, s: SphereIn, ctx: &str| {
        for k in 0..3 {
            e.write_u32(center + 4 * k as u64, s.center[k].to_bits());
            e.write_u32(origin + 4 * k as u64, s.origin[k].to_bits());
            e.write_u32(extents + 4 * k as u64, s.extents[k].to_bits());
            e.write_u32(ebp - site.ndir + 4 * k as u64, s.ndir[k].to_bits());
        }
        for k in 0..4 {
            e.write_u32(rec + 4 * k, 0);
        }
        e.write_u32(ebp - site.scale, s.scale.to_bits());
        e.write_u32(ebp - site.len, s.len.to_bits());
        e.write_u32(ebp - site.slot, 0);
        e.write_u32(ebp - site.index, 0);
        e.write_u32(ebp + 8, origin as u32);
        e.set_reg(RegisterX86::EAX, center);
        e.set_reg(site.extents_reg, extents);
        e.set_reg(site.scene_reg, scene);
        e.set_reg(RegisterX86::ESI, node);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(site.begin, site.join);
        let h = model(s.center, s.origin, s.extents, s.scale, s.ndir);
        let ours = h.hit && sphere_ahead(h.t_far_live) && sphere_within(h.t_near_live, s.len);
        assert_eq!(sphere_meets(h.perp2, h.radius2), h.hit, "{ctx}");
        let recorded = e.read_u32(rec) == node as u32;
        assert_eq!(recorded, ours, "{ctx}: recorded");
        if ours {
            assert_f32(
                e.read_f32(rec + 4),
                sphere_near(h.t_near),
                &format!("{ctx} near"),
            );
            assert_f32(e.read_f32(rec + 8), h.t_far, &format!("{ctx} far"));
        }
    };
    for (s, ctx) in sphere_probes() {
        run(e, s, ctx);
    }
    let mut s = site.begin as u32;
    for i in 0..3000 {
        let input = sphere_fuzz(&mut s);
        run(e, input, &format!("fuzz[{i}]"));
    }
}

/// `0x7099e0` from `0x709cf2` to the node loop at `0x709e36`, the hit list at the scene's `+0x134`.
#[test]
fn segment_sphere_paths_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let site = SphereSite {
        begin: 0x709cf2,
        join: 0x709e36,
        extents_reg: RegisterX86::EDI,
        scene_reg: RegisterX86::EBX,
        scale: 0x34,
        ndir: 0x28,
        len: 0x18,
        slot: 0x14,
        index: 0x1c,
    };
    check_sphere_site(&mut e, &site, segment_sphere);
}

/// `0x7089c0` from `0x708d4c` to the node loop at `0x708e8e`, the hit list at the scene's `+0x134`.
#[test]
fn hit_sphere_paths_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let site = SphereSite {
        begin: 0x708d4c,
        join: 0x708e8e,
        extents_reg: RegisterX86::EBX,
        scene_reg: RegisterX86::EDI,
        scale: 0x68,
        ndir: 0x44,
        len: 0x50,
        slot: 0x4,
        index: 0x38,
    };
    check_sphere_site(&mut e, &site, hit_sphere);
}

/// `0x7089c0` from `0x70951f` to `0x709644` and from `0x70983e` to `0x709963`, the best at
/// `[ebp+0xc]` +∞ so an accepted triangle always replaces it: whether `[ebp-0x30]` takes the
/// candidate against [`tri_predicate`]. A determinant at, under and above 9.9999997e-6, each of
/// `u`, `v`, `w` and `t` at and under 0, and NaN probe the compares.
#[test]
fn tri_predicate_paths_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let verts = e.alloc(0x40);
    let p0 = e.alloc(0x20);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let run = |e: &mut Emu, a: [f32; 3], b: [f32; 3], c: [f32; 3], pp: [f32; 3], ctx: &str| {
        let ours = tri_predicate(a, b, c, pp);
        for (begin, end) in [(0x70951f, 0x709644), (0x70983e, 0x709963)] {
            for (i, v) in [a, b, c].iter().enumerate() {
                for k in 0..3 {
                    e.write_u32(verts + 0x10 * i as u64 + 4 * k, v[k as usize].to_bits());
                }
            }
            for k in 0..3 {
                e.write_u32(p0 + 4 * k, pp[k as usize].to_bits());
            }
            e.write_u32(ebp + 8, p0 as u32);
            e.write_u32(ebp + 0xc, f32::INFINITY.to_bits());
            e.write_u32(ebp - 0x48, 0);
            e.write_u32(ebp - 0x30, 0);
            e.write_u32(ebp - 0x2c, 0x5eed);
            e.set_reg(RegisterX86::ECX, verts);
            e.set_reg(RegisterX86::EDX, verts + 0x10);
            e.set_reg(RegisterX86::ESI, verts + 0x20);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(begin, end);
            let taken = e.read_u32(ebp - 0x30) == 0x5eed;
            assert_eq!(taken, ours.accept, "{ctx} at {begin:#x}");
            if taken {
                assert_f32(e.read_f32(ebp + 0xc), ours.t, &format!("{ctx} t"));
            }
        }
    };
    let eps = f32::from_bits(EPS_BITS);
    let tri = |x: f32| ([0.0, 0.0, 1.0], [x, 0.0, 1.0], [0.0, 1.0, 1.0]);
    for (x, ctx) in [
        (eps, "det eps"),
        (f32::from_bits(EPS_BITS - 1), "det under eps"),
        (f32::from_bits(EPS_BITS + 1), "det over eps"),
    ] {
        let (a, b, c) = tri(x);
        run(&mut e, a, b, c, [x * 0.25, 0.25, 0.0], ctx);
    }
    let (a, b, c) = ([0.0, 0.0, 1.0], [4.0, 0.0, 2.0], [0.0, 4.0, 3.0]);
    for (pp, ctx) in [
        ([1.0, 1.0, 0.0], "inside"),
        ([0.0, 0.0, 0.0], "at v0"),
        ([4.0, 0.0, 0.0], "at v1"),
        ([0.0, 4.0, 0.0], "at v2"),
        ([2.0, 2.0, 0.0], "on the far edge"),
        ([-1.0, 1.0, 0.0], "outside u"),
        ([1.0, -1.0, 0.0], "outside v"),
        ([3.0, 3.0, 0.0], "outside w"),
        ([f32::NAN, 1.0, 0.0], "NaN point"),
    ] {
        run(&mut e, a, b, c, pp, ctx);
    }
    run(
        &mut e,
        [0.0, 0.0, -1.0],
        [4.0, 0.0, -2.0],
        [0.0, 4.0, -3.0],
        [1.0, 1.0, 0.0],
        "t below 0",
    );
    run(
        &mut e,
        [0.0, 0.0, 0.0],
        [4.0, 0.0, 0.0],
        [0.0, 4.0, 0.0],
        [1.0, 1.0, 0.0],
        "t 0",
    );
    run(
        &mut e,
        [0.0, 0.0, f32::NAN],
        [4.0, 0.0, 2.0],
        [0.0, 4.0, 3.0],
        [1.0, 1.0, 0.0],
        "t NaN",
    );
    run(
        &mut e,
        [f32::NAN, 0.0, 1.0],
        [4.0, 0.0, 2.0],
        [0.0, 4.0, 3.0],
        [1.0, 1.0, 0.0],
        "det NaN",
    );
    let mut s = 0x7095_2020u32;
    for i in 0..3000 {
        let a = [
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -5.0, 10.0),
        ];
        let b = [
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -5.0, 10.0),
        ];
        let c = [
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -10.0, 10.0),
            frand(&mut s, -5.0, 10.0),
        ];
        let pp = [frand(&mut s, -10.0, 10.0), frand(&mut s, -10.0, 10.0), 0.0];
        run(&mut e, a, b, c, pp, &format!("fuzz[{i}]"));
    }
}
