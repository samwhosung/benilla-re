//! Drawing a model (`systems/models/render.md`): the skinning routines and `0x719c20` called whole,
//! the material, lighting and batch stretches as slices. A stretch run in an island is the exe's
//! own bytes, checked, with a store appended.

#![allow(clippy::needless_range_loop)] // index loops follow the `+off + 4·k` layout

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::render::{
    add_f32, batch_alpha_resolve, batch_clamp_pack, batch_negate4, color_scale, decal_texgen_pack,
    m2_decal_projection, m2_vertex_skin, m2_vertex_transform, m2_vertex_transform_sse,
    model_matrix_translate, normalize_residual, rgba_pack_value, scale_f32, unit_length_gate,
    DecalTexgen, DecalVert, UNIT_LEN_DELTA,
};
use common::{assert_f32, assert_f64, frand, island_bytes_match, next, read_f64, Lcg, SLICE_ESP};

/// The vertex the skinning routines read, 0x30 bytes.
#[derive(Clone, Copy)]
struct Vert {
    pos: [f32; 3],
    weights: [u8; 4],
    indices: [u8; 4],
    normal: [f32; 3],
    /// `+0x20` to `+0x30`.
    tail: [u32; 4],
}

/// A vertex with `n_bones` non-zero weights.
fn gen_vert(rng: &mut Lcg, n_bones: usize, num_matrices: u8) -> Vert {
    let mut weights = [0u8; 4];
    for w in weights.iter_mut().take(n_bones) {
        *w = rng.byte().max(1);
    }
    let indices = [0; 4].map(|_: u8| rng.byte() % num_matrices);
    Vert {
        pos: [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()],
        weights,
        indices,
        normal: [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()],
        tail: [rng.next(), rng.next(), rng.next(), rng.next()],
    }
}

/// The instance, its shared model and buffers for one vertex.
struct Scene {
    model: u64,
    batch: u64,
    vert_base: u64,
    out: u64,
}

const NUM_MATRICES: u8 = 6;

/// The instance `+0x30` → shared model, `+0x130` → buffers, `+0x48` → vertices; `+0x94` the bone
/// matrices.
fn build_scene(emu: &mut Emu, matrices: &[[f32; 16]]) -> Scene {
    let model = emu.alloc(0x100);
    let skel = emu.alloc(0x140);
    let vbd = emu.alloc(0x50);
    let bone_base = emu.alloc(0x40 * matrices.len() as u64);
    let vert_base = emu.alloc(0x30);
    let batch = emu.alloc(0x10);
    let out = emu.alloc(0x40);
    emu.write_u32(model + 0x30, skel as u32);
    emu.write_u32(model + 0x94, bone_base as u32);
    emu.write_u32(skel + 0x130, vbd as u32);
    emu.write_u32(vbd + 0x48, vert_base as u32);
    write_matrices(emu, bone_base, matrices);
    Scene {
        model,
        batch,
        vert_base,
        out,
    }
}

fn write_matrices(emu: &mut Emu, base: u64, matrices: &[[f32; 16]]) {
    for (i, m) in matrices.iter().enumerate() {
        for (j, &f) in m.iter().enumerate() {
            emu.write_u32(base + 0x40 * i as u64 + 4 * j as u64, f.to_bits());
        }
    }
}

/// Writes a vertex at `base`.
fn write_vert_at(emu: &mut Emu, base: u64, v: &Vert) {
    for i in 0..3 {
        emu.write_u32(base + 4 * i as u64, v.pos[i].to_bits());
        emu.write_u32(base + 0x14 + 4 * i as u64, v.normal[i].to_bits());
    }
    emu.write(base + 0x0c, &v.weights);
    emu.write(base + 0x10, &v.indices);
    for i in 0..4 {
        emu.write_u32(base + 0x20 + 4 * i as u64, v.tail[i]);
    }
}

/// One vertex, a submesh of start 0 and count 1.
fn write_vert(emu: &mut Emu, sc: &Scene, v: &Vert) {
    write_vert_at(emu, sc.vert_base, v);
    emu.write_u32(sc.batch + 4, 0);
    emu.write_u32(sc.batch + 6, 1);
}

fn gen_matrices(rng: &mut Lcg) -> Vec<[f32; 16]> {
    (0..NUM_MATRICES)
        .map(|_| std::array::from_fn(|_| rng.f32_mixed()))
        .collect()
}

/// `0x71a720` whole on one vertex with zero to four bones, the 0x20-byte output read back.
#[test]
fn m2_vertex_transform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut rng = Lcg(0x6d32_7472);
    let mats = gen_matrices(&mut rng);
    let sc = build_scene(&mut emu, &mats);
    let bones = |i: u8| mats[i as usize];
    for n_bones in 0..=4usize {
        for _ in 0..120 {
            let v = gen_vert(&mut rng, n_bones, NUM_MATRICES);
            write_vert(&mut emu, &sc, &v);
            let _ = emu.call(0x71a720, sc.model as u32, sc.batch as u32, &[sc.out as u32]);
            let tex = [f32::from_bits(v.tail[0]), f32::from_bits(v.tail[1])];
            let ours = m2_vertex_transform(v.pos, v.normal, v.weights, v.indices, tex, &bones);
            for i in 0..8 {
                let got = emu.read_f32(sc.out + 4 * i as u64);
                assert_f32(got, ours[i], &format!("out[{i}] bones={n_bones}"));
            }
        }
    }
}

/// `0x71a460` whole on one vertex with zero to four bones, the 0x28-byte output read back.
#[test]
fn m2_vertex_skin_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut rng = Lcg(0x736b_696e);
    let mats = gen_matrices(&mut rng);
    let sc = build_scene(&mut emu, &mats);
    let bones = |i: u8| mats[i as usize];
    for n_bones in 0..=4usize {
        for _ in 0..120 {
            let v = gen_vert(&mut rng, n_bones, NUM_MATRICES);
            write_vert(&mut emu, &sc, &v);
            let _ = emu.call(0x71a460, sc.model as u32, sc.batch as u32, &[sc.out as u32]);
            let ours = m2_vertex_skin(v.pos, v.normal, v.weights, v.indices, v.tail, &bones);
            for i in 0..10 {
                let got = emu.read_f32(sc.out + 4 * i as u64);
                assert_f32(got, ours[i], &format!("out[{i}] bones={n_bones}"));
            }
        }
    }
}

/// `0x71a9e0` whole on one vertex with an aligned output, zero to four bones.
#[test]
fn m2_vertex_transform_sse_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut rng = Lcg(0x7373_6573);
    let mats = gen_matrices(&mut rng);
    let sc = build_scene(&mut emu, &mats);
    assert_eq!(sc.out & 0xf, 0, "the output must be 16-byte aligned");
    let bones = |i: u8| mats[i as usize];
    for n_bones in 0..=4usize {
        for _ in 0..120 {
            let v = gen_vert(&mut rng, n_bones, NUM_MATRICES);
            write_vert(&mut emu, &sc, &v);
            let _ = emu.call(0x71a9e0, sc.model as u32, sc.batch as u32, &[sc.out as u32]);
            let tex = [f32::from_bits(v.tail[0]), f32::from_bits(v.tail[1])];
            let ours = m2_vertex_transform_sse(v.pos, v.normal, v.weights, v.indices, tex, &bones);
            for i in 0..8 {
                let got = emu.read_f32(sc.out + 4 * i as u64);
                assert_f32(got, ours[i], &format!("out[{i}] bones={n_bones}"));
            }
        }
    }
}

/// The instance, shared model, scene record and buffers `0x719c20` reads.
struct DecalScene {
    model: u64,
    batch: u64,
    aabb_out: u64,
    texgen_out: u64,
    vert_base: u64,
    shared: u64,
    bone_base: u64,
}

/// The instance `+0x2c` → scene record (`+0xdc` the matrix, `+0x10c` its translation), `+0x30` →
/// shared model, `+0x94` → bones; the skin index map the identity.
fn alloc_decal_scene(emu: &mut Emu, n_matrices: usize, n_verts: usize) -> DecalScene {
    let model = emu.alloc(0x100);
    let skel = emu.alloc(0x140);
    let vbd = emu.alloc(0x50);
    let shared = emu.alloc(0x140);
    let bone_base = emu.alloc(0x40 * n_matrices as u64);
    let vert_base = emu.alloc(0x30 * n_verts as u64);
    let skin_base = emu.alloc(2 * n_verts as u64 + 8);
    let skin_obj = emu.alloc(0x10);
    let batch = emu.alloc(0x10);
    let aabb_out = emu.alloc(0x20);
    let texgen_out = emu.alloc(0x40);
    emu.write_u32(model + 0x2c, shared as u32);
    emu.write_u32(model + 0x30, skel as u32);
    emu.write_u32(model + 0x94, bone_base as u32);
    emu.write_u32(skel + 0x130, vbd as u32);
    emu.write_u32(skel + 0x138, skin_obj as u32);
    emu.write_u32(vbd + 0x48, vert_base as u32);
    emu.write_u32(skin_obj + 4, skin_base as u32);
    for i in 0..n_verts {
        emu.write(skin_base + 2 * i as u64, &(i as u16).to_le_bytes());
    }
    DecalScene {
        model,
        batch,
        aabb_out,
        texgen_out,
        vert_base,
        shared,
        bone_base,
    }
}

/// Runs `0x719c20` on `verts` and compares the box, the result and, when 1, the texture matrix.
fn run_decal(
    emu: &mut Emu,
    sc: &DecalScene,
    mats: &[[f32; 16]],
    world: &[f32; 16],
    verts: &[Vert],
    ctx: &str,
) {
    for (j, &f) in world.iter().enumerate() {
        emu.write_u32(sc.shared + 0xdc + 4 * j as u64, f.to_bits());
    }
    write_matrices(emu, sc.bone_base, mats);
    for (i, v) in verts.iter().enumerate() {
        write_vert_at(emu, sc.vert_base + 0x30 * i as u64, v);
    }
    emu.write_u32(sc.batch + 4, 0);
    emu.write_u32(sc.batch + 6, verts.len() as u32);
    let args = [sc.batch as u32, sc.aabb_out as u32, sc.texgen_out as u32];
    let ret = emu.call(0x719c20, sc.model as u32, 0, &args);

    let bones = |i: u8| mats[i as usize];
    let dverts: Vec<DecalVert> = verts
        .iter()
        .map(|v| DecalVert {
            pos: v.pos,
            weights: v.weights,
            indices: v.indices,
            texcoord: [f32::from_bits(v.tail[0]), f32::from_bits(v.tail[1])],
        })
        .collect();
    // The record's `+0x10c` is the matrix's translation.
    let pivot = [world[12], world[13], world[14]];
    let ours = m2_decal_projection(verts.len() as u16, &dverts, &bones, world, pivot);
    for i in 0..3 {
        let o = 4 * i as u64;
        assert_f32(
            emu.read_f32(sc.aabb_out + o),
            ours.aabb_min[i],
            &format!("{ctx} min[{i}]"),
        );
        let got = emu.read_f32(sc.aabb_out + 0xc + o);
        assert_f32(got, ours.aabb_max[i], &format!("{ctx} max[{i}]"));
    }
    assert_eq!(ret != 0, ours.accepted, "{ctx} result");
    if ours.accepted {
        for i in 0..16 {
            let got = emu.read_f32(sc.texgen_out + 4 * i as u64);
            assert_f32(got, ours.texgen[i], &format!("{ctx} texgen[{i}]"));
        }
    }
}

/// `0x719c20` whole, its matrix product `0x74a7c8` run through the thunk slot `0x876504`. Random
/// submeshes of 4, 5 and 8 vertices; and with identity bones and scene, first-three vertices whose
/// y are a NaN, a tie, or span just under 2.3841858e-7 (the difference rounds to it as `f32`).
#[test]
fn m2_decal_projection_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.write_u32(0x0087_6504, 0x0074_a7c8);
    let mut rng = Lcg(0x6465_6361);

    let identity: [f32; 16] = std::array::from_fn(|i| if i % 5 == 0 { 1.0 } else { 0.0 });
    let sc = alloc_decal_scene(&mut emu, NUM_MATRICES as usize, 4);
    let mats = vec![identity; NUM_MATRICES as usize];
    let at = |x: f32, y: f32| Vert {
        pos: [x, y, 0.5],
        weights: [255, 0, 0, 0],
        indices: [0; 4],
        normal: [0.0; 3],
        tail: [0.25f32.to_bits(), 0.75f32.to_bits(), 0, 0],
    };
    let tiny = f32::from_bits(0x2680_0000); // 2^-50
    let eps = f32::from_bits(0x3480_0000);
    let nan = f32::NAN;
    for (ys, ctx) in [
        (
            [tiny, eps, f32::from_bits(0x3080_0000)],
            "span just under eps",
        ),
        ([0.0, eps, 1.0e-9], "span exactly eps"),
        ([nan, 1.0, 2.0], "NaN first"),
        ([1.0, nan, 2.0], "NaN second"),
        ([1.0, 1.0, 2.0], "tie low"),
        ([2.0, 1.0, 1.0], "tie high"),
        ([1.0, 1.0, 1.0], "all equal"),
    ] {
        let verts = [at(0.0, ys[0]), at(1.0, ys[1]), at(2.0, ys[2]), at(5.0, 5.0)];
        run_decal(&mut emu, &sc, &mats, &identity, &verts, ctx);
    }

    for &n_verts in &[4usize, 5, 8] {
        let sc = alloc_decal_scene(&mut emu, NUM_MATRICES as usize, n_verts);
        for i in 0..200 {
            let mats = gen_matrices(&mut rng);
            let world: [f32; 16] = std::array::from_fn(|_| rng.f32_mixed());
            let verts: Vec<Vert> = (0..n_verts)
                .map(|k| gen_vert(&mut rng, (k % 4) + 1, NUM_MATRICES))
                .collect();
            let ctx = format!("fuzz[{i}] n={n_verts}");
            run_decal(&mut emu, &sc, &mats, &world, &verts, &ctx);
        }
    }
}

/// Copies `[src, src+len)` into an island, appends `fstp qword [out]`, and returns the end.
fn island_f64(e: &mut Emu, isl: u64, src: u64, len: usize, out: u64) -> u64 {
    let mut code = e.read(src, len);
    code.extend_from_slice(&[0xdd, 0x1d]);
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(e, isl, src, len);
    isl + code.len() as u64
}

/// `0x70c190` at `0x70c256` to `0x70c25f` in an island with `fstp qword` appended: the material at
/// `ecx` (`+0x10`), the product read as the `f64` `__ftol` takes.
#[test]
fn color_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let mat = e.alloc(0x40);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x40);
    let end = island_f64(&mut e, isl, 0x70c256, 0x70c25f - 0x70c256, out);

    let run_case = |e: &mut Emu, v: f32, ctx: &str| {
        e.write_u32(mat + 0x10, v.to_bits());
        e.set_reg(RegisterX86::ECX, mat);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(isl, end);
        assert_f64(read_f64(e, out), color_scale(v), ctx);
    };
    for v in [0.0f32, 1.0, -1.0, 0.5, 2.0, -0.0, f32::NAN] {
        run_case(&mut e, v, &format!("v={v}"));
    }
    let mut s = 0x70c2_5959u32;
    for i in 0..3000u32 {
        let v = frand(&mut s, -2.0, 2.0);
        run_case(&mut e, v, &format!("fuzz[{i}]"));
    }
}

/// `0x70c190` from `0x70c36a` to `0x70c38a` in an island with `fstp qword` appended: the vector at
/// `eax`. The copy at `0x70c56f` has the same bytes from its first multiply on.
#[test]
fn normalize_residual_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let rep = e.read(0x70c372, 0x70c390 - 0x70c372);
    assert_eq!(
        rep,
        e.read(0x70c57f, 0x70c59d - 0x70c57f),
        "the copy at 0x70c56f"
    );
    let xyz = e.alloc(0x10);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x40);
    let end = island_f64(&mut e, isl, 0x70c36a, 0x70c38a - 0x70c36a, out);

    let run_case = |e: &mut Emu, x: f32, y: f32, z: f32, ctx: &str| {
        for (k, v) in [x, y, z].into_iter().enumerate() {
            e.write_u32(xyz + 4 * k as u64, v.to_bits());
        }
        e.set_reg(RegisterX86::EAX, xyz);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(isl, end);
        assert_f64(read_f64(e, out), normalize_residual(x, y, z), ctx);
    };
    run_case(&mut e, 1.0, 0.0, 0.0, "unit x");
    run_case(&mut e, 0.0, 1.0, 0.0, "unit y");
    run_case(&mut e, 0.0, 0.0, 0.0, "zero");
    let k = (1.0f64 / 3.0f64.sqrt()) as f32;
    run_case(&mut e, k, k, k, "diagonal");
    run_case(&mut e, f32::NAN, 0.0, 0.0, "NaN");
    let mut s = 0x70c3_6a6au32;
    for i in 0..4000u32 {
        let (x, y, z) = (
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
        );
        run_case(&mut e, x, y, z, &format!("fuzz[{i}]"));
    }
}

/// The multiplies at `0x70c631`, `0x70c63f`, `0x70c64b` and the adds at `0x70c669`, `0x70c67e`,
/// `0x70c687` are `d8` operations on a `dword`; each is run as `fld dword`, the operation with an
/// absolute operand, `fstp dword`.
#[test]
fn scale_add_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    for site in [
        0x70c631u64,
        0x70c63f,
        0x70c64b,
        0x70c669,
        0x70c67e,
        0x70c687,
    ] {
        assert_eq!(e.read(site, 1), vec![0xd8], "{site:#x}");
    }
    let a_mem = e.alloc(0x10);
    let b_mem = e.alloc(0x10);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x40);
    let run = |e: &mut Emu, modrm: u8, a: f32, b: f32| -> f32 {
        let mut code = vec![0xd9u8, 0x05];
        code.extend_from_slice(&(a_mem as u32).to_le_bytes());
        code.extend_from_slice(&[0xd8, modrm]);
        code.extend_from_slice(&(b_mem as u32).to_le_bytes());
        code.extend_from_slice(&[0xd9, 0x1d]);
        code.extend_from_slice(&(out as u32).to_le_bytes());
        e.write(isl, &code);
        e.write_u32(a_mem, a.to_bits());
        e.write_u32(b_mem, b.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(isl, isl + code.len() as u64);
        e.read_f32(out)
    };
    let check = |e: &mut Emu, a: f32, b: f32, ctx: &str| {
        assert_f32(run(e, 0x0d, a, b), scale_f32(a, b), &format!("{ctx} mul"));
        assert_f32(run(e, 0x05, a, b), add_f32(a, b), &format!("{ctx} add"));
    };
    check(&mut e, 0.0, 5.0, "0, 5");
    check(&mut e, 1.0, -1.0, "1, −1");
    check(&mut e, 0.5, 0.5, "halves");
    let mut s = 0x70c6_3131u32;
    for i in 0..2500u32 {
        let (a, b) = (frand(&mut s, -10.0, 10.0), frand(&mut s, -10.0, 10.0));
        check(&mut e, a, b, &format!("fuzz[{i}]"));
    }
}

/// `0x70c190` from `0x70c77e` to `0x70c7c0` in an island with `fstp qword` appended: the channel
/// at `[ebp-0x14]`, read as the `f64` `__ftol` takes. The seven other channels' multiply and add
/// are checked to be the same bytes. Exactly 0 and 1, one step below 1, and NaN probe the ladder.
#[test]
fn rgba_pack_value_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pack = vec![
        0xd8u8, 0x0d, 0x58, 0xfe, 0x7f, 0x00, 0xd8, 0x05, 0x24, 0xfa, 0x7f, 0x00,
    ];
    for site in [
        0x70c7b4u64,
        0x70c7fb,
        0x70c842,
        0x70c88c,
        0x70c8e5,
        0x70c92c,
        0x70c973,
    ] {
        assert_eq!(e.read(site, 12), pack, "{site:#x}");
    }
    assert_eq!(e.read(0x70c9f8, 6), pack[..6].to_vec(), "0x70c9f8");
    assert_eq!(e.read(0x70ca01, 6), pack[6..].to_vec(), "0x70ca01");

    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let end = island_f64(&mut e, isl, 0x70c77e, 0x70c7c0 - 0x70c77e, out);
    let run_case = |e: &mut Emu, v: f32, ctx: &str| {
        e.write_u32(ebp - 0x14, v.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(isl, end);
        assert_f64(read_f64(e, out), rgba_pack_value(v), ctx);
    };
    for v in [
        -1.0f32,
        0.0,
        -0.0,
        1.0,
        2.0,
        0.5,
        f32::from_bits(0x3f7f_ffff),
        f32::from_bits(1),
    ] {
        run_case(&mut e, v, &format!("v={v}"));
    }
    run_case(&mut e, f32::NAN, "NaN");
    let mut s = 0x70c7_7e7eu32;
    for i in 0..4000u32 {
        let v = match (next(&mut s) >> 8) % 3 {
            0 => frand(&mut s, 0.0, 1.0),
            1 => frand(&mut s, -1.0, 0.0),
            _ => frand(&mut s, 1.0, 3.0),
        };
        run_case(&mut e, v, &format!("fuzz[{i}]"));
    }
}

/// `0x70baf0` from `0x70bdbb` to `0x70bddf`: the record at `eax`, the outputs at `esi+0x230` and
/// `+0x234`.
#[test]
fn batch_alpha_resolve_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let light = e.alloc(0x200);
    let this = e.alloc(0x400);
    let run_case = |e: &mut Emu, l188: f32, l18c: f32, ctx: &str| {
        e.write_u32(light + 0x188, l188.to_bits());
        e.write_u32(light + 0x18c, l18c.to_bits());
        e.write_u32(this + 0x230, 0xdead_beef);
        e.write_u32(this + 0x234, 0xdead_beef);
        e.set_reg(RegisterX86::EAX, light);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x70bdbb, 0x70bddf);
        let ours = batch_alpha_resolve(l188, l18c);
        assert_f32(e.read_f32(this + 0x230), ours[0], &format!("{ctx} +0x230"));
        assert_f32(e.read_f32(this + 0x234), ours[1], &format!("{ctx} +0x234"));
    };
    run_case(&mut e, 2.0, 3.0, "generic");
    run_case(&mut e, 0.0, 0.0, "zero");
    run_case(&mut e, -5.0, 7.0, "negative");
    run_case(&mut e, 1.0, f32::NAN, "NaN");
    let mut rng = Lcg(0x616c_7068);
    for i in 0..3000u32 {
        let (a, b) = (rng.f32_mixed(), rng.f32_mixed());
        run_case(&mut e, a, b, &format!("fuzz[{i}]"));
    }
}

/// `0x70baf0` from `0x70beb5` to `0x70bfaa`: the record at `ecx` (`+0x190` to `+0x198`); the word
/// stored through `[ebp-0x8]`, which the stretch points at its own push.
#[test]
fn batch_clamp_pack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let base = e.alloc(0x400);
    let frame = e.alloc(0x100);
    let pack_slot = SLICE_ESP - 4;
    let run_case = |e: &mut Emu, c198: f32, c194: f32, c190: f32, ctx: &str| {
        e.write_u32(base + 0x198, c198.to_bits());
        e.write_u32(base + 0x194, c194.to_bits());
        e.write_u32(base + 0x190, c190.to_bits());
        e.write_u32(pack_slot, 0xdead_beef);
        e.set_reg(RegisterX86::ECX, base);
        e.set_reg(RegisterX86::EBP, frame + 0x80);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x70beb5, 0x70bfaa);
        let bin = e.read_u32(pack_slot);
        let ours = batch_clamp_pack(c198, c194, c190);
        assert_eq!(bin, ours, "{ctx}: binary={bin:#010x} ours={ours:#010x}");
    };
    run_case(&mut e, 0.0, 0.0, 0.0, "zero");
    run_case(&mut e, 1.0, 1.0, 1.0, "one");
    run_case(&mut e, 0.5, 0.5, 0.5, "half");
    run_case(&mut e, -1.0, -1.0, -1.0, "negative");
    run_case(&mut e, 2.0, 2.0, 2.0, "two");
    run_case(&mut e, 0.1, 0.5, 0.9, "distinct");
    run_case(&mut e, f32::NAN, 0.5, 1.0, "NaN");
    let mut rng = Lcg(0x7063_6b31);
    for i in 0..3000u32 {
        let (a, b, c) = (rng.clamp_chan(), rng.clamp_chan(), rng.clamp_chan());
        run_case(&mut e, a, b, c, &format!("fuzz[{i}]"));
    }
}

/// `0x70baf0` from `0x70c10f` to `0x70c147`: the plane at `[ebp-0x28]` negated in place.
#[test]
fn batch_negate4_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let run_case = |e: &mut Emu, v: [f32; 4], ctx: &str| {
        for k in 0..4 {
            e.write_u32(ebp - 0x28 + 4 * k as u64, v[k].to_bits());
        }
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x70c10f, 0x70c147);
        let ours = batch_negate4(v);
        for k in 0..4 {
            let got = e.read_f32(ebp - 0x28 + 4 * k as u64);
            assert_f32(got, ours[k], &format!("{ctx} [{k}]"));
        }
    };
    run_case(&mut e, [1.0, -2.0, 3.0, -4.0], "generic");
    run_case(&mut e, [0.0, -0.0, 0.0, -0.0], "zeros");
    run_case(
        &mut e,
        [f32::NAN, f32::INFINITY, -f32::INFINITY, 1.0],
        "special",
    );
    let mut rng = Lcg(0x6e65_6734);
    for i in 0..3000u32 {
        let v = [
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
        ];
        run_case(&mut e, v, &format!("fuzz[{i}]"));
    }
}

/// `0x70ca50` from `0x70ca9b` to `0x70caef`: the matrix at `[ebp-0x40]`, the offset at `eax`; the
/// translation read from `[ebp-0x10]`.
#[test]
fn model_matrix_translate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let vec = e.alloc(0x20);
    let run_case = |e: &mut Emu, m: [f32; 16], v: [f32; 3], ctx: &str| {
        for (i, &f) in m.iter().enumerate() {
            e.write_u32(ebp - 0x40 + 4 * i as u64, f.to_bits());
        }
        for k in 0..3 {
            e.write_u32(vec + 4 * k as u64, v[k].to_bits());
        }
        e.set_reg(RegisterX86::EAX, vec);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x70ca9b, 0x70caef);
        let ours = model_matrix_translate(m, v);
        for k in 0..3 {
            let got = e.read_f32(ebp - 0x10 + 4 * k as u64);
            assert_f32(got, ours[12 + k], &format!("{ctx} [{}]", 12 + k));
        }
    };
    let mut ident: [f32; 16] = std::array::from_fn(|i| if i % 5 == 0 { 1.0 } else { 0.0 });
    ident[12..15].copy_from_slice(&[7.0, -3.0, 11.0]);
    run_case(&mut e, ident, [2.0, 5.0, -4.0], "identity");
    run_case(&mut e, ident, [0.0; 3], "zero offset");
    run_case(&mut e, [0f32; 16], [1.0, 2.0, 3.0], "zero matrix");
    let mut rng = Lcg(0x70ca_5000);
    for i in 0..4000u32 {
        let m: [f32; 16] = std::array::from_fn(|_| rng.f32_mixed());
        let v = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        run_case(&mut e, m, v, &format!("fuzz[{i}]"));
    }
}

/// The inputs [`decal_texgen_pack`] takes.
#[derive(Clone, Copy)]
struct TexgenIn {
    aabb_min: f32,
    aabb_max: f32,
    ref_depth: f32,
    mrow: [f32; 3],
    scale: [f32; 3],
    cond: bool,
    offset: [f32; 3],
    comp3: f32,
}

/// `0x70cf70` from `0x70d07e` to `0x70d2d0`: the instance at `esi` and the records it points to
/// written, the depth range, scale, offset, colour and word read from the frame. Each colour
/// channel at exactly 0 and 1, and NaN, probe the ladder.
#[test]
fn decal_texgen_pack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;
    let this = e.alloc(0x4000);
    let ivar4 = e.alloc(0x400);
    let edi = e.alloc(0x400);
    let ptr_q = e.alloc(0x40);
    let ptr_r = e.alloc(0x80);
    let ptr_p = e.alloc(0x40);
    let ptr_s = e.alloc(0x4000);
    e.write_u32(this + 0x3310, ivar4 as u32);
    e.write_u32(this + 0x40, edi as u32);
    e.write_u32(this + 0x3338, ptr_q as u32);
    e.write_u32(this + 0x48, ptr_r as u32);
    e.write_u32(this + 0x3300, ptr_p as u32);
    e.write_u32(ivar4 + 0xa0, ptr_s as u32);

    let run_case = |e: &mut Emu, t: TexgenIn, ctx: &str| {
        e.write_u32(ebp - 0x20, t.aabb_min.to_bits());
        e.write_u32(ebp - 0x14, t.aabb_max.to_bits());
        e.write_u32(edi + 0x114, t.ref_depth.to_bits());
        for k in 0..3 {
            e.write_u32(ivar4 + 0x1a0 + 4 * k as u64, t.mrow[k].to_bits());
            e.write_u32(ivar4 + 0x1ac + 4 * k as u64, t.offset[k].to_bits());
        }
        e.write_u32(ptr_p + 0x10, t.comp3.to_bits());
        // The scale applies when the word at `[esi+0x3338]+8` is below `[esi+0x48]+0x54`.
        let (index, limit): (u16, u32) = if t.cond { (1, 5) } else { (5, 1) };
        e.write_u32(ptr_q + 8, u32::from(index));
        e.write_u32(ptr_r + 0x54, limit);
        let sbase = ptr_s + u64::from(index) * 0x50 + 0xc;
        for k in 0..3 {
            e.write_u32(sbase + 4 * k as u64, t.scale[k].to_bits());
        }
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x70d07e, 0x70d2d0);
        let bin = DecalTexgen {
            plane_min: e.read_f32(ebp - 0x20),
            plane_max: e.read_f32(ebp - 0x14),
            inv_range: e.read_f32(ebp - 0x48),
            depth: e.read_f32(ebp - 0x38),
            comp_y: e.read_f32(ebp - 0x8),
            comp_z: e.read_f32(ebp - 0x4),
            packed: e.read_u32(ebp - 0x10),
        };
        let ours = decal_texgen_pack(
            t.aabb_min,
            t.aabb_max,
            t.ref_depth,
            t.mrow,
            t.scale,
            t.cond,
            t.offset,
            t.comp3,
        );
        assert_f32(bin.plane_min, ours.plane_min, &format!("{ctx} min"));
        assert_f32(bin.plane_max, ours.plane_max, &format!("{ctx} max"));
        assert_f32(bin.inv_range, ours.inv_range, &format!("{ctx} inv"));
        assert_f32(bin.depth, ours.depth, &format!("{ctx} depth"));
        assert_f32(bin.comp_y, ours.comp_y, &format!("{ctx} y"));
        assert_f32(bin.comp_z, ours.comp_z, &format!("{ctx} z"));
        assert_eq!(
            bin.packed, ours.packed,
            "{ctx} word {:#010x} {:#010x}",
            bin.packed, ours.packed
        );
    };

    let base = TexgenIn {
        aabb_min: 0.0,
        aabb_max: 100.0,
        ref_depth: 50.0,
        mrow: [0.5, -0.3, 1.7],
        scale: [1.0; 3],
        cond: false,
        offset: [0.0; 3],
        comp3: 0.5,
    };
    run_case(&mut e, base, "generic");
    let scaled = TexgenIn {
        aabb_min: -10.0,
        aabb_max: 10.0,
        ref_depth: 0.0,
        mrow: [0.25, 0.5, 0.75],
        scale: [2.0, 0.5, 1.5],
        cond: true,
        offset: [0.1, -0.2, 0.3],
        comp3: 1.0,
    };
    run_case(&mut e, scaled, "scaled");
    for (mrow, comp3, ctx) in [
        ([0.0, 1.0, -0.0], 0.0, "0 and 1"),
        ([f32::NAN, 0.5, 0.5], f32::NAN, "NaN"),
        ([1.0, f32::from_bits(0x3f7f_ffff), 2.0], 1.0, "around 1"),
    ] {
        run_case(
            &mut e,
            TexgenIn {
                mrow,
                comp3,
                ..base
            },
            ctx,
        );
    }
    let mut rng = Lcg(0x6463_6770);
    for i in 0..4000u32 {
        let t = TexgenIn {
            aabb_min: rng.f32_mixed(),
            aabb_max: rng.f32_mixed(),
            ref_depth: rng.f32_mixed(),
            mrow: [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()],
            scale: [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()],
            cond: (rng.next() >> 16) & 1 == 0,
            offset: [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()],
            comp3: rng.f32_mixed(),
        };
        run_case(&mut e, t, &format!("fuzz[{i}]"));
    }
}

/// `0x70d330` from `0x70d717` to `0x70d73d` in an island with `fstp qword` appended (the
/// deviation), and from `0x70d717` to `0x70d751` with `[ebp-0xc]` preset to 1 (the verdict). A
/// deviation just above 0.001 that rounds to it as `f32`, and NaN, probe the compare.
#[test]
fn unit_length_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let dev_out = e.alloc(0x10);
    let island = e.alloc(0x80);
    let island_end = island_f64(&mut e, island, 0x70d717, 0x70d73d - 0x70d717, dev_out);
    let vert = e.alloc(0x40);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, v: [f32; 3], ctx: &str| {
        for (i, &f) in v.iter().enumerate() {
            e.write_u32(vert + 4 * i as u64, f.to_bits());
        }
        e.set_reg(RegisterX86::EAX, vert);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(island, island_end);
        let bin_dev = read_f64(e, dev_out);
        e.write_u32(ebp - 0xc, 1);
        e.set_reg(RegisterX86::EAX, vert);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x70d717, 0x70d751);
        let cleared = e.read_u32(ebp - 0xc) == 0;
        let ours = unit_length_gate(v);
        assert_f64(bin_dev, ours.deviation, &format!("{ctx} deviation"));
        assert_eq!(cleared, ours.flag_cleared, "{ctx} cleared");
    };

    let above = [
        f32::from_bits(0x3f80_105e),
        f32::from_bits(0x3a67_07fa),
        0.0,
    ];
    let dev = unit_length_gate(above).deviation;
    assert!(dev > f64::from(UNIT_LEN_DELTA) && dev as f32 == UNIT_LEN_DELTA);
    run_case(&mut e, above, "just above 0.001");
    run_case(&mut e, [1.0, 0.0, 0.0], "unit x");
    let k = 0.577_350_3;
    run_case(&mut e, [k, k, k], "diagonal");
    run_case(&mut e, [0.0; 3], "zero");
    run_case(&mut e, [2.0, 0.0, 0.0], "length 2");
    let near = |f: f32| (1.0f32 + UNIT_LEN_DELTA * f).sqrt();
    run_case(&mut e, [near(0.5), 0.0, 0.0], "half the tolerance");
    run_case(&mut e, [near(2.0), 0.0, 0.0], "twice the tolerance");
    run_case(&mut e, [f32::NAN, 0.0, 0.0], "NaN");

    let mut rng = Lcg(0x70d3_3000);
    for i in 0..3000u32 {
        let dir = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        let v = if len > 1e-12 {
            let s = (1.0 + (rng.f32_mixed() * 0.01)) / len;
            dir.map(|c| c * s)
        } else {
            dir
        };
        run_case(&mut e, v, &format!("near unit[{i}]"));
    }
    for i in 0..3000u32 {
        let v = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        run_case(&mut e, v, &format!("fuzz[{i}]"));
    }
}
