//! The arithmetic of the map object liquids, as slices: `DrawGroupMagma` (`0x6b68f0`),
//! `DrawExteriorGroupLiquid` (`0x6b6630`), `DrawInteriorGroupLiquid` (`0x6b6420`),
//! `FindNearestGroupLiquid` (`0x68adc0`) and `SetDefWaterPlane` (`0x6a7650`); and
//! `BuildGroupDepthRamp` (`0x6b6b60`) run whole. A stretch run in an island is the exe's own bytes,
//! checked, copied between a load and a store.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::mapobjliquid::{
    clip_plane_negate, mliq_advance, mliq_depth_alpha_lut, mliq_np_delta, mliq_np_dist2,
    mliq_np_grid_coord, mliq_render_material_scale, mliq_render_uv, MLIQ_CELL_STEP,
    MLIQ_RENDER_SCALE, MLIQ_UV_SCALE,
};
use common::{assert_f32, frand, island_bytes_match, next, Lcg, SLICE_ESP};

/// The grid spacing `0x810cc0`.
const GRID_STRIDE_ADDR: u64 = 0x0081_0cc0;

/// Any `f32`, from random bits.
fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// `0x6b68f0` at `0x6b69a7`: `fild` of a sign-extended 16-bit coordinate times 1/256, in an island.
#[test]
fn mliq_render_uv_u_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let scratch = e.alloc(0x80);

    let run_case = |e: &mut Emu, u: i32, ctx: &str| {
        e.write_u32(ebp - 0xc, u as u32);
        e.set_reg(RegisterX86::EAX, scratch);
        e.set_reg(RegisterX86::ECX, scratch);
        e.set_reg(RegisterX86::EDX, scratch);
        e.set_reg(RegisterX86::EBX, 0);
        e.set_reg(RegisterX86::EDI, u as u32 as u64);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b69a0, 0x6b69b6); // fild; fmul [0x80a8ac]; fstp [ebp-0x1c]
        let bin = e.read_f32(ebp - 0x1c);
        let ours = mliq_render_uv(u, MLIQ_UV_SCALE);
        assert_f32(bin, ours, &format!("{ctx} u_tex"));
    };

    // i16-range probes: extremes, zero, ±1, and a value whose ×1/256 is exact vs one that rounds.
    for &u in &[0i32, 1, -1, 32767, -32768, 256, 255, 12345, -9999, 100] {
        run_case(&mut e, u, &format!("u={u}"));
    }
    let mut s = 0x6b69_a7a7u32;
    for i in 0..3000u32 {
        let u = (next(&mut s) as i32 as i16) as i32; // any i16
        run_case(&mut e, u, &format!("u fuzz[{i}]"));
    }
}

/// `0x6b68f0` at `0x6b69c5`, the second coordinate, in an island.
#[test]
fn mliq_render_uv_v_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let scratch = e.alloc(0x80);

    let run_case = |e: &mut Emu, v: i32, ctx: &str| {
        e.write_u32(ebp - 0xc, v as u32);
        e.set_reg(RegisterX86::ECX, scratch + 0x40); // ecx-0x18 lands inside scratch
        e.set_reg(RegisterX86::EDX, scratch);
        e.set_reg(RegisterX86::EBX, 0);
        e.set_reg(RegisterX86::EDI, v as u32 as u64);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b69b9, 0x6b69ce); // fild; fmul [0x80a8ac]; fstp [ebp-0x18]
        let bin = e.read_f32(ebp - 0x18);
        let ours = mliq_render_uv(v, MLIQ_UV_SCALE);
        assert_f32(bin, ours, &format!("{ctx} v_tex"));
    };

    for &v in &[0i32, 1, -1, 32767, -32768, 256, 255, 12345, -9999, 100] {
        run_case(&mut e, v, &format!("v={v}"));
    }
    let mut s = 0x6b69_c5c5u32;
    for i in 0..3000u32 {
        let v = (next(&mut s) as i32 as i16) as i32;
        run_case(&mut e, v, &format!("v fuzz[{i}]"));
    }
}

/// `0x6b68f0` at `0x6b69d4`, `fadd dword [0x810cc0]`, in an island.
#[test]
fn mliq_render_advance_x_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x6b69d4; // fadd dword [0x810cc0]
    const LEN: usize = 6;
    let seed = e.alloc(0x10);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);

    let mut code = Vec::new();
    code.extend_from_slice(&[0xd9, 0x05]); // fld dword [abs32]
    code.extend_from_slice(&(seed as u32).to_le_bytes());
    let fadd = e.read(SRC, LEN); // the exe's fadd
    code.extend_from_slice(&fadd);
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [abs32]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    // byte-verify ONLY the copied fadd (offset 6 in the island) == WoW.exe.
    island_bytes_match(&e, isl + 6, SRC, LEN);
    let end = isl + code.len() as u64;

    let run_case = |e: &mut Emu, coord: f32, ctx: &str| {
        e.write_u32(seed, coord.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(isl, end);
        let bin = e.read_f32(out);
        let ours = mliq_advance(coord, MLIQ_CELL_STEP);
        assert_f32(bin, ours, &format!("{ctx} adv_x"));
    };

    // probes: 0, base-like values, an odd-mantissa coord (the +cell_step rounds), negatives.
    for &c in &[0.0f32, 10.0, -10.0, 4.166_666_5, 100.0, -1024.5, 1e6, -1e-6] {
        run_case(&mut e, c, &format!("c={c}"));
    }
    let mut s = 0x6b69_d4d4u32;
    for i in 0..3000u32 {
        let c = frand(&mut s, -4000.0, 4000.0);
        run_case(&mut e, c, &format!("adv_x fuzz[{i}]"));
    }
}

/// `0x6b68f0` at `0x6b69ec`, `fadd dword [0x810cc0]`, in an island.
#[test]
fn mliq_render_advance_y_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x6b69ec; // fadd dword [0x810cc0] (the y advance — distinct address)
    const LEN: usize = 6;
    let seed = e.alloc(0x10);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);

    let mut code = Vec::new();
    code.extend_from_slice(&[0xd9, 0x05]);
    code.extend_from_slice(&(seed as u32).to_le_bytes());
    let fadd = e.read(SRC, LEN);
    code.extend_from_slice(&fadd);
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl + 6, SRC, LEN);
    let end = isl + code.len() as u64;

    let run_case = |e: &mut Emu, coord: f32, ctx: &str| {
        e.write_u32(seed, coord.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(isl, end);
        let bin = e.read_f32(out);
        let ours = mliq_advance(coord, MLIQ_CELL_STEP);
        assert_f32(bin, ours, &format!("{ctx} adv_y"));
    };

    for &c in &[0.0f32, 20.0, -20.0, 4.166_666_5, 200.0, -2048.5, 1e6, -1e-6] {
        run_case(&mut e, c, &format!("c={c}"));
    }
    let mut s = 0x6b69_ececu32;
    for i in 0..3000u32 {
        let c = frand(&mut s, -4000.0, 4000.0);
        run_case(&mut e, c, &format!("adv_y fuzz[{i}]"));
    }
}

/// `0x6b68f0` at `0x6b6ac7`, `fmul dword [0x801620]`, in an island.
#[test]
fn mliq_render_material_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x6b6ac7; // fmul dword [0x801620]
    const LEN: usize = 6;
    let seed = e.alloc(0x10);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);

    let mut code = Vec::new();
    code.extend_from_slice(&[0xd9, 0x05]);
    code.extend_from_slice(&(seed as u32).to_le_bytes());
    let fmul = e.read(SRC, LEN);
    code.extend_from_slice(&fmul);
    code.extend_from_slice(&[0xd9, 0x1d]);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl + 6, SRC, LEN);
    let end = isl + code.len() as u64;

    let run_case = |e: &mut Emu, v: f32, ctx: &str| {
        e.write_u32(seed, v.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        e.run(isl, end);
        let bin = e.read_f32(out);
        let ours = mliq_render_material_scale(v, MLIQ_RENDER_SCALE);
        assert_f32(bin, ours, &format!("{ctx} mat_scale"));
    };

    for &v in &[0.0f32, 1.0, -1.0, 10.0, 0.5, 3.0, 255.0, 1e6, -1e-6] {
        run_case(&mut e, v, &format!("v={v}"));
    }
    let mut s = 0x6b6a_c7c7u32;
    for i in 0..3000u32 {
        let v = frand(&mut s, -1000.0, 1000.0);
        run_case(&mut e, v, &format!("mat_scale fuzz[{i}]"));
    }
}

/// `0x6b6630` at `0x6b67b4`: `fld dword [0x810cc0]` then `fadd` of the running x, in an island.
#[test]
fn mliq_build_advance_x_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x200); // ecx (+0xf8 read)

    let run_case = |e: &mut Emu, running: f32, ctx: &str| {
        e.write_u32(ebp - 0x8, running.to_bits());
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::EAX, 0);
        e.set_reg(RegisterX86::ESI, 0);
        e.set_reg(RegisterX86::EBX, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b67ae, 0x6b67c7); // fld [0x810cc0]; fadd [ebp-0x8]; fstp [ebp-0x8]
        let bin = e.read_f32(ebp - 0x8);
        let ours = mliq_advance(running, MLIQ_CELL_STEP);
        assert_f32(bin, ours, &format!("{ctx} build_x"));
    };

    for &r in &[0.0f32, 10.0, -10.0, 4.166_666_5, 100.0, -1024.5, 1e6, -1e-6] {
        run_case(&mut e, r, &format!("r={r}"));
    }
    let mut s = 0x6b67_b4b4u32;
    for i in 0..3000u32 {
        let r = frand(&mut s, -4000.0, 4000.0);
        run_case(&mut e, r, &format!("build_x fuzz[{i}]"));
    }
}

/// `0x6b6630` at `0x6b67dd`: the same for the running y.
#[test]
fn mliq_build_advance_y_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x200); // eax (+0xfc read)

    let run_case = |e: &mut Emu, running: f32, ctx: &str| {
        e.write_u32(ebp - 0xc, running.to_bits());
        e.write_u32(ebp - 0x10, 0); // [ebp-0x10] loaded into ecx (harmless)
        e.set_reg(RegisterX86::EAX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b67d4, 0x6b67ec); // fld [0x810cc0]; fadd [ebp-0xc]; fstp [ebp-0xc]
        let bin = e.read_f32(ebp - 0xc);
        let ours = mliq_advance(running, MLIQ_CELL_STEP);
        assert_f32(bin, ours, &format!("{ctx} build_y"));
    };

    for &r in &[0.0f32, 20.0, -20.0, 4.166_666_5, 200.0, -2048.5, 1e6, -1e-6] {
        run_case(&mut e, r, &format!("r={r}"));
    }
    let mut s = 0x6b67_ddddu32;
    for i in 0..3000u32 {
        let r = frand(&mut s, -4000.0, 4000.0);
        run_case(&mut e, r, &format!("build_y fuzz[{i}]"));
    }
}

/// `0x68adc0` from `0x68ae46` to `0x68ae8f` in an island with stores appended: a grid cell's
/// deltas to the query and its squared distance, the stored `f32`s and the unrounded sum.
#[test]
fn mliq_nearest_point_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    const SRC: u64 = 0x68ae43; // fild [ebp+8] .. (just before fcom@68ae91)
    const LEN: usize = 0x68ae91 - 0x68ae43; // 0x4e = 78 bytes

    let seed = e.alloc(0x10); // fVar2 (the z source on st0 at block entry)
    let out = e.alloc(0x10); // captured dist²
    let isl = e.alloc(0x100);
    let this = e.alloc(0x200); // ecx: base.x @ +0x108, base.y @ +0x10c
    let query = e.alloc(0x20); // esi: query.x/y/z @ +0/+4/+8
    let param3 = e.alloc(0x40); // [ebp+0xc] → eax for the `mov [eax],1` write
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    // island = fld dword [seed] ; <the bytes 0x68ae43..0x68ae91> ; fstp dword [out]
    let mut code = Vec::new();
    code.extend_from_slice(&[0xd9, 0x05]); // fld dword [abs32]
    code.extend_from_slice(&(seed as u32).to_le_bytes());
    let block = e.read(SRC, LEN);
    code.extend_from_slice(&block);
    code.extend_from_slice(&[0xd9, 0x1d]); // fstp dword [abs32]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl + 6, SRC, LEN); // verify ONLY the copied block
    let end = isl + code.len() as u64;

    let run_case =
        |e: &mut Emu, i: i32, j: i32, base: [f32; 2], q: [f32; 3], fvar2: f32, ctx: &str| {
            e.write_u32(seed, fvar2.to_bits());
            e.write_u32(this + 0x108, base[0].to_bits());
            e.write_u32(this + 0x10c, base[1].to_bits());
            e.write_u32(query, q[0].to_bits());
            e.write_u32(query + 4, q[1].to_bits());
            e.write_u32(query + 8, q[2].to_bits());
            e.write_u32(ebp + 8, i as u32); // fild [ebp+8] (i)
            e.write_u32(ebp - 0xc, j as u32); // fild [ebp-0xc] (j)
            e.write_u32(ebp + 0xc, param3 as u32); // mov eax,[ebp+0xc]
            e.set_reg(RegisterX86::ECX, this);
            e.set_reg(RegisterX86::ESI, query);
            e.set_reg(RegisterX86::EDX, 0); // tile-type 0 → write lands at [param3]
            e.set_reg(RegisterX86::ESP, SLICE_ESP);
            e.set_reg(RegisterX86::EBP, ebp);
            e.run(isl, end);

            let bin_fv5 = e.read_f32(ebp - 0x18);
            let bin_fv6 = e.read_f32(ebp - 0x14);
            let bin_dz = e.read_f32(ebp - 0x10);
            let bin_dist2 = e.read_f32(out);

            let gx = mliq_np_grid_coord(i, MLIQ_CELL_STEP, base[0]);
            let gy = mliq_np_grid_coord(j, MLIQ_CELL_STEP, base[1]);
            let fv5 = mliq_np_delta(gx, q[0]);
            let fv6 = mliq_np_delta(gy, q[1]);
            let dz_live = f64::from(fvar2) - f64::from(q[2]);
            let dz_f32 = dz_live as f32;
            let dist2 = mliq_np_dist2(fv5, fv6, dz_live) as f32;

            assert_f32(bin_fv5, fv5, &format!("{ctx} fv5"));
            assert_f32(bin_fv6, fv6, &format!("{ctx} fv6"));
            assert_f32(bin_dz, dz_f32, &format!("{ctx} dz"));
            assert_f32(bin_dist2, dist2, &format!("{ctx} dist2"));
        };

    // probes: origin cell, far cell, exact integers, negatives.
    run_case(
        &mut e,
        0,
        0,
        [10.0, 20.0],
        [10.0, 20.0, 0.0],
        0.0,
        "np origin",
    );
    run_case(
        &mut e,
        3,
        2,
        [10.0, 20.0],
        [25.0, 30.0, 5.0],
        7.0,
        "np cell-3-2",
    );
    run_case(
        &mut e,
        -1,
        -1,
        [0.0, 0.0],
        [0.0, 0.0, 0.0],
        0.0,
        "np neg-cell",
    );
    run_case(
        &mut e,
        100,
        100,
        [-500.0, -500.0],
        [0.0, 0.0, 0.0],
        100.0,
        "np far",
    );

    let mut s = 0x68ad_c0c0u32;
    for it in 0..4000u32 {
        let i = (next(&mut s) as i32) % 256;
        let j = (next(&mut s) as i32) % 256;
        let base = [
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
        ];
        let q = [
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -2000.0, 2000.0),
            frand(&mut s, -500.0, 500.0),
        ];
        let fvar2 = frand(&mut s, -500.0, 500.0);
        run_case(&mut e, i, j, base, q, fvar2, &format!("np fuzz[{it}]"));
    }
}

/// `0x6b6420` at `0x6b652c` and `0x6b6555`: the spacing plus the running coordinate.
#[test]
fn grid_advance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x100); // edi: the int loop control reads [edi+0xf8]/[edi+0xfc] (counts)

    // the stride lives in the binary's mapped .data — read it for the reference fn input.
    let stride = e.read_f32(GRID_STRIDE_ADDR);

    // x advance: accumulator at [ebp-0x8], run(0x6b652c, 0x6b6545).
    let run_x = |e: &mut Emu, coord: f32, ctx: &str| {
        e.write_u32(ebp - 0x8, coord.to_bits());
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b652c, 0x6b6545);
        let bin = e.read_f32(ebp - 0x8);
        assert_f32(bin, mliq_advance(stride, coord), ctx);
    };
    // y advance: accumulator at [ebp-0xc], run(0x6b6555, 0x6b656d).
    let run_y = |e: &mut Emu, coord: f32, ctx: &str| {
        e.write_u32(ebp - 0xc, coord.to_bits());
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6b6555, 0x6b656d);
        let bin = e.read_f32(ebp - 0xc);
        assert_f32(bin, mliq_advance(stride, coord), ctx);
    };

    // probes: the grid origin (0), a negative origin (liquid tiles span ±), and a large coordinate.
    for &c in &[0.0f32, -25.0, 100.0, -533.3333, 1234.5] {
        run_x(&mut e, c, &format!("grid x advance c={c}"));
        run_y(&mut e, c, &format!("grid y advance c={c}"));
    }

    let mut s = 0x6b64_2020u32;
    for i in 0..4000u32 {
        let c = frand(&mut s, -2000.0, 2000.0);
        run_x(&mut e, c, &format!("grid x fuzz[{i}]"));
        run_y(&mut e, c, &format!("grid y fuzz[{i}]"));
    }
    // tiny / huge magnitudes (rounding edge of stride + coord).
    let mut s = 0x6b64_4444u32;
    for i in 0..1000u32 {
        let c = frand(&mut s, -1e-4, 1e-4);
        run_x(&mut e, c, &format!("grid x tiny fuzz[{i}]"));
        let c2 = frand(&mut s, 1e6, 1e7);
        run_y(&mut e, c2, &format!("grid y huge fuzz[{i}]"));
    }
}

/// `0x6b6b60` whole, `GetLightState` (`0x6d48b0`) replaced to return a state whose `+0x114` and
/// `+0x118` the test writes; the 256 bytes are read from `0xca7f10`.
#[test]
fn mliq_depth_alpha_lut_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);

    const ENV: u64 = 0xce9b60; // getter 0x6d48b0 returns this fixed global
    const LUT: u64 = 0xca7f10; // published LUT base

    let mut rng = Lcg(0x6d_6c_69_71);
    let (mut neg_step, mut flat, mut pos_step) = (0u32, 0u32, 0u32);
    // a depth-ish range: mostly [0,1] (the endpoints clamp into the byte range), plus out-of-range.
    let depth = |s: &mut Lcg| -> f32 {
        let pick = (s.next() >> 8) % 4;
        let m = f64::from(s.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => m as f32,                // [0,1)
            1 => (m * 2.0 - 0.5) as f32,  // [-0.5,1.5) — over/under-flow the byte
            2 => (m * 10.0 - 5.0) as f32, // way out of range
            _ => (m * 0.4 + 0.3) as f32,  // mid band
        }
    };
    for _ in 0..1500u32 {
        let near = depth(&mut rng);
        let far = depth(&mut rng);
        emu.write_u32(ENV + 0x114, near.to_bits());
        emu.write_u32(ENV + 0x118, far.to_bits());
        // poison the LUT so a no-op would be caught.
        for i in 0..256u64 {
            emu.write(LUT + i, &[0xa5]);
        }
        // no-arg call (the getter runs native and returns ENV).
        emu.call(0x6b6b60, 0, 0, &[]);

        let ours = mliq_depth_alpha_lut(near, far);
        for i in 0..256u64 {
            let b = (emu.read_u32(LUT + (i & !3)) >> ((i & 3) * 8)) as u8;
            assert_eq!(
                b, ours[i as usize],
                "mliq_lut[{i}] near={near} far={far}: binary={b:#04x} ours={:#04x}",
                ours[i as usize]
            );
        }
        // branch bookkeeping via the recomputed endpoints.
        let ep = |x: f32| -> i32 {
            let v = ((f64::from(x) * 255.0) + 512.0) as f32;
            ((v.to_bits() >> 14) & 0xff) as i32
        };
        let (nb, fb) = (ep(near), ep(far));
        match fb.cmp(&nb) {
            std::cmp::Ordering::Less => neg_step += 1,
            std::cmp::Ordering::Equal => flat += 1,
            std::cmp::Ordering::Greater => pos_step += 1,
        }
    }
    assert!(
        neg_step > 0 && flat > 0 && pos_step > 0,
        "branches: neg_step={neg_step} flat={flat} pos_step={pos_step}"
    );
}

/// `0x6a7650` from `0x6a7792` to `0x6a77c5`: the definition's `+0x98` negated, read from
/// `[ebp-0x44]`.
#[test]
fn clip_plane_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x100); // ESI; the block reads [esi+0x98] = scalar
    let plane = e.alloc(0x200); // [ebp+8] = param_2; writes at +0x19c..+0x1ac
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, scalar: f32, ctx: &str| {
        e.write_u32(this + 0x98, scalar.to_bits());
        e.write_u32(ebp + 8, plane as u32); // [ebp+8] → param_2 (the plane buffer)
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x6a7785, 0x6a77ce);
        let bin_slot = e.read_f32(ebp - 0x44); // the fstp dword target
        let bin_plane = e.read_f32(plane + 0x1ac); // the plane.w copy ([eax+0xc])
        let ours = clip_plane_negate(scalar);
        assert_f32(bin_slot, ours, &format!("{ctx} [ebp-0x44]"));
        assert_f32(bin_plane, ours, &format!("{ctx} plane+0x1ac"));
    };

    // boundary probes: 0 (negate of zero → ±0, bit-distinct), +1, -1, plus the plane literals.
    run_case(&mut e, 0.0, "clip s=0");
    run_case(&mut e, -0.0, "clip s=-0");
    run_case(&mut e, 1.0, "clip s=1");
    run_case(&mut e, -1.0, "clip s=-1");
    run_case(&mut e, 12345.678, "clip s=big");

    let mut s = 0x6a77_8585u32;
    for i in 0..3000u32 {
        run_case(
            &mut e,
            frand(&mut s, -1.0e6, 1.0e6),
            &format!("clip fuzz[{i}]"),
        );
    }
    // raw-bit fuzz incl. negatives/denormals/inf (NaN handled by assert_f32; the negate is a sign
    // flip).
    let mut sb = 0x1357_9bdfu32;
    for i in 0..3000u32 {
        run_case(&mut e, frand_bits(&mut sb), &format!("clip bits[{i}]"));
    }
}
