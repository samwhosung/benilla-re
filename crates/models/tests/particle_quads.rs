//! The quads one particle writes (`systems/models/particles.md#head-quads`,
//! `systems/models/particles.md#tail-quads`): `CParticleEmitter2::WriteQuads` (`0x7b2a50`) called
//! whole on each leg, with the particle matrix, the gradient record and the streams in memory,
//! and the vertices read back from the streams.

#![allow(clippy::too_many_arguments)] // a case takes every field it writes

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_models::particles::quads::{
    caps_swap_color, quad_expand_billboard, quad_expand_rotated, quad_expand_stretched,
    quad_expand_stretched_degenerate, quad_expand_stretched_is_degenerate, quad_expand_tiled,
    quad_expand_tiled_rotated, rotation_angle, tex_base, BillboardInputs, QuadStreams, QuadVertex,
    StretchedInputs, TiledInputs, TiledRotatedInputs,
};
use common::{assert_f32, next};

const G_MUL: f32 = f32::from_bits(0x3f7d_70a4);

const G_ADD: f32 = f32::from_bits(0x3ba3_d70a);

const MAGIC: f32 = f32::from_bits(0x4400_0000);

const VIEW_MATRIX_VA: u64 = 0x00cf_5b68;

const NORMAL0_VA: u64 = 0x00cf_5878;

const CAPS_HOLDER_VA: u64 = 0x00c0_ed38;

fn mat4_point(m: &[f32; 16], v: &[f32; 3]) -> [f32; 3] {
    let row = |i: usize| -> f32 {
        (f64::from(m[i]) * f64::from(v[0])
            + f64::from(m[i + 4]) * f64::from(v[1])
            + f64::from(m[i + 8]) * f64::from(v[2])
            + f64::from(m[i + 12])) as f32
    };
    [row(0), row(1), row(2)]
}

#[derive(Clone, Copy)]
struct Ramp {
    u8_0: u8,
    u8_1: u8,
    u8_2: u8,
    u8_3: u8,
    i_04: i32,
    i_08: i32,
    i_0c: i32,
    i_10: i32,
    i_14: i32,
    i_18: i32,
    f_24: f32,
    f_28: f32,
    f_2c: f32,
    f_30: f32,
}

impl Ramp {
    /// Write the ramp into emulator memory at `base` (`edi+0xbc`), with `+0x50 = 1.0` so the fast
    /// leg runs, and the unread tail zeroed by the arena.
    fn write(&self, emu: &mut Emu, base: u64) {
        emu.write_u32(
            base,
            u32::from_le_bytes([self.u8_0, self.u8_1, self.u8_2, self.u8_3]),
        );
        emu.write_u32(base + 0x04, self.i_04 as u32);
        emu.write_u32(base + 0x08, self.i_08 as u32);
        emu.write_u32(base + 0x0c, self.i_0c as u32);
        emu.write_u32(base + 0x10, self.i_10 as u32);
        emu.write_u32(base + 0x14, self.i_14 as u32);
        emu.write_u32(base + 0x18, self.i_18 as u32);
        emu.write_u32(base + 0x24, self.f_24.to_bits());
        emu.write_u32(base + 0x28, self.f_28.to_bits());
        emu.write_u32(base + 0x2c, self.f_2c.to_bits());
        emu.write_u32(base + 0x30, self.f_30.to_bits());
        emu.write_u32(base + 0x50, 0x3f80_0000); // == 1.0  → the closed fast leg
    }
}

fn gradient_fast(ramp: &Ramp, mult: f32, age: f32) -> (u32, u32, f32) {
    let t = ((f64::from(age) - f64::from(ramp.f_2c)) * f64::from(ramp.f_30) * f64::from(G_MUL)
        + f64::from(G_ADD)) as f32;
    let tf = f64::from(t);
    // b3 uses the extra `·mult`; b2/b1/b0 do not.
    let byte_mul = |coef: i32, base: u8| -> u32 {
        let sum =
            ((f64::from(coef) * tf + f64::from(base)) * f64::from(mult) + f64::from(MAGIC)) as f32;
        (sum.to_bits() >> 14) & 0xff
    };
    let byte = |coef: i32, base: u8| -> u32 {
        let sum = (f64::from(coef) * tf + f64::from(base) + f64::from(MAGIC)) as f32;
        (sum.to_bits() >> 14) & 0xff
    };
    let b3 = byte_mul(ramp.i_04, ramp.u8_3);
    let b2 = byte(ramp.i_08, ramp.u8_2);
    let b1 = byte(ramp.i_0c, ramp.u8_1);
    let b0 = byte(ramp.i_10, ramp.u8_0);
    let color = (b3 << 24) | (b2 << 16) | (b1 << 8) | b0;
    let scale = (tf * f64::from(ramp.f_28) + f64::from(ramp.f_24)) as f32;
    let tex0 = {
        let sum = (f64::from(ramp.i_18) * tf + f64::from(ramp.i_14) + f64::from(MAGIC)) as f32;
        (sum.to_bits() >> 14) & 0xff
    };
    (color, tex0, scale)
}

fn fz(s: &mut u32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (m * 40.0 - 20.0) as f32
}

fn mat4_dir(m: &[f32; 16], v: &[f32; 4]) -> [f32; 3] {
    let row = |i: usize| -> f32 {
        (f64::from(m[i]) * f64::from(v[0])
            + f64::from(m[i + 4]) * f64::from(v[1])
            + f64::from(m[i + 8]) * f64::from(v[2])
            + f64::from(m[i + 12]) * f64::from(v[3])) as f32
    };
    [row(0), row(1), row(2)]
}

struct LegCase {
    bin: [QuadVertex; 4],
    center: [f32; 3],
    color: u32,
    tex0: u32,
    ramp: Ramp,
    scale: f32,
    cell_u: f32,
    cell_v: f32,
    atlas_dim: u32,
    vshift: u32,
    m: [f32; 16],
    pos: [f32; 3],
    age: f32,
    nrm: [u32; 3],
    end_count: u32,
    start_count: u32,
}

fn run_leg(
    emu: &mut Emu,
    s: &mut u32,
    emitter: u64,
    particle: u64,
    descriptor: u64,
    streams: &[u64; 4],
    caps_obj: u64,
    nrm: [u32; 3],
    stride: u32,
    case: u32,
    extra: impl FnOnce(&mut Emu, u64, u64),
) -> LegCase {
    // ---- view matrix ----
    let m: [f32; 16] = {
        let mut m = [0f32; 16];
        for v in m.iter_mut() {
            *v = fz(s);
        }
        if case.is_multiple_of(5) {
            m = [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ];
        }
        m
    };
    for (i, v) in m.iter().enumerate() {
        emu.write_u32(VIEW_MATRIX_VA + (i as u64) * 4, v.to_bits());
    }
    // ---- particle: pos @ +0, vel @ +0x10, colorset @ +0x0c, age @ +0x1c, flags = low byte of +0
    // ----
    let pos = [fz(s), fz(s), fz(s)];
    let age = (f64::from(next(s) >> 8) / f64::from(1u32 << 24)) as f32;
    for (i, p) in pos.iter().enumerate() {
        emu.write_u32(particle + (i as u64) * 4, p.to_bits());
    }
    emu.write_u32(particle + 0x0c, 0);
    emu.write_u32(particle + 0x1c, age.to_bits());

    // ---- gradient ramp at emitter+0xbc ----
    let ramp = Ramp {
        u8_0: (next(s) >> 24) as u8,
        u8_1: (next(s) >> 24) as u8,
        u8_2: (next(s) >> 24) as u8,
        u8_3: (next(s) >> 24) as u8,
        i_04: ((next(s) >> 24) as i32) - 128,
        i_08: ((next(s) >> 24) as i32) - 128,
        i_0c: ((next(s) >> 24) as i32) - 128,
        i_10: ((next(s) >> 24) as i32) - 128,
        i_14: ((next(s) >> 26) as i32) % 4,
        i_18: ((next(s) >> 26) as i32) % 4,
        f_24: fz(s).abs() * 0.05 + 0.2,
        f_28: fz(s) * 0.1,
        f_2c: fz(s) * 0.1,
        f_30: fz(s) * 0.1,
    };
    ramp.write(emu, emitter + 0xbc);
    let mult = fz(s).abs() * 0.5 + 0.5;
    emu.write_u32(emitter + 0x1a8, mult.to_bits());

    // ---- emitter atlas params ----
    let cell_u = fz(s).abs() * 0.1 + 0.05;
    let cell_v = fz(s).abs() * 0.1 + 0.05;
    let atlas_dim: u32 = 1 << ((next(s) >> 28) % 3 + 1);
    let vshift: u32 = (next(s) >> 28) % 3 + 1;
    emu.write_u32(emitter + 0x10, cell_u.to_bits());
    emu.write_u32(emitter + 0x14, cell_v.to_bits());
    emu.write_u32(emitter + 0x0c, vshift);
    emu.write_u32(emitter + 0x19c, atlas_dim);

    // ---- entry guards: distance-alpha off, no cull, no fade, no extra-scale ----
    emu.write_u32(emitter + 0x1b4, 0x3f80_0000); // == 1.0
    emu.write_u32(emitter + 0x1c0, 0x0000_0000); // == 0.0
    emu.write_u32(emitter + 0x1b0, 0x3f80_0000);
    emu.write_u32(emitter + 0x1b8, 0x0000_0000);
    emu.write_u32(emitter + 0x264, 0x3f80_0000);

    // ---- caps color-swap flag ----
    let swap = case % 2 == 1;
    emu.write_u32(caps_obj + 0x23c + 0x1c, u32::from(swap));

    // ---- descriptor ----
    for (i, &p) in streams.iter().enumerate() {
        emu.write_u32(descriptor + (i as u64) * 4, p as u32);
        emu.write_u32(descriptor + 0x10 + (i as u64) * 4, stride);
    }
    let start_count = next(s) & 0xffff;
    emu.write_u32(descriptor + 0x20, start_count);
    for &base in streams.iter() {
        for k in 0..4u64 {
            for b in 0..3u64 {
                emu.write_u32(base + k * u64::from(stride) + b * 4, 0xdead_beef);
            }
        }
    }

    // ---- leg-specific fields (flags, +0x18c, +0xb4, +0x1ae, the 0xcf5b34 corner table, …) ----
    extra(emu, emitter, particle);

    // ---- run ----
    emu.clear_calls();
    let ret = emu.call(
        0x7b2a50,
        emitter as u32,
        0,
        &[particle as u32, descriptor as u32],
    );
    assert_eq!(ret, 1, "case {case}: expected leg to return 1");

    // ---- re-derive resolved inputs ----
    let center = mat4_point(&m, &pos);
    let (raw_color, tex0, scale) = gradient_fast(&ramp, mult, age);
    let color = if swap {
        caps_swap_color(raw_color)
    } else {
        raw_color
    };

    // ---- read back the four binary vertices ----
    let mut bin = [QuadVertex {
        pos: [0.0; 3],
        normal_const: [0; 3],
        color: 0,
        uv: [0.0; 2],
    }; 4];
    for (k, vtx) in bin.iter_mut().enumerate() {
        let off = k as u64 * u64::from(stride);
        vtx.pos = [
            emu.read_f32(streams[0] + off),
            emu.read_f32(streams[0] + off + 4),
            emu.read_f32(streams[0] + off + 8),
        ];
        vtx.normal_const = [
            emu.read_u32(streams[1] + off),
            emu.read_u32(streams[1] + off + 4),
            emu.read_u32(streams[1] + off + 8),
        ];
        vtx.color = emu.read_u32(streams[2] + off);
        vtx.uv = [
            emu.read_f32(streams[3] + off),
            emu.read_f32(streams[3] + off + 4),
        ];
    }
    let end_count = emu.read_u32(descriptor + 0x20);
    LegCase {
        bin,
        center,
        color,
        tex0,
        ramp,
        scale,
        cell_u,
        cell_v,
        atlas_dim,
        vshift,
        m,
        pos,
        age,
        nrm,
        end_count,
        start_count,
    }
}

fn gradient_tex1(ramp: &Ramp, age: f32, i_1c: i32, i_20: i32) -> u32 {
    let t = ((f64::from(age) - f64::from(ramp.f_2c)) * f64::from(ramp.f_30) * f64::from(G_MUL)
        + f64::from(G_ADD)) as f32;
    let tf = f64::from(t);
    let sum = (f64::from(i_20) * tf + f64::from(i_1c) + f64::from(MAGIC)) as f32;
    (sum.to_bits() >> 14) & 0xff
}

fn diff_quads(bin: &[QuadVertex; 4], ours: &[QuadVertex; 4], case: u32) {
    for (k, (b, o)) in bin.iter().zip(ours.iter()).enumerate() {
        for (i, (&bp, &op)) in b.pos.iter().zip(o.pos.iter()).enumerate() {
            assert_f32(bp, op, &format!("case {case} v{k} pos[{i}]"));
        }
        assert_eq!(
            b.normal_const, o.normal_const,
            "case {case} v{k} normal_const"
        );
        assert_eq!(
            b.color, o.color,
            "case {case} v{k} color: binary={:#010x} ours={:#010x}",
            b.color, o.color
        );
        assert_f32(b.uv[0], o.uv[0], &format!("case {case} v{k} u"));
        assert_f32(b.uv[1], o.uv[1], &format!("case {case} v{k} v"));
    }
}

/// `0x7b2a50` whole, the head square.
#[test]
fn particle_quad_expand_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // --- arenas ---
    let emitter = emu.alloc(0x400); // edi (param_1) — the CParticleEmitter2
    let particle = emu.alloc(0x40); // ebx (param_2[?]) — the CParticle2 record (pos @ +0, age @ +0x1c)
    let descriptor = emu.alloc(0x40); // esi (param_3) — the 9-dword GX stream descriptor
                                      // four output streams, generously strided so vertices don't
                                      // overlap.
    let stride = 0x40u32;
    let stream_pos = emu.alloc(0x400);
    let stream_nrm = emu.alloc(0x400);
    let stream_col = emu.alloc(0x400);
    let stream_tex = emu.alloc(0x400);
    // the caps singleton: *[0xc0ed38] -> obj; caps = obj+0x23c; [caps+0x1c] = swap flag.
    let caps_obj = emu.alloc(0x400);

    // the three vertex-normal constants at 0xcf5878/587c/5880 (bss; we choose them).
    let nrm: [u32; 3] = [0x3f80_0000, 0x0000_0000, 0xbf80_0000];
    emu.write_u32(NORMAL0_VA, nrm[0]);
    emu.write_u32(NORMAL0_VA + 4, nrm[1]);
    emu.write_u32(NORMAL0_VA + 8, nrm[2]);

    // wire the caps holder.
    emu.write_u32(CAPS_HOLDER_VA, caps_obj as u32);

    let mut s = 0x7b2a_5000u32;

    // a structured set of (view matrix, particle pos, age, ramp, emitter atlas params, caps flag,
    // descriptor strides) then a fuzz sweep — every case takes the billboard branch (flag 0x4,
    // +0x18c==0, +0x2000 clear, +0x1c0==0, +0x1b4==1.0, +0x200/fade clear).
    let cases = 220;
    for case in 0..cases {
        // ---- view matrix (row-major, the 0x7bca80 layout) ----
        let m: [f32; 16] = {
            let mut m = [0f32; 16];
            for v in m.iter_mut() {
                *v = fz(&mut s);
            }
            if case % 5 == 0 {
                // an identity-ish view for a clean center == pos case.
                m = [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ];
            }
            m
        };
        for (i, v) in m.iter().enumerate() {
            emu.write_u32(VIEW_MATRIX_VA + (i as u64) * 4, v.to_bits());
        }

        // ---- particle: pos @ +0, age @ +0x1c, colorset byte @ +0x0c = 0 (ramp at edi+0xbc) ----
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let age = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24)) as f32; // [0,1) age fraction
        for (i, p) in pos.iter().enumerate() {
            emu.write_u32(particle + (i as u64) * 4, p.to_bits());
        }
        emu.write_u32(particle + 0x0c, 0); // colorset index 0
        emu.write_u32(particle + 0x1c, age.to_bits());

        // ---- gradient ramp at emitter+0xbc ----
        let ramp = Ramp {
            u8_0: (next(&mut s) >> 24) as u8,
            u8_1: (next(&mut s) >> 24) as u8,
            u8_2: (next(&mut s) >> 24) as u8,
            u8_3: (next(&mut s) >> 24) as u8,
            i_04: ((next(&mut s) >> 24) as i32) - 128,
            i_08: ((next(&mut s) >> 24) as i32) - 128,
            i_0c: ((next(&mut s) >> 24) as i32) - 128,
            i_10: ((next(&mut s) >> 24) as i32) - 128,
            i_14: ((next(&mut s) >> 26) as i32) % 4, // small atlas row/col coefficients
            i_18: ((next(&mut s) >> 26) as i32) % 4,
            f_24: fz(&mut s).abs() * 0.05 + 0.2, // a positive base scale
            f_28: (fz(&mut s)) * 0.1,
            f_2c: fz(&mut s) * 0.1,
            f_30: fz(&mut s) * 0.1,
        };
        ramp.write(&mut emu, emitter + 0xbc);
        let mult = fz(&mut s).abs() * 0.5 + 0.5; // [edi+0x1a8] color multiplier (positive)
        emu.write_u32(emitter + 0x1a8, mult.to_bits());

        // ---- emitter atlas params ----
        let cell_u = fz(&mut s).abs() * 0.1 + 0.05;
        let cell_v = fz(&mut s).abs() * 0.1 + 0.05;
        let atlas_dim: u32 = 1 << ((next(&mut s) >> 28) % 3 + 1); // 2,4,8
        let vshift: u32 = (next(&mut s) >> 28) % 3 + 1;
        emu.write_u32(emitter + 0x10, cell_u.to_bits());
        emu.write_u32(emitter + 0x14, cell_v.to_bits());
        emu.write_u32(emitter + 0x0c, vshift);
        emu.write_u32(emitter + 0x19c, atlas_dim);

        // ---- emitter flags + guards: billboard branch, no rotation, no tiling, no extra scale
        // ----
        emu.write_u32(emitter + 0x1ac, 0x0000_0004); // bit 0x4 only
        emu.write_u32(emitter + 0x18c, 0x0000_0000); // +0x18c == 0  → unrotated
        emu.write_u32(emitter + 0x1b4, 0x3f80_0000); // == 1.0 → no distance-alpha, no cull
        emu.write_u32(emitter + 0x1c0, 0x0000_0000); // == 0.0 → first block skipped
        emu.write_u32(emitter + 0x1b0, 0x3f80_0000); // unused on this path but laid sane
        emu.write_u32(emitter + 0x1b8, 0x0000_0000);
        emu.write_u32(emitter + 0x264, 0x3f80_0000);

        // ---- caps color-swap flag ----
        let swap = case % 2 == 1;
        emu.write_u32(caps_obj + 0x23c + 0x1c, u32::from(swap));

        // ---- descriptor: 4 stream ptrs, 4 strides, count ----
        for (i, &p) in [stream_pos, stream_nrm, stream_col, stream_tex]
            .iter()
            .enumerate()
        {
            emu.write_u32(descriptor + (i as u64) * 4, p as u32);
            emu.write_u32(descriptor + 0x10 + (i as u64) * 4, stride);
        }
        let start_count = next(&mut s) & 0xffff;
        emu.write_u32(descriptor + 0x20, start_count);

        // clear the output streams so stale bytes can't fake a match.
        for base in [stream_pos, stream_nrm, stream_col, stream_tex] {
            for k in 0..4u64 {
                for b in 0..3u64 {
                    emu.write_u32(base + k * u64::from(stride) + b * 4, 0xdead_beef);
                }
            }
        }

        // ---- run the binary: __thiscall ecx=emitter; stack args (particle, descriptor). ret 8.
        // ----
        emu.clear_calls();
        let ret = emu.call(
            0x7b2a50,
            emitter as u32,
            0,
            &[particle as u32, descriptor as u32],
        );
        assert_eq!(ret, 1, "case {case}: expected billboard branch to return 1");

        // ---- re-derive the resolved loop inputs from the same memory ----
        let center = mat4_point(&m, &pos);
        let (raw_color, tex_index, scale) = gradient_fast(&ramp, mult, age);
        let color = if swap {
            caps_swap_color(raw_color)
        } else {
            raw_color
        };
        let (base_u, base_v) = tex_base(tex_index, atlas_dim, vshift, cell_u, cell_v);
        let inp = BillboardInputs {
            center,
            scale,
            color,
            tex_base_u: base_u,
            tex_base_v: base_v,
            cell_u,
            cell_v,
            normal_const: nrm,
        };
        let ours: [QuadVertex; 4] = quad_expand_billboard(&inp);

        // ---- read back the four binary vertices and diff bit-exact ----
        let _streams = QuadStreams {
            ptr: [
                stream_pos as u32,
                stream_nrm as u32,
                stream_col as u32,
                stream_tex as u32,
            ],
            stride: [stride; 4],
            count: start_count,
        };
        for (k, o) in ours.iter().enumerate() {
            let off = k as u64 * u64::from(stride);
            let bpos = [
                emu.read_f32(stream_pos + off),
                emu.read_f32(stream_pos + off + 4),
                emu.read_f32(stream_pos + off + 8),
            ];
            let bnrm = [
                emu.read_u32(stream_nrm + off),
                emu.read_u32(stream_nrm + off + 4),
                emu.read_u32(stream_nrm + off + 8),
            ];
            let bcol = emu.read_u32(stream_col + off);
            let buv = [
                emu.read_f32(stream_tex + off),
                emu.read_f32(stream_tex + off + 4),
            ];
            for (i, (&b, &ours_p)) in bpos.iter().zip(o.pos.iter()).enumerate() {
                assert_f32(b, ours_p, &format!("case {case} v{k} pos[{i}]"));
            }
            assert_eq!(bnrm, o.normal_const, "case {case} v{k} normal_const");
            assert_eq!(
                bcol, o.color,
                "case {case} v{k} color: binary={bcol:#010x} ours={:#010x}",
                o.color
            );
            assert_f32(buv[0], o.uv[0], &format!("case {case} v{k} u"));
            assert_f32(buv[1], o.uv[1], &format!("case {case} v{k} v"));
        }
        // the descriptor's vertex count advanced by 4.
        let end_count = emu.read_u32(descriptor + 0x20);
        assert_eq!(
            end_count,
            start_count.wrapping_add(4),
            "case {case}: vertex count advanced by 4"
        );
    }
    let _ = next(&mut s);
}

/// `0x7b2a50` whole, the flat square.
#[test]
fn particle_quad_expand_tiled_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let emitter = emu.alloc(0x400);
    let particle = emu.alloc(0x40);
    let descriptor = emu.alloc(0x40);
    let stride = 0x40u32;
    let streams = [
        emu.alloc(0x400),
        emu.alloc(0x400),
        emu.alloc(0x400),
        emu.alloc(0x400),
    ];
    let caps_obj = emu.alloc(0x400);
    let nrm: [u32; 3] = [0x3f80_0000, 0x0000_0000, 0xbf80_0000];
    emu.write_u32(NORMAL0_VA, nrm[0]);
    emu.write_u32(NORMAL0_VA + 4, nrm[1]);
    emu.write_u32(NORMAL0_VA + 8, nrm[2]);
    emu.write_u32(CAPS_HOLDER_VA, caps_obj as u32);
    // the runtime-built 3D corner table `0xcf5b34` (BSS); the loop reads corner k at 0xcf5b2c+12k.
    const CORNER_BASE_VA: u64 = 0x00cf_5b2c;

    let mut s = 0x7b2a_7000u32;
    let cases = 220;
    for case in 0..cases {
        // A corner table in memory (its builder is tested in particle_draw), mixed offsets across
        // the four corners.
        let corner: [[f32; 3]; 4] = {
            let mut c = [[0f32; 3]; 4];
            for ci in c.iter_mut() {
                for comp in ci.iter_mut() {
                    *comp = fz(&mut s) * 0.1;
                }
            }
            c
        };
        let lc = run_leg(
            &mut emu,
            &mut s,
            emitter,
            particle,
            descriptor,
            &streams,
            caps_obj,
            nrm,
            stride,
            case,
            |emu, emitter, _particle| {
                emu.write_u32(emitter + 0x1ac, 0x0000_2004); // bit 0x4 + bit 0x2000 (tiled)
                emu.write_u32(emitter + 0x18c, 0x0000_0000); // +0x18c == 0 → unrotated
                for (k, cc) in corner.iter().enumerate() {
                    for (j, &comp) in cc.iter().enumerate() {
                        emu.write_u32(
                            CORNER_BASE_VA + (k as u64) * 12 + (j as u64) * 4,
                            comp.to_bits(),
                        );
                    }
                }
            },
        );
        let (base_u, base_v) = tex_base(lc.tex0, lc.atlas_dim, lc.vshift, lc.cell_u, lc.cell_v);
        let ours = quad_expand_tiled(&TiledInputs {
            center: lc.center,
            scale: lc.scale,
            color: lc.color,
            tex_base_u: base_u,
            tex_base_v: base_v,
            cell_u: lc.cell_u,
            cell_v: lc.cell_v,
            normal_const: lc.nrm,
            corner,
        });
        diff_quads(&lc.bin, &ours, case);
        assert_eq!(
            lc.end_count,
            lc.start_count.wrapping_add(4),
            "case {case}: tiled count advanced by 4"
        );
        let _ = (lc.m, lc.pos, lc.age, lc.ramp);
    }
}

/// `0x7b2a50` whole, the turning square.
#[test]
fn particle_quad_expand_rotated_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let emitter = emu.alloc(0x400);
    // two particle slots 0x20 apart on a 0x40 boundary: bit 5 of the ADDRESS is clear in one, set
    // in the other (the negate guard reads `bl & 0x20` of the particle pointer, 0x7b2dea).
    let pool = emu.alloc(0x100);
    let slot = [(pool + 0x3f) & !0x3f, ((pool + 0x3f) & !0x3f) + 0x20];
    let descriptor = emu.alloc(0x40);
    let stride = 0x40u32;
    let streams = [
        emu.alloc(0x400),
        emu.alloc(0x400),
        emu.alloc(0x400),
        emu.alloc(0x400),
    ];
    let caps_obj = emu.alloc(0x400);
    let nrm: [u32; 3] = [0x3f80_0000, 0x0000_0000, 0xbf80_0000];
    emu.write_u32(NORMAL0_VA, nrm[0]);
    emu.write_u32(NORMAL0_VA + 4, nrm[1]);
    emu.write_u32(NORMAL0_VA + 8, nrm[2]);
    emu.write_u32(CAPS_HOLDER_VA, caps_obj as u32);

    let mut s = 0x7b2a_8000u32;
    let cases = 220;
    for case in 0..cases {
        // The angle stays under 25 (rate in [-25, 25), age in [0, 1)), where the host's sine and
        // cosine match `fsincos`.
        let rate = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 50.0 - 25.0) as f32;
        // the negate guard reads bit 5 of the particle POINTER's low byte (a heap-alignment quirk,
        // not particle data) — feed the synthesized particle allocation's low byte. sweep the two
        // negate conditions independently of the rate's sign: runtime flag 0x8000 (bit 15, read by
        // `0x7b2dda test dh,dh`) and particle-address bit 5 (`0x7b2dea test bl,0x20`).
        let particle = slot[(case & 1) as usize];
        let flags: u32 = if case & 2 != 0 {
            0x0000_8004
        } else {
            0x0000_0004
        };
        let lc = run_leg(
            &mut emu,
            &mut s,
            emitter,
            particle,
            descriptor,
            &streams,
            caps_obj,
            nrm,
            stride,
            case,
            |emu, emitter, _particle| {
                emu.write_u32(emitter + 0x1ac, flags); // bit 0x4 (+0x8000 on half), bit 0x2000 clear
                emu.write_u32(emitter + 0x18c, rate.to_bits()); // +0x18c != 0 (unless rate==0)
            },
        );
        let center = lc.center;
        let (base_u, base_v) = tex_base(lc.tex0, lc.atlas_dim, lc.vshift, lc.cell_u, lc.cell_v);
        let inp = BillboardInputs {
            center,
            scale: lc.scale,
            color: lc.color,
            tex_base_u: base_u,
            tex_base_v: base_v,
            cell_u: lc.cell_u,
            cell_v: lc.cell_v,
            normal_const: lc.nrm,
        };
        let angle = rotation_angle(rate, lc.age, flags, particle as u32);
        let ours = quad_expand_rotated(&inp, angle);
        diff_quads(&lc.bin, &ours, case);
        assert_eq!(
            lc.end_count,
            lc.start_count.wrapping_add(4),
            "case {case}: rotated count advanced by 4"
        );
        let _ = lc.ramp;
    }
}

/// `0x7b2a50` whole, the turning flat square, `0x7be490` included.
#[test]
fn particle_quad_expand_tiled_rotated_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let emitter = emu.alloc(0x400);
    let pool = emu.alloc(0x100);
    let slot = [(pool + 0x3f) & !0x3f, ((pool + 0x3f) & !0x3f) + 0x20];
    let descriptor = emu.alloc(0x40);
    let stride = 0x40u32;
    let streams = [
        emu.alloc(0x400),
        emu.alloc(0x400),
        emu.alloc(0x400),
        emu.alloc(0x400),
    ];
    let caps_obj = emu.alloc(0x400);
    let nrm: [u32; 3] = [0x3f80_0000, 0x0000_0000, 0xbf80_0000];
    emu.write_u32(NORMAL0_VA, nrm[0]);
    emu.write_u32(NORMAL0_VA + 4, nrm[1]);
    emu.write_u32(NORMAL0_VA + 8, nrm[2]);
    emu.write_u32(CAPS_HOLDER_VA, caps_obj as u32);
    const CORNER_BASE_VA: u64 = 0x00cf_5b2c; // corner k at +12k (loop reads [ebx-4]/[ebx]/[ebx+4])
    const AXIS_OFF: u64 = 0x284; // [edi+0x284] Rodrigues axis

    let mut s = 0x7b2a_9000u32;
    let cases = 240;
    for case in 0..cases {
        // rate ∈ [-25,25); force != 0 so the dispatch reaches the tiled-ROTATED leg (0x7b2e00).
        let mut rate = (f64::from(next(&mut s) >> 8) / f64::from(1u32 << 24) * 50.0 - 25.0) as f32;
        if rate == 0.0 {
            rate = 1.0;
        }
        // flag 0x8000 × particle-address bit 5, independent of the rate's sign (see the rotated
        // test).
        let particle = slot[(case & 1) as usize];
        let flags: u32 = if case & 2 != 0 {
            0x0000_a004
        } else {
            0x0000_2004
        };
        // the Rodrigues axis (raw — used as-is, normalized=true skips renormalize).
        let axis: [f32; 3] = [fz(&mut s), fz(&mut s), fz(&mut s)];
        // the runtime corner table 0xcf5b34.
        let corner: [[f32; 3]; 4] = {
            let mut c = [[0f32; 3]; 4];
            for ci in c.iter_mut() {
                for comp in ci.iter_mut() {
                    *comp = fz(&mut s) * 0.1;
                }
            }
            c
        };
        let lc = run_leg(
            &mut emu,
            &mut s,
            emitter,
            particle,
            descriptor,
            &streams,
            caps_obj,
            nrm,
            stride,
            case,
            |emu, emitter, _particle| {
                emu.write_u32(emitter + 0x1ac, flags); // bit 0x4 + bit 0x2000 (tiled), +0x8000 on half
                emu.write_u32(emitter + 0x18c, rate.to_bits()); // +0x18c != 0 → tiled-ROTATED (0x7b2e00)
                for (j, &a) in axis.iter().enumerate() {
                    emu.write_u32(emitter + AXIS_OFF + (j as u64) * 4, a.to_bits());
                }
                for (k, cc) in corner.iter().enumerate() {
                    for (j, &comp) in cc.iter().enumerate() {
                        emu.write_u32(
                            CORNER_BASE_VA + (k as u64) * 12 + (j as u64) * 4,
                            comp.to_bits(),
                        );
                    }
                }
            },
        );
        let (base_u, base_v) = tex_base(lc.tex0, lc.atlas_dim, lc.vshift, lc.cell_u, lc.cell_v);
        let angle = rotation_angle(rate, lc.age, flags, particle as u32);
        let ours = quad_expand_tiled_rotated(&TiledRotatedInputs {
            base: TiledInputs {
                center: lc.center,
                scale: lc.scale,
                color: lc.color,
                tex_base_u: base_u,
                tex_base_v: base_v,
                cell_u: lc.cell_u,
                cell_v: lc.cell_v,
                normal_const: lc.nrm,
                corner,
            },
            axis,
            angle,
        });
        diff_quads(&lc.bin, &ours, case);
        assert_eq!(
            lc.end_count,
            lc.start_count.wrapping_add(4),
            "case {case}: tiled-rotated count advanced by 4"
        );
        let _ = (lc.m, lc.pos, lc.ramp);
    }
}

/// `0x7b2a50` whole, the tail and the short tail.
#[test]
fn particle_quad_expand_stretched_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let emitter = emu.alloc(0x400);
    let particle = emu.alloc(0x40);
    let descriptor = emu.alloc(0x40);
    let stride = 0x40u32;
    let streams = [
        emu.alloc(0x400),
        emu.alloc(0x400),
        emu.alloc(0x400),
        emu.alloc(0x400),
    ];
    let caps_obj = emu.alloc(0x400);
    let scratch = emu.alloc(0x40); // for the standalone 0x7bcb40 vproj derivation
    let nrm: [u32; 3] = [0x3f80_0000, 0x0000_0000, 0xbf80_0000];
    emu.write_u32(NORMAL0_VA, nrm[0]);
    emu.write_u32(NORMAL0_VA + 4, nrm[1]);
    emu.write_u32(NORMAL0_VA + 8, nrm[2]);
    emu.write_u32(CAPS_HOLDER_VA, caps_obj as u32);

    let mut s = 0x7b2a_9000u32;
    let cases: u32 = 240;
    for case in 0..cases {
        // particle velocity at +0x10 (3 floats); the leg projects the NEGATED velocity.
        let vel = [fz(&mut s), fz(&mut s), fz(&mut s)];
        // length cap [edi+0xb4]; some cases tiny (drive the degenerate fallback).
        let force_degen = case.is_multiple_of(7);
        let len = if force_degen {
            fz(&mut s).abs() * 1.0e-4
        } else {
            fz(&mut s).abs() * 2.0 + 0.5
        };
        // the second gradient atlas index reads ramp+0x1c / +0x20 (zero in the billboard `Ramp`);
        // write small int coefficients so tex1 actually varies across cases.
        let i_1c = ((next(&mut s) >> 26) as i32) % 4;
        let i_20 = ((next(&mut s) >> 26) as i32) % 4;
        let lc = run_leg(
            &mut emu,
            &mut s,
            emitter,
            particle,
            descriptor,
            &streams,
            caps_obj,
            nrm,
            stride,
            case,
            |emu, emitter, particle| {
                emu.write_u32(emitter + 0x1ac, 0x0000_0008); // bit 0x8 (stretched) ONLY (no bit 0x4)
                emu.write_u32(emitter + 0xb4, len.to_bits()); // length cap
                emu.write_u32(emitter + 0x18c, 0x0000_0000);
                // [edi+0x1ae] & 1 → length-clamp-to-age; keep clear so `len` passes through.
                // (1ae is the byte at +0x1ac+2; the flag word we wrote is 0x8 → byte +2 is 0.)
                emu.write_u32(emitter + 0xbc + 0x1c, i_1c as u32); // ramp+0x1c (tex1 base)
                emu.write_u32(emitter + 0xbc + 0x20, i_20 as u32); // ramp+0x20 (tex1 coef)
                for (i, &c) in vel.iter().enumerate() {
                    emu.write_u32(particle + 0x10 + (i as u64) * 4, c.to_bits());
                }
            },
        );
        // vproj = 0x7bcb40(view, -vel) with w=0 — run the binary's own transform on scratch memory.
        let inbuf = scratch;
        let outbuf = scratch + 0x20;
        emu.write_u32(inbuf, (-vel[0]).to_bits());
        emu.write_u32(inbuf + 4, (-vel[1]).to_bits());
        emu.write_u32(inbuf + 8, (-vel[2]).to_bits());
        emu.write_u32(inbuf + 0xc, 0); // w = 0 (direction)
        emu.clear_calls();
        emu.call(
            0x7bcb40,
            outbuf as u32,
            inbuf as u32,
            &[VIEW_MATRIX_VA as u32],
        );
        let vproj = [
            emu.read_f32(outbuf),
            emu.read_f32(outbuf + 4),
            emu.read_f32(outbuf + 8),
        ];
        // cross-check the standalone transform against mat4_dir (a derivation sanity check).
        let vproj_ref = mat4_dir(&lc.m, &[-vel[0], -vel[1], -vel[2], 0.0]);
        for i in 0..3 {
            assert_f32(vproj[i], vproj_ref[i], &format!("case {case} vproj[{i}]"));
        }

        // the stretched leg recomputes tex bases from the SECOND gradient index (tex1).
        let tex1 = gradient_tex1(&lc.ramp, lc.age, i_1c, i_20);
        let (base_u, base_v) = tex_base(tex1, lc.atlas_dim, lc.vshift, lc.cell_u, lc.cell_v);
        let sinp = StretchedInputs {
            center: lc.center,
            scale: lc.scale,
            color: lc.color,
            tex_base_u: base_u,
            tex_base_v: base_v,
            cell_u: lc.cell_u,
            cell_v: lc.cell_v,
            normal_const: lc.nrm,
            vproj,
            len,
        };
        let ours = if quad_expand_stretched_is_degenerate(&sinp) {
            // the degenerate path emits a plain billboard quad with the stretched leg's tex bases.
            quad_expand_stretched_degenerate(&BillboardInputs {
                center: lc.center,
                scale: lc.scale,
                color: lc.color,
                tex_base_u: base_u,
                tex_base_v: base_v,
                cell_u: lc.cell_u,
                cell_v: lc.cell_v,
                normal_const: lc.nrm,
            })
        } else {
            quad_expand_stretched(&sinp)
        };
        diff_quads(&lc.bin, &ours, case);
        assert_eq!(
            lc.end_count,
            lc.start_count.wrapping_add(4),
            "case {case}: stretched count advanced by 4"
        );
        let _ = (lc.pos, lc.age, lc.tex0);
    }
}

/// A squared tail length under the threshold in the register whose `f32` equals it.
#[test]
fn quad_expand_stretched_degenerate_half_ulp_boundary() {
    let sinp = StretchedInputs {
        center: [0.0; 3],
        scale: 1.0,
        color: 0,
        tex_base_u: 0.0,
        tex_base_v: 0.0,
        cell_u: 0.0,
        cell_v: 0.0,
        normal_const: [0; 3],
        vproj: [
            f32::from_bits(0x3ca0_e0b0),
            f32::from_bits(0x3ca0_ef39),
            0.0,
        ],
        len: 1.0,
    };
    assert!(
        quad_expand_stretched_is_degenerate(&sinp),
        "live-f64 sq (7.7160494e-4) < STRETCH_EPS ⇒ degenerate; an `as f32` compare (f32(sq)==eps) \
         would return stretched"
    );
}
