//! A model instance (`systems/models/model.md`): `0x710a80` called whole, the rest as slices of
//! their functions with the frame and records they read written and the stores read back.

#![allow(clippy::needless_range_loop)] // index loops follow the `+off + 4·k` layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::model::{
    attach_fold_712de0, attach_world_transform, bbox_center, billboard_tangent_frame,
    m2_orthonormal_basis, part_span_block1, part_span_block2, part_span_block3, part_span_block4,
    query_attach_event, shadow_normalize_axes, shadow_transform_normal, spline_complement,
    spline_param_fdiv, spline_param_fidiv, Axis3, PlaneEq,
};
use common::{
    assert_f32, assert_f64, fuzz_f32, fuzz_i32, fuzz_scales, island_bytes_match, Lcg, SLICE_ESP,
};

/// A random bone matrix and offset.
fn gen_inputs(rng: &mut Lcg) -> ([f32; 16], [f32; 3]) {
    let mut m = [0f32; 16];
    for f in m.iter_mut() {
        *f = rng.f32_mixed();
    }
    let v = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
    (m, v)
}

/// `0x712de0` from `0x712e49` to the call at `0x712eac`: the bone matrix at `[ebp-0x40]`, the
/// attachment at `eax` (`+0x8`); the translation read from `[ebp-0x10]`.
#[test]
fn attach_fold_712de0_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100); // EBP frame; B lives at [ebp-0x40]
    let ebp = frame + 0x80;
    let attach = e.alloc(0x20); // EAX → attach descriptor; vec at +0x8/0xc/0x10
    let this = e.alloc(0x40); // EBX → CM2Model `this`: the slice does `mov ecx,[ebx+0x2c]` (a
                              // matmul-arg pointer, consumed only by the halted-before call) — EBX
                              // must point somewhere mapped so that read succeeds; its value is
                              // irrelevant.

    let run_case = |e: &mut Emu, m: [f32; 16], v: [f32; 3], ctx: &str| {
        // load B (bone matrix) directly into the scratch buffer at [ebp-0x40].
        for (i, &f) in m.iter().enumerate() {
            e.write_u32(ebp - 0x40 + 4 * i as u64, f.to_bits());
        }
        // attach vector v0/v1/v2 at [EAX+0x8/0xc/0x10].
        e.write_u32(attach + 0x8, v[0].to_bits());
        e.write_u32(attach + 0xc, v[1].to_bits());
        e.write_u32(attach + 0x10, v[2].to_bits());
        e.set_reg(RegisterX86::EAX, attach);
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x712e49, 0x712eac); // fold rows 0..2, halt at `call 0x74a7c0`

        let ours = attach_fold_712de0(m, v);
        // B[12]=[ebp-0x10], B[13]=[ebp-0xc], B[14]=[ebp-0x8].
        assert_f32(
            e.read_f32(ebp - 0x10),
            ours[12],
            &format!("{ctx} row0 (B[12])"),
        );
        assert_f32(
            e.read_f32(ebp - 0xc),
            ours[13],
            &format!("{ctx} row1 (B[13])"),
        );
        assert_f32(
            e.read_f32(ebp - 0x8),
            ours[14],
            &format!("{ctx} row2 (B[14])"),
        );
    };

    // ==threshold / edge probes: identity rotation, zero vector, zero rotation, sign mix.
    let mut ident = [0f32; 16];
    ident[0] = 1.0;
    ident[5] = 1.0;
    ident[10] = 1.0;
    ident[15] = 1.0;
    ident[12] = 7.0;
    ident[13] = -3.0;
    ident[14] = 11.0;
    run_case(&mut e, ident, [2.0, 5.0, -4.0], "identity+trans");
    run_case(&mut e, ident, [0.0, 0.0, 0.0], "identity zero-vec");
    run_case(&mut e, [0f32; 16], [1.0, 2.0, 3.0], "zero matrix");

    let mut rng = Lcg(0x712d_e000);
    for i in 0..4000u32 {
        let (m, v) = gen_inputs(&mut rng);
        run_case(&mut e, m, v, &format!("712de0 fuzz[{i}]"));
    }
}

/// `0x711a20` from `0x711ab6` to the call at `0x711b17`: the instance at `esi`, the record at
/// `eax`; the scale read from `ebx+0x8`, the translation from the three arguments pushed.
#[test]
fn attach_world_transform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x100); // ESI → CM2Model: bone column at +0xbc/+0xc0/+0xc4
    let record = e.alloc(0x40); // EAX → record base; slice adds 0x24 (rec[0xc] scale, rec[0x24..0x38])
    let out = e.alloc(0x40); // EBX → param_4: scale at +0x8; rep-movsd dest at +0xc
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    #[allow(clippy::too_many_arguments)]
    let run_case = |e: &mut Emu,
                    c0: f32,
                    c1: f32,
                    c2: f32,
                    rec_scale: f32,
                    rec24: f32,
                    rec28: f32,
                    rec2c: f32,
                    rec30: f32,
                    rec34: f32,
                    rec38: f32,
                    ctx: &str| {
        // bone column at this+0xc0/0xc4/0xbc = c0/c1/c2 (the three squared floats).
        e.write_u32(this + 0xc0, c0.to_bits());
        e.write_u32(this + 0xc4, c1.to_bits());
        e.write_u32(this + 0xbc, c2.to_bits());
        // record fields: scale at +0xc; six translate floats at +0x24..+0x38.
        e.write_u32(record + 0xc, rec_scale.to_bits());
        e.write_u32(record + 0x24, rec24.to_bits());
        e.write_u32(record + 0x28, rec28.to_bits());
        e.write_u32(record + 0x2c, rec2c.to_bits());
        e.write_u32(record + 0x30, rec30.to_bits());
        e.write_u32(record + 0x34, rec34.to_bits());
        e.write_u32(record + 0x38, rec38.to_bits());
        // junk-seed the scale output slot so a non-write would diverge.
        e.write_u32(out + 0x8, 0xdead_beef);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EAX, record);
        e.set_reg(RegisterX86::EBX, out);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP); // in-slice `sub esp,0xc` (711ad9) frees the arg space
        e.run(0x711ab6, 0x711b17); // length-sqrt + scale fmul + 3 translate fadds; halt at call 0x4549a0

        let bin_scale = e.read_f32(out + 0x8);
        let bin_x = e.read_f32(SLICE_ESP - 0xc); // [esp]
        let bin_y = e.read_f32(SLICE_ESP - 0x8); // [esp+4]
        let bin_z = e.read_f32(SLICE_ESP - 0x4); // [esp+8]

        let ours = attach_world_transform(
            c0, c1, c2, rec_scale, rec24, rec28, rec2c, rec30, rec34, rec38,
        );
        assert_f32(bin_scale, ours.scale, &format!("{ctx} scale"));
        assert_f32(bin_x, ours.off_x, &format!("{ctx} off_x"));
        assert_f32(bin_y, ours.off_y, &format!("{ctx} off_y"));
        assert_f32(bin_z, ours.off_z, &format!("{ctx} off_z"));
    };

    // ==threshold / edge probes: zero column (len 0 → scale 0), unit column, negative components,
    // zero record scale, large column (sqrt of big sum).
    run_case(
        &mut e, 0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, "zero col",
    );
    run_case(
        &mut e,
        1.0,
        0.0,
        0.0,
        2.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        "unit col x",
    );
    run_case(
        &mut e,
        3.0,
        -4.0,
        0.0,
        1.0,
        -1.0,
        -2.0,
        -3.0,
        1.0,
        2.0,
        3.0,
        "3-4-5 col",
    );
    run_case(
        &mut e,
        1.0,
        2.0,
        2.0,
        0.0,
        7.0,
        8.0,
        9.0,
        10.0,
        11.0,
        12.0,
        "zero rec_scale",
    );
    run_case(
        &mut e, 1.0e3, 1.0e3, 1.0e3, 0.001, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, "big col",
    );

    let mut rng = Lcg(0x711a_2000);
    for i in 0..5000u32 {
        let c0 = rng.f32_mixed();
        let c1 = rng.f32_mixed();
        let c2 = rng.f32_mixed();
        let rec_scale = rng.f32_mixed();
        let r = [
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
        ];
        run_case(
            &mut e,
            c0,
            c1,
            c2,
            rec_scale,
            r[0],
            r[1],
            r[2],
            r[3],
            r[4],
            r[5],
            &format!("xform fuzz[{i}]"),
        );
    }
}

/// `0x7106c0` from `0x710769` to `0x7108a9`, with both calls of `0x4549f0` run: the normal at `ebx`
/// (`+0xbc`), the vector through `[ebp+0x18]`; the rows read from `+0xbc`, `+0xcc` and `+0xdc`. A
/// length of exactly 2.3841858e-7, one step under it and NaN probe the normalize.
#[test]
fn billboard_tangent_frame_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let model = emu.alloc(0x200); // ecx/edi = model; dir @ +0xbc, up @ +0xcc, fwd @ +0xdc
    let frame = emu.alloc(0x200);
    let ebp = frame + 0x100;
    let p6 = emu.alloc(0x10); // [ebp+0x18] → param_6 vec3

    let mut rng = Lcg(0x6231_316f); // "b11o"

    let run_case = |emu: &mut Emu, dir: [f32; 3], param_6: [f32; 3], ctx: &str| {
        // dir at model+0xbc/0xc0/0xc4 (= [ebx]/[ebx+4]/[ebx+8]).
        for i in 0..3 {
            emu.write_u32(model + 0xbc + 4 * i as u64, dir[i].to_bits());
            emu.write_u32(p6 + 4 * i as u64, param_6[i].to_bits());
        }
        emu.set_reg(RegisterX86::EBX, model + 0xbc); // ebx = model+0xbc
        emu.set_reg(RegisterX86::ESI, model + 0xcc); // esi = model+0xcc
        emu.set_reg(RegisterX86::EDI, model); // edi = model
        emu.set_reg(RegisterX86::ESP, SLICE_ESP);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.write_u32(ebp + 0x18, p6 as u32); // [ebp+0x18] = &param_6
        emu.run(0x710769, 0x7108a9); // tangent build; halt at `mov eax,[edi+0x10]`
        let up = [
            emu.read_f32(model + 0xcc),
            emu.read_f32(model + 0xd0),
            emu.read_f32(model + 0xd4),
        ];
        let right = [
            emu.read_f32(model + 0xbc),
            emu.read_f32(model + 0xc0),
            emu.read_f32(model + 0xc4),
        ];
        let fwd = [
            emu.read_f32(model + 0xdc),
            emu.read_f32(model + 0xe0),
            emu.read_f32(model + 0xe4),
        ];
        let ours = billboard_tangent_frame(dir, param_6);
        for k in 0..3 {
            assert_f32(up[k], ours.up[k], &format!("{ctx} up[{k}]"));
            assert_f32(right[k], ours.right[k], &format!("{ctx} right[{k}]"));
            assert_f32(fwd[k], ours.fwd[k], &format!("{ctx} fwd[{k}]"));
        }
    };

    // ==threshold / degenerate probes (normalize-eps gate both ways).
    run_case(&mut emu, [0.0, 0.0, 0.0], [1.0, 0.0, 0.0], "tf zero-dir");
    run_case(
        &mut emu,
        [1.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        "tf dir==p6 (cross≈0)",
    );
    run_case(&mut emu, [0.0, 1.0, 0.0], [0.0, 1.0, 0.0], "tf dir==p6 y");
    run_case(&mut emu, [1.0, 2.0, 3.0], [4.0, 5.0, 6.0], "tf generic");
    run_case(
        &mut emu,
        [1e-7, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        "tf tiny-dir (up-seed below eps)",
    );
    let eps = f32::from_bits(0x3480_0000);
    let below = f32::from_bits(0x3480_0000 - 1);
    run_case(
        &mut emu,
        [eps, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        "up of length eps",
    );
    run_case(
        &mut emu,
        [below, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        "up just under eps",
    );
    run_case(&mut emu, [f32::NAN, 1.0, 0.0], [0.0, 0.0, 1.0], "NaN x");
    run_case(
        &mut emu,
        [1.0, 2.0, 3.0],
        [f32::NAN, 0.0, 1.0],
        "NaN vector",
    );

    for i in 0..4000u32 {
        // ¼ degenerate (dir ∥ p6 → some cross collapses), ¾ generic.
        let dir = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        let param_6 = if (rng.next() >> 8).is_multiple_of(4) {
            dir
        } else {
            [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()]
        };
        run_case(&mut emu, dir, param_6, &format!("tf fuzz[{i}]"));
    }
}

/// `0x7106c0` from `0x710968` to `0x710974` in an island with `fstp qword` appended: the count at
/// `[ebp-0x8]`, the divisor at `[ebp-0x1c]`.
#[test]
fn spline_param_fdiv_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x710968;
    const LEN: usize = 0x710974 - 0x710968; // 0xc = 12 (through+incl. the fdiv)
    let mut code = e.read(SRC, LEN);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    let end = isl + code.len() as u64;

    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, num: u32, denom: f32, ctx: &str| {
        e.write_u32(ebp - 0x8, num); // [ebp-0x8] = numerator low dword
        e.write_u32(ebp - 0x4, 0); // [ebp-0x4] = 0 (high dword, as the binary forces)
        e.write_u32(ebp - 0x1c, denom.to_bits()); // [ebp-0x1c] = f32 denom
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let mut b = [0u8; 8];
        b.copy_from_slice(&e.read(out, 8));
        let bin = f64::from_bits(u64::from_le_bytes(b));
        let ours = spline_param_fdiv(i64::from(num), denom);
        assert_f64(bin, ours, ctx);
    };

    // probes: denom==0 (+Inf), exact reciprocals, big numerator.
    run_case(&mut e, 0, 1.0, "fdiv 0/1");
    run_case(&mut e, 10, 0.0, "fdiv 10/0 (+Inf)");
    run_case(&mut e, 1, 4.0, "fdiv 1/4");
    run_case(&mut e, 7, 2.0, "fdiv 7/2");
    run_case(&mut e, 0xffff_ffff, 3.0, "fdiv big/3");

    let mut rng = Lcg(0x6664_6976); // "fdiv"
    for i in 0..3000u32 {
        let num = rng.next() % 1_000_000;
        let denom = rng.f32_mixed();
        run_case(&mut e, num, denom, &format!("fdiv fuzz[{i}]"));
    }
}

/// `0x7106c0` from `0x71098b` to `0x710991` in an island with `fstp qword` appended.
#[test]
fn spline_param_fidiv_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x71098b;
    const LEN: usize = 0x710991 - 0x71098b; // 6 (fild dword + fidiv dword)
    let mut code = e.read(SRC, LEN);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    let end = isl + code.len() as u64;

    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, iquot: i32, istep: i32, ctx: &str| {
        e.write_u32(ebp + 0x10, iquot as u32); // [ebp+0x10] = quotient (dword int)
        e.write_u32(ebp + 0xc, istep as u32); // [ebp+0xc] = step (dword int divisor)
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let mut b = [0u8; 8];
        b.copy_from_slice(&e.read(out, 8));
        let bin = f64::from_bits(u64::from_le_bytes(b));
        let ours = spline_param_fidiv(iquot, istep);
        assert_f64(bin, ours, ctx);
    };

    // probes: step==0 (Inf/NaN), exact divides, negatives.
    run_case(&mut e, 0, 1, "fidiv 0/1");
    run_case(&mut e, 5, 0, "fidiv 5/0");
    run_case(&mut e, 10, 4, "fidiv 10/4");
    run_case(&mut e, 8, 2, "fidiv 8/2");
    run_case(&mut e, -7, 3, "fidiv -7/3");

    let mut rng = Lcg(0x6669_6476); // "fidv"
    for i in 0..3000u32 {
        let iquot = (rng.next() % 200_000) as i32 - 100_000;
        let istep = ((rng.next() % 64) as i32) + 1; // physical step counts, non-zero
        run_case(&mut e, iquot, istep, &format!("fidiv fuzz[{i}]"));
    }
}

/// `0x7106c0` from `0x7109db` to `0x7109e4` in an island with `fstp dword` appended; the copy at
/// `0x710a05` is checked to be the same bytes.
#[test]
fn spline_complement_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x7109db;
    const LEN: usize = 0x7109e4 - 0x7109db; // 9 (fld [1.0] + fsub [ebp+8])
    let mut code = e.read(SRC, LEN);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    // byte-verify the clone-family twin at 0x710a05 is the identical instruction prefix.
    assert_eq!(
        e.read(0x710a05, LEN),
        e.read(SRC, LEN),
        "clone twin 0x710a05 differs from 0x7109db"
    );
    let end = isl + code.len() as u64;

    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, param: f32, ctx: &str| {
        e.write_u32(ebp + 8, param.to_bits()); // [ebp+8] = param
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let bin = e.read_f32(out);
        let ours = spline_complement(param);
        assert_f32(bin, ours, ctx);
    };

    // probes: param=0 (→1.0), 1.0 (→0.0), 0.5, negatives, the param band [0,1].
    run_case(&mut e, 0.0, "compl 0");
    run_case(&mut e, 1.0, "compl 1");
    run_case(&mut e, 0.5, "compl 0.5");
    run_case(&mut e, -1.0, "compl -1");

    let mut rng = Lcg(0x636f_6d70); // "comp"
    for i in 0..3000u32 {
        let param = rng.f32_mixed();
        run_case(&mut e, param, &format!("compl fuzz[{i}]"));
    }
}

/// `0x710a80` whole, with the vectors apart and inside one block: the stores through `edx` read
/// back. Parallel vectors, a cross product of length exactly 2.3841858e-7 and NaN probe the
/// normalize.
#[test]
fn m2_orthonormal_basis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut rng = Lcg(0x6f72_7468); // "orth"

    // Two persistent layouts, allocated once.
    // L1: fwd and frame in separate small allocations (site-1 style — disjoint regions).
    let l1_fwd = emu.alloc(0x10);
    let l1_frame = emu.alloc(0x30); // ref_up(+0)/right(+0x10)/fwd-copy(+0x20)
                                    // L2: a larger block; the frame struct lives at +0x10 inside it
                                    // (site-2 stack-scratch style), and fwd at the block head, so
                                    // the surrounding bytes differ from L1.
    let l2_block = emu.alloc(0x60);
    let l2_fwd = l2_block; // fwd vec3 at the head
    let l2_frame = l2_block + 0x10; // frame struct offset into the block

    // case kinds: 0 = ordinary fuzzed fwd/ref_up; 1 = degenerate (ref_up == fwd → cross ≈ 0 →
    // skip).
    for layout in 0..2u32 {
        for kind in 0..2u32 {
            for _ in 0..150 {
                let fwd = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
                let ref_up = if kind == 1 {
                    fwd // parallel → cross ≈ 0 → len < ε → normalize skipped
                } else {
                    [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()]
                };

                let (fwd_ptr, frame_ptr) = if layout == 0 {
                    (l1_fwd, l1_frame)
                } else {
                    // dirty the L2 block's surrounding bytes so a stray read would diverge.
                    for k in 0..0x60u64 / 4 {
                        emu.write_u32(l2_block + 4 * k, rng.next());
                    }
                    (l2_fwd, l2_frame)
                };

                // write fwd (ecx) and ref_up (edx head).
                for i in 0..3 {
                    emu.write_u32(fwd_ptr + 4 * i as u64, fwd[i].to_bits());
                    emu.write_u32(frame_ptr + 4 * i as u64, ref_up[i].to_bits());
                }

                // __fastcall ecx=fwd, edx=frame, no stack args.
                let _ = emu.call(0x710a80, fwd_ptr as u32, frame_ptr as u32, &[]);
                // 0x710a80 returns with `ret` (no stack-arg cleanup) — call() halts on the
                // sentinel.
                let _ = emu.get_reg(RegisterX86::EAX);

                let ours = m2_orthonormal_basis(fwd, ref_up);

                for i in 0..3 {
                    assert_f32(
                        emu.read_f32(frame_ptr + 4 * i as u64),
                        ours.up[i],
                        &format!("m2_orthonormal_basis up[{i}] (layout={layout} kind={kind})"),
                    );
                    assert_f32(
                        emu.read_f32(frame_ptr + 0x10 + 4 * i as u64),
                        ours.right[i],
                        &format!("m2_orthonormal_basis right[{i}] (layout={layout} kind={kind})"),
                    );
                    assert_f32(
                        emu.read_f32(frame_ptr + 0x20 + 4 * i as u64),
                        ours.fwd[i],
                        &format!("m2_orthonormal_basis fwd[{i}] (layout={layout} kind={kind})"),
                    );
                }
            }
        }
    }

    let eps = f32::from_bits(0x3480_0000);
    for (fwd, ref_up, ctx) in [
        ([1.0, 0.0, 0.0], [0.0, 0.0, eps], "cross of length eps"),
        (
            [1.0, 0.0, 0.0],
            [0.0, 0.0, f32::from_bits(0x3480_0000 - 1)],
            "just under eps",
        ),
        ([f32::NAN, 1.0, 0.0], [0.0, 0.0, 1.0], "NaN"),
        ([1.0, 0.0, 0.0], [0.0, f32::INFINITY, 1.0], "infinite"),
    ] {
        for i in 0..3 {
            emu.write_u32(l1_fwd + 4 * i as u64, fwd[i].to_bits());
            emu.write_u32(l1_frame + 4 * i as u64, ref_up[i].to_bits());
        }
        let _ = emu.call(0x710a80, l1_fwd as u32, l1_frame as u32, &[]);
        let ours = m2_orthonormal_basis(fwd, ref_up);
        for i in 0..3 {
            let o = 4 * i as u64;
            assert_f32(
                emu.read_f32(l1_frame + o),
                ours.up[i],
                &format!("{ctx} up[{i}]"),
            );
            let got = emu.read_f32(l1_frame + 0x10 + o);
            assert_f32(got, ours.right[i], &format!("{ctx} right[{i}]"));
        }
    }
}

/// `0x7137c0` from `0x713996` to `0x713a4a`, with the three calls of `0x4549f0` run: the rows at
/// `[ebp-0x44]` normalized in place. Lengths at, under and above 2.3841858e-7 and NaN probe the
/// gate.
#[test]
fn shadow_normalize_axes_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    // the 9 axis-float slots: axis0 @ [ebp-0x44..], axis1 @ [ebp-0x38..], axis2 @ [ebp-0x2c..].
    let slot = |i: usize| -> u64 {
        let off = [0x44, 0x40, 0x3c, 0x38, 0x34, 0x30, 0x2c, 0x28, 0x24][i];
        ebp - off
    };

    let run_case = |e: &mut Emu, a0: [f32; 3], a1: [f32; 3], a2: [f32; 3], ctx: &str| {
        let ins = [
            a0[0], a0[1], a0[2], a1[0], a1[1], a1[2], a2[0], a2[1], a2[2],
        ];
        for (i, &v) in ins.iter().enumerate() {
            e.write_u32(slot(i), v.to_bits());
        }
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x713996, 0x713a4a); // lea ecx; call 0x4549f0; fsqrt..fdivr..fmul×3  ×3 axes
        let bin: Vec<f32> = (0..9).map(|i| e.read_f32(slot(i))).collect();
        let ours = shadow_normalize_axes(a0, a1, a2);
        let flat = |ax: &Axis3| [ax.x, ax.y, ax.z];
        let o = [flat(&ours[0]), flat(&ours[1]), flat(&ours[2])].concat();
        for i in 0..9 {
            assert_f32(bin[i], o[i], &format!("{ctx} [{i}]"));
        }
    };

    // ε-gate probes: a degenerate near-zero axis (len < ε → skipped, raw survives), an exact unit,
    // a zero axis, and ordinary fuzzed axes.
    let tiny = f32::from_bits(0x0000_0001); // subnormal → len far below ε
    run_case(
        &mut e,
        [tiny, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        "norm ε-gate (tiny/zero/unit)",
    );
    run_case(
        &mut e,
        [3.0, 4.0, 0.0],
        [1.0, 2.0, 2.0],
        [0.0, 0.0, 5.0],
        "norm exact (3-4-0 etc)",
    );
    let eps = f32::from_bits(0x3480_0000);
    run_case(
        &mut e,
        [eps, 0.0, 0.0],
        [f32::from_bits(0x3480_0000 - 1), 0.0, 0.0],
        [f32::from_bits(0x3480_0001), 0.0, 0.0],
        "eps, just under, just above",
    );
    run_case(
        &mut e,
        [f32::NAN, 1.0, 0.0],
        [1.0, f32::NAN, 0.0],
        [f32::INFINITY, 1.0, 0.0],
        "NaN and infinite",
    );

    let mut rng = Lcg(0x7368_646e); // "shdn"
    for i in 0..3000u32 {
        let mut ax = || {
            // mix of unit-ish, large, and near-degenerate magnitudes.
            let k = rng.next() & 3;
            let s = match k {
                0 => 1.0,
                1 => 1000.0,
                2 => 1.0e-4,
                _ => 1.0e-9, // straddles ε → exercises the skip leg
            };
            [
                rng.f32_mixed() * s,
                rng.f32_mixed() * s,
                rng.f32_mixed() * s,
            ]
        };
        let (a0, a1, a2) = (ax(), ax(), ax());
        run_case(&mut e, a0, a1, a2, &format!("norm fuzz[{i}]"));
    }
}

/// `0x7137c0` from `0x713bbd` to `0x713c40`: the rows at `[ebp-0x44]`, the normal through
/// `[[ebp-0x4]+0x100]`, the vertex at `eax+0x10`; the plane read from `eax`.
#[test]
fn shadow_transform_normal_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    // ecx-source: [ebp-0x4] = buf_a; [buf_a+0x100] = normal_base; [ebp+8] = 0 → ecx = normal_base.
    let buf_a = e.alloc(0x200);
    let normal_base = e.alloc(0x40);
    let out = e.alloc(0x80); // eax: n' at +0/+4/+8, w at +0xc; vert0 at +0x10..+0x18.

    // the 9 normalized-basis slots (already-normalized inputs to the transform).
    let bslot = |i: usize| -> u64 {
        let off = [0x44, 0x40, 0x3c, 0x38, 0x34, 0x30, 0x2c, 0x28, 0x24][i];
        ebp - off
    };

    let run_case = |e: &mut Emu, basis: [Axis3; 3], n: [f32; 3], v0: [f32; 3], ctx: &str| {
        let bflat = [
            basis[0].x, basis[0].y, basis[0].z, basis[1].x, basis[1].y, basis[1].z, basis[2].x,
            basis[2].y, basis[2].z,
        ];
        for (i, &v) in bflat.iter().enumerate() {
            e.write_u32(bslot(i), v.to_bits());
        }
        // face normal at normal_base (ecx[0/4/8]).
        for i in 0..3 {
            e.write_u32(normal_base + 4 * i as u64, n[i].to_bits());
        }
        // vert0 at out+0x10 (edx = eax+0x10).
        for i in 0..3 {
            e.write_u32(out + 0x10 + 4 * i as u64, v0[i].to_bits());
        }
        // ecx-source wiring.
        e.write_u32(ebp - 0x4, buf_a as u32);
        e.write_u32(buf_a + 0x100, normal_base as u32);
        e.write_u32(ebp + 8, 0);
        // registers the slice expects live: eax = out, edx = eax+0x10.
        e.set_reg(RegisterX86::EAX, out);
        e.set_reg(RegisterX86::EDX, out + 0x10);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // start at 0x713bbd so ecx (= [[ebp-0x4]+0x100]+[ebp+8] = the face-normal ptr) is derived.
        e.run(0x713bbd, 0x713c40); // 3 transformed-normal rows + plane w (faddp×8, fmul×12, fchs)
        let bin = PlaneEq {
            nx: e.read_f32(out),
            ny: e.read_f32(out + 4),
            nz: e.read_f32(out + 8),
            w: e.read_f32(out + 0xc),
        };
        let ours = shadow_transform_normal(basis, n, v0);
        assert_f32(bin.nx, ours.nx, &format!("{ctx} nx"));
        assert_f32(bin.ny, ours.ny, &format!("{ctx} ny"));
        assert_f32(bin.nz, ours.nz, &format!("{ctx} nz"));
        assert_f32(bin.w, ours.w, &format!("{ctx} w"));
    };

    let ident = [
        Axis3 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        Axis3 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        Axis3 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    ];
    run_case(
        &mut e,
        ident,
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0],
        "xform identity",
    );
    run_case(
        &mut e,
        ident,
        [0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0],
        "xform zero-normal",
    );

    let mut rng = Lcg(0x7866_726d); // "xfrm"
    for i in 0..3000u32 {
        let ax = |rng: &mut Lcg| Axis3 {
            x: rng.f32_mixed(),
            y: rng.f32_mixed(),
            z: rng.f32_mixed(),
        };
        let basis = [ax(&mut rng), ax(&mut rng), ax(&mut rng)];
        let n = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        let v0 = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        run_case(&mut e, basis, n, v0, &format!("xform fuzz[{i}]"));
    }
}

/// `0x713680` from `0x7136a0` to `0x7136fa`: the header at `eax` (`+0xb4`), the sphere stored
/// through `[ebp+8]`.
#[test]
fn bbox_center_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let strukt = e.alloc(0x200); // EAX → the struct; bbox floats at +0xb4..+0xcc
    let out = e.alloc(0x20); // [ebp+8] → the centroid/radius out buffer (4 floats)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, bbox: [f32; 14], ctx: &str| {
        // bbox[0..7] → the seven floats at struct +0xb4 .. +0xcc: +0xb4 min_x, +0xb8 min_y, +0xbc
        // min_z, +0xc0 max_x, +0xc4 max_y, +0xc8 max_z, +0xcc radius
        for (i, &f) in bbox.iter().take(7).enumerate() {
            e.write_u32(strukt + 0xb4 + 4 * i as u64, f.to_bits());
        }
        // junk-seed the out buffer so a non-write would diverge.
        for i in 0..4 {
            e.write_u32(out + 4 * i as u64, 0xdead_0000 + i as u32);
        }
        e.set_reg(RegisterX86::EAX, strukt);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.write_u32(ebp + 8, out as u32); // [ebp+8] = out ptr
        e.run(0x7136a0, 0x7136fa); // fld..fstp the centroid + radius; halt at `mov esp,ebp`

        let ours = bbox_center(&bbox);
        for i in 0..3 {
            assert_f32(
                e.read_f32(out + 4 * i as u64),
                ours.center[i],
                &format!("{ctx} center[{i}]"),
            );
        }
        assert_f32(e.read_f32(out + 0xc), ours.radius, &format!("{ctx} radius"));
    };

    // ==threshold / edge probes: zero box, symmetric box (center 0), odd-mantissa (·0.5 loses a
    // bit), negatives, equal min==max (center == that value).
    let mut z = [0f32; 14];
    run_case(&mut e, z, "bbox zero");
    z[0] = -2.0;
    z[1] = -4.0;
    z[2] = -6.0;
    z[3] = 2.0;
    z[4] = 4.0;
    z[5] = 6.0;
    z[6] = 7.211_102; // radius passthrough
    run_case(&mut e, z, "bbox symmetric");
    let odd = [
        1.0, 3.0, 5.000_001, 2.0, 5.0, 9.000_003, 1.5, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    run_case(&mut e, odd, "bbox odd-mantissa");
    let eqm = [
        3.5, -7.25, 11.0, 3.5, -7.25, 11.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    run_case(&mut e, eqm, "bbox min==max");

    let mut rng = Lcg(0x7136_8000);
    for i in 0..4000u32 {
        let mut bbox = [0f32; 14];
        for f in bbox.iter_mut().take(7) {
            *f = rng.f32_mixed();
        }
        run_case(&mut e, bbox, &format!("bbox fuzz[{i}]"));
    }
}

/// `0x70ebd0`'s four stretches from `0x70ff3f` to `0x70ffd7`: the record at `esi`, the copies
/// and differences read from `edi+0x1b8` on.
#[test]
fn part_span_deltas_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let src = e.alloc(0x200);
    let dst = e.alloc(0x200);

    let mut s = 0x70eb_d0d0u32;

    // structured edges + fuzz; each iteration re-seeds the eight source floats the four blocks
    // read.
    let edges: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7fc0_0000),
        1e30,
        -1e30,
        f32::from_bits(0x0000_0001),
    ];

    let run_iter = |e: &mut Emu, vals: [f32; 8], ctx: &str| {
        // source offsets the blocks read: 188,18c (blk1); 19c,1a8 (blk2); 1a0,1ac (blk3); 1a4,1b0
        // (blk4)
        let offs: [u64; 8] = [0x188, 0x18c, 0x19c, 0x1a8, 0x1a0, 0x1ac, 0x1a4, 0x1b0];
        for (o, v) in offs.iter().zip(vals.iter()) {
            e.write_u32(src + o, v.to_bits());
        }
        e.set_reg(RegisterX86::ESI, src);
        e.set_reg(RegisterX86::EDI, dst);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x80);

        // --- block 1: fsub@70ff67 (and the fst-no-pop LIVE copy of src_188) ---
        e.run(0x70ff3f, 0x70ff71);
        let b1 = (
            e.read_f32(dst + 0x1b8),
            e.read_f32(dst + 0x1bc),
            e.read_f32(dst + 0x1c0),
        );
        let o1 = part_span_block1(vals[0], vals[1]);
        assert_f32(b1.0, o1.0, &format!("{ctx} blk1 copy_188"));
        assert_f32(b1.1, o1.1, &format!("{ctx} blk1 copy_18c"));
        assert_f32(b1.2, o1.2, &format!("{ctx} blk1 delta"));

        // --- block 2: fsub@70ff91 ---
        e.set_reg(RegisterX86::ESI, src);
        e.set_reg(RegisterX86::EDI, dst);
        e.run(0x70ff7d, 0x70ff9b);
        let b2 = (e.read_f32(dst + 0x1c8), e.read_f32(dst + 0x1cc));
        let o2 = part_span_block2(vals[2], vals[3]);
        assert_f32(b2.0, o2.0, &format!("{ctx} blk2 copy_19c"));
        assert_f32(b2.1, o2.1, &format!("{ctx} blk2 delta"));

        // --- block 3: fsub@70ffaf ---
        e.set_reg(RegisterX86::ESI, src);
        e.set_reg(RegisterX86::EDI, dst);
        e.run(0x70ff9b, 0x70ffb9);
        let b3 = (e.read_f32(dst + 0x1d0), e.read_f32(dst + 0x1d4));
        let o3 = part_span_block3(vals[4], vals[5]);
        assert_f32(b3.0, o3.0, &format!("{ctx} blk3 copy_1a0"));
        assert_f32(b3.1, o3.1, &format!("{ctx} blk3 delta"));

        // --- block 4: fsub@70ffcd ---
        e.set_reg(RegisterX86::ESI, src);
        e.set_reg(RegisterX86::EDI, dst);
        e.run(0x70ffb9, 0x70ffd7);
        let b4 = (e.read_f32(dst + 0x1d8), e.read_f32(dst + 0x1dc));
        let o4 = part_span_block4(vals[6], vals[7]);
        assert_f32(b4.0, o4.0, &format!("{ctx} blk4 copy_1a4"));
        assert_f32(b4.1, o4.1, &format!("{ctx} blk4 delta"));
    };

    // every edge×edge pair on the first two lanes (block 1), edges spread across the rest.
    for (i, &a) in edges.iter().enumerate() {
        for (j, &b) in edges.iter().enumerate() {
            let vals = [
                a,
                b,
                edges[(i + 1) % edges.len()],
                edges[(j + 2) % edges.len()],
                edges[(i + 3) % edges.len()],
                edges[(j + 4) % edges.len()],
                edges[(i + 5) % edges.len()],
                edges[(j + 6) % edges.len()],
            ];
            run_iter(&mut e, vals, &format!("edge[{i},{j}]"));
        }
    }
    for _ in 0..3000u32 {
        let vals = [
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
        ];
        run_iter(&mut e, vals, "fuzz");
    }
}

/// `0x711fe0` from `0x712041` to `0x712061`, `__ftol` run: the bone at `esi`, the time in `ecx`;
/// the frame read from `edi+8`. NaN and products past 2^63 give `__ftol`'s 0.
#[test]
fn query_attach_event_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let entry = e.alloc(0x200); // ESI → the 0x118-stride entry; reads +0xa8/+0xb0/+0xb8
    let out = e.alloc(0x40); // EDI → param_3 out buffer; result int at +8
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, curtime: i32, a8: i32, scale: f32, b8: i32, ctx: &str| {
        e.write_u32(entry + 0xa8, a8 as u32);
        e.write_u32(entry + 0xb0, scale.to_bits());
        e.write_u32(entry + 0xb8, b8 as u32);
        e.write_u32(out + 8, 0xdead_beef); // junk-seed so a non-write would diverge
        e.set_reg(RegisterX86::ESI, entry);
        e.set_reg(RegisterX86::EDI, out);
        e.set_reg(RegisterX86::ECX, curtime as u32 as u64); // ecx = curtime (the sub's lhs)
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // run(0x712041,0x712061): sub; fild; fmul; __ftol(inline); add; mov [edi+8]. PAST the
        // store.
        e.run(0x712041, 0x712061);
        let bin = e.read_u32(out + 8) as i32;
        let ours = query_attach_event(curtime, a8, scale, b8);
        assert_eq!(bin, ours, "{ctx}: binary={bin:#x} ours={:#x}", ours as u32);
    };

    // exact / edge probes: trivial scales, negative deltas, overflowing products, b8 offsets.
    run_case(&mut e, 0, 0, 1.0, 0, "all-zero");
    run_case(&mut e, 100, 10, 1.0, 0, "delta 90 ×1.0");
    run_case(&mut e, 100, 10, 0.5, 0, "delta 90 ×0.5 (floor 45)");
    run_case(&mut e, 10, 100, 1.0, 0, "negative delta -90");
    run_case(&mut e, 7, 0, 2.5, 1000, "×2.5 +b8");
    run_case(&mut e, 5, 0, 1.0e12, 0, "huge prod (i32 wrap)");
    run_case(&mut e, 100, 0, -1.5, -7, "neg scale + neg b8");
    run_case(&mut e, 3, 5, 0.0, 42, "scale 0 (prod 0)");
    run_case(&mut e, 3, 5, f32::NAN, 42, "NaN scale");
    run_case(&mut e, 1, 0, 1.0e30, 42, "past 2^63");

    let mut s = 0x711f_e000u32;
    for i in 0..6000u32 {
        let curtime = fuzz_i32(&mut s);
        let a8 = fuzz_i32(&mut s);
        let scale = fuzz_scales(&mut s);
        let b8 = fuzz_i32(&mut s);
        run_case(&mut e, curtime, a8, scale, b8, &format!("qae fuzz[{i}]"));
    }
}
