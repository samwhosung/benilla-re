//! The arithmetic of `CMapObj::CrossPortals` (`0x6a3f80`) and `CMapObj::FootprintCells`
//! (`0x6a5270`) as slices, and `CeilLog2` (`0x6a5590`) run whole. A stretch run in an island is the
//! exe's own bytes, checked, copied between a load and a store. The tiling's cell size `0xca7ebc`,
//! its divisor `0xca7eec` and its border `0xca8098` are set at run time, so the tests write them.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::portal::{
    footprint_cell_rect, footprint_cell_size, footprint_count_bias, footprint_count_scale,
    footprint_extent_scale, mip_level_count, portal_extent_scale, portal_plane_side,
    portal_result_scale, portal_seg_normalize,
};
use common::{assert_f32, frand, island_bytes_match, next, SLICE_ESP};

/// The tiling's divisor.
const CELL_RECIP_DIVISOR: u64 = 0xca7eec;

/// The tiling's cell size.
const CELL_SIZE: u64 = 0xca7ebc;

/// Twice the cell size, set on first use.
const BORDER: u64 = 0xca8098;

/// The flag that marks the border set.
const BORDER_FLAG: u64 = 0xca7ed0;

/// `0x6a5590` whole on `x`, its result read from `eax`.
fn check(e: &mut Emu, x: f32, ctx: &str) {
    let bin = e.call(0x6a5590, 0, 0, &[x.to_bits()]) as i32;
    let ours = mip_level_count(x);
    assert_eq!(
        bin,
        ours,
        "{ctx}: x={x} (0x{:08x}) binary={bin} ours={ours}",
        x.to_bits()
    );
}

/// `0x6a3f80` from `0x6a3f8a` to `0x6a4018`: the segment at `ebx`; the unit direction is read from
/// `[ebp-0x3c]`, the length from `[ebp-0x18]` and its inverse from `[ebp-0xc]`. A zero segment
/// divides by zero.
#[test]
fn portal_seg_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let seg = e.alloc(0x40); // ebx → seg[0..6] (start xyz, end xyz)
    let param3 = e.alloc(0x10); // [ebp+0xc] → extent scalar (read by the later slice; harmless here)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, s: [f32; 6], ctx: &str| {
        for (i, v) in s.iter().enumerate() {
            e.write_u32(seg + (i * 4) as u64, v.to_bits());
        }
        e.set_reg(RegisterX86::EBX, seg);
        e.set_reg(RegisterX86::ECX, frame); // immaterial; stored to [ebp-0x14]
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 0xc, param3 as u32); // [ebp+0xc] = &scalar (unused here)
        e.run(0x6a3f8a, 0x6a4018); // halt at `fld [ebp-0x18]` (the extent-scale load)
        let bin_unit = [
            f32::from_bits(e.read_u32(ebp - 0x3c)),
            f32::from_bits(e.read_u32(ebp - 0x38)),
            f32::from_bits(e.read_u32(ebp - 0x34)),
        ];
        let bin_len = f32::from_bits(e.read_u32(ebp - 0x18));
        let bin_inv = f32::from_bits(e.read_u32(ebp - 0xc));
        let (unit, len, inv) = portal_seg_normalize(&s);
        assert_f32(bin_len, len, &format!("{ctx} len"));
        assert_f32(bin_inv, inv, &format!("{ctx} inv"));
        assert_f32(bin_unit[0], unit[0], &format!("{ctx} ux"));
        assert_f32(bin_unit[1], unit[1], &format!("{ctx} uy"));
        assert_f32(bin_unit[2], unit[2], &format!("{ctx} uz"));
    };

    // ==threshold / edge probes: zero-length (len=0 → inv=+Inf, unit=NaN/Inf), axis-aligned unit,
    // equal endpoints, exact-square length (3,4,0 → len 5), negatives.
    run_case(
        &mut e,
        [0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        "norm zero-seg (len 0)",
    );
    run_case(
        &mut e,
        [1.0, 2.0, 3.0, 1.0, 2.0, 3.0],
        "norm equal-endpoints",
    );
    run_case(&mut e, [0.0, 0.0, 0.0, 3.0, 4.0, 0.0], "norm 3-4-0 (len 5)");
    run_case(&mut e, [0.0, 0.0, 0.0, 1.0, 0.0, 0.0], "norm unit-x");
    run_case(&mut e, [5.0, 5.0, 5.0, 4.0, 3.0, -7.0], "norm negatives");
    run_case(
        &mut e,
        [0.0, 0.0, 0.0, 1.0e-20, 0.0, 0.0],
        "norm tiny (subnormal len²)",
    );

    let mut s = 0x6a3f_8080u32;
    for i in 0..3000u32 {
        let seg = [
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
        ];
        run_case(&mut e, seg, &format!("norm fuzz[{i}]"));
    }
}

/// `0x6a3f80` from `0x6a4018` to `0x6a4045`: the length at `[ebp-0x18]` times the scale through
/// `[ebp+0xc]`, stored back.
#[test]
fn portal_extent_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let dummy = e.alloc(0x20);
    let scalar_slot = e.alloc(0x10);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, len: f32, scalar: f32, ctx: &str| {
        e.write_u32(ebp - 0x18, len.to_bits());
        e.write_u32(scalar_slot, scalar.to_bits());
        e.set_reg(RegisterX86::EBX, dummy);
        e.set_reg(RegisterX86::ESI, frame); // slice does `mov ecx,[esi+0x168]` (read+discard)
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 0xc, scalar_slot as u32); // [ebp+0xc] = &scalar
        e.run(0x6a4018, 0x6a4045); // fld[ebp-0x18]; fmul[*param_3]; fstp[ebp-0x18]; halt at mov edx
        let bin = f32::from_bits(e.read_u32(ebp - 0x18));
        let ours = portal_extent_scale(len, scalar);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0.0, 5.0, "ext len=0");
    run_case(&mut e, 5.0, 0.0, "ext scalar=0");
    run_case(&mut e, 1.0, 1.0, "ext unit");
    run_case(&mut e, 100.0, -0.5, "ext negatives");

    let mut s = 0x6a40_1818u32;
    for i in 0..3000u32 {
        let len = frand(&mut s, -3000.0, 3000.0);
        let scalar = frand(&mut s, -10.0, 10.0);
        run_case(&mut e, len, scalar, &format!("ext fuzz[{i}]"));
    }
}

/// `0x6a3f80` from `0x6a414d` to `0x6a4179`: the plane at `esi`, the point through `[ebp+8]`; C0
/// of the compare is read from `ah`. Sums of exactly 0, a small positive sum and a NaN probe the
/// branch.
#[test]
fn portal_plane_side_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let plane = e.alloc(0x20); // esi → [n.x, n.y, n.z, n.w]
    let pt = e.alloc(0x20); // [ebp+8] → segment start [p.x, p.y, p.z]
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, pl: [f32; 4], p: [f32; 3], ctx: &str| {
        for (i, v) in pl.iter().enumerate() {
            e.write_u32(plane + (i * 4) as u64, v.to_bits());
        }
        for (i, v) in p.iter().enumerate() {
            e.write_u32(pt + (i * 4) as u64, v.to_bits());
        }
        e.set_reg(RegisterX86::ESI, plane);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 8, pt as u32); // [ebp+8] → pt (loaded into eax at 0x6a414d, before start)
                                         // start at 0x6a414d so eax = [ebp+8]; halt at `test ah,1`
                                         // (0x6a4179, exclusive).
        e.run(0x6a414d, 0x6a4179);
        let ah = ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8;
        let bin_jne = (ah & 0x1) != 0; // C0 set ⇒ `test ah,1; jne` taken
        let ours = portal_plane_side(&pl, &p);
        assert_eq!(
            bin_jne, ours,
            "{ctx}: binary jne(C0)={bin_jne} ours={ours} (ah={ah:#04x})"
        );
    };

    // ==threshold probes: sum exactly 0.0 (on-plane — C0 clear, jne NOT taken), sum > 0, sum < 0,
    // and a NaN input (C0 set).
    run_case(
        &mut e,
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "side sum=0 on-plane",
    );
    run_case(&mut e, [1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0], "side sum=+1");
    run_case(
        &mut e,
        [1.0, 0.0, 0.0, 0.0],
        [-1.0, 0.0, 0.0],
        "side sum=-1",
    );
    run_case(
        &mut e,
        [1.0, 1.0, 1.0, -3.0],
        [1.0, 1.0, 1.0],
        "side sum=0 (1+1+1-3)",
    );
    run_case(
        &mut e,
        [1.0, 1.0, 1.0, f32::from_bits(0x3dcc_cccd)],
        [1.0, 1.0, 1.0],
        "side sum=+eps",
    );
    run_case(
        &mut e,
        [f32::NAN, 0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0],
        "side NaN (C0 set)",
    );

    let mut s = 0x6a41_5050u32;
    for i in 0..4000u32 {
        let pl = [
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let p = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        run_case(&mut e, pl, p, &format!("side fuzz[{i}]"));
    }
}

/// `0x6a3f80` from `0x6a41ec` to `0x6a41f5` in an island with a store appended: `t` at `[ebp-0x18]`
/// times the inverse at `[ebp-0xc]`.
#[test]
fn portal_result_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // 0x6a41ec..0x6a41f5 (9 bytes): fld [ebp-0x18]; mov edx,[ebp+0xc]; fmul [ebp-0xc]. The result
    // is LIVE in st0 (the binary's next op is `fstp [edx]`); we append our own `fstp dword [out]`.
    const SRC: u64 = 0x6a41ec;
    const LEN: usize = 0x6a41f5 - 0x6a41ec; // 9
    let mut code = e.read(SRC, LEN);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    let end = isl + code.len() as u64;

    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, t_hit: f32, inv: f32, ctx: &str| {
        e.write_u32(ebp - 0x18, t_hit.to_bits());
        e.write_u32(ebp - 0xc, inv.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        let bin = f32::from_bits(e.read_u32(out));
        let ours = portal_result_scale(t_hit, inv);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0.0, 0.5, "res t=0");
    run_case(&mut e, 5.0, 0.0, "res inv=0");
    run_case(&mut e, 10.0, 0.2, "res t·inv=2");
    run_case(&mut e, -3.0, 0.25, "res negatives");

    let mut s = 0x6a41_ecec_u32;
    for i in 0..3000u32 {
        let t_hit = frand(&mut s, -100.0, 100.0);
        let inv = frand(&mut s, -1.0, 1.0);
        run_case(&mut e, t_hit, inv, &format!("res fuzz[{i}]"));
    }
}

/// `0x6a5270` from `0x6a52a6` to `0x6a52d5`: the box at `esi`; read from `[ebp-0x10]`, `[ebp-0xc]`
/// and `[ebp-0x8]`.
#[test]
fn footprint_extent_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let aabb = e.alloc(0x40); // esi → bounds
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case =
        |e: &mut Emu, lo_x: f32, lo_y: f32, hi_x: f32, hi_y: f32, divisor: f32, ctx: &str| {
            e.write_u32(aabb, lo_x.to_bits());
            e.write_u32(aabb + 4, lo_y.to_bits());
            e.write_u32(aabb + 0xc, hi_x.to_bits());
            e.write_u32(aabb + 0x10, hi_y.to_bits());
            e.write_u32(CELL_RECIP_DIVISOR, divisor.to_bits());
            e.set_reg(RegisterX86::ESI, aabb);
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(0x6a52a6, 0x6a52d5); // halt at `mov esi,[ebp-0x8]`
            let bin = [
                f32::from_bits(e.read_u32(ebp - 0x10)),
                f32::from_bits(e.read_u32(ebp - 0xc)),
                f32::from_bits(e.read_u32(ebp - 0x8)),
            ];
            let aabb_arr = [lo_x, lo_y, 0.0, hi_x, hi_y, 0.0];
            let (ex, ey, sy) = footprint_extent_scale(&aabb_arr, divisor);
            assert_f32(bin[0], ex, &format!("{ctx} ext_x"));
            assert_f32(bin[1], ey, &format!("{ctx} ext_y"));
            assert_f32(bin[2], sy, &format!("{ctx} scaled_y"));
        };

    // ==threshold probes: zero extent, divisor=1, divisor that loses precision, lo>hi (neg extent).
    run_case(&mut e, 0.0, 0.0, 0.0, 0.0, 1.0, "fes zero-extent");
    run_case(&mut e, 0.0, 0.0, 64.0, 64.0, 64.0, "fes one-tile");
    run_case(&mut e, -100.0, -100.0, 100.0, 100.0, 33.333, "fes spread");
    run_case(&mut e, 10.0, 10.0, 5.0, 5.0, 16.0, "fes lo>hi (neg)");

    let mut s = 0x6a52_a6a6u32;
    for i in 0..3000u32 {
        let lo_x = frand(&mut s, -5000.0, 5000.0);
        let lo_y = frand(&mut s, -5000.0, 5000.0);
        let hi_x = frand(&mut s, -5000.0, 5000.0);
        let hi_y = frand(&mut s, -5000.0, 5000.0);
        let divisor = frand(&mut s, 1.0, 600.0);
        run_case(
            &mut e,
            lo_x,
            lo_y,
            hi_x,
            hi_y,
            divisor,
            &format!("fes fuzz[{i}]"),
        );
    }
}

/// `0x6a5270` from `0x6a5337` to `0x6a5343`: the x extent at `[ebp-0x10]` times 256, read from
/// `[ebp-0x4]`.
#[test]
fn footprint_count_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, ext_x: f32, ctx: &str| {
        e.write_u32(ebp - 0x10, ext_x.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a5337, 0x6a5343); // fld[ebp-0x10]; fmul[256]; fstp[ebp-0x4]; halt at mov esi
        let bin = f32::from_bits(e.read_u32(ebp - 0x4));
        let ours = footprint_count_scale(ext_x);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0.0, "fcs zero");
    run_case(&mut e, 1.0, "fcs one");
    run_case(&mut e, 0.125, "fcs frac");
    run_case(&mut e, -2.5, "fcs neg");

    let mut s = 0x6a53_3737u32;
    for i in 0..3000u32 {
        let ext_x = frand(&mut s, -1000.0, 1000.0);
        run_case(&mut e, ext_x, &format!("fcs fuzz[{i}]"));
    }
}

/// `0x6a5270` at `0x6a53a9`, `fadd dword [0x8029cc]`, in an island between a load and a store. The
/// exe adds it to `ceil`'s integer result, so an integer input is exact.
#[test]
fn footprint_count_bias_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // island: fld dword [in] ; <fadd dword [0x8029cc] copied> ; fstp dword [out]
    const SRC: u64 = 0x6a53a9;
    const LEN: usize = 0x6a53af - 0x6a53a9; // 6 (the `fadd dword [0x8029cc]`)
    let inp = e.alloc(0x10);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    let fadd = e.read(SRC, LEN);
    let mut code = vec![0xd9u8, 0x05]; // fld dword [abs32]
    code.extend_from_slice(&(inp as u32).to_le_bytes());
    let fadd_off = code.len();
    code.extend_from_slice(&fadd); // the copied `fadd dword [0x8029cc]`
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    // byte-verify ONLY the copied middle == WoW.exe[SRC..]
    island_bytes_match(&e, isl + fadd_off as u64, SRC, LEN);
    let end = isl + code.len() as u64;

    let run_case = |e: &mut Emu, ceil_ext: f32, ctx: &str| {
        e.write_u32(inp, ceil_ext.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(isl, end);
        let bin = f32::from_bits(e.read_u32(out));
        let ours = footprint_count_bias(ceil_ext);
        assert_f32(bin, ours, ctx);
    };

    // ceil produces exact integers; the mantissa-bit extraction uses 0..256-ish counts. Probe the
    // integer values that occur plus the boundary at 512 (where bit 0xe of the f32 mantissa flips).
    for &c in &[
        0.0f32, 1.0, 32.0, 64.0, 128.0, 256.0, 255.0, 16384.0, -1.0, -64.0,
    ] {
        run_case(&mut e, c, &format!("fcb ceil={c}"));
    }
    let mut s = 0x6a53_a9a9u32;
    for i in 0..2000u32 {
        let c = (next(&mut s) % 1024) as f32; // exact integers, the ceil domain
        run_case(&mut e, c, &format!("fcb fuzz[{i}] ceil={c}"));
    }
}

/// `0x6a5270` at `0x6a53dc` and `0x6a53f8`, `fild` of a count then `fmul dword [0xca7ebc]`.
#[test]
fn footprint_cell_size_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, count: i32, cell: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, count as u32);
        e.write_u32(CELL_SIZE, cell.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a53d1, 0x6a53ec); // fild[ebp-0x4]; fmul[0xca7ebc]; fstp[ebp-0x44]; halt at fild
        let bin = f32::from_bits(e.read_u32(ebp - 0x44));
        let ours = footprint_cell_size(count, cell);
        assert_f32(bin, ours, ctx);
    };

    // counts are powers of two in 32..256; probe those + 0/1 + a precision-losing count.
    for &n in &[0i32, 1, 32, 64, 128, 256, 100, 16777217] {
        run_case(&mut e, n, 33.333_332, &format!("fcz count={n}"));
    }
    let mut s = 0x6a53_d1d1u32;
    for i in 0..3000u32 {
        let count = (next(&mut s) % 300) as i32;
        let cell = frand(&mut s, 0.1, 600.0);
        run_case(&mut e, count, cell, &format!("fcz fuzz[{i}]"));
    }
}

/// `0x6a5270` from `0x6a541d` to `0x6a54e4`, one cell: `ebx`, `edi` the cell, `edx`, `esi` the
/// counts, the cell sizes at `[ebp-0x44]` and `[ebp-0x40]`, the map object through `[ebp-0x18]`.
/// The border flag is cleared each time so the border is set from the cell size. Each edge case and
/// a 1×1 tiling probe the four border tests.
#[test]
#[allow(clippy::too_many_arguments)]
fn footprint_cell_rect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x80); // [ebp-0x18] → this (base at +0x14/+0x18)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu,
                    ix: i32,
                    iy: i32,
                    nx: i32,
                    ny: i32,
                    cell_w: f32,
                    cell_h: f32,
                    base_x: f32,
                    base_y: f32,
                    cell: f32,
                    ctx: &str| {
        e.set_reg(RegisterX86::EBX, ix as u32 as u64);
        e.set_reg(RegisterX86::EDI, iy as u32 as u64);
        e.set_reg(RegisterX86::EDX, nx as u32 as u64);
        e.set_reg(RegisterX86::ESI, ny as u32 as u64);
        e.write_u32(ebp - 0x10, ix as u32);
        e.write_u32(ebp - 0xc, iy as u32);
        e.write_u32(ebp - 0x44, cell_w.to_bits());
        e.write_u32(ebp - 0x40, cell_h.to_bits());
        e.write_u32(ebp - 0x18, this as u32);
        e.write_u32(this + 0x14, base_x.to_bits());
        e.write_u32(this + 0x18, base_y.to_bits());
        e.write_u32(CELL_SIZE, cell.to_bits());
        e.write_u32(BORDER_FLAG, 0); // force the 2·cell border re-init from [0xca7ebc]
        e.write_u32(BORDER, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a541d, 0x6a54e4); // through the 4 edge expands; halt at `mov ecx,[ebp+0xc]`
        let bin = [
            f32::from_bits(e.read_u32(ebp - 0x68)),
            f32::from_bits(e.read_u32(ebp - 0x64)),
            f32::from_bits(e.read_u32(ebp - 0x5c)),
            f32::from_bits(e.read_u32(ebp - 0x58)),
        ];
        let ours = footprint_cell_rect(ix, iy, nx, ny, cell_w, cell_h, base_x, base_y, cell);
        for k in 0..4 {
            assert_f32(bin[k], ours[k], &format!("{ctx} edge[{k}]"));
        }
    };

    // ==threshold probes for the 4 edge conditions: interior cell (no border), each corner
    // (ix==0 / iy==0 / ix==nx-1 / iy==ny-1), and the all-corner 1×1 grid (all four expand).
    run_case(
        &mut e,
        1,
        1,
        4,
        4,
        50.0,
        60.0,
        1000.0,
        2000.0,
        16.0,
        "fcr interior",
    );
    run_case(
        &mut e,
        0,
        1,
        4,
        4,
        50.0,
        60.0,
        1000.0,
        2000.0,
        16.0,
        "fcr ix==0",
    );
    run_case(
        &mut e,
        1,
        0,
        4,
        4,
        50.0,
        60.0,
        1000.0,
        2000.0,
        16.0,
        "fcr iy==0",
    );
    run_case(
        &mut e,
        3,
        1,
        4,
        4,
        50.0,
        60.0,
        1000.0,
        2000.0,
        16.0,
        "fcr ix==nx-1",
    );
    run_case(
        &mut e,
        1,
        3,
        4,
        4,
        50.0,
        60.0,
        1000.0,
        2000.0,
        16.0,
        "fcr iy==ny-1",
    );
    run_case(
        &mut e,
        0,
        0,
        1,
        1,
        50.0,
        60.0,
        1000.0,
        2000.0,
        16.0,
        "fcr all-corners 1x1",
    );
    run_case(
        &mut e,
        0,
        0,
        4,
        4,
        50.0,
        60.0,
        0.0,
        0.0,
        0.0,
        "fcr zero-cell",
    );

    let mut s = 0x6a54_1d1du32;
    for i in 0..3000u32 {
        let nx = (next(&mut s) % 8) as i32 + 1;
        let ny = (next(&mut s) % 8) as i32 + 1;
        let ix = (next(&mut s) % (nx as u32)) as i32;
        let iy = (next(&mut s) % (ny as u32)) as i32;
        let cell_w = frand(&mut s, 1.0, 500.0);
        let cell_h = frand(&mut s, 1.0, 500.0);
        let base_x = frand(&mut s, -5000.0, 5000.0);
        let base_y = frand(&mut s, -5000.0, 5000.0);
        let cell = frand(&mut s, 1.0, 256.0);
        run_case(
            &mut e,
            ix,
            iy,
            nx,
            ny,
            cell_w,
            cell_h,
            base_x,
            base_y,
            cell,
            &format!("fcr fuzz[{i}]"),
        );
    }
}

/// `0x6a5590` whole: every power of two from 2^-11 to 2^23 and a step either side (where `ceil`
/// turns), 0, −0, +∞, the extremes, and positive values over several scales.
#[test]
fn mip_level_count_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // ---- deterministic ceil-boundary probes: every 2^n and its ±1 ULP neighbours ----
    // (the only inputs where log2 is exactly integral; the structural-transcendental boundary.)
    for n in -11..=23i32 {
        let p = (2.0f64).powi(n) as f32;
        if !p.is_finite() || p <= 0.0 {
            continue;
        }
        check(&mut e, p, &format!("2^{n} exact"));
        check(
            &mut e,
            f32::from_bits(p.to_bits() + 1),
            &format!("2^{n}+1ulp"),
        );
        check(
            &mut e,
            f32::from_bits(p.to_bits() - 1),
            &format!("2^{n}-1ulp"),
        );
    }

    // ---- specials in the modeled (positive / +Inf / zero) domain ----
    check(&mut e, 1.0, "x=1 → 0");
    check(&mut e, 0.0, "x=+0 → 0");
    check(&mut e, -0.0, "x=-0 → 0");
    check(&mut e, f32::INFINITY, "+Inf → 0 (ftol indefinite)");
    check(&mut e, f32::MAX, "f32::MAX → 128");
    check(&mut e, f32::MIN_POSITIVE, "smallest normal");
    check(&mut e, f32::from_bits(1), "smallest denormal");
    // a spread of non-power values across the integer cells
    for &x in &[
        1.5f32, 2.5, 3.0, 3.9999, 4.0001, 5.0, 7.0, 9.0, 15.0, 17.0, 100.0, 255.0, 257.0, 1000.0,
        0.5, 0.4, 0.6, 0.001, 0.3, 1.0e6, 1.0e-6, 1.0e30, 1.0e-30,
    ] {
        check(&mut e, x, "named");
    }

    // ---- positive-finite fuzz across scales ----
    let mut s = 0x006a_5590u32;
    let mut checked = 0u32;
    for _ in 0..30_000u32 {
        let m = f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24);
        let x = match (next(&mut s) >> 8) % 5 {
            0 => (m * 4.0) as f32,             // sub-1 .. 4
            1 => (m * 1.0e6) as f32,           // up to mega
            2 => (m * 1.0e-3 + 1.0e-9) as f32, // tiny positives (large-negative counts)
            3 => (m * 65536.0) as f32,         // texture-dimension scale
            _ => (m * 16.0) as f32,            // dense around the small cells
        };
        if !(x.is_finite() && x > 0.0) {
            continue;
        }
        check(&mut e, x, "fuzz");
        checked += 1;
    }

    eprintln!(
        "mip_level_count: {checked} fuzz + 105 boundary/named cases bit-exact vs WoW.exe @ 0x6a5590 \
         (ceil(log2 x); c=0x802940=2.0)"
    );
}
