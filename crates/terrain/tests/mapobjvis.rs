//! The arithmetic of the map object visibility and traces, as slices: `SeedInteriorVisibility`
//! (`0x6b3b20`), `SeedExteriorVisibility` (`0x6b3dd0`), `DrawGroupNormals` (`0x6b5f90`),
//! `CMapObjDef::TraceGroup` (`0x6a2600`), `TraceMapObjectPlanes` (`0x6aa1f0`) and
//! `IntersectMapObjects` (`0x6be250`); and `MaxPortalDistance` (`0x6b4680`) run whole. A stretch
//! run in an island is the exe's own bytes, checked, copied between a load and a store.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::mapobjvis::{
    camera_inside_bias, camera_outside_bias, cmapobjgroup_normal_endpoint, nearest_param_delta,
    portal_plane_maxdist, segpt_interp, surface_plane_negate,
};
use common::{assert_f32, frand, frand3, next, read_f64, Lcg, SLICE_ESP};

/// A finite `f32` over seven ranges, two near half of `f32::MAX`, where `2x` overflows.
fn fuzz_bias(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 7;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,
        1 => (m * 20.0 - 10.0) as f32,
        2 => (m * 2000.0 - 1000.0) as f32,
        3 => (m * 1.0e6 - 5.0e5) as f32,
        4 => (m * 1.0e-6) as f32,
        5 => (m * 2.0 - 1.0) as f32 * 2.0e38, // straddle f32::MAX/2 (2·x overflow boundary)
        _ => (m * 2.0 - 1.0) as f32 * 1.7e38, // near f32::MAX/2 exactly
    }
}

/// Any `f32`, from random bits.
fn fuzz_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// A finite `f32` over six ranges, the last near ±1000.
fn fuzz_offsets(s: &mut u32) -> f32 {
    let pick = (next(s) >> 8) % 6;
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    match pick {
        0 => (m * 2.0 - 1.0) as f32,
        1 => (m * 20.0 - 10.0) as f32,
        2 => (m * 2000.0 - 1000.0) as f32,
        3 => (m * 1.0e6 - 5.0e5) as f32,
        4 => (m * 1.0e-6) as f32,
        _ => (m * 2.0e3 - 1.0e3) as f32, // near the ±1000/±1760 const magnitudes (cancellation)
    }
}

/// `0x6b3b20` from `0x6b3ca9` to `0x6b3cf7`: the rectangle at `esi` and `edi`; read from
/// `[ebp-0x20]` up.
#[test]
fn camera_inside_bias_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    // ESI → a 1-float input (x0 = [esi]); EDI → a buffer holding x1=[edi-4], x2=[edi], x3=[edi+4].
    let esi_buf = e.alloc(0x40);
    let edi_buf = e.alloc(0x40);
    let ebx_buf = e.alloc(0x200); // ebx read @6b3cad (`mov eax,[ebx+0x168]`) — needs +0x168 mapped
    let frame = e.alloc(0x100); // EBP frame: out0..out3 at [ebp-0x1c]/[ebp-0x18]/[ebp-0x14]/[ebp-0x10]
    let ebp = frame + 0x80;
    let edi = edi_buf + 0x10; // so [edi-4] stays in bounds

    let run_case = |e: &mut Emu, x: [f32; 4], ctx: &str| {
        e.write_u32(esi_buf, x[0].to_bits()); // [esi]   = x0
        e.write_u32(edi - 4, x[1].to_bits()); // [edi-4] = x1
        e.write_u32(edi, x[2].to_bits()); // [edi]   = x2
        e.write_u32(edi + 4, x[3].to_bits()); // [edi+4] = x3
                                              // junk-seed the out slots so a non-write would
                                              // diverge.
        for off in [0x1c, 0x18, 0x14, 0x10u64] {
            e.write_u32(ebp - off, 0xdead_beef);
        }
        e.set_reg(RegisterX86::ESI, esi_buf);
        e.set_reg(RegisterX86::EDI, edi);
        e.set_reg(RegisterX86::EBX, ebx_buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b3ca9, 0x6b3cf7);
        let bin = [
            e.read_f32(ebp - 0x1c), // out0 (LIVE f64)
            e.read_f32(ebp - 0x18), // out1 (LIVE f64)
            e.read_f32(ebp - 0x14), // out2 (f32-reloaded)
            e.read_f32(ebp - 0x10), // out3 (f32-reloaded)
        ];
        let ours = camera_inside_bias(x);
        for k in 0..4 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    // exact / ==threshold probes around the bias const and the doubling/overflow boundary.
    run_case(&mut e, [0.0, 0.0, 0.0, 0.0], "x=0 → -1.0 each");
    run_case(&mut e, [0.5, 0.5, 0.5, 0.5], "x=0.5 → 0.0 exact each");
    run_case(&mut e, [1.0, 1.0, 1.0, 1.0], "x=1 → 1.0 each");
    run_case(&mut e, [-0.5, -0.5, -0.5, -0.5], "x=-0.5 → -2.0 each");
    // overflow split: 2·x finite in f64 but ±inf in f32 (out0/out1 finite, out2/out3 inf).
    run_case(
        &mut e,
        [2.0e38, 2.0e38, 2.0e38, 2.0e38],
        "x=2e38 (2·x f32-overflow: LIVE finite vs reload inf)",
    );
    run_case(
        &mut e,
        [-2.0e38, -2.0e38, -2.0e38, -2.0e38],
        "x=-2e38 (overflow neg)",
    );
    run_case(
        &mut e,
        [f32::from_bits(0x7f00_0000); 4],
        "x just under f32::MAX/2",
    );

    let mut s = 0x696e_7369u32; // "insi"
    for i in 0..6000u32 {
        let x = [
            fuzz_bias(&mut s),
            fuzz_bias(&mut s),
            fuzz_bias(&mut s),
            fuzz_bias(&mut s),
        ];
        run_case(&mut e, x, &format!("inside fuzz[{i}]"));
    }
}

/// `0x6b3dd0` from `0x6b3e96` to `0x6b3eea`: the rectangle at `esi`; read from `[ebp-0x20]` up.
#[test]
fn camera_outside_bias_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let esi_buf = e.alloc(0x40); // ESI → the 4-float input ([esi]+0/+4/+8/+0xc)
    let ebx_buf = e.alloc(0x200); // ebx read inside (`mov ecx,[ebx+4]`@6b3e98 etc.)
    let frame = e.alloc(0x100); // out0..out3 at [ebp-0x20]/[ebp-0x1c]/[ebp-0x18]/[ebp-0x14]
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, x: [f32; 4], ctx: &str| {
        for k in 0..4u64 {
            e.write_u32(esi_buf + 4 * k, x[k as usize].to_bits());
        }
        for off in [0x20, 0x1c, 0x18, 0x14u64] {
            e.write_u32(ebp - off, 0xdead_beef);
        }
        e.set_reg(RegisterX86::ESI, esi_buf);
        e.set_reg(RegisterX86::EBX, ebx_buf);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b3e96, 0x6b3eea);
        let bin = [
            e.read_f32(ebp - 0x20), // out0 (LIVE f64)
            e.read_f32(ebp - 0x1c), // out1 (LIVE f64)
            e.read_f32(ebp - 0x18), // out2 (f32-reloaded)
            e.read_f32(ebp - 0x14), // out3 (f32-reloaded)
        ];
        let ours = camera_outside_bias(x);
        for k in 0..4 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    run_case(&mut e, [0.0, 0.0, 0.0, 0.0], "x=0 → -1.0 each");
    run_case(&mut e, [0.5, 0.5, 0.5, 0.5], "x=0.5 → 0.0 exact each");
    run_case(&mut e, [1.0, 1.0, 1.0, 1.0], "x=1 → 1.0 each");
    run_case(&mut e, [-0.5, -0.5, -0.5, -0.5], "x=-0.5 → -2.0 each");
    run_case(
        &mut e,
        [2.0e38, 2.0e38, 2.0e38, 2.0e38],
        "x=2e38 (2·x f32-overflow: LIVE finite vs reload inf)",
    );
    run_case(
        &mut e,
        [-2.0e38, -2.0e38, -2.0e38, -2.0e38],
        "x=-2e38 (overflow neg)",
    );
    run_case(
        &mut e,
        [f32::from_bits(0x7f00_0000); 4],
        "x just under f32::MAX/2",
    );

    let mut s = 0x6f75_7473u32; // "outs"
    for i in 0..6000u32 {
        let x = [
            fuzz_bias(&mut s),
            fuzz_bias(&mut s),
            fuzz_bias(&mut s),
            fuzz_bias(&mut s),
        ];
        run_case(&mut e, x, &format!("outside fuzz[{i}]"));
    }
}

/// `0x6b4680` whole: the plane written at `0xca7ef0`, the portal's vertices through the map
/// object's `+0x134`; the result is read from `st0`. Vertex counts from 0, distances of 0, ties and
/// NaN.
#[test]
fn portal_plane_maxdist_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);

    let this = emu.alloc(0x200);
    let mopv = emu.alloc(0x40 * 0xc + 0x10); // up to 0x40 verts, stride 0xc
    emu.write_u32(this + 0x134, mopv as u32);
    let pref = emu.alloc(0x10); // the u16[2] {start_idx, count}

    // The running max is the f32 RELOAD `fld dword [ebp+8]` (0x6b46da), not the live f64 the
    // `fcomp` uses — so a non-f32 `d` must re-round to f32. Each (plane, vert) pair below sums
    // multiple f32 products to a NON-f32 f64 `d` (count=1 ⇒ `d` IS the running max), where the
    // binary's f64 st0 = f64(f32(d)) but a `max = dval` (live f64) returns f64(d) — a divergence
    // the old `call_f32` truncation collapsed. `call_f64` lands it bit-exact.
    for (pl, v) in [
        ([0.1f32, 0.3, 0.7, 0.0], [1.0f32, 1.0, 1.0]), // d ≈ 1.1 (non-f32 sum)
        ([0.2f32, 0.2, 0.2, 0.1], [1.0f32, 1.0, 1.0]), // d ≈ 0.7 (non-f32 sum)
        ([0.1f32, 0.1, 0.1, 0.0], [3.0f32, 3.0, 3.0]), // d ≈ 0.9 (non-f32 sum)
        ([0.7f32, 0.0, 0.0, 0.7], [0.1f32, 0.0, 0.0]), // d ≈ 0.77 (0.7·0.1 non-f32, + 0.7)
    ] {
        emu.write_u32(0xca7ef0, pl[0].to_bits());
        emu.write_u32(0xca7ef4, pl[1].to_bits());
        emu.write_u32(0xca7ef8, pl[2].to_bits());
        emu.write_u32(0xca7efc, pl[3].to_bits());
        emu.write(pref, &0u16.to_le_bytes());
        emu.write(pref + 2, &1u16.to_le_bytes());
        emu.write_u32(mopv, v[0].to_bits());
        emu.write_u32(mopv + 4, v[1].to_bits());
        emu.write_u32(mopv + 8, v[2].to_bits());
        let binary = emu.call_f64(0x6b4680, this as u32, 0, &[pref as u32]);
        let ours = portal_plane_maxdist(&[v], pl);
        assert_eq!(
            binary.to_bits(),
            ours.to_bits(),
            "portal_plane_maxdist fst-no-pop probe pl={pl:?} v={v:?}: binary={binary:e} ours={ours:e}"
        );
    }

    // A tie, a zero and a NaN distance never replace the largest.
    let x_plane = [1.0f32, 0.0, 0.0, 0.0];
    for verts in [
        vec![[f32::NAN, 0.0, 0.0], [1.0, 0.0, 0.0], [f32::NAN, 0.0, 0.0]],
        vec![[1.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        vec![[0.0, 5.0, 0.0], [-0.0, 0.0, 0.0]],
        vec![[-1.0, 0.0, 0.0]],
    ] {
        for (a, c) in x_plane.iter().enumerate() {
            emu.write_u32(0xca7ef0 + 4 * a as u64, c.to_bits());
        }
        emu.write(pref, &0u16.to_le_bytes());
        emu.write(pref + 2, &(verts.len() as u16).to_le_bytes());
        for (k, v) in verts.iter().enumerate() {
            for (a, c) in v.iter().enumerate() {
                emu.write_u32(mopv + k as u64 * 0xc + a as u64 * 4, c.to_bits());
            }
        }
        let binary = emu.call_f64(0x6b4680, this as u32, 0, &[pref as u32]);
        let ours = portal_plane_maxdist(&verts, x_plane);
        assert_eq!(binary.to_bits(), ours.to_bits(), "{verts:?}");
    }

    let mut rng = Lcg(0x90_72_4d_70);
    let (mut zero_count, mut took, mut stayed_zero) = (0u32, 0u32, 0u32);
    for _ in 0..800u32 {
        // plane coeffs (write the runtime globals the binary reads).
        let plane = [rng.coord(), rng.coord(), rng.coord(), rng.coord()];
        emu.write_u32(0xca7ef0, plane[0].to_bits());
        emu.write_u32(0xca7ef4, plane[1].to_bits());
        emu.write_u32(0xca7ef8, plane[2].to_bits());
        emu.write_u32(0xca7efc, plane[3].to_bits());

        let count = (rng.next() >> 8) % 0x10; // 0..15 verts (incl. the count==0 path)
        let start = (rng.next() >> 8) % 0x20; // start index within the MOPV run
        emu.write(pref, &(start as u16).to_le_bytes());
        emu.write(pref + 2, &(count as u16).to_le_bytes());

        let mut verts: Vec<[f32; 3]> = Vec::new();
        for k in 0..count as u64 {
            let idx = start as u64 + k;
            let v = [rng.coord(), rng.coord(), rng.coord()];
            for (a, c) in v.iter().enumerate() {
                emu.write_u32(mopv + idx * 0xc + a as u64 * 4, c.to_bits());
            }
            verts.push(v);
        }

        // __thiscall: ecx = this, stack = pref. Returns the max in st0 — diff the f64 st0
        // bit-exact (`call_f64`), NOT a `call_f32` truncation that collapses the f32-reround.
        let binary = emu.call_f64(0x6b4680, this as u32, 0, &[pref as u32]);
        let ours = portal_plane_maxdist(&verts, plane);
        assert_eq!(
            binary.to_bits(),
            ours.to_bits(),
            "portal_plane_maxdist count={count} plane={plane:?}: binary={binary:e} ours={ours:e}"
        );

        if count == 0 {
            zero_count += 1;
        } else if ours > 0.0 {
            took += 1;
        } else {
            stayed_zero += 1;
        }
    }
    assert!(
        zero_count > 0 && took > 0 && stayed_zero > 0,
        "branches: count0={zero_count} took={took} stayed0={stayed_zero}"
    );
}

/// `0x6b5f90` from `0x6b6098` to `0x6b60f5`: one vertex and its normal; the end is read from
/// `[ebp-0x28]`.
#[test]
fn cmapobjgroup_normal_endpoint_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200); // EDI: this; [this+0xd4]=MOVN base, [this+0xd0]=MOVT base
    let movn = e.alloc(0x40); // normals[v]
    let movt = e.alloc(0x40); // positions[v]
    let out = e.alloc(0x40); // EAX: out-vertex buffer (the island writes endpoint x/y via eax+0x10)
    let frame = e.alloc(0x100); // EBP frame: endpoint at [ebp-0x28]/[ebp-0x24]/[ebp-0x20]
    let ebp = frame + 0x80;

    // [this+0xd4] = MOVN base, [this+0xd0] = MOVT base. esi = 0 (vertex 0 byte offset). The island
    // reads n at [movn+0], pos at [movt+0]. eax must be valid mapped mem (it is +=0x10'd then
    // written through); point eax so eax+0x10 stays in `out`.
    e.write_u32(this + 0xd4, movn as u32);
    e.write_u32(this + 0xd0, movt as u32);

    let run_case = |e: &mut Emu, pos: [f32; 3], n: [f32; 3], ctx: &str| {
        for k in 0..3u64 {
            e.write_u32(movn + 4 * k, n[k as usize].to_bits());
            e.write_u32(movt + 4 * k, pos[k as usize].to_bits());
        }
        // junk-seed the endpoint slots so a non-write would diverge.
        e.write_u32(ebp - 0x28, 0xdead_beef);
        e.write_u32(ebp - 0x24, 0xdead_beef);
        e.write_u32(ebp - 0x20, 0xdead_beef);
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::ESI, 0); // vertex 0 byte offset
        e.set_reg(RegisterX86::EAX, out); // out buffer; eax+0x10 written through, in-bounds
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b6098, 0x6b60f5);
        let bin = [
            e.read_f32(ebp - 0x28), // X (pos.x + n.x*0.75 LIVE f64)
            e.read_f32(ebp - 0x24), // Y (pos.y + n.y*0.75 LIVE f64)
            e.read_f32(ebp - 0x20), // Z (pos.z + n.z*0.75 reloaded f32)
        ];
        let ours = cmapobjgroup_normal_endpoint(pos, n);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} endpoint[{k}]"));
        }
    };

    // exact / ==threshold probes — incl. unit normals (n*0.75 exact), zero, and a case where the
    // LIVE-f64 (X/Y) vs f32-reloaded (Z) products differ in the low bits.
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        "n=1 → (0.75,0.75,0.75)",
    );
    run_case(
        &mut e,
        [10.0, 20.0, 30.0],
        [0.0, 0.0, 0.0],
        "n=0 → pos unchanged",
    );
    run_case(
        &mut e,
        [1.0, 2.0, 3.0],
        [1.0, 1.0, 1.0],
        "(1,2,3)+0.75 each",
    );
    run_case(
        &mut e,
        [-5.0, -5.0, -5.0],
        [-1.0, -1.0, -1.0],
        "neg → (-5.75,-5.75,-5.75)",
    );
    run_case(&mut e, [0.0, 0.0, 0.0], [4.0, 4.0, 4.0], "n=4 → 3.0 exact");
    // a normal whose ·0.75 has a long binary expansion so f32-round (Z) ≠ f64-live (X/Y) low bits.
    run_case(
        &mut e,
        [1.0e7, 1.0e7, 1.0e7],
        [
            f32::from_bits(0x3fab_cdef),
            f32::from_bits(0x3fab_cdef),
            f32::from_bits(0x3fab_cdef),
        ],
        "live-vs-reload low-bit straddle (X/Y vs Z)",
    );

    let mut s = 0x6e6f_726du32; // "norm"
    for i in 0..6000u32 {
        let pos = [
            fuzz_offsets(&mut s),
            fuzz_offsets(&mut s),
            fuzz_offsets(&mut s),
        ];
        let n = [
            fuzz_offsets(&mut s),
            fuzz_offsets(&mut s),
            fuzz_offsets(&mut s),
        ];
        run_case(&mut e, pos, n, &format!("normal fuzz[{i}]"));
    }
}

/// `0x6a2600` from `0x6a2652` to `0x6a26a4`: the ends at `esi` and `ebx`, `t` through `[ebp+0x1c]`;
/// the point is read through `[ebp+0x20]`.
#[test]
fn segpt_interp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let p0m = e.alloc(0x20); // esi: p0 (3 f32)
    let p1m = e.alloc(0x20); // ebx: p1 (3 f32)
    let tm = e.alloc(0x10); // the t scalar (pointed-to by [ebp+0x1c])
    let outm = e.alloc(0x20); // the out slot ([ebp+0x18])
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, p0: [f32; 3], p1: [f32; 3], t: f32, ctx: &str| {
        for k in 0..3 {
            e.write_u32(p0m + 4 * k as u64, p0[k].to_bits());
            e.write_u32(p1m + 4 * k as u64, p1[k].to_bits());
        }
        e.write_u32(tm, t.to_bits());
        e.write_u32(ebp + 0x1c, tm as u32); // [ebp+0x1c] = &t (binary reloads t = *[ebp+0x1c])
        e.write_u32(ebp + 0x18, outm as u32); // [ebp+0x18] = out ptr
        e.set_reg(RegisterX86::ESI, p0m);
        e.set_reg(RegisterX86::EBX, p1m);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a2652, 0x6a26a4);
        let bin = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let ours = segpt_interp(p0, p1, t);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} out[{k}]"));
        }
    };

    // probes: t=0 (out=p0), t=1 (out=p1), t=0.5 (midpoint), equal endpoints, zeros.
    run_case(&mut e, [1.0, 2.0, 3.0], [4.0, 5.0, 6.0], 0.0, "interp t=0");
    run_case(&mut e, [1.0, 2.0, 3.0], [4.0, 5.0, 6.0], 1.0, "interp t=1");
    run_case(
        &mut e,
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0],
        0.5,
        "interp t=0.5",
    );
    run_case(
        &mut e,
        [7.0, 8.0, 9.0],
        [7.0, 8.0, 9.0],
        0.5,
        "interp equal",
    );
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        0.5,
        "interp zeros",
    );
    run_case(
        &mut e,
        [-3.0, -4.0, -5.0],
        [6.0, 7.0, 8.0],
        0.25,
        "interp t=0.25",
    );

    let mut s = 0x6a26_0000u32;
    for i in 0..4000u32 {
        let p0 = frand3(&mut s, -500.0, 500.0);
        let p1 = frand3(&mut s, -500.0, 500.0);
        let t = frand(&mut s, 0.0, 1.0);
        run_case(&mut e, p0, p1, t, &format!("interp fuzz[{i}]"));
    }
    // a few out-of-[0,1] t (the callee can return t outside the unit interval at grazing hits).
    let mut s = 0x6a26_2222u32;
    for i in 0..1000u32 {
        let p0 = frand3(&mut s, -1000.0, 1000.0);
        let p1 = frand3(&mut s, -1000.0, 1000.0);
        let t = frand(&mut s, -2.0, 3.0);
        run_case(&mut e, p0, p1, t, &format!("interp wide-t fuzz[{i}]"));
    }
}

/// `0x6aa1f0` from `0x6aa3a1` to `0x6aa3a6` in an island after a load: the offset negated and
/// stored at `ebx+0xc`.
#[test]
fn surface_plane_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // The copied fchs bytes are the exe's.
    const FCHS_VA: u64 = 0x006a_a3a1;
    let fchs = e.read(FCHS_VA, 2); // fchs (d9 e0); fstp [ebx+0xc] (d9 5b 0c) follows — we keep fchs only
    assert_eq!(
        &fchs[..2],
        &[0xd9, 0xe0],
        "surface_plane_negate island: fchs bytes @0x6aa3a1 changed — re-derive"
    );

    let in_slot = e.alloc(0x10);
    let out_slot = e.alloc(0x10);
    let island = e.alloc(0x40);

    // island: fld dword [in_slot] ; fchs (the exe's) ; fstp dword [out_slot]
    let mut code: Vec<u8> = Vec::new();
    code.extend_from_slice(&[0xd9, 0x05]); // fld dword [abs32]
    code.extend_from_slice(&(in_slot as u32).to_le_bytes());
    code.extend_from_slice(&[0xd9, 0xe0]); // the verified fchs
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [abs32]
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    e.write(island, &code);
    let island_end = island + code.len() as u64;

    let run_case = |e: &mut Emu, d: f32, ctx: &str| {
        e.write_u32(in_slot, d.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);
        e.run(island, island_end);
        let bin = e.read_f32(out_slot);
        let ours = surface_plane_negate(d);
        assert_f32(bin, ours, ctx);
    };

    // boundary probes: ±0, ±1, exact.
    run_case(&mut e, 0.0, "splane d=0");
    run_case(&mut e, -0.0, "splane d=-0");
    run_case(&mut e, 1.0, "splane d=1");
    run_case(&mut e, -1.0, "splane d=-1");
    run_case(&mut e, 250.5, "splane d=250.5");

    let mut s = 0x6aa3_a1a1u32;
    for i in 0..3000u32 {
        run_case(
            &mut e,
            frand(&mut s, -1.0e6, 1.0e6),
            &format!("splane fuzz[{i}]"),
        );
    }
    let mut sb = 0x2468_ace0u32;
    for i in 0..3000u32 {
        run_case(&mut e, fuzz_bits(&mut sb), &format!("splane bits[{i}]"));
    }
}

/// `0x6be250` from `0x6be421` to `0x6be429` in an island with a store appended: the candidate at
/// `[ebp-0xc]` less the best through `ecx`, read unrounded.
#[test]
fn nearest_param_delta_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // 0x6be421..0x6be429 (8 bytes): fld [ebp-0xc]; mov ecx,[ebp+8]; fsub [ecx], copied.
    const ISL_VA: u64 = 0x006b_e421;
    const ISL_LEN: usize = 0x6be429 - 0x6be421; // 8
    let bytes = e.read(ISL_VA, ISL_LEN);
    assert_eq!(
        bytes,
        vec![0xd9, 0x45, 0xf4, 0x8b, 0x4d, 0x08, 0xd8, 0x21],
        "nearest_param_delta island: bytes @0x6be421 changed — re-derive"
    );

    let best = e.alloc(0x10); // [ebp+8] → param_3 (ecx); fsub reads [ecx] = best
    let out = e.alloc(0x10); // f64 capture slot
    let island = e.alloc(0x40);

    // island: <8 copied bytes> ; fstp qword [out]  (DD /3, disp32 absolute)
    let mut code = bytes;
    code.extend_from_slice(&[0xdd, 0x1d]);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(island, &code);
    let island_end = island + code.len() as u64;

    // ebp must be valid: the island reads [ebp-0xc] (cand) and [ebp+8] (the best ptr).
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, cand: f32, best_v: f32, ctx: &str| {
        e.write_u32(ebp - 0xc, cand.to_bits()); // local_10 = [ebp-0xc]
        e.write_u32(ebp + 8, best as u32); // [ebp+8] → param_3
        e.write_u32(best, best_v.to_bits()); // *param_3 = best
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(island, island_end);
        let bin = read_f64(e, out);
        let ours = nearest_param_delta(cand, best_v);
        // f64 bit-exact: the binary's LIVE st0 value (the un-rounded f64 the fcomp consumes).
        if bin.is_nan() && ours.is_nan() {
            // ok
        } else {
            assert_eq!(
                bin.to_bits(),
                ours.to_bits(),
                "{ctx}: cand={cand} best={best_v} binary={:#018x} ours={:#018x}",
                bin.to_bits(),
                ours.to_bits()
            );
        }
    };

    // boundary probes: equal (delta==0, the `< eps` boundary), cand<best, cand>best, the 1.05
    // const.
    run_case(&mut e, 5.0, 5.0, "delta equal");
    run_case(&mut e, 5.0, 7.0, "delta neg");
    run_case(&mut e, 7.0, 5.0, "delta pos");
    run_case(&mut e, 1.05, 1.0, "delta 1.05-1");
    run_case(&mut e, 0.0, 0.0, "delta 0-0");
    run_case(&mut e, f32::from_bits(0x3f866666), 1.0, "delta const-1"); // 0x3f866666 = the binary's seed

    let mut s = 0x6be4_2121u32;
    for i in 0..3000u32 {
        let cand = frand(&mut s, -1.0e4, 1.0e4);
        let best_v = frand(&mut s, -1.0e4, 1.0e4);
        run_case(&mut e, cand, best_v, &format!("delta fuzz[{i}]"));
    }
    // raw-bit fuzz (negatives/denormals/inf); NaN handled above.
    let mut sb = 0x0f0f_e1e1u32;
    for i in 0..3000u32 {
        let cand = fuzz_bits(&mut sb);
        let best_v = fuzz_bits(&mut sb);
        run_case(&mut e, cand, best_v, &format!("delta bits[{i}]"));
    }
}
