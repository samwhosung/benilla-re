//! The ribbons (`systems/models/ribbons.md`): the node frame, the edge, the texture cell, the
//! setters and the vertex transform called whole; the update and the initialize as slices run to
//! the stores or the `fnstsw` they make.

#![allow(clippy::too_many_arguments)] // a case takes every field it writes

mod common;

use benilla_re_cmath::transform::transform_point as transform_point4;
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_models::ribbons::{
    add_gravity, advance_sim, byte_setter, clamp_span, dt_clamp, edge_expired, expire_sum,
    floor_arg, gravity_term, init_buffer_count, interp_factor, next_counter, node_place_translate,
    particle_tile_uv_rect, phase_frac, recip_span, record_dwords, record_value,
    render_world_translate, ribbon_color_pack, ribbon_fractions, ribbon_frame_build,
    ribbon_live_index_advance, ribbon_segment_emit, rotate_bases, segment_emit_arg,
    segment_fraction, Bases, RibbonEmitFrame, RibbonFractions, RibbonFrame, RibbonLiveIndex,
    RibbonSegmentOut, TileUvIn, SPAN_CLAMP,
};
use common::{
    assert_f32, assert_f64, frand, fuzz_f32, island_bytes_match, next, read_ah, read_f64, SLICE_ESP,
};

fn emit_layout(
    emu: &mut Emu,
    this: u64,
    f: &RibbonEmitFrame,
    cap: u32,
    index: u32,
    render_base: u64,
    scalar_base: u64,
) {
    let w3 = |emu: &mut Emu, off: u64, v: [f32; 3]| {
        for i in 0..3u64 {
            emu.write_u32(off + i * 4, v[i as usize].to_bits());
        }
    };
    w3(emu, this + 0xac, f.width_c);
    w3(emu, this + 0xb8, f.width_d);
    w3(emu, this + 0xc4, f.corner0);
    w3(emu, this + 0xd0, f.corner1);
    w3(emu, this + 0xdc, f.corner2);
    w3(emu, this + 0xe8, f.corner3);
    emu.write_u32(this + 0x08, cap); // ring capacity (list helper 0x7b6f00 reads [esi+8])
    emu.write_u32(this + 0x0c, scalar_base as u32); // per-vertex scalar array base [esi+0xc]
    emu.write_u32(this + 0x14, index); // live index [esi+0x14]
    emu.write_u32(this + 0x40, render_base as u32); // render record base [esi+0x40]
}

fn make_emit_frame(s: &mut u32) -> RibbonEmitFrame {
    RibbonEmitFrame {
        width_c: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
        width_d: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
        corner0: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
        corner1: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
        corner2: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
        corner3: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
    }
}

fn make_emit_frame_const() -> RibbonEmitFrame {
    RibbonEmitFrame {
        width_c: [0.5, -0.25, 1.5],
        width_d: [-0.75, 0.5, 0.125],
        corner0: [1.0, 2.0, 3.0],
        corner1: [-1.0, 0.5, 2.0],
        corner2: [3.0, -1.0, 0.0],
        corner3: [0.25, 0.75, -0.5],
    }
}

struct AgeRng(u32);

impl AgeRng {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A finite f32 in a mixed range, from the LCG's high bits.
    fn f32_mixed(&mut self) -> f32 {
        let pick = (self.next() >> 8) % 6;
        let m = f64::from(self.next() >> 8) / f64::from(1u32 << 24);
        match pick {
            0 => (m * 2.0 - 1.0) as f32,   // [-1, 1)
            1 => (m * 20.0 - 10.0) as f32, // small
            2 => (m * 2.0) as f32,         // [0, 2) — factor-like
            3 => (m * 2000.0 - 1000.0) as f32,
            4 => (m * 1.0e6) as f32,
            _ => (m * 1.0e-6) as f32,
        }
    }
}

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

fn frand_mixed(s: &mut u32) -> f32 {
    let pick = (lcg(s) >> 8) % 5;
    let m = f64::from(lcg(s) >> 8) / f64::from(1u32 << 24);
    let v = match pick {
        0 => m * 2.0 - 1.0,            // [-1, 1)
        1 => m * 100.0,                // moderate
        2 => -(m * 100.0),             // negative
        3 => m * 1.0e6 - 5e5,          // large
        _ => (m * 2.0 - 1.0) * 1.0e-6, // tiny
    };
    v as f32
}

const RIBBONBASIS_VA: u64 = 0x7b7bc0;

struct BasisRng(u32);

impl BasisRng {
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
    fn vec3(&mut self) -> [f32; 3] {
        [self.f32_mixed(), self.f32_mixed(), self.f32_mixed()]
    }
    /// 16-float column-major C44Matrix, fuzzed.
    fn mat(&mut self) -> [f32; 16] {
        let mut m = [0f32; 16];
        for f in m.iter_mut() {
            *f = self.f32_mixed();
        }
        m
    }
}

fn basis_case(
    e: &mut Emu,
    this: u64,
    matrix: u64,
    arr: u64,
    m: [f32; 16],
    bases: Bases,
    p20: [f32; 3],
    p14c: [f32; 3],
    pts: &[[f32; 3]],
    ctx: &str,
) {
    let count = pts.len() as u32;

    // --- lay out `this` ---
    let put3 = |e: &mut Emu, off: u64, v: [f32; 3]| {
        e.write_u32(this + off, v[0].to_bits());
        e.write_u32(this + off + 4, v[1].to_bits());
        e.write_u32(this + off + 8, v[2].to_bits());
    };
    put3(e, 0x20, p20);
    put3(e, 0x14c, p14c);
    put3(e, 0x94, bases.b94);
    put3(e, 0x7c, bases.b7c);
    put3(e, 0xa0, bases.ba0);
    put3(e, 0x88, bases.b88);
    e.write_u32(this + 0x3c, count); // array count (int)
    e.write_u32(this + 0x40, arr as u32); // array base pointer

    // --- the array of points (stride 0x14; only the first 12 bytes is the transformed point) ---
    for (i, p) in pts.iter().enumerate() {
        let base = arr + (i as u64) * 0x14;
        e.write_u32(base, p[0].to_bits());
        e.write_u32(base + 4, p[1].to_bits());
        e.write_u32(base + 8, p[2].to_bits());
    }

    // --- the matrix (column-major [f32;16]) ---
    for (i, &f) in m.iter().enumerate() {
        e.write_u32(matrix + 4 * i as u64, f.to_bits());
    }

    // run the whole function: __thiscall ecx=this, stack arg = matrix ptr.
    e.call(RIBBONBASIS_VA, this as u32, 0, &[matrix as u32]);

    // --- expected: point legs via cmath transform_point4 (in place), bases via rotate_bases ---
    let exp_b = rotate_bases(bases, &m);
    let exp_p20 = transform_point4(&p20, &m);
    let exp_p14c = transform_point4(&p14c, &m);

    let chk3 = |e: &Emu, off: u64, exp: [f32; 3], tag: &str| {
        assert_f32(e.read_f32(this + off), exp[0], &format!("{ctx} {tag}.x"));
        assert_f32(
            e.read_f32(this + off + 4),
            exp[1],
            &format!("{ctx} {tag}.y"),
        );
        assert_f32(
            e.read_f32(this + off + 8),
            exp[2],
            &format!("{ctx} {tag}.z"),
        );
    };
    chk3(e, 0x94, exp_b.b94, "b94");
    chk3(e, 0x7c, exp_b.b7c, "b7c");
    chk3(e, 0xa0, exp_b.ba0, "ba0");
    chk3(e, 0x88, exp_b.b88, "b88");
    chk3(e, 0x20, exp_p20, "p20");
    chk3(e, 0x14c, exp_p14c, "p14c");

    for (i, p) in pts.iter().enumerate() {
        let exp = transform_point4(p, &m);
        let base = arr + (i as u64) * 0x14;
        assert_f32(e.read_f32(base), exp[0], &format!("{ctx} arr[{i}].x"));
        assert_f32(e.read_f32(base + 4), exp[1], &format!("{ctx} arr[{i}].y"));
        assert_f32(e.read_f32(base + 8), exp[2], &format!("{ctx} arr[{i}].z"));
    }
}

fn fuzz_range(s: &mut u32) -> f32 {
    match (next(s) >> 8) % 5 {
        0 => frand(s, -1.0, 1.0),
        1 => frand(s, -1000.0, 1000.0),
        2 => frand(s, -1.0e30, 1.0e30),
        3 => frand(s, -1.0e-20, 1.0e-20),
        _ => frand(s, -100.0, 100.0),
    }
}

/// `0x7b6990` whole.
#[test]
fn ribbon_frame_build_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let this = emu.alloc(0x200); // CRibbonEmitter

    let mut s = 0x7b69_9000u32;

    // a structured base frame, then variations + fuzz.
    let make = |s: &mut u32, degenerate: bool| -> RibbonFrame {
        let p0 = [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)];
        let p1 = if degenerate {
            p0
        } else {
            [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)]
        };
        RibbonFrame {
            p0,
            p1,
            va: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
            vb: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
            wc: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
            wd: [fuzz_f32(s), fuzz_f32(s), fuzz_f32(s)],
            s: fuzz_f32(s),
            s2: fuzz_f32(s),
        }
    };

    let structured: Vec<RibbonFrame> = vec![
        RibbonFrame {
            p0: [0.0, 0.0, 0.0],
            p1: [1.0, 0.0, 0.0],
            va: [0.0, 1.0, 0.0],
            vb: [0.0, 0.0, 1.0],
            wc: [1.0, 0.0, 0.0],
            wd: [0.0, 1.0, 0.0],
            s: 0.5,
            s2: 0.5,
        },
        // degenerate: p1 == p0 (len 0).
        RibbonFrame {
            p0: [3.0, 4.0, 5.0],
            p1: [3.0, 4.0, 5.0],
            va: [1.0, 1.0, 1.0],
            vb: [1.0, 0.0, 0.0],
            wc: [0.0, 0.0, 1.0],
            wd: [1.0, 1.0, 0.0],
            s: -2.0,
            s2: 3.0,
        },
    ];

    let fuzz = (0..400).map(|i| make(&mut s, i % 17 == 0));
    for f in structured.into_iter().chain(fuzz) {
        // lay out every field the function reads.
        let w3 = |emu: &mut Emu, off: u64, v: [f32; 3]| {
            for i in 0..3u64 {
                emu.write_u32(off + i * 4, v[i as usize].to_bits());
            }
        };
        w3(&mut emu, this + 0x14c, f.p0);
        w3(&mut emu, this + 0x20, f.p1);
        w3(&mut emu, this + 0x7c, f.va);
        w3(&mut emu, this + 0x88, f.vb);
        w3(&mut emu, this + 0x94, f.wc);
        w3(&mut emu, this + 0xa0, f.wd);
        emu.write_u32(this + 0x160, f.s.to_bits());
        emu.write_u32(this + 0x15c, f.s2.to_bits());
        // clear the outputs.
        for off in [0xc4u64, 0xd0, 0xdc, 0xe8, 0xac, 0xb8] {
            for i in 0..3u64 {
                emu.write_u32(this + off + i * 4, 0xdead_beef);
            }
        }
        // __thiscall: ecx = this, no stack args. ret.
        emu.call(0x7b6990, this as u32, 0, &[]);
        let r3 = |emu: &Emu, off: u64| -> [f32; 3] {
            [
                emu.read_f32(this + off),
                emu.read_f32(this + off + 4),
                emu.read_f32(this + off + 8),
            ]
        };
        let ours = ribbon_frame_build(f);
        let bout: [([f32; 3], [f32; 3], &str); 6] = [
            (r3(&emu, 0xc4), ours.corner0, "corner0"),
            (r3(&emu, 0xd0), ours.corner1, "corner1"),
            (r3(&emu, 0xdc), ours.corner2, "corner2"),
            (r3(&emu, 0xe8), ours.corner3, "corner3"),
            (r3(&emu, 0xac), ours.width_c, "width_c"),
            (r3(&emu, 0xb8), ours.width_d, "width_d"),
        ];
        for (b, o, name) in bout {
            for i in 0..3 {
                assert_f32(b[i], o[i], &format!("frame {name}[{i}]"));
            }
        }
    }
    let _ = next(&mut s);
}

/// `0x7b6c30` whole with `0x7b6f00`, then with it stubbed.
#[test]
fn ribbon_segment_emit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };

    // (frame, tex, t, cap, index) — structured edge cases then a fuzz sweep.
    let mut s = 0x7b6c_3000u32;
    let cases: Vec<(RibbonEmitFrame, f32, f32, u32, u32)> = {
        let mut v = vec![
            (make_emit_frame_const(), 0.0, 0.0, 64, 0), // t = 0 (w = 1)
            (make_emit_frame_const(), -0.5, 1.0, 64, 3), // t = 1 (w = 0)
            (make_emit_frame_const(), 0.25, 0.5, 10, 9), // index 9 +1 == cap 10 -> wrap to 0
            (make_emit_frame_const(), 2.0, -1.5, 10, 0), // t outside [0, 1]: 0x7b6c30 does not clamp it
            (make_emit_frame_const(), 0.5, 0.5, 4, 3),   // index 3 +1 == cap 4 -> wrap to 0
        ];
        for i in 0..400u32 {
            let f = make_emit_frame(&mut s);
            let t = fuzz_f32(&mut s) * 0.5 + 0.5; // sweep around [0,1] incl. negative/over-1
            let tex = fuzz_f32(&mut s);
            let cap = 4 + (i % 13);
            let index = i % cap;
            v.push((f, tex, t, cap, index));
        }
        v
    };

    // ---- pass 1: real callee (0x7b6f00 runs). Diff the two vertices, the scalar, AND the
    // live-index advance, all bit-exact against the binary. ----
    {
        let mut emu = Emu::new(&wow);
        emu.set_reg(RegisterX86::FPCW, 0x027F);
        let this = emu.alloc(0x200); // CRibbonEmitter
        let render_base = emu.alloc(0x4000); // record array (0x28 stride)
        let scalar_base = emu.alloc(0x1000); // per-vertex scalar array (4 stride)

        for (f, tex, t, cap, index) in cases.iter().copied() {
            emit_layout(&mut emu, this, &f, cap, index, render_base, scalar_base);
            let rec = render_base + u64::from(index) * 0x28;
            for j in 0..3u64 {
                emu.write_u32(rec + j * 4, 0xdead_beef);
                emu.write_u32(rec + 0x14 + j * 4, 0xdead_beef);
            }
            emu.write_u32(scalar_base + u64::from(index) * 4, 0xdead_beef);
            // __thiscall: ecx = this, stack args = (tex, t, inc). inc = 1 (per-segment step). ret
            // 0xc.
            emu.call(0x7b6c30, this as u32, 0, &[tex.to_bits(), t.to_bits(), 1]);

            let b_a = [
                emu.read_f32(rec),
                emu.read_f32(rec + 4),
                emu.read_f32(rec + 8),
            ];
            let b_b = [
                emu.read_f32(rec + 0x14),
                emu.read_f32(rec + 0x14 + 4),
                emu.read_f32(rec + 0x14 + 8),
            ];
            let b_tex = emu.read_f32(scalar_base + u64::from(index) * 4);
            let b_index = emu.read_u32(this + 0x14);

            let ours: RibbonSegmentOut = ribbon_segment_emit(f, tex, t);
            for j in 0..3 {
                assert_f32(b_a[j], ours.vert_a[j], &format!("seg vert_a[{j}] t={t}"));
                assert_f32(b_b[j], ours.vert_b[j], &format!("seg vert_b[{j}] t={t}"));
            }
            assert_f32(b_tex, ours.tex, &format!("seg tex t={t}"));
            assert_eq!(
                b_index,
                ribbon_live_index_advance(RibbonLiveIndex { index, cap }, 1),
                "seg live-index advance (real callee) index={index} cap={cap}"
            );
        }
    }

    // ---- pass 2: stub 0x7b6f00 and assert (a) the vertex
    // math is unchanged (the callee is a pure side-effect that runs AFTER all stores), and (b) the
    // recorded invocation = exactly one call with args (&live_index, inc). Stubbing also proves the
    // index advance is wholly the callee's (the stubbed index stays put). ----
    {
        let mut emu = Emu::new(&wow);
        emu.set_reg(RegisterX86::FPCW, 0x027F);
        let this = emu.alloc(0x200);
        let render_base = emu.alloc(0x4000);
        let scalar_base = emu.alloc(0x1000);
        emu.stub(0x7b6f00, 0, 2); // __thiscall, 2 stack args -> `ret 8`

        let mut s2 = 0x7b6c_3002u32;
        for i in 0..96u32 {
            let f = make_emit_frame(&mut s2);
            let t = fuzz_f32(&mut s2) * 0.5 + 0.5;
            let tex = fuzz_f32(&mut s2);
            let inc = 1 + (i % 3); // vary inc so the recorded arg is meaningfully checked
            emit_layout(&mut emu, this, &f, 64, 0, render_base, scalar_base);
            let rec = render_base;
            for j in 0..3u64 {
                emu.write_u32(rec + j * 4, 0xdead_beef);
                emu.write_u32(rec + 0x14 + j * 4, 0xdead_beef);
            }
            emu.clear_calls();
            emu.call(0x7b6c30, this as u32, 0, &[tex.to_bits(), t.to_bits(), inc]);

            let s_a = [
                emu.read_f32(rec),
                emu.read_f32(rec + 4),
                emu.read_f32(rec + 8),
            ];
            let s_b = [
                emu.read_f32(rec + 0x14),
                emu.read_f32(rec + 0x14 + 4),
                emu.read_f32(rec + 0x14 + 8),
            ];
            let ours = ribbon_segment_emit(f, tex, t);
            for j in 0..3 {
                assert_f32(s_a[j], ours.vert_a[j], &format!("seg(stub) vert_a[{j}]"));
                assert_f32(s_b[j], ours.vert_b[j], &format!("seg(stub) vert_b[{j}]"));
            }
            // invocation record: exactly one call to 0x7b6f00 with args (&live_index, inc).
            let calls = emu.calls();
            assert_eq!(calls.len(), 1, "seg(stub): one 0x7b6f00 call");
            assert_eq!(calls[0].addr, 0x7b6f00, "seg(stub): callee addr");
            assert_eq!(
                calls[0].args[0],
                (this + 0x14) as u32,
                "seg(stub): arg0 = &live_index"
            );
            assert_eq!(calls[0].args[1], inc, "seg(stub): arg1 = inc");
            assert_eq!(
                emu.read_u32(this + 0x14),
                0,
                "seg(stub): index untouched when callee stubbed"
            );
        }
        let _ = next(&mut s2);
    }

    let _ = next(&mut s);
}

/// `0x7b6f30` whole.
#[test]
fn particle_tile_uv_rect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let this = emu.alloc(0x200); // CRibbonEmitter / particle emitter

    // (frame, cols, cell_w, cell_h, base_u, base_v)
    let structured: &[(u32, u32, f32, f32, f32, f32)] = &[
        (0, 4, 0.25, 0.25, 0.0, 0.0), // frame 0: col=row=0 → both corners = base
        (3, 4, 0.25, 0.25, 0.0, 0.0), // frame == cols-1: max col, row 0
        (4, 4, 0.25, 0.25, 0.0, 0.0), // frame == cols: col 0, row 1
        (7, 4, 0.25, 0.25, 0.125, 0.5), // col 3, row 1, with a base offset
        (15, 4, 0.25, 0.25, 0.0, 0.0), // col 3, row 3
        (1, 1, 0.5, 0.5, 0.0, 0.0),   // single column: col always 0, row == frame
        (123, 7, -0.1, 0.2, -0.3, 0.4), // negative cell_w / base_u sign propagation
        (1000, 16, 1.0e6, 1.0e-6, 1.0, -1.0), // large/tiny cell sizes
        (0xffff_ffff, 256, 0.00390625, 0.00390625, 0.0, 0.0), // max u32 frame
    ];

    let mut s = 0x7b6f_3000u32;
    let fuzz = (0..600).map(|_| {
        let frame = next(&mut s);
        let cols = 1 + (next(&mut s) % 512); // cols in [1, 512] (never 0 → no #DE)
        (
            frame,
            cols,
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
            fuzz_f32(&mut s),
        )
    });

    for (frame, cols, cell_w, cell_h, base_u, base_v) in structured.iter().copied().chain(fuzz) {
        // lay out every field the function reads.
        emu.write_u32(this + 0x158, frame);
        emu.write_u32(this + 0x144, cols);
        emu.write_u32(this + 0x5c, cell_w.to_bits());
        emu.write_u32(this + 0x60, cell_h.to_bits());
        emu.write_u32(this + 0x134, base_u.to_bits());
        emu.write_u32(this + 0x130, base_v.to_bits());
        // clear the four outputs.
        for off in [0x70u64, 0x6c, 0x78, 0x74] {
            emu.write_u32(this + off, 0xdead_beef);
        }
        // __thiscall: ecx = this, no stack args. ret.
        emu.call(0x7b6f30, this as u32, 0, &[]);
        let b_u0 = emu.read_f32(this + 0x70);
        let b_v0 = emu.read_f32(this + 0x6c);
        let b_u1 = emu.read_f32(this + 0x78);
        let b_v1 = emu.read_f32(this + 0x74);

        let ours = particle_tile_uv_rect(TileUvIn {
            frame,
            cols,
            cell_w,
            cell_h,
            base_u,
            base_v,
        });
        let tag = format!("tile_uv frame={frame} cols={cols}");
        assert_f32(b_u0, ours.u0, &format!("{tag} u0"));
        assert_f32(b_v0, ours.v0, &format!("{tag} v0"));
        assert_f32(b_u1, ours.u1, &format!("{tag} u1"));
        assert_f32(b_v1, ours.v1, &format!("{tag} v1"));
    }
    let _ = next(&mut s);
}

/// `0x7b7e9b..0x7b7ea9`: the expiry test read at its `fnstsw`, an `f32` step and NaN included.
#[test]
fn ribbonage_expire_sum_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x180);
    let sim = e.alloc(0x100); // [esi+0xc] base; element 0 = the vertex_age f32
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    e.write_u32(this + 0xc, sim as u32); // [esi+0xc] = sim array base

    // Run the binary's compare leg, read AH after fnstsw; compare to our own X-vs-age disposition.
    let run_case = |e: &mut Emu, dt: f32, vage: f32, age: f32, ctx: &str| {
        e.write_u32(sim, vage.to_bits()); // vertex_age at [esi+0xc]+0
        e.write_u32(this + 0xf8, age.to_bits()); // [esi+0xf8] = age
        e.write_u32(ebp + 8, dt.to_bits()); // [ebp+8] = dt
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ECX, sim); // ecx = [esi+0xc]
        e.set_reg(RegisterX86::EAX, 0); // eax = index 0
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b7e9b, 0x7b7ea9); // fld;fadd;fcomp;fnstsw — halt at `test ah,0x41`
        let ah = read_ah(e);
        assert_eq!(
            ah & 0x41 == 0,
            edge_expired(dt, vage, age),
            "{ctx}: binary ah={ah:#04x} (X={} L={age})",
            expire_sum(dt, vage)
        );
    };

    // ==threshold probes: X exactly == age (C3 boundary), X just over/under age.
    run_case(&mut e, 1.0, 4.0, 5.0, "X==age (5.0)"); // 1+4 == 5 -> C3
    run_case(&mut e, 1.0, 3.999999, 5.0, "X just under age");
    run_case(&mut e, 1.0, 4.0000005, 5.0, "X just over age");
    run_case(&mut e, 0.0, 0.0, 0.0, "all-zero (X==age==0)");
    run_case(&mut e, -1.0, 1.0, 0.0, "X==0==age via negatives");
    run_case(
        &mut e,
        1.0,
        f32::from_bits(0x3080_0000),
        1.0,
        "X above L by 2^-30, in the register",
    );
    run_case(&mut e, f32::NAN, 1.0, 5.0, "NaN dt keeps the edge");
    run_case(&mut e, 1.0, 1.0, f32::NAN, "NaN L keeps the edge");

    let mut rng = AgeRng(0x7b7e_9b00);
    for i in 0..4000u32 {
        let dt = rng.f32_mixed();
        let vage = rng.f32_mixed();
        let age = rng.f32_mixed();
        run_case(&mut e, dt, vage, age, &format!("expire fuzz[{i}]"));
    }
}

/// `0x7b7ee0..0x7b7ef8`: the edge accumulator.
#[test]
fn ribbonage_interp_factor_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x180);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;

    let run_case = |e: &mut Emu, dt: f32, rate: f32, phase: f32, ctx: &str| {
        e.write_u32(ebp + 8, dt.to_bits()); // [ebp+8] = dt
        e.write_u32(this + 0xf4, rate.to_bits()); // [esi+0xf4] = rate
        e.write_u32(this + 0x1c, phase.to_bits()); // [esi+0x1c] = phase
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        // fld;fmul;fadd;fstp [ebp+0xc] — one run through the fstp (until is exclusive, halt at the
        // following `call 0x7b6990` @0x7b7ef8). A split run would reset the x87 stack mid-sequence.
        e.run(0x7b7ee0, 0x7b7ef8);
        let bin = e.read_f32(ebp + 0xc);
        let ours = interp_factor(dt, rate, phase);
        assert_f32(bin, ours, ctx);
    };

    // ==threshold probes: factor lands exactly on 1.0 (the next branch's threshold), and 0.
    run_case(&mut e, 0.5, 2.0, 0.0, "factor==1.0"); // 0.5*2 + 0 = 1.0
    run_case(&mut e, 0.0, 5.0, 1.0, "factor==1.0 via phase"); // 0 + 1
    run_case(&mut e, 0.0, 0.0, 0.0, "factor==0");
    run_case(&mut e, 1.0, -1.0, 0.5, "factor==-0.5 (negative)");

    let mut rng = AgeRng(0x7b7e_e000);
    for i in 0..4000u32 {
        let dt = rng.f32_mixed();
        let rate = rng.f32_mixed();
        let phase = rng.f32_mixed();
        run_case(&mut e, dt, rate, phase, &format!("interp fuzz[{i}]"));
    }
}

/// `0x7b7f0d..0x7b7f2b`: the reciprocal and the floor argument.
#[test]
fn ribbonage_recip_floorarg_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x180);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;

    let run_case = |e: &mut Emu, factor: f32, phase: f32, ctx: &str| {
        e.write_u32(ebp + 0xc, factor.to_bits()); // [ebp+0xc] = factor
        e.write_u32(this + 0x1c, phase.to_bits()); // [esi+0x1c] = phase
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b7f0d, 0x7b7f2b); // both blocks; halt at `call 0x73ff3f`
        let bin_recip = e.read_f32(ebp - 0x10);
        // floor arg is the f64 at [esp] (the qword pushed for the floor call). ESP here = SLICE_ESP
        // - 8 (the `sub esp,8` at 0x7b7f10 reserved it).
        let mut b = [0u8; 8];
        b.copy_from_slice(&e.read(SLICE_ESP - 8, 8));
        let bin_floorarg = f64::from_bits(u64::from_le_bytes(b));

        assert_f32(
            bin_recip,
            recip_span(factor, phase),
            &format!("{ctx} recip"),
        );
        let ours_fa = floor_arg(factor);
        assert_eq!(
            bin_floorarg.to_bits(),
            ours_fa.to_bits(),
            "{ctx} floor_arg: binary={bin_floorarg} ours={ours_fa}"
        );
    };

    // ==threshold probes: factor==1.0 (floor_arg==0.0); span exactly 2.0 (recip==0.5).
    run_case(&mut e, 1.0, 0.0, "factor==1.0 (floor_arg 0)");
    run_case(&mut e, 3.0, 1.0, "span==2 (recip 0.5)");
    run_case(&mut e, 5.0, 4.0, "span==1 (recip 1.0)");
    run_case(&mut e, 2.0, 1.5, "span==0.5 (recip 2.0)");

    let mut rng = AgeRng(0x7b7f_0d00);
    let mut n = 0u32;
    let mut i = 0u32;
    while n < 4000 {
        i += 1;
        let factor = 1.0 + rng.f32_mixed().abs() % 1.0e6; // factor >= 1.0
        let phase = rng.f32_mixed();
        if (factor - phase).abs() < 1.0e-6 {
            continue; // avoid div-by-zero span (the binary would emit inf too, but skip the edge)
        }
        run_case(&mut e, factor, phase, &format!("recip fuzz[{i}]"));
        n += 1;
    }
}

/// `0x7b7f40..0x7b7f60`: an edge's `t` and age.
#[test]
fn ribbonage_segment_iter_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x180);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;

    let run_case = |e: &mut Emu, counter: f32, phase: f32, recip: f32, dt: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, counter.to_bits()); // [ebp-0x4] = counter
        e.write_u32(this + 0x1c, phase.to_bits()); // [esi+0x1c] = phase
        e.write_u32(ebp - 0x10, recip.to_bits()); // [ebp-0x10] = recip-span
        e.write_u32(ebp + 8, dt.to_bits()); // [ebp+8] = dt
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b7f40, 0x7b7f60); // both halves; halt at `call 0x7b6c30`
        let bin_frac = e.read_f32(ebp - 0x8);
        // emit_arg was `fstp dword [esp]` — the push at 0x7b7f43 (push 0x1) lowered ESP by 4, then
        // pushes at 0x7b7f54/0x7b7f58, so [esp] is below SLICE_ESP. Read the f32 the fstp wrote: it
        // is the top-of-stack dword at the point of the call. ESP at the store = SLICE_ESP - 0xc.
        let bin_arg = e.read_f32(SLICE_ESP - 0xc);

        let frac = segment_fraction(counter, phase, recip);
        assert_f32(bin_frac, frac, &format!("{ctx} seg_frac"));
        assert_f32(
            bin_arg,
            segment_emit_arg(frac, dt),
            &format!("{ctx} emit_arg"),
        );
    };

    // ==threshold probes: counter==phase (frac 0 -> arg 0/-0), and frac·dt==exact.
    run_case(&mut e, 1.0, 1.0, 3.0, 0.5, "counter==phase (frac 0)");
    run_case(&mut e, 2.0, 1.0, 1.0, 1.0, "frac==1, arg==-1");
    run_case(&mut e, 1.0, 0.0, 2.0, 0.0, "dt==0 (arg -0)");
    run_case(&mut e, 3.0, 1.0, 0.5, -2.0, "negative dt");

    let mut rng = AgeRng(0x7b7f_4000);
    for i in 0..4000u32 {
        let counter = rng.f32_mixed();
        let phase = rng.f32_mixed();
        let recip = rng.f32_mixed();
        let dt = rng.f32_mixed();
        run_case(&mut e, counter, phase, recip, dt, &format!("seg fuzz[{i}]"));
    }
}

/// `0x7b7f66..0x7b7f72`: the edge counter.
#[test]
fn ribbonage_next_counter_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;

    let run_case = |e: &mut Emu, counter: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, counter.to_bits());
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b7f66, 0x7b7f72); // fld;fadd 1.0;fstp [ebp-0x4]
        let bin = e.read_f32(ebp - 0x4);
        assert_f32(bin, next_counter(counter), ctx);
    };

    run_case(&mut e, 1.0, "counter 1->2");
    run_case(&mut e, 0.0, "counter 0->1");
    run_case(&mut e, -1.0, "counter -1->0");
    run_case(&mut e, 16777216.0, "counter 2^24 (1.0 lost to rounding)");

    let mut rng = AgeRng(0x7b7f_6600);
    for i in 0..4000u32 {
        run_case(&mut e, rng.f32_mixed(), &format!("counter fuzz[{i}]"));
    }
}

/// `0x7b7f75..0x7b7f98`: the carry, `floor` included.
#[test]
fn ribbonage_phase_frac_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x180);
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;

    let run_case = |e: &mut Emu, factor: f32, ctx: &str| {
        e.write_u32(ebp + 0xc, factor.to_bits()); // [ebp+0xc] = factor
                                                  // The block at 0x7b7f81/0x7b7f82 pushes
                                                  // [ebp-0x10] then [ebp-0x14] to form the qword
                                                  // floor arg = [ebp-0x14]_lo : [ebp-0x10]_hi. The
                                                  // store `fstp qword [ebp-0x14]` at 0x7b7f78
                                                  // writes BOTH dwords of (f64)factor, so
                                                  // [ebp-0x14] (lo) and [ebp-0x10] are overwritten
                                                  // correctly by the routine itself before the push
                                                  // — no pre-seed needed.
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b7f75, 0x7b7f98); // through the floor call + fsubr + fstp [esi+0x1c]
        let bin = e.read_f32(this + 0x1c);
        assert_f32(bin, phase_frac(factor), ctx);
    };

    // ==threshold probes: integers (frac 0), exact halves, negative, near-integer boundary.
    run_case(&mut e, 3.0, "factor==3 (frac 0)");
    run_case(&mut e, 0.0, "factor==0");
    run_case(&mut e, 2.5, "factor==2.5 (frac 0.5)");
    run_case(&mut e, -1.5, "factor==-1.5 (frac 0.5, floor -2)");
    run_case(&mut e, 0.9999999, "just under 1");
    run_case(&mut e, -0.0000001, "just under 0 (floor -1)");

    let mut rng = AgeRng(0x7b7f_7500);
    for i in 0..4000u32 {
        // keep factor finite & in a range where floor is well-defined (avoid huge magnitudes where
        // factor-floor underflows to 0 trivially — still valid, just include some).
        let f = rng.f32_mixed();
        run_case(&mut e, f, &format!("frac fuzz[{i}]"));
    }
}

/// `0x7b7ff6..0x7b806f`: an edge's gravity, age and texture words.
#[test]
fn ribbonage_gravity_emit_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x180);
    let sim = e.alloc(0x100); // [esi+0xc] base
    let render = e.alloc(0x400); // [esi+0x40] base; record stride 0x28
    let frame = e.alloc(0x80);
    let ebp = frame + 0x40;
    e.write_u32(this + 0xc, sim as u32); // [esi+0xc] = sim base
    e.write_u32(this + 0x40, render as u32); // [esi+0x40] = render base

    #[allow(clippy::too_many_arguments)]
    let run_case = |e: &mut Emu,
                    idx: u32,
                    sim_g: f32,
                    dt: f32,
                    gravity: f32,
                    f8: f32,
                    f1c: f32,
                    c5c: f32,
                    c58: f32,
                    b70: f32,
                    f6c: f32,
                    f74: f32,
                    ctx: &str| {
        let sim_el = sim + 4 * u64::from(idx);
        let rec = render + 0x28 * u64::from(idx);
        e.write_u32(sim_el, sim_g.to_bits()); // sim[idx]
        e.write_u32(rec + 8, f8.to_bits()); // [eax+8] velocity field
        e.write_u32(rec + 0x1c, f1c.to_bits()); // [eax+0x1c] velocity field
        e.write_u32(this + 0x164, gravity.to_bits()); // [esi+0x164] gravity
        e.write_u32(this + 0x5c, c5c.to_bits()); // [esi+0x5c]
        e.write_u32(this + 0x58, c58.to_bits()); // [esi+0x58]
        e.write_u32(this + 0x70, b70.to_bits()); // [esi+0x70]
        e.write_u32(this + 0x6c, f6c.to_bits()); // [esi+0x6c]
        e.write_u32(this + 0x74, f74.to_bits()); // [esi+0x74]
        e.write_u32(ebp + 8, dt.to_bits()); // [ebp+8] = dt
        e.set_reg(RegisterX86::ESI, this);
        e.set_reg(RegisterX86::ECX, u64::from(idx)); // ecx = index
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.run(0x7b7ff6, 0x7b806f); // the whole loop body; halt at `call 0x7b6f00`

        // --- our reference: replicate the binary's exact op sequence ---
        let term = gravity_term(sim_g, dt, gravity);
        let (our_v8, our_v1c) = add_gravity(f8, f1c, term);
        let sim_updated = advance_sim(sim_g, dt);
        let v = record_value(sim_updated, c5c, c58, b70);
        let (d_0xc, d_0x10, d_0x20, d_0x24) = record_dwords(v, f6c, f74);

        assert_f32(e.read_f32(rec + 8), our_v8, &format!("{ctx} [eax+8] vel"));
        assert_f32(
            e.read_f32(rec + 0x1c),
            our_v1c,
            &format!("{ctx} [eax+0x1c] vel"),
        );
        assert_f32(e.read_f32(sim_el), sim_updated, &format!("{ctx} sim+=dt"));
        assert_f32(
            e.read_f32(rec + 0xc),
            d_0xc,
            &format!("{ctx} [eax+0xc] V(fst-mem)"),
        );
        assert_f32(
            e.read_f32(rec + 0x10),
            d_0x10,
            &format!("{ctx} [eax+0x10]=[+0x6c]"),
        );
        assert_f32(
            e.read_f32(rec + 0x20),
            d_0x20,
            &format!("{ctx} [eax+0x20] V(live-st0)"),
        );
        assert_f32(
            e.read_f32(rec + 0x24),
            d_0x24,
            &format!("{ctx} [eax+0x24]=[+0x74]"),
        );
    };

    // ==threshold / edge probes exercising the live-st0 `fst` boundary and the 2g self-add:
    //  - V exactly on a representable f32 (no rounding difference between the mem copy & live st0);
    //  - V needing a round (the two rounds must still agree);
    //  - g==0 (2g==0), dt==0 (term 0), sign mixes.
    run_case(
        &mut e, 0, 1.0, 0.5, 2.0, 10.0, 20.0, 3.0, 4.0, 5.0, 7.0, 9.0, "nominal",
    );
    run_case(
        &mut e,
        1,
        0.0,
        1.0,
        1.0,
        0.0,
        0.0,
        1.0,
        1.0,
        0.0,
        0.0,
        0.0,
        "g==0,b==0",
    );
    run_case(
        &mut e,
        2,
        5.0,
        0.0,
        9.0,
        1.0,
        2.0,
        2.0,
        3.0,
        4.0,
        1.0,
        1.0,
        "dt==0 (term 0)",
    );
    // V chosen to require an f32 round: sim_updated·c5c·c58 + b70 with an odd-bit product.
    run_case(
        &mut e, 3, 1.0000001, 0.0, 1.0, 0.0, 0.0, 1.0000001, 1.0000001, 0.1, 0.0, 0.0, "V rounds",
    );
    run_case(
        &mut e,
        4,
        -2.0,
        -1.0,
        -3.0,
        -1.0,
        -1.0,
        -2.0,
        -2.0,
        -1.0,
        -1.0,
        -1.0,
        "all-negative",
    );
    run_case(
        &mut e,
        5,
        1.0e6,
        1.0e-6,
        1.0,
        1.0,
        1.0,
        1.0e6,
        1.0e6,
        1.0,
        0.0,
        0.0,
        "big·small",
    );

    let mut rng = AgeRng(0x7b7f_f600);
    for i in 0..5000u32 {
        let idx = (rng.next() >> 8) % 0x19; // 25·0x28 = 0x3e8 < 0x400 — keep records in bounds
        run_case(
            &mut e,
            idx,
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            rng.f32_mixed(),
            &format!("grav fuzz[{i}]"),
        );
    }
}

/// `0x7b7e69..0x7b7e8c` and `0x7b8099`: the `dt` clamp, NaN included.
#[test]
fn dt_clamp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x300); // esi (life @ +0xf8)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, dt: f32, life: f32, ctx: &str| {
        e.write_u32(emitter + 0xf8, life.to_bits());
        e.set_reg(RegisterX86::ESI, emitter);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 8, dt.to_bits());
        e.run(0x7b7e69, 0x7b7e8c); // clamp lattice, both head/tail merge at 0x7b7e8c
        let bin = e.read_f32(ebp + 8);
        let ours = dt_clamp(dt, life);
        assert_f32(bin, ours, ctx);
    };

    // ==threshold probes: dt<0→0, dt==0 keep, dt==life→life, dt>life→life, dt<life keep.
    run_case(&mut e, -1.0, 5.0, "dt<0");
    run_case(&mut e, 0.0, 5.0, "dt==0");
    run_case(&mut e, 5.0, 5.0, "dt==life");
    run_case(&mut e, 7.0, 5.0, "dt>life");
    run_case(&mut e, 3.0, 5.0, "dt<life");
    run_case(&mut e, -0.0, 5.0, "dt==-0");
    run_case(&mut e, f32::NAN, 5.0, "NaN dt (L)");
    run_case(&mut e, 3.0, f32::NAN, "NaN L (L)");
    run_case(&mut e, -1.0, f32::NAN, "dt<0 with NaN L (0)");
    let mut s = 0x7b7e_6969u32;
    for i in 0..3000u32 {
        let dt = frand_mixed(&mut s);
        let life = frand_mixed(&mut s).abs();
        run_case(&mut e, dt, life, &format!("dtclamp fuzz[{i}]"));
    }
}

/// The bytes `0x7b7e9b..0x7b7ea1`, copied: the expiry sum as the register holds it.
#[test]
fn expire_sum_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);

    // 0x7b7e9b..0x7b7ea1 (6 bytes): fld [ebp+8]; fadd [ecx+4*eax]. Append `fstp qword [out]`.
    const SRC: u64 = 0x7b7e9b;
    const LEN: usize = 0x7b7ea1 - 0x7b7e9b;
    let mut code = e.read(SRC, LEN);
    let out = e.alloc(0x10);
    let isl = e.alloc(0x80);
    code.extend_from_slice(&[0xdd, 0x1d]); // fstp qword [out]
    code.extend_from_slice(&(out as u32).to_le_bytes());
    e.write(isl, &code);
    island_bytes_match(&e, isl, SRC, LEN);
    let end = isl + code.len() as u64;

    let agearr = e.alloc(0x40); // ecx points here (eax=index → [ecx+4*eax])
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, dt: f32, age: f32, ctx: &str| {
        e.write_u32(agearr, age.to_bits());
        e.set_reg(RegisterX86::ECX, agearr);
        e.set_reg(RegisterX86::EAX, 0);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 8, dt.to_bits());
        e.run(isl, end);
        let bin = read_f64(e, out);
        let ours = expire_sum(dt, age);
        assert_f64(bin, ours, ctx);
    };

    run_case(&mut e, 0.1, 1.0, "expire 0.1+1");
    run_case(&mut e, 0.0, 0.0, "expire zeros");
    run_case(&mut e, -1.0, 1.0, "expire cancel");
    run_case(
        &mut e,
        1.0,
        f32::from_bits(0x3080_0000),
        "expire sum below an f32 step",
    );
    let mut s = 0x7b7e_9b9bu32;
    for i in 0..3000u32 {
        let dt = frand_mixed(&mut s);
        let age = frand_mixed(&mut s);
        run_case(&mut e, dt, age, &format!("expire fuzz[{i}]"));
    }
}

/// `0x7b7ee0..0x7b7ef8`: the edge accumulator.
#[test]
fn emit_interp_head_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x300); // esi (rate @ +0xf4, carry @ +0x1c)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, dt: f32, rate: f32, carry: f32, ctx: &str| {
        e.write_u32(emitter + 0xf4, rate.to_bits());
        e.write_u32(emitter + 0x1c, carry.to_bits());
        e.set_reg(RegisterX86::ESI, emitter);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 8, dt.to_bits());
        e.run(0x7b7ee0, 0x7b7ef8); // fld dt; fmul rate; fadd carry; fstp [ebp+0xc]; halt at call
        let bin = e.read_f32(ebp + 0xc);
        let ours = interp_factor(dt, rate, carry);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 0.1, 10.0, 0.5, "interp basic");
    run_case(&mut e, 0.0, 0.0, 0.0, "interp zeros");
    let mut s = 0x7b7e_e0e0u32;
    for i in 0..3000u32 {
        run_case(
            &mut e,
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            &format!("interp fuzz[{i}]"),
        );
    }
}

/// `0x7b7f0d..0x7b7f2b`: the reciprocal and the floor argument.
#[test]
fn emit_t_recip_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x300); // esi (carry @ +0x1c)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, h: f32, carry: f32, ctx: &str| {
        e.write_u32(emitter + 0x1c, carry.to_bits());
        e.write_u32(ebp + 0xc, h.to_bits()); // [ebp+0xc] = h
        e.set_reg(RegisterX86::ESI, emitter);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b7f0d, 0x7b7f2b); // fsub; fdivr [1.0] → t; fsub [1.0] → floor-arg qword[esp]; halt at floor call
        let bin_t = e.read_f32(ebp - 0x10);
        // floor arg stored as qword at [esp] after `sub esp,8`: esp = SLICE_ESP - 8.
        let lo = e.read_u32(SLICE_ESP - 8);
        let hi = e.read_u32(SLICE_ESP - 8 + 4);
        let bin_floor_arg = f64::from_bits(u64::from(lo) | (u64::from(hi) << 32));
        let ours_t = recip_span(h, carry);
        let ours_floor_arg = floor_arg(h);
        assert_f32(bin_t, ours_t, &format!("{ctx} t"));
        assert_eq!(
            bin_floor_arg.to_bits(),
            ours_floor_arg.to_bits(),
            "{ctx} floor-arg: binary={:#018x} ours={:#018x}",
            bin_floor_arg.to_bits(),
            ours_floor_arg.to_bits()
        );
    };

    // h>=1 is the caller's gate; we exercise denom==0 (t=+Inf), exacts, and spread.
    run_case(&mut e, 2.0, 1.0, "trecip denom=1");
    run_case(&mut e, 1.0, 1.0, "trecip denom=0 (t=+Inf)");
    run_case(&mut e, 3.0, 1.0, "trecip denom=2 (t=0.5)");
    let mut s = 0x7b7f_0d0du32;
    for i in 0..3000u32 {
        let h = frand_mixed(&mut s);
        let carry = frand_mixed(&mut s);
        run_case(&mut e, h, carry, &format!("trecip fuzz[{i}]"));
    }
}

/// `0x7b7f40..0x7b7f60`: an edge's `t` and age.
#[test]
fn emit_segment_offset_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x300); // esi (carry @ +0x1c)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, frac: f32, carry: f32, t: f32, dt: f32, ctx: &str| {
        e.write_u32(emitter + 0x1c, carry.to_bits());
        e.write_u32(ebp - 0x4, frac.to_bits()); // [ebp-0x4] = frac (local_8)
        e.write_u32(ebp - 0x10, t.to_bits()); // [ebp-0x10] = t (the reciprocal)
        e.set_reg(RegisterX86::ESI, emitter);
        e.set_reg(RegisterX86::ECX, emitter);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 8, dt.to_bits()); // [ebp+8] = dt
        e.run(0x7b7f40, 0x7b7f60); // s=(frac−carry)·t → [ebp-0x8]; neg=−(s·dt) → [esp]; halt at emit call
        let bin_s = e.read_f32(ebp - 0x8);
        // neg stored by `fstp dword [esp]` after push 1 / push edx / push ecx: esp = SLICE_ESP -
        // 12.
        let bin_neg = e.read_f32(SLICE_ESP - 12);
        let s_out = segment_fraction(frac, carry, t);
        let neg = segment_emit_arg(s_out, dt);
        assert_f32(bin_s, s_out, &format!("{ctx} s"));
        assert_f32(bin_neg, neg, &format!("{ctx} neg"));
    };

    run_case(&mut e, 1.0, 0.0, 1.0, 0.1, "seg basic");
    run_case(&mut e, 0.0, 0.0, 0.0, 0.0, "seg zeros");
    run_case(&mut e, 2.0, 1.0, 0.5, -2.0, "seg neg dt");
    let mut s = 0x7b7f_4040u32;
    for i in 0..3000u32 {
        run_case(
            &mut e,
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            &format!("seg fuzz[{i}]"),
        );
    }
}

/// `0x7b7f66..0x7b7f72`: the edge counter.
#[test]
fn emit_frac_advance_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu, frac: f32, ctx: &str| {
        e.write_u32(ebp - 0x4, frac.to_bits());
        e.set_reg(RegisterX86::EBX, 2); // dec ebx at 0x7b7f65 — keep the jne off our halt; non-FP
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b7f66, 0x7b7f72); // fld [ebp-0x4]; fadd [1.0]; fstp [ebp-0x4]; halt at jne
        let bin = e.read_f32(ebp - 0x4);
        let ours = next_counter(frac);
        assert_f32(bin, ours, ctx);
    };

    run_case(&mut e, 1.0, "frac 1");
    run_case(&mut e, 0.0, "frac 0");
    run_case(&mut e, -1.0, "frac -1");
    let mut s = 0x7b7f_6666u32;
    for i in 0..3000u32 {
        run_case(&mut e, frand_mixed(&mut s), &format!("frac fuzz[{i}]"));
    }
}

/// `0x7b8002..0x7b8069`: an edge's gravity, age and texture coordinate.
#[test]
fn segment_point_write_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let emitter = e.alloc(0x300); // esi
    let agearr = e.alloc(0x40); // [esi+0xc] points here (age slot at +0, ecx index=0)
    let dest = e.alloc(0x80); // eax (vertex record: y@+8/+0x1c, p@+0xc/+0x20)
    let frame = e.alloc(0x200);
    let ebp = frame + 0x100;

    let run_case = |e: &mut Emu,
                    age: f32,
                    dt: f32,
                    gravity: f32,
                    scroll: f32,
                    texscale: f32,
                    base: f32,
                    ctx: &str| {
        e.write_u32(emitter + 0xc, agearr as u32); // [esi+0xc] = age array base
        e.write_u32(agearr, age.to_bits()); // age[0]
        e.write_u32(emitter + 0x164, gravity.to_bits());
        e.write_u32(emitter + 0x5c, scroll.to_bits());
        e.write_u32(emitter + 0x58, texscale.to_bits());
        e.write_u32(emitter + 0x70, base.to_bits());
        // preset the two y-fields to 0.0 so the stored `existing + g` isolates g.
        e.write_u32(dest + 8, 0u32);
        e.write_u32(dest + 0x1c, 0u32);
        e.set_reg(RegisterX86::ESI, emitter);
        e.set_reg(RegisterX86::EAX, dest); // eax = vertex dest pointer (set by pre-block 0x7b7ffc)
        e.set_reg(RegisterX86::EDX, agearr); // edx = [esi+0xc] (set by pre-block 0x7b7fff)
        e.set_reg(RegisterX86::ECX, 0); // index = 0
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.write_u32(ebp + 8, dt.to_bits());
        e.run(0x7b8002, 0x7b8069); // g into y-fields, age advance, texcoord p; halt at expire call
        let bin_g0 = e.read_f32(dest + 8);
        let bin_g1 = e.read_f32(dest + 0x1c);
        let bin_age = e.read_f32(agearr); // age slot overwritten with age'
        let bin_p0 = e.read_f32(dest + 0xc);
        let bin_p1 = e.read_f32(dest + 0x20);
        let (g0, g1) = add_gravity(0.0, 0.0, gravity_term(age, dt, gravity));
        let age_adv = advance_sim(age, dt);
        let p = record_value(age_adv, scroll, texscale, base) as f32;
        assert_f32(bin_g0, g0, &format!("{ctx} g(y@+8)"));
        assert_f32(bin_g1, g1, &format!("{ctx} g(y@+0x1c)"));
        assert_f32(bin_age, age_adv, &format!("{ctx} age'"));
        assert_f32(bin_p0, p, &format!("{ctx} p(@+0xc)"));
        assert_f32(bin_p1, p, &format!("{ctx} p(@+0x20)"));
    };

    run_case(&mut e, 1.0, 0.1, 9.8, 0.5, 2.0, 0.0, "segpt basic");
    run_case(&mut e, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, "segpt zeros");
    let mut s = 0x7b80_0202u32;
    for i in 0..4000u32 {
        run_case(
            &mut e,
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            frand_mixed(&mut s),
            &format!("segpt fuzz[{i}]"),
        );
    }
}

/// `0x7b7bc0` whole, `0x7bcc60` included.
#[test]
fn ribbonbasis_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let this = e.alloc(0x200); // ≥0x160; the function touches up to this+0x14c+8
    let matrix = e.alloc(0x40); // C44Matrix (16 f32)
    let arr = e.alloc(0x14 * 8); // up to 8 array points, stride 0x14
                                 // ECX/EDX scratch handled by call(); registers reset per call.
    let _ = RegisterX86::EAX;

    let ident = {
        let mut m = [0f32; 16];
        m[0] = 1.0;
        m[5] = 1.0;
        m[10] = 1.0;
        m[15] = 1.0;
        m[12] = 7.0;
        m[13] = -3.0;
        m[14] = 11.0;
        m
    };
    let bases0 = Bases {
        b94: [1.0, 0.0, 0.0],
        b7c: [0.0, 1.0, 0.0],
        ba0: [0.0, 0.0, 1.0],
        b88: [1.0, 1.0, 1.0],
    };

    // --- deterministic edge / branch-boundary probes ---
    // count=0: the `cmp [edi+0x3c],ebx; jbe` skips the loop (no array element transformed).
    basis_case(
        &mut e,
        this,
        matrix,
        arr,
        ident,
        bases0,
        [2.0, 5.0, -4.0],
        [-1.0, 3.0, 9.0],
        &[],
        "identity count=0",
    );
    // count=1: the loop's ENTER branch (one element transformed).
    basis_case(
        &mut e,
        this,
        matrix,
        arr,
        ident,
        bases0,
        [2.0, 5.0, -4.0],
        [-1.0, 3.0, 9.0],
        &[[0.5, -0.5, 2.0]],
        "identity count=1",
    );
    // count=2: multi-iteration loop (stride 0x14 advance).
    basis_case(
        &mut e,
        this,
        matrix,
        arr,
        ident,
        bases0,
        [2.0, 5.0, -4.0],
        [-1.0, 3.0, 9.0],
        &[[0.5, -0.5, 2.0], [-3.0, 7.0, 0.25]],
        "identity count=2",
    );
    // zero matrix (all components → translation only / 0): exercises the additive-zero path.
    basis_case(
        &mut e,
        this,
        matrix,
        arr,
        [0f32; 16],
        bases0,
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0],
        &[[1.0, 1.0, 1.0]],
        "zero matrix count=1",
    );
    // zero basis vectors + zero points.
    basis_case(
        &mut e,
        this,
        matrix,
        arr,
        ident,
        Bases {
            b94: [0.0; 3],
            b7c: [0.0; 3],
            ba0: [0.0; 3],
            b88: [0.0; 3],
        },
        [0.0; 3],
        [0.0; 3],
        &[[0.0, 0.0, 0.0]],
        "all-zero count=1",
    );

    // --- fuzz: random matrix, bases, points, varying array counts 0..=4 ---
    let mut rng = BasisRng(0x7b7b_c000);
    for i in 0..4000u32 {
        let m = rng.mat();
        let bases = Bases {
            b94: rng.vec3(),
            b7c: rng.vec3(),
            ba0: rng.vec3(),
            b88: rng.vec3(),
        };
        let p20 = rng.vec3();
        let p14c = rng.vec3();
        let count = ((rng.next() >> 8) % 5) as usize; // 0..=4, from HIGH bits
        let pts: Vec<[f32; 3]> = (0..count).map(|_| rng.vec3()).collect();
        basis_case(
            &mut e,
            this,
            matrix,
            arr,
            m,
            bases,
            p20,
            p14c,
            &pts,
            &format!("fuzz[{i}]"),
        );
    }
}

/// `0x7b73d7..0x7b73f6`: the ring size, past `i32` and `i64`.
#[test]
fn ribbon_init_buffer_count_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);

    // The `fmul [ebp+0xc]` and `fadd [0x801628]` are where the model says.
    assert_eq!(
        e.read(0x7b73dd, 3),
        vec![0xd8u8, 0x4d, 0x0c],
        "0x7b73dd fmul [ebp+0xc] drifted"
    );
    assert_eq!(
        e.read(0x7b73e8, 6),
        vec![0xd8u8, 0x05, 0x28, 0x16, 0x80, 0x00],
        "0x7b73e8 fadd [0x801628] drifted"
    );

    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, fv11: f32, span: f32, ctx: &str| {
        e.write_u32(ebp + 8, fv11.to_bits()); // [ebp+8] = the first ceil result (f32)
        e.write_u32(ebp + 0xc, span.to_bits()); // [ebp+0xc] = clamped span
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b73d7, 0x7b73f6);
        let bin = e.get_reg(RegisterX86::EAX) as u32 as i32;
        let ours = init_buffer_count(fv11, span);
        assert_eq!(
            bin, ours,
            "{ctx}: binary={bin} ours={ours} (fv11={fv11} span={span})"
        );
    };

    // edge probes: integer products (no ceil change), fractional (ceil rounds up), the +2 pad.
    run_case(&mut e, 1.0, 1.0, "count 1·1"); // ceil(1)+2 = 3
    run_case(&mut e, 4.0, 0.25, "count 4·0.25"); // ceil(1)+2 = 3
    run_case(&mut e, 5.0, 0.3, "count 5·0.3=1.5"); // ceil(1.5)+2 = 4
    run_case(&mut e, 10.0, 0.25, "count 10·0.25=2.5"); // ceil(2.5)+2 = 5
    run_case(&mut e, 100.0, 0.25, "count 100·0.25=25"); // 27
    run_case(&mut e, 0.0, 0.25, "count 0"); // ceil(0)+2 = 2
    run_case(&mut e, 1.0e30, 1.0e10, "count past i64 (the indefinite, 0)");
    run_case(&mut e, 3.0e9, 1.0, "count past i32 (the low dword)");

    let mut s = 0x7b73_a0a0u32;
    for i in 0..4000u32 {
        // fv11 is a ceil'd count (integer-ish, but fuzz arbitrary positive f32); span ≥ 0.25.
        let fv11 = frand(&mut s, 0.0, 500.0);
        let span = frand(&mut s, 0.25, 4.0);
        run_case(&mut e, fv11, span, &format!("count fuzz[{i}]"));
    }
}

/// `0x7b73bc..0x7b73ca`: the lifespan test read at its `fnstsw`.
#[test]
fn ribbon_init_clamp_gate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);
    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;

    let run_case = |e: &mut Emu, span: f32, ctx: &str| {
        e.write_u32(ebp + 0xc, span.to_bits());
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b73bc, 0x7b73ca); // fld [0.25]; add esp,8; fcomp [ebp+0xc]; fnstsw ax
        let ah = read_ah(e);
        // `test ah,0x41; jne` keeps span ⇔ (ah & 0x41) ≠ 0; clamp ⇔ == 0.
        let bin_keeps = (ah & 0x41) != 0;
        let ours_clamps = clamp_span(span).to_bits() == SPAN_CLAMP.to_bits() && span != SPAN_CLAMP;
        // keep ⇔ not clamp. (clamp_span returns 0.25 also when span==0.25, but that's "keep" — so
        // ours_clamps must exclude the equal case: a TRUE clamp replaced a distinct value.)
        assert_eq!(
            bin_keeps, !ours_clamps,
            "{ctx}: binary keeps={bin_keeps} ours clamps={ours_clamps} (span={span} ah={ah:#04x})"
        );
    };

    // ==0.25 boundary: span==0.25 KEEPS (no clamp); just-below clamps; just-above keeps.
    run_case(&mut e, SPAN_CLAMP, "gate span==0.25 (keep)");
    run_case(
        &mut e,
        f32::from_bits(SPAN_CLAMP.to_bits() - 1),
        "gate span just-below-0.25 (clamp)",
    );
    run_case(
        &mut e,
        f32::from_bits(SPAN_CLAMP.to_bits() + 1),
        "gate span just-above-0.25 (keep)",
    );
    run_case(&mut e, 0.0, "gate span=0 (clamp)");
    run_case(&mut e, -1.0, "gate span<0 (clamp)");
    run_case(&mut e, 1.0, "gate span=1 (keep)");
    run_case(&mut e, f32::NAN, "gate span=NaN (keep)");

    let mut s = 0x7b73_bcbcu32;
    for i in 0..4000u32 {
        let span = match (next(&mut s) >> 8) % 3 {
            0 => frand(&mut s, 0.0, 0.5), // straddles 0.25
            1 => frand(&mut s, -2.0, 2.0),
            _ => frand(&mut s, 0.0, 100.0),
        };
        run_case(&mut e, span, &format!("gate fuzz[{i}] span={span}"));
    }
}

/// `0x7b74f5..0x7b7562`: the reciprocals.
#[test]
fn ribbon_fractions_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);

    let frame = e.alloc(0x100);
    let ebp = frame + 0x80;
    let this = e.alloc(0x200); // esi: +0x58/+0x5c/+0x60/+0x64/+0x68 outputs
    let p_arr = e.alloc(0x20); // the float4 at edi (p[0..3])

    let run_case = |e: &mut Emu, span: f32, p: [f32; 4], n9: i32, n10: i32, ctx: &str| {
        for (k, v) in p.iter().enumerate() {
            e.write_u32(p_arr + (k as u64) * 4, v.to_bits());
        }
        e.write_u32(ebp + 0xc, span.to_bits()); // [ebp+0xc] = clamped span
        e.write_u32(ebp + 0x20, p_arr as u32); // [ebp+0x20] = p ptr → loaded into edi
        e.write_u32(ebp + 0x24, n9 as u32); // [ebp+0x24] = param_9
        e.write_u32(ebp + 0x28, n10 as u32); // [ebp+0x28] = param_10
        for off in [0x58u64, 0x5c, 0x60, 0x64, 0x68] {
            e.write_u32(this + off, 0xdead_beef);
        }
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.set_reg(RegisterX86::ESI, this);
        // also need edx valid for the trailing `mov eax,[edx]` at 0x7b7562? No — slice halts at
        // 0x7b7562 (exclusive), so that `mov` is NOT executed.
        e.run(0x7b74f5, 0x7b7562);
        let bin = RibbonFractions {
            inv_span: e.read_f32(this + 0x58),
            step_uv: e.read_f32(this + 0x5c),
            step_pos: e.read_f32(this + 0x60),
            inv_step_uv: e.read_f32(this + 0x64),
            inv_step_pos: e.read_f32(this + 0x68),
        };
        let ours = ribbon_fractions(span, p, n9, n10);
        assert_f32(
            bin.inv_span,
            ours.inv_span,
            &format!("{ctx} +0x58 (1/span)"),
        );
        assert_f32(bin.step_uv, ours.step_uv, &format!("{ctx} +0x5c (step_uv)"));
        assert_f32(
            bin.step_pos,
            ours.step_pos,
            &format!("{ctx} +0x60 (step_pos)"),
        );
        assert_f32(
            bin.inv_step_uv,
            ours.inv_step_uv,
            &format!("{ctx} +0x64 (1/step_uv)"),
        );
        assert_f32(
            bin.inv_step_pos,
            ours.inv_step_pos,
            &format!("{ctx} +0x68 (1/step_pos)"),
        );
    };

    // edge probes: unit span/steps, equal p (zero numerator → step 0 → reciprocal ±Inf), n=1.
    run_case(&mut e, 1.0, [0.0, 0.0, 1.0, 1.0], 1, 1, "frac unit");
    run_case(&mut e, 0.25, [0.0, 0.0, 1.0, 1.0], 4, 4, "frac span=0.25");
    run_case(
        &mut e,
        2.0,
        [1.0, 1.0, 1.0, 1.0],
        2,
        3,
        "frac equal p (step=0 → recip Inf)",
    );
    run_case(&mut e, 0.5, [0.0, 1.0, 2.0, 3.0], 8, 16, "frac spread");
    run_case(&mut e, 1.0, [-1.0, -2.0, 3.0, 4.0], 5, 7, "frac signed");

    let mut s = 0x74f5_74f5u32;
    for i in 0..6000u32 {
        let span = frand(&mut s, 0.25, 8.0);
        let p = [
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
            frand(&mut s, -50.0, 50.0),
        ];
        // nonzero integer divisors (avoid 0 → fidiv by 0; the Inf path is exercised via step=0
        // above through the reciprocals, not by a zero integer divisor here).
        let n9 = ((next(&mut s) >> 8) % 256 + 1) as i32;
        let n10 = ((next(&mut s) >> 8) % 256 + 1) as i32;
        run_case(&mut e, span, p, n9, n10, &format!("frac fuzz[{i}]"));
    }
}

/// `0x7b7a80` whole, bytes past their range included.
#[test]
fn ribbon_color_pack_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);

    // The three channel legs are the same bytes:
    // `fmul [0x7ffe58]` (d8 0d 58 fe 7f 00) + `fadd [0x7ffa24]` (d8 05 24 fa 7f 00).
    let fmul255 = vec![0xd8u8, 0x0d, 0x58, 0xfe, 0x7f, 0x00];
    let fadd_half = vec![0xd8u8, 0x05, 0x24, 0xfa, 0x7f, 0x00];
    for site in [0x7b7aafu64, 0x7b7ac7, 0x7b7ae0] {
        assert_eq!(
            e.read(site, 6),
            fmul255,
            "0x7b7a80 channel fmul [0x7ffe58] site {site:#x} drifted"
        );
    }
    for site in [0x7b7ab9u64, 0x7b7acf, 0x7b7aea] {
        assert_eq!(
            e.read(site, 6),
            fadd_half,
            "0x7b7a80 channel fadd [0x7ffa24] site {site:#x} drifted"
        );
    }
    // the alpha leg: two consecutive `fmul [0x7ffe58]` then the shared `fadd [0x7ffa24]`.
    assert_eq!(e.read(0x7b7a95, 6), fmul255, "0x7b7a95 alpha fmul1 drifted");
    assert_eq!(e.read(0x7b7a9b, 6), fmul255, "0x7b7a9b alpha fmul2 drifted");
    assert_eq!(
        e.read(0x7b7aa1, 6),
        fadd_half,
        "0x7b7aa1 alpha fadd drifted"
    );

    let this = e.alloc(0x200); // CRibbonEmitter (alpha byte @ +0x12f, packed dword out @ +0x12c)

    let run_case = |e: &mut Emu, r: f32, g: f32, b: f32, alpha: u8, ctx: &str| {
        e.write_u32(this + 0x12c, 0xdead_beef); // junk-seed the output dword
                                                // alpha byte at +0x12f (the high byte of the dword
                                                // at +0x12c — write the whole dword's high byte
                                                // cleanly: place alpha as the byte at +0x12f).
        let mut probe = [0u8; 1];
        probe[0] = alpha;
        e.write(this + 0x12f, &probe);
        // __fastcall: ecx = this, stack args = (r, g, b). ret 0xc.
        e.call(
            0x7b7a80,
            this as u32,
            0,
            &[r.to_bits(), g.to_bits(), b.to_bits()],
        );
        let bin = e.read_u32(this + 0x12c);
        let ours = ribbon_color_pack(r, g, b, alpha);
        assert_eq!(
            bin, ours,
            "{ctx}: binary={bin:#010x} ours={ours:#010x} (r={r} g={g} b={b} a={alpha})"
        );
    };

    // ==/edge probes: 0.0, 1.0 (→255), >1, <0, alpha 0/255, the round-bias boundary 0.5
    // (→127.5→127).
    run_case(&mut e, 0.0, 0.0, 0.0, 0, "color all-zero");
    run_case(&mut e, 1.0, 1.0, 1.0, 255, "color all-one a=255");
    run_case(&mut e, 0.5, 0.5, 0.5, 128, "color half a=128"); // 0.5·255+0.5=128.0 → 128
    run_case(&mut e, -0.0, -0.0, -0.0, 0, "color neg-zero");
    run_case(
        &mut e,
        1.0e30,
        f32::INFINITY,
        f32::NAN,
        9,
        "color past i64 and NaN (byte 0)",
    );
    run_case(
        &mut e,
        1.7e7,
        -1.7e7,
        3.0e9,
        9,
        "color past a byte (the low byte)",
    );
    run_case(&mut e, 2.0, -1.0, 0.25, 1, "color out-of-range a=1");
    run_case(&mut e, 1.0, 0.0, 0.5, 200, "color mixed a=200");
    run_case(
        &mut e,
        f32::from_bits(0x3f000001),
        0.0,
        1.0,
        17,
        "color just-above-0.5 a=17",
    );

    let mut s = 0x7b7a_8080u32;
    for i in 0..6000u32 {
        // a mix of in-range [0,1], over-1, negative, and full-scale channels.
        let chan = |s: &mut u32| match (next(s) >> 8) % 4 {
            0 => frand(s, 0.0, 1.0),
            1 => frand(s, 1.0, 3.0),
            2 => frand(s, -2.0, 0.0),
            _ => frand(s, -1.0e3, 1.0e3),
        };
        let r = chan(&mut s);
        let g = chan(&mut s);
        let b = chan(&mut s);
        let alpha = ((next(&mut s) >> 16) & 0xff) as u8;
        run_case(&mut e, r, g, b, alpha, &format!("color fuzz[{i}]"));
    }
}

/// `0x7b76e8..0x7b770a`: the offset row.
#[test]
fn node_place_translate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;
    let esi = e.alloc(0x20);

    let run_case = |e: &mut Emu, a: [f32; 3], v: [f32; 3], ctx: &str| {
        // junk-seed the three slots first so a stale value can never fake a match.
        for off in [0x10u64, 0xc, 0x8] {
            e.write_u32(ebp - off, 0xdead_beef);
        }
        e.write_u32(ebp - 0x10, a[0].to_bits());
        e.write_u32(ebp - 0xc, a[1].to_bits());
        e.write_u32(ebp - 0x8, a[2].to_bits());
        e.write_u32(esi, v[0].to_bits());
        e.write_u32(esi + 4, v[1].to_bits());
        e.write_u32(esi + 8, v[2].to_bits());
        // the slice loads ESI from [ebp+0xc] (param_3 = the vec3 ptr) and ECX from [ebp+0x10]
        // (param_4, only `test`ed); write those frame slots rather than setting ESI directly.
        e.write_u32(ebp + 0xc, esi as u32);
        e.write_u32(ebp + 0x10, 0); // param_4 (only `test ecx,ecx`)
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b76e8, 0x7b770a);
        let bin = [
            e.read_f32(ebp - 0x10),
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
        ];
        let ours = node_place_translate(a, v);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} [{k}]"));
        }
    };

    run_case(&mut e, [0.0, 0.0, 0.0], [0.0, 0.0, 0.0], "node zero");
    run_case(&mut e, [1.0, 2.0, 3.0], [10.0, 20.0, 30.0], "node basic");
    run_case(&mut e, [-1.0, -2.0, -3.0], [1.0, 2.0, 3.0], "node cancel"); // → 0
    run_case(&mut e, [1e30, 1e30, 1e30], [1e30, 1e30, 1e30], "node huge");
    run_case(
        &mut e,
        [-0.0, 0.0, -0.0],
        [0.0, -0.0, -0.0],
        "node signed-zero",
    );
    let mut s = 0x7b76_e8e8u32;
    for i in 0..3000u32 {
        let a = [fuzz_range(&mut s), fuzz_range(&mut s), fuzz_range(&mut s)];
        let v = [fuzz_range(&mut s), fuzz_range(&mut s), fuzz_range(&mut s)];
        run_case(&mut e, a, v, &format!("node fuzz_range[{i}]"));
    }
}

/// `0x7b815d..0x7b8180`: the frame less the offset.
#[test]
fn render_world_translate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);
    let ebp_frame = e.alloc(0x100);
    let ebp = ebp_frame + 0x80;
    let ebx = e.alloc(0x200); // CRibbonEmitter (emit-origin at +0x2c/0x30/0x34)

    let run_case = |e: &mut Emu, a: [f32; 3], origin: [f32; 3], ctx: &str| {
        for off in [0x10u64, 0xc, 0x8] {
            e.write_u32(ebp - off, 0xdead_beef);
        }
        e.write_u32(ebp - 0x10, a[0].to_bits());
        e.write_u32(ebp - 0xc, a[1].to_bits());
        e.write_u32(ebp - 0x8, a[2].to_bits());
        e.write_u32(ebx + 0x2c, origin[0].to_bits());
        e.write_u32(ebx + 0x30, origin[1].to_bits());
        e.write_u32(ebx + 0x34, origin[2].to_bits());
        e.set_reg(RegisterX86::EBX, ebx);
        e.set_reg(RegisterX86::ESP, SLICE_ESP);
        e.set_reg(RegisterX86::EBP, ebp);
        e.run(0x7b815d, 0x7b8180);
        let bin = [
            e.read_f32(ebp - 0x10),
            e.read_f32(ebp - 0xc),
            e.read_f32(ebp - 0x8),
        ];
        let ours = render_world_translate(a, origin);
        for k in 0..3 {
            assert_f32(bin[k], ours[k], &format!("{ctx} [{k}]"));
        }
    };

    // identity-0 `a` (the `param_2 == NULL` case): `0 − origin == −origin`.
    run_case(
        &mut e,
        [0.0, 0.0, 0.0],
        [10.0, 20.0, 30.0],
        "world identity (−origin)",
    );
    // NON-zero `a`: proves it is `a − origin`, not `−origin`.
    run_case(&mut e, [5.0, 6.0, 7.0], [1.0, 2.0, 3.0], "world a−origin");
    run_case(&mut e, [1.0, 2.0, 3.0], [1.0, 2.0, 3.0], "world cancel"); // → 0
    run_case(
        &mut e,
        [-1e30, 1e30, -1e30],
        [1e30, -1e30, 1e30],
        "world huge",
    );
    run_case(
        &mut e,
        [-0.0, 0.0, -0.0],
        [0.0, -0.0, -0.0],
        "world signed-zero",
    );
    let mut s = 0x7b81_5d5du32;
    for i in 0..3000u32 {
        // half the cases force a==0 (identity / −origin), the rest free `a` (a−origin).
        let a = if i % 2 == 0 {
            [0.0, 0.0, 0.0]
        } else {
            [fuzz_range(&mut s), fuzz_range(&mut s), fuzz_range(&mut s)]
        };
        let origin = [fuzz_range(&mut s), fuzz_range(&mut s), fuzz_range(&mut s)];
        run_case(&mut e, a, origin, &format!("world fuzz_range[{i}]"));
    }
}

/// `0x7b7b10` whole, past `i32`, past `i64`, infinity and NaN.
#[test]
fn byte_setter_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    e.set_reg(RegisterX86::FPCW, 0x027F);

    // The constant sites (fmul [0x7ffe58]=255, fadd [0x7ffa24]=0.5), and
    // the stored f32 bits are 255.0 / 0.5.
    assert_eq!(
        e.read(0x7b7b17, 6),
        vec![0xd8u8, 0x0d, 0x58, 0xfe, 0x7f, 0x00],
        "0x7b7b17 fmul [0x7ffe58] drifted"
    );
    assert_eq!(
        e.read(0x7b7b1f, 6),
        vec![0xd8u8, 0x05, 0x24, 0xfa, 0x7f, 0x00],
        "0x7b7b1f fadd [0x7ffa24] drifted"
    );
    assert_eq!(
        e.read_u32(0x7ffe58),
        255.0f32.to_bits(),
        "0x7ffe58 255.0 drifted"
    );
    assert_eq!(
        e.read_u32(0x7ffa24),
        0.5f32.to_bits(),
        "0x7ffa24 0.5 drifted"
    );

    let this = e.alloc(0x200); // CRibbonEmitter (byte out @ +0x12f)

    let run_case = |e: &mut Emu, v: f32, ctx: &str| {
        e.write(this + 0x12f, &[0xab]); // junk-seed the output byte
                                        // __thiscall: ecx = this, stack arg = v. ret 4.
        e.call(0x7b7b10, this as u32, 0, &[v.to_bits()]);
        let bin = e.read(this + 0x12f, 1)[0];
        let ours = byte_setter(v);
        assert_eq!(
            bin, ours,
            "{ctx}: binary={bin:#04x} ours={ours:#04x} (v={v})"
        );
    };

    // ==/edge probes: 0.0→0, 1.0→255, 0.5→ trunc(127.5+0.5)=128, neg, >1 (wraps via low byte),
    // -0.0.
    run_case(&mut e, 0.0, "byte v=0");
    run_case(&mut e, 1.0, "byte v=1 (→255)");
    run_case(&mut e, 0.5, "byte v=0.5 (→128)");
    run_case(&mut e, -0.0, "byte v=-0.0");
    run_case(&mut e, -1.0, "byte v=-1 (neg → low byte)");
    run_case(&mut e, 2.0, "byte v=2 (>1 → low byte)");
    run_case(&mut e, f32::from_bits(0x3f000001), "byte v just-above-0.5");

    run_case(&mut e, 1.0e30, "past i64 (the indefinite, byte 0)");
    run_case(&mut e, f32::INFINITY, "infinity (byte 0)");
    run_case(&mut e, f32::NAN, "NaN (byte 0)");
    run_case(&mut e, 3.0e9, "past i32 (the low byte)");
    let mut s = 0x7b7b_1010u32;
    for i in 0..6000u32 {
        let v = match (next(&mut s) >> 8) % 4 {
            0 => frand(&mut s, 0.0, 1.0),
            1 => frand(&mut s, 1.0, 3.0),
            2 => frand(&mut s, -2.0, 0.0),
            _ => frand(&mut s, -1.0e3, 1.0e3),
        };
        run_case(&mut e, v, &format!("byte fuzz_range[{i}] v={v}"));
    }
}
