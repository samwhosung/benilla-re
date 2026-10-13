//! The draw side (`systems/models/particles.md#particle-view-space`,
//! `systems/models/particles.md#draw-order-and-batches`, `systems/models/particles.md#head-quads`,
//! `systems/models/model.md#reference-frame`): slices of the render, the model particle draw, the
//! sort keys, the frame and grid setters, and the particles' turn with the frame.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::particles::draw::{
    alpha_distance_scale, color_byte_to_f32, depth_dot, flag200_scale, lod_index_scale,
    mat3x4_transform, orient_velocity, quat_billboard_matrix, render_negate, render_normalize,
    tile_grid_recip, tile_grid_valid, transform_basis_scale, Mat3x4Inputs, RECIP_ONE,
};
use common::{assert_f32, frand, next, SLICE_ESP};

const IN_BASE: u64 = 0x00cf_5af8;

const OUT_BASE: u64 = 0x00cf_5b2c;

const M_VAS: [u64; 9] = [
    0x00cf_5888,
    0x00cf_588c,
    0x00cf_5890,
    0x00cf_5898,
    0x00cf_589c,
    0x00cf_58a0,
    0x00cf_58a8,
    0x00cf_58ac,
    0x00cf_58b0,
];

const NORM_EPS2: f64 = 2.384_185_8e-7;

struct OrientRng(u32);

impl OrientRng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A finite f32 in a mixed range, from the LCG's high bits.
    fn f32_mixed(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 5;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,
            1 => (m * 20.0 - 10.0) as f32,
            2 => (m * 2000.0 - 1000.0) as f32,
            3 => (m * 1.0e6) as f32,
            _ => (m * 1.0e-6) as f32,
        }
    }
}

const AXIS_X: u64 = 0xcf5b70;

const AXIS_Y: u64 = 0xcf5b80;

const AXIS_Z: u64 = 0xcf5b90;

const AXIS_OFF: u64 = 0xcf5ba0;

/// `0x7b4549..0x7b4560`: the twinkle index.
#[test]
fn lod_index_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emit = e.alloc(0x300); // edi
    let part = e.alloc(0x40); // esi
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, scale: f32, radius: f32, ctx: &str| {
        e.write_u32(emit + 0x1b0, scale.to_bits());
        e.write_u32(part + 0x1c, radius.to_bits());
        e.set_reg(RegisterX86::EDI, emit);
        e.set_reg(RegisterX86::ESI, part);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // 0x7b4549 fld[edi+0x1b0]; mov eax,esi; fmul[esi+0x1c]; shr; fadd[512]; fstp[ebp+0x8] @
        // 0x7b455d. `run` is exclusive of `until`, so extend PAST the store to 0x7b4560 (the next
        // `mov ebx,[ebp+0x8]`); ending at 0x7b455d would read the slot before the store.
        e.run(0x7b4549, 0x7b4560);
        let bin = e.read_f32(ebp + 0x8);
        assert_f32(bin, lod_index_scale(scale, radius), ctx);
    };

    run_case(&mut e, 0.0, 0.0, "lod zero");
    run_case(&mut e, 1.0, 1.0, "lod 1·1+512");
    run_case(&mut e, -1.0, 1024.0, "lod neg");
    let mut s = 0x7b45_4949u32;
    for i in 0..3000u32 {
        let sc = frand(&mut s, -2000.0, 2000.0);
        let r = frand(&mut s, -2000.0, 2000.0);
        run_case(&mut e, sc, r, &format!("lod fuzz[{i}]"));
    }
}

/// `0x7b45f8..0x7b4611`: the twinkle size.
#[test]
fn alpha_distance_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emit = e.alloc(0x300);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    const LOD_TABLE: u64 = 0x00cf_58f0;

    // ebx is the LOD index; the slice reads [4*ebx + 0xcf58f0]. Use a fixed index and write the
    // entry there.
    let idx: u64 = 7;

    let run_case = |e: &mut Emu, table: f32, k: f32, bias: f32, param: f32, ctx: &str| {
        e.write_u32(LOD_TABLE + 4 * idx, table.to_bits());
        e.write_u32(emit + 0x1c0, k.to_bits());
        e.write_u32(emit + 0x1b8, bias.to_bits());
        e.write_u32(ebp + 0x8, param.to_bits());
        e.set_reg(RegisterX86::EDI, emit);
        e.set_reg(RegisterX86::EBX, idx);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // 0x7b45f8 fld[4*ebx+0xcf58f0]; fmul[edi+0x1c0]; fadd[edi+0x1b8]; fmul[ebp+0x8];
        // fstp[ebp+0x8]. Halt at 0x7b4611 (mov eax,[edi+0x1ac], the BB after the store).
        e.run(0x7b45f8, 0x7b4611);
        let bin = e.read_f32(ebp + 0x8);
        assert_f32(bin, alpha_distance_scale(table, k, bias, param), ctx);
    };

    run_case(&mut e, 0.0, 0.0, 0.0, 0.0, "alpha zero");
    run_case(&mut e, 1.0, 2.0, 3.0, 4.0, "alpha (1·2+3)·4");
    run_case(&mut e, -1.0, -1.0, 0.5, -2.0, "alpha neg");
    let mut s = 0x7b45_f8f8u32;
    for i in 0..3000u32 {
        let table = frand(&mut s, -10.0, 10.0);
        let k = frand(&mut s, -5.0, 5.0);
        let bias = frand(&mut s, -5.0, 5.0);
        let param = frand(&mut s, -100.0, 100.0);
        run_case(&mut e, table, k, bias, param, &format!("alpha fuzz[{i}]"));
    }
}

/// `0x7b461c..0x7b4628`: the flag 0x200 scale.
#[test]
fn flag200_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emit = e.alloc(0x300);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, param: f32, mul: f32, ctx: &str| {
        e.write_u32(ebp + 0x8, param.to_bits());
        e.write_u32(emit + 0x264, mul.to_bits());
        e.set_reg(RegisterX86::EDI, emit);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // 0x7b461c fld[ebp+0x8]; fmul[edi+0x264]; fstp[ebp+0x8]; halt at 0x7b4628 (fld[esi+0x20]).
        e.run(0x7b461c, 0x7b4628);
        let bin = e.read_f32(ebp + 0x8);
        assert_f32(bin, flag200_scale(param, mul), ctx);
    };

    run_case(&mut e, 0.0, 0.0, "f200 zero");
    run_case(&mut e, 3.0, 2.0, "f200 3·2");
    run_case(&mut e, -1.5, 4.0, "f200 neg");
    let mut s = 0x7b46_1c1cu32;
    for i in 0..3000u32 {
        let param = frand(&mut s, -100.0, 100.0);
        let mul = frand(&mut s, -10.0, 10.0);
        run_case(&mut e, param, mul, &format!("f200 fuzz[{i}]"));
    }
}

/// `0x7b4628..0x7b46ee`: the model particle's rotation.
#[test]
fn quat_billboard_matrix_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let part = e.alloc(0x40); // esi (quat at +0x20..+0x2c)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, q: [f32; 4], ctx: &str| {
        for (k, c) in q.iter().enumerate() {
            e.write_u32(part + 0x20 + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::ESI, part);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // 0x7b4628 fld[esi+0x20]; sub esp,0x24; ... the 9 matrix elements -> [esp+0x20]..[esp];
        // 0x7b46ec fstp st(0) (cleanup); halt at 0x7b46ee (call 0x5f8d20). The `sub esp,0x24` puts
        // the stores at SLICE_ESP-0x24 + {0x20..0x0}.
        e.run(0x7b4628, 0x7b46ee);
        let base = SLICE_ESP - 0x24;
        // m0..m8 are stored at [esp+0x20], [esp+0x1c], ..., [esp].
        let bin: [f32; 9] = std::array::from_fn(|i| e.read_f32(base + (0x20 - 4 * i as u64)));
        let ours = quat_billboard_matrix(q);
        for k in 0..9 {
            assert_f32(bin[k], ours[k], &format!("{ctx} m[{k}]"));
        }
    };

    // structured: identity, unit-axis quats, and a normalized arbitrary quat.
    run_case(&mut e, [0.0, 0.0, 0.0, 1.0], "quat identity");
    run_case(&mut e, [1.0, 0.0, 0.0, 0.0], "quat i");
    run_case(&mut e, [0.0, 1.0, 0.0, 0.0], "quat j");
    run_case(&mut e, [0.0, 0.0, 1.0, 0.0], "quat k");
    run_case(&mut e, [0.5, 0.5, 0.5, 0.5], "quat half");
    run_case(&mut e, [0.0, 0.0, 0.0, 0.0], "quat zero");

    let mut s = 0x7b46_2828u32;
    for i in 0..3000u32 {
        let q = [
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
        ];
        run_case(&mut e, q, &format!("quat fuzz[{i}]"));
    }
}

/// `0x7b47ec..0x7b47f8`: a colour byte as a float.
#[test]
fn color_byte_to_f32_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, byte: u32, ctx: &str| {
        e.write_u32(ebp + 0xc, byte);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // 0x7b47ec fild[ebp+0xc]; fmul[0x8026c8]; fstp[ebp-0x64]; halt at 0x7b47f8 (call 0x710cf0).
        e.run(0x7b47ec, 0x7b47f8);
        let bin = e.read_f32(ebp - 0x64);
        assert_f32(bin, color_byte_to_f32(byte), ctx);
    };

    // every byte value 0..=255 plus a few wider ints (the fild reads a signed dword).
    for b in 0u32..=255 {
        run_case(&mut e, b, &format!("color byte {b}"));
    }
    run_case(&mut e, 0, "color 0");
    run_case(&mut e, 255, "color 255");
}

/// `0x7b3e51..0x7b3e90`: the translation row.
#[test]
fn render_negate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x300); // ebx (emit; the three floats at +0x23c/0x240/0x244)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, v: [f32; 3], ctx: &str| {
        e.write_u32(this + 0x23c, v[0].to_bits());
        e.write_u32(this + 0x240, v[1].to_bits());
        e.write_u32(this + 0x244, v[2].to_bits());
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // 0x7b3e51 fld[ebx+0x23c]; fchs; fstp[ebp-0xc]; fld[ebx+0x240]; fchs; fstp[ebp-0x8];
        // fld[ebx+0x244]; fchs; fstp[ebp-0x4]; halt at 0x7b3e90 (mov edx,[ebp-0x4]).
        e.run(0x7b3e51, 0x7b3e90);
        let bin = [
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
            e.read_f32(ebp - 0x4),
        ];
        let ours = render_negate(v);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} neg[{k}]"));
        }
    };

    run_case(&mut e, [0.0, -0.0, 1.0], "negate zero/neg/one");
    run_case(&mut e, [1.0, -1.0, 2.5], "negate ±");
    let mut s = 0x7b3e_5151u32;
    for i in 0..2000u32 {
        let v = [
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        run_case(&mut e, v, &format!("negate fuzz[{i}]"));
    }
}

/// `0x7b41a3..0x7b4249`: the flat corners.
#[test]
fn mat3x4_transform_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, m: [f32; 9], cols: [[f32; 3]; 4], ctx: &str| {
        for (k, c) in m.iter().enumerate() {
            e.write_u32(M_VAS[k], c.to_bits());
        }
        for (col, cc) in cols.iter().enumerate() {
            for (r, c) in cc.iter().enumerate() {
                e.write_u32(IN_BASE + 0xc * col as u64 + 4 * r as u64, c.to_bits());
            }
        }
        // clear the output region so stale bytes can't fake a match.
        for k in 0..12u64 {
            e.write_u32(OUT_BASE + 4 * k, 0xdead_beef);
        }
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // 0x7b41a3 xor eax,eax; the do-while loop body (out=M·in over 4 columns); 0x7b4243 jb back;
        // halt at 0x7b4249 (mov ecx,[0xcf58a8], the BB after the loop).
        e.run(0x7b41a3, 0x7b4249);
        let bin: [[f32; 3]; 4] = std::array::from_fn(|col| {
            std::array::from_fn(|r| e.read_f32(OUT_BASE + 4 * (3 * col as u64 + r as u64)))
        });
        let ours = mat3x4_transform(&Mat3x4Inputs { m, cols });
        for col in 0..4 {
            for r in 0..3 {
                assert_f32(bin[col][r], ours[col][r], &format!("{ctx} out[{col}][{r}]"));
            }
        }
    };

    let identity_m = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0];
    let in_seq: [[f32; 3]; 4] =
        std::array::from_fn(|c| std::array::from_fn(|r| (3 * c + r) as f32));
    run_case(&mut e, identity_m, in_seq, "mat identity-ish");
    run_case(&mut e, [0.0; 9], in_seq, "mat zero-M");

    let mut s = 0x7b41_a3a3u32;
    for i in 0..2500u32 {
        let m: [f32; 9] = std::array::from_fn(|_| frand(&mut s, -2.0, 2.0));
        let cols: [[f32; 3]; 4] = std::array::from_fn(|_| {
            [
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
                frand(&mut s, -100.0, 100.0),
            ]
        });
        run_case(&mut e, m, cols, &format!("mat fuzz[{i}]"));
    }
}

/// `0x7b426b..0x7b42a3`: the quad-plane normal about the threshold, NaN and infinity.
#[test]
fn render_normalize_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let vec = e.alloc(0x20); // esi (the 3 floats to normalize, +0x0/+0x4/+0x8)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, v: [f32; 3], ctx: &str| {
        for (k, c) in v.iter().enumerate() {
            e.write_u32(vec + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::ESI, vec);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        // 0x7b426b mov ecx,esi; call 0x4549f0 (len² of esi); fsqrt; fld st0; fabs; fcomp [eps];
        // jnp 0x7b42a1; (fdivr [1.0]; 3× fld st0; fmul [esi+N]; fstp [esi+N]); 0x7b42a1 fstp st(0);
        // halt at 0x7b42a3 (call 0x589f40). `Vector3LengthSquared` (0x4549f0) runs as called.
        e.run(0x7b426b, 0x7b42a3);
        let bin = [e.read_f32(vec), e.read_f32(vec + 4), e.read_f32(vec + 8)];
        let ours = render_normalize(v);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} norm[{k}]"));
        }
    };

    // ==threshold probes: len at / around the eps gate (the `fcomp [eps]; jnp` skip on C0|C2), zero
    // (skip), unit, exact sqrt.
    let eps = NORM_EPS2;
    let comp_eps = (eps / 3.0).sqrt() as f32; // |v|² ≈ eps with equal components
    run_case(&mut e, [comp_eps, comp_eps, comp_eps], "norm ~=eps");
    run_case(&mut e, [0.0, 0.0, 0.0], "norm zero (skip)");
    run_case(&mut e, [3.0, 4.0, 0.0], "norm 3-4-0 (len 5)");
    run_case(&mut e, [1.0, 0.0, 0.0], "norm unit-x");
    run_case(&mut e, [1e-4, 0.0, 0.0], "norm tiny (scale)");
    run_case(&mut e, [1e-5, 0.0, 0.0], "norm tinier (skip below eps)");
    run_case(
        &mut e,
        [f32::from_bits(0x3480_0000), 0.0, 0.0],
        "norm length == eps (scales)",
    );
    run_case(&mut e, [f32::NAN, 1.0, 1.0], "norm NaN (scales)");
    run_case(&mut e, [f32::INFINITY, 0.0, 0.0], "norm infinite");

    let mut s = 0x7b42_6b6bu32;
    for i in 0..3000u32 {
        // mix of normalizable (large) and below-gate (tiny) vectors.
        let v = if (next(&mut s) >> 8).is_multiple_of(4) {
            [
                frand(&mut s, -1e-4, 1e-4),
                frand(&mut s, -1e-4, 1e-4),
                frand(&mut s, -1e-4, 1e-4),
            ]
        } else {
            [
                frand(&mut s, -500.0, 500.0),
                frand(&mut s, -500.0, 500.0),
                frand(&mut s, -500.0, 500.0),
            ]
        };
        run_case(&mut e, v, &format!("norm fuzz[{i}]"));
    }
}

/// `0x7b5eb0..0x7b5f0f`: a particle's velocity through the frame.
#[test]
fn orient_velocity_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x100); // EBP frame for the result temps
    let ebp = frame + 0x80;
    let matrix = e.alloc(0x40); // esi → param_2 orientation matrix (16 floats)
    let particle = e.alloc(0x40); // ebx → particle; velocity at +0x10/0x14/0x18

    let run_case = |e: &mut Emu, m: [f32; 16], v: [f32; 3], ctx: &str| {
        for (i, &f) in m.iter().enumerate() {
            e.write_u32(matrix + 4 * i as u64, f.to_bits());
        }
        e.write_u32(particle + 0x10, v[0].to_bits());
        e.write_u32(particle + 0x14, v[1].to_bits());
        e.write_u32(particle + 0x18, v[2].to_bits());
        e.set_reg(RegisterX86::ESI, matrix);
        e.set_reg(RegisterX86::EBX, particle);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b5eb0, 0x7b5f0f); // the three rows + in-place write-back, halt at the loop tail

        let ours = orient_velocity(&m, v);
        for (k, &o) in ours.iter().enumerate() {
            assert_f32(
                e.read_f32(particle + 0x10 + 4 * k as u64),
                o,
                &format!("{ctx} out[{k}]"),
            );
        }
    };

    // Edge probes: identity (out = v), zero matrix (out = 0), zero velocity (out = 0), sign mix.
    let mut ident = [0f32; 16];
    ident[0] = 1.0;
    ident[5] = 1.0;
    ident[10] = 1.0;
    ident[15] = 1.0;
    run_case(&mut e, ident, [2.0, 5.0, -4.0], "identity");
    run_case(&mut e, ident, [0.0, 0.0, 0.0], "identity zero-vel");
    run_case(&mut e, [0f32; 16], [1.0, -2.0, 3.0], "zero matrix");

    let mut rng = OrientRng(0x7b5e_6000);
    for i in 0..4000u32 {
        let mut m = [0f32; 16];
        for f in m.iter_mut() {
            *f = rng.f32_mixed();
        }
        let v = [rng.f32_mixed(), rng.f32_mixed(), rng.f32_mixed()];
        run_case(&mut e, m, v, &format!("fuzz[{i}]"));
    }
}

/// `0x7b3a5b..0x7b3aa3`: the sort key.
#[test]
fn depth_dot_render_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let pos = e.alloc(0x20);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, axis: [f32; 4], p: [f32; 3], ctx: &str| {
        e.write_u32(AXIS_X, axis[0].to_bits());
        e.write_u32(AXIS_Y, axis[1].to_bits());
        e.write_u32(AXIS_Z, axis[2].to_bits());
        e.write_u32(AXIS_OFF, axis[3].to_bits());
        for (k, c) in p.iter().enumerate() {
            e.write_u32(pos + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::EAX, pos); // EDX = EAX + ECX → pos
        e.set_reg(RegisterX86::ECX, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b3a5b, 0x7b3aa3); // fld axis.z .. fstp dword [ebp-0x14] (in-range)
        let bin = e.read_f32(ebp - 0x14);
        let ours = depth_dot(axis, p);
        assert_f32(bin, ours, ctx);
    };

    // probes: zero pos (key=offset), unit axes, exact products.
    run_case(
        &mut e,
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "render zero-pos",
    );
    run_case(
        &mut e,
        [1.0, 1.0, 1.0, 0.0],
        [2.0, 3.0, 4.0],
        "render unit-axis",
    );
    run_case(
        &mut e,
        [0.0, 0.0, 1.0, 5.0],
        [7.0, 9.0, 11.0],
        "render z-only+off",
    );
    run_case(
        &mut e,
        [0.5, -0.5, 0.25, -1.0],
        [1.0, 1.0, 1.0],
        "render mixed-sign",
    );

    let mut s = 0x7b3a_1010u32;
    for i in 0..4000u32 {
        let axis = [
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        let p = [
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
        ];
        run_case(&mut e, axis, p, &format!("render fuzz[{i}]"));
    }
}

/// `0x7b4970..0x7b49a9`: the model particles' sort key.
#[test]
fn depth_dot_drawall_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x100); // EBX = emitter; [+0x5c]=index array, [+0x3c]=particle base
    let idx = e.alloc(0x10); // index array (index[0]=0)
    let pos = e.alloc(0x40); // particle base (pos at +0/4/8)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, axis: [f32; 4], p: [f32; 3], ctx: &str| {
        // axis row at the ebp slots: z=[ebp-0x74], y=[ebp-0x84], x=[ebp-0x94], offset=[ebp-0x64].
        e.write_u32(ebp - 0x94, axis[0].to_bits());
        e.write_u32(ebp - 0x84, axis[1].to_bits());
        e.write_u32(ebp - 0x74, axis[2].to_bits());
        e.write_u32(ebp - 0x64, axis[3].to_bits());
        e.write_u32(idx, 0); // index[0] = 0
        e.write_u32(this + 0x5c, idx as u32); // [ebx+0x5c] = index array
        e.write_u32(this + 0x3c, pos as u32); // [ebx+0x3c] = particle base
        for (k, c) in p.iter().enumerate() {
            e.write_u32(pos + 4 * k as u64, c.to_bits());
        }
        e.set_reg(RegisterX86::EBX, this);
        e.set_reg(RegisterX86::EAX, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b4970, 0x7b49a9); // mov edx,[ebx+0x5c]; fld axis.z .. fstp dword [ebp-0x10] (in-range)
        let bin = e.read_f32(ebp - 0x10);
        let ours = depth_dot(axis, p);
        assert_f32(bin, ours, ctx);
    };

    run_case(
        &mut e,
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 0.0, 0.0],
        "drawall zero-pos",
    );
    run_case(
        &mut e,
        [1.0, 1.0, 1.0, 0.0],
        [2.0, 3.0, 4.0],
        "drawall unit-axis",
    );
    run_case(
        &mut e,
        [0.0, 0.0, 1.0, 5.0],
        [7.0, 9.0, 11.0],
        "drawall z-only+off",
    );
    run_case(
        &mut e,
        [0.5, -0.5, 0.25, -1.0],
        [1.0, 1.0, 1.0],
        "drawall mixed-sign",
    );

    let mut s = 0x7b48_4040u32;
    for i in 0..4000u32 {
        let axis = [
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -2.0, 2.0),
            frand(&mut s, -1000.0, 1000.0),
        ];
        let p = [
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
        ];
        run_case(&mut e, axis, p, &format!("drawall fuzz[{i}]"));
    }
}

/// `0x7b51f9..0x7b521f`: the frame's scale.
#[test]
fn transform_basis_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let col0 = e.alloc(0x20); // ESI — the matrix's first column (3 floats)
    let this = e.alloc(0x300); // EDI — `this` (scale stored at +0x264)

    let run_case = |e: &mut Emu, c: [f32; 3], ctx: &str| {
        for (k, v) in c.iter().enumerate() {
            e.write_u32(col0 + 4 * k as u64, v.to_bits());
        }
        e.set_reg(RegisterX86::ESI, col0);
        e.set_reg(RegisterX86::EDI, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, SLICE_ESP + 0x100);
        // 0x7b51f9 fld[esi+8]; fld[esi+4]; fld[esi]; … fsqrt; 0x7b5219 fstp dword [edi+0x264]
        // (6 bytes) → end exclusive at 0x7b521f.
        e.run(0x7b51f9, 0x7b521f);
        let bin = e.read_f32(this + 0x264);
        assert_f32(bin, transform_basis_scale(c), ctx);
    };

    // ==threshold / edge probes: zero (sqrt(0)=0), exact-square magnitudes, unit axis, negatives.
    run_case(&mut e, [0.0, 0.0, 0.0], "scale zero");
    run_case(&mut e, [3.0, 4.0, 0.0], "scale 3-4-0 (=5)");
    run_case(&mut e, [1.0, 2.0, 2.0], "scale 1-2-2 (=3)");
    run_case(&mut e, [1.0, 0.0, 0.0], "scale unit-x");
    run_case(&mut e, [-1.0, -2.0, -2.0], "scale neg (=3)");

    let mut s = 0x7b51_6060u32;
    for i in 0..4000u32 {
        let c = [
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
            frand(&mut s, -5000.0, 5000.0),
        ];
        run_case(&mut e, c, &format!("scale fuzz[{i}]"));
    }
}

/// `0x7b4ed6..0x7b4f50`: the grid reciprocals and sizes.
#[test]
fn tile_grid_recip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // The numerator at 0x7ff9d8 is the f32 1.0 the model divides.
    assert_eq!(
        e.read_f32(0x7ff9d8).to_bits(),
        RECIP_ONE.to_bits(),
        "tile-grid divisor [0x7ff9d8] must be 1.0f"
    );

    let this = e.alloc(0x300); // ECX — `this` (inverses at +0x10/+0x14)
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, cols: u32, rows: u32, ctx: &str| {
        assert!(
            tile_grid_valid(cols, rows),
            "{ctx}: feed only valid pow2 dims"
        );
        e.set_reg(RegisterX86::ECX, this);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 8, cols); // [ebp+8] = cols (param2 / esi)
        e.write_u32(ebp + 0xc, rows); // [ebp+0xc] = rows (param3 / eax)
                                      // 0x7b4ed6 mov esi,[ebp+8]; the pow2 guard (falls through for
                                      // valid dims); the int stores; fild/fdivr/fstp ×2; halt at
                                      // 0x7b4f50 `mov esp,ebp` (after fstp [ecx+0x14] @0x7b4f4d).
        e.run(0x7b4ed6, 0x7b4f50);

        let bin_inv_rows = e.read_f32(this + 0x10);
        let bin_inv_cols = e.read_f32(this + 0x14);
        let (inv_rows, inv_cols) = tile_grid_recip(cols, rows);
        assert_f32(bin_inv_rows, inv_rows, &format!("{ctx} inv_rows (+0x10)"));
        assert_f32(bin_inv_cols, inv_cols, &format!("{ctx} inv_cols (+0x14)"));

        // The integer stores of the same block, which say which size is which.
        assert_eq!(e.read_u32(this + 0x198), cols, "{ctx}: [this+0x198] = cols");
        assert_eq!(e.read_u32(this + 0x19c), rows, "{ctx}: [this+0x19c] = rows");
        let log2_rows = 31 - rows.leading_zeros();
        assert_eq!(
            e.read_u32(this + 0xc),
            log2_rows,
            "{ctx}: [this+0xc] = log2(rows)"
        );
    };

    // Every power-of-two pair across the plausible tile-grid range (1..=256 each).
    let pow2s: [u32; 9] = [1, 2, 4, 8, 16, 32, 64, 128, 256];
    for &cols in &pow2s {
        for &rows in &pow2s {
            run_case(&mut e, cols, rows, &format!("tile cols={cols} rows={rows}"));
        }
    }

    // The guard rejection is integer-flag logic — verify `tile_grid_valid` against the binary's
    // condition `(d & (d-1)) == 0 && d != 0` directly (the error-stub path is not slice-run).
    for d in [0u32, 1, 2, 3, 5, 6, 7, 8, 0xffff_ffff, 0x8000_0000] {
        let want = (d & d.wrapping_sub(1)) == 0 && d != 0;
        assert_eq!(tile_grid_valid(d, 8), want, "tile_grid_valid({d}, 8)");
        assert_eq!(tile_grid_valid(8, d), want, "tile_grid_valid(8, {d})");
    }
}
