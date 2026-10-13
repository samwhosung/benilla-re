//! Ripples, map lights, detail doodads, liquid meshes, the nearest liquid and the model fade against `0x68a1a0`, `0x6a7ac0`, `0x6b2600`, `0x68a400`, `0x68b0d0` and `0x683f80`.

#![allow(clippy::needless_range_loop)]

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::scatter::{
    chunk_add_light, detail_doodad_verts, doodad_fade_decision, liquid_lod_mesh, nearest_liquid,
    ripple_age, DetailPlacement, FadeDoodad, InstanceVert, LiquidWorld, MapLight, Ripple,
    TemplateVert,
};
use common::assert_f32;

struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A finite float.
    fn f32_finite(&mut self) -> f32 {
        loop {
            let f = f32::from_bits(self.next());
            if f.is_finite() {
                return f;
            }
        }
    }
}

/// The area `0x68b0d0` reads, from the emulator's memory.
struct LiqEmu<'a, 'b> {
    emu: &'a Emu<'b>,
    tile: u32,
}

impl LiquidWorld for LiqEmu<'_, '_> {
    fn chunk_ptr(&self, cell_index: usize) -> u32 {
        self.emu
            .read_u32(u64::from(self.tile) + 0x278 + cell_index as u64 * 4)
    }
    fn liquid_layer(&self, chunk: u32, layer: usize) -> u32 {
        self.emu
            .read_u32(u64::from(chunk) + 0x118 + layer as u64 * 4)
    }
    fn tile_nibble(&self, layer: u32, tx: usize, ty: usize) -> u8 {
        let block = self.emu.read_u32(u64::from(layer) + 0x10);
        let va = u64::from(block) + ty as u64 * 8 + tx as u64;
        (self.emu.read_u32(va & !3) >> ((va & 3) * 8)) as u8
    }
    fn chunk_origin(&self, chunk: u32) -> [f32; 2] {
        [
            self.emu.read_f32(u64::from(chunk) + 0x6c),
            self.emu.read_f32(u64::from(chunk) + 0x70),
        ]
    }
    fn layer_z(&self, layer: u32) -> [f32; 2] {
        [
            self.emu.read_f32(u64::from(layer) + 0x8),
            self.emu.read_f32(u64::from(layer) + 0x4),
        ]
    }
}

/// `AgeRipple` (`0x68a1a0`) called whole: the result, the age, and the size, inner radius and
/// alpha when it lives; NaN ages and lifetimes live on.
#[test]
fn ripple_age_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40);
    let mut rng = Lcg(0x1234_5678);
    let mut checked = 0u32;
    let (mut alive, mut dead) = (0u32, 0u32);
    let probe = |t: f32, duration: f32, dt: f32| {
        (
            Ripple {
                t,
                duration,
                rate: 2.0,
                inset: 0.5,
                decay: 0.25,
                size: 0.0,
                inner: 0.0,
                alpha: 0.0,
            },
            dt,
        )
    };
    let mut cases = vec![
        probe(1.0, 2.0, 1.0),
        probe(1.0, 2.0, 1.5),
        probe(f32::NAN, 2.0, 1.0),
        probe(1.0, f32::NAN, 1.0),
        probe(1.0, 2.0, f32::NAN),
    ];
    for _ in 0..600u32 {
        let r = Ripple {
            t: rng.f32_finite() % 100.0,
            duration: rng.f32_finite() % 100.0,
            rate: rng.f32_finite(),
            inset: rng.f32_finite(),
            decay: rng.f32_finite(),
            size: 0.0,
            inner: 0.0,
            alpha: 0.0,
        };
        cases.push((r, rng.f32_finite() % 10.0));
    }
    for (r, dt) in cases {
        emu.write_u32(this + 0xc, r.t.to_bits());
        emu.write_u32(this + 0x30, r.duration.to_bits());
        emu.write_u32(this + 0x38, r.rate.to_bits());
        emu.write_u32(this + 0x2c, r.inset.to_bits());
        emu.write_u32(this + 0x1c, r.decay.to_bits());
        for off in [0x8u64, 0x10, 0x14] {
            emu.write_u32(this + off, 0xdead_beef);
        }
        let beax = emu.call(0x68a1a0, this as u32, 0, &[dt.to_bits()]);
        let bt = emu.read_f32(this + 0xc);
        let (ours_alive, ours) = ripple_age(&r, dt);
        assert_eq!(beax != 0, ours_alive, "ripple_age alive flag");
        assert_f32(bt, ours.t, "ripple.t");
        if ours_alive {
            alive += 1;
            assert_f32(emu.read_f32(this + 0x14), ours.size, "ripple.size");
            assert_f32(emu.read_f32(this + 0x10), ours.inner, "ripple.inner");
            assert_f32(emu.read_f32(this + 0x8), ours.alpha, "ripple.alpha");
        } else {
            dead += 1;
        }
        checked += 1;
    }
    assert!(alive > 0 && dead > 0, "both alive/dead paths must fire");
    eprintln!(
        "ripple_age: {checked} cases bit-exact vs WoW.exe @ 0x68a1a0 (alive={alive} dead={dead})"
    );
}

/// `BuildLiquidLodMesh` (`0x68a400`) called whole for each of 1, 3, 7 and 15, its arrays sized so
/// they do not grow: the positions, texture coordinates and strip.
#[test]
fn liquid_lod_mesh_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // counts and buffers at +0x4/+0x8, +0x14/+0x18, +0x24/+0x28, and the strip at +0x38/+0x3c
    let this = emu.alloc(0x80);
    let vbuf = emu.alloc(0x4000);
    let tbuf = emu.alloc(0x4000);
    let ibuf = emu.alloc(0x4000);
    let sbuf = emu.alloc(0x8000);
    let mut checked = 0u32;
    for &n in &[1u32, 3, 7, 15] {
        for i in 0..0x20u64 {
            emu.write_u32(this + i * 4, 0);
        }
        let np2 = n + 2;
        let nverts = (np2 * np2) as u64;
        emu.write_u32(this + 0x4, nverts as u32);
        emu.write_u32(this + 0x8, vbuf as u32);
        emu.write_u32(this + 0x14, nverts as u32);
        emu.write_u32(this + 0x18, tbuf as u32);
        let nidx = ((n + 1) * np2 * 2 + 2) as u64;
        emu.write_u32(this + 0x24, nidx as u32);
        emu.write_u32(this + 0x28, ibuf as u32);
        emu.write_u32(this + 0x38, 0x2000);
        emu.write_u32(this + 0x3c, sbuf as u32);
        for i in 0..0x1000u64 {
            emu.write_u32(vbuf + i * 4, 0);
            emu.write_u32(tbuf + i * 4, 0);
            emu.write_u32(ibuf + i * 4, 0);
        }
        emu.call(0x68a400, this as u32, 0, &[n]);
        let ours = liquid_lod_mesh(n);
        for v in 0..nverts {
            let bx = emu.read_f32(vbuf + v * 8);
            let by = emu.read_f32(vbuf + v * 8 + 4);
            assert_f32(
                bx,
                ours.positions[v as usize][0],
                &format!("lod{n} pos[{v}].x"),
            );
            assert_f32(
                by,
                ours.positions[v as usize][1],
                &format!("lod{n} pos[{v}].y"),
            );
            let tx = emu.read_f32(tbuf + v * 8);
            let ty = emu.read_f32(tbuf + v * 8 + 4);
            assert_f32(
                tx,
                ours.texcoords[v as usize][0],
                &format!("lod{n} tex[{v}].u"),
            );
            assert_f32(
                ty,
                ours.texcoords[v as usize][1],
                &format!("lod{n} tex[{v}].v"),
            );
        }
        for k in 0..nidx {
            let bi = (emu.read_u32(ibuf + (k / 2) * 4) >> ((k % 2) * 16)) as u16 as i16;
            assert_eq!(bi, ours.indices[k as usize], "lod{n} idx[{k}]");
        }
        checked += 1;
    }
    eprintln!("liquid_lod_mesh: {checked} LODs bit-exact vs WoW.exe @ 0x68a400");
}

#[test]
fn nearest_liquid_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let tile = emu.alloc(0x4000);
    let mut rng = Lcg(0x5151_a0a0);
    // 16 chunks of one liquid each, in about half of the 256 slots
    let mut chunks = Vec::new();
    for _ in 0..16 {
        let c = emu.alloc(0x200);
        emu.write_u32(c + 0x6c, (rng.f32_finite() % 1000.0).to_bits());
        emu.write_u32(c + 0x70, (rng.f32_finite() % 1000.0).to_bits());
        let layer = emu.alloc(0x100);
        let block = emu.alloc(0x80);
        emu.write_u32(layer + 0x4, (rng.f32_finite() % 50.0).to_bits());
        emu.write_u32(layer + 0x8, (rng.f32_finite() % 50.0).to_bits());
        emu.write_u32(layer + 0x10, block as u32);
        for b in 0..0x40u64 {
            let nib = match (rng.next() >> 8) % 3 {
                0 => 0x0fu8,
                1 => ((rng.next() >> 12) & 0xf) as u8 & 0x7,
                _ => 0x02,
            };
            emu.write(block + b, &[nib]);
        }
        emu.write_u32(c + 0x118, layer as u32);
        emu.write_u32(c + 0x11c, 0);
        emu.write_u32(c + 0x120, 0);
        emu.write_u32(c + 0x124, 0);
        chunks.push(c);
    }
    for ci in 0..256u64 {
        let v = if (rng.next() >> 8).is_multiple_of(2) {
            chunks[(rng.next() as usize >> 8) % chunks.len()] as u32
        } else {
            0
        };
        emu.write_u32(tile + 0x278 + ci * 4, v);
    }
    let pos = emu.alloc(0x10);
    let flags = emu.alloc(0x40);
    let vecs = emu.alloc(0xc0);
    let dist2 = emu.alloc(0x40);
    let mut checked = 0u32;
    let mut any_flag = 0u32;
    for _ in 0..40u32 {
        let p = [
            (rng.next() % 1066) as f32 + 16000.0,
            (rng.next() % 1066) as f32 + 16000.0,
            rng.f32_finite() % 50.0,
        ];
        let rad = (rng.next() % 200) as f32 + 1.0;
        emu.write_u32(pos, p[0].to_bits());
        emu.write_u32(pos + 4, p[1].to_bits());
        emu.write_u32(pos + 8, p[2].to_bits());
        let init = [f32::INFINITY; 16];
        for t in 0..16u64 {
            emu.write_u32(flags + t * 4, 0);
            emu.write_u32(dist2 + t * 4, f32::INFINITY.to_bits());
            for k in 0..3u64 {
                emu.write_u32(vecs + t * 0xc + k * 4, 0);
            }
        }
        // ecx = the area, then the point, the radius, and the three outputs
        emu.call(
            0x68b0d0,
            tile as u32,
            0,
            &[
                pos as u32,
                rad.to_bits(),
                flags as u32,
                vecs as u32,
                dist2 as u32,
            ],
        );
        // the window from the client's own `0x73fdf5` of `radius·0.24`
        let arg = f64::from(rad) * f64::from(f32::from_bits(0x3e75_c290));
        let bits = arg.to_bits();
        let atan = emu.call_f32(0x73fdf5, 0, 0, &[bits as u32, (bits >> 32) as u32]);
        let w = (((f64::from(atan) + 512.0) as f32).to_bits() >> 0xe & 0xff) as i32;
        let ours = nearest_liquid(
            &LiqEmu {
                emu: &emu,
                tile: tile as u32,
            },
            p,
            w,
            init,
        );
        for t in 0..16usize {
            assert_eq!(
                emu.read_u32(flags + t as u64 * 4),
                ours.flags[t],
                "nl flags[{t}]"
            );
            if ours.flags[t] != 0 {
                any_flag += 1;
            }
            assert_f32(
                emu.read_f32(dist2 + t as u64 * 4),
                ours.dist2[t],
                &format!("nl dist2[{t}]"),
            );
            for k in 0..3 {
                assert_f32(
                    emu.read_f32(vecs + t as u64 * 0xc + k as u64 * 4),
                    ours.vecs[t][k],
                    &format!("nl vecs[{t}][{k}]"),
                );
            }
        }
        checked += 1;
    }
    assert!(any_flag > 0, "expected ≥1 liquid type found in the window");
    eprintln!(
        "nearest_liquid: {checked} queries bit-exact vs WoW.exe @ 0x68b0d0 (flagged={any_flag})"
    );
}

/// `FillDetailDoodadVertices` (`0x6b2600`) called whole on one placement, the model's vertices from a
/// stubbed `0x710e20`, the buffer from a stubbed `0x58a080`, and `0x58a230` stubbed so both colours
/// are set.
#[test]
fn detail_doodad_verts_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let outbuf = emu.alloc(0x4000);
    let handle = emu.alloc(0x80);
    let templ_meta = emu.alloc(0x20);
    let templ_verts = emu.alloc(0x30 * 16);
    let idx_buf = emu.alloc(0x40);

    // the buffer, the flush, and a caps object with +0x1c = 1
    emu.stub(0x58a080, outbuf as u32, 0);
    let caps = emu.alloc(0x40);
    emu.write_u32(caps + 0x1c, 1);
    emu.stub(0x58a0a0, 0, 2);
    emu.stub(0x58a230, caps as u32, 0);
    // the model: +0x50 its count and indices, +0x48 its vertices
    emu.write_u32(handle + 0x50, templ_meta as u32);
    emu.write_u32(handle + 0x48, templ_verts as u32);
    emu.stub(0x710e20, handle as u32, 0);
    // the model lookup before `0x710e20` reads `[[[0xcafffc] + [placement+4]·4] + 4]`
    let tmpl_table = emu.alloc(0x40);
    let tmpl_obj = emu.alloc(0x40);
    emu.write_u32(0xcafffc, tmpl_table as u32);
    emu.write_u32(tmpl_table, tmpl_obj as u32);
    emu.write_u32(tmpl_obj + 4, 0);

    let mut rng = Lcg(0xd00d_ad00);
    let placement = emu.alloc(0x40);
    let arg = emu.alloc(0x40);
    let mut checked = 0u32;
    let (mut white, mut colored) = (0u32, 0u32);
    for _ in 0..120u32 {
        let count = 1 + (rng.next() >> 8) % 8;
        emu.write_u32(templ_meta, count);
        emu.write_u32(templ_meta + 4, idx_buf as u32);
        let mut template: Vec<TemplateVert> = Vec::new();
        for k in 0..count as u64 {
            emu.write_u32(idx_buf + k * 2, 0);
            emu.write(idx_buf + k * 2, &(k as u16).to_le_bytes());
            let pos = [
                rng.f32_finite() % 10.0,
                rng.f32_finite() % 10.0,
                rng.f32_finite() % 10.0,
            ];
            let uv = [rng.f32_finite() % 1.0, rng.f32_finite() % 1.0];
            let base = templ_verts + k * 0x30;
            for i in 0..3 {
                emu.write_u32(base + i as u64 * 4, pos[i].to_bits());
            }
            emu.write_u32(base + 0x20, uv[0].to_bits());
            emu.write_u32(base + 0x24, uv[1].to_bits());
            template.push(TemplateVert { pos, uv });
        }
        let flag = (rng.next() >> 8) as u8 & 1;
        let pl = DetailPlacement {
            flag,
            origin: [
                rng.f32_finite() % 100.0,
                rng.f32_finite() % 100.0,
                rng.f32_finite() % 100.0,
            ],
            sin: rng.f32_finite() % 1.0,
            cos: rng.f32_finite() % 1.0,
            scale: rng.f32_finite() % 5.0,
            normal: [
                rng.f32_finite() % 1.0,
                rng.f32_finite() % 1.0,
                rng.f32_finite() % 1.0,
            ],
        };
        emu.write(placement, &[flag]);
        emu.write_u32(placement + 4, 0);
        emu.write_u32(placement + 0x8, pl.origin[0].to_bits());
        emu.write_u32(placement + 0xc, pl.origin[1].to_bits());
        emu.write_u32(placement + 0x10, pl.origin[2].to_bits());
        emu.write_u32(placement + 0x14, pl.sin.to_bits());
        emu.write_u32(placement + 0x18, pl.cos.to_bits());
        emu.write_u32(placement + 0x1c, pl.scale.to_bits());
        emu.write_u32(placement + 0x20, pl.normal[0].to_bits());
        emu.write_u32(placement + 0x24, pl.normal[1].to_bits());
        emu.write_u32(placement + 0x28, pl.normal[2].to_bits());
        // the slot: +0x18 the count, +0x1c the placements
        emu.write_u32(arg + 0x18, 1);
        emu.write_u32(arg + 0x1c, placement as u32);
        emu.call(0x6b2600, arg as u32, 0, &[]);
        let ours: Vec<InstanceVert> =
            detail_doodad_verts(&pl, &template, [0xffff_ffff, 0xffc0_c0c0]);
        for (vi, ov) in ours.iter().enumerate() {
            let b = outbuf + vi as u64 * 9 * 4;
            for i in 0..3 {
                assert_f32(
                    emu.read_f32(b + i as u64 * 4),
                    ov.pos[i],
                    &format!("dv pos[{vi}][{i}]"),
                );
            }
            for i in 0..3 {
                assert_f32(
                    emu.read_f32(b + (3 + i) as u64 * 4),
                    ov.normal[i],
                    &format!("dv normal[{vi}][{i}]"),
                );
            }
            assert_eq!(emu.read_u32(b + 6 * 4), ov.color, "dv color[{vi}]");
            assert_f32(emu.read_f32(b + 7 * 4), ov.uv[0], &format!("dv uv[{vi}].u"));
            assert_f32(emu.read_f32(b + 8 * 4), ov.uv[1], &format!("dv uv[{vi}].v"));
        }
        if flag == 0 {
            white += 1;
        } else {
            colored += 1;
        }
        checked += 1;
    }
    assert!(white > 0 && colored > 0, "both color branches fire");
    eprintln!("detail_doodad_verts: {checked} placements bit-exact vs WoW.exe @ 0x6b2600 (white={white} colored={colored})");
}

/// `AddMapLight` (`0x6a7ac0`) with a falloff of 0.0, called whole: the 27 dwords copied into the
/// node `0x6a0ba0` gives.
#[test]
fn chunk_add_light_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let node = emu.alloc(0x100);
    let link = emu.alloc(0x40);
    let iv3 = emu.alloc(0x40);
    emu.stub(0x6a0ba0, node as u32, 0);
    emu.stub(0x6a9420, link as u32, 0);
    emu.stub(0x6a93e0, 0, 0);
    let chunk = emu.alloc(0x200);
    let light = emu.alloc(0x100);
    emu.write_u32(chunk + 0x80, iv3 as u32);
    emu.write_u32(iv3 + 4, 0);
    let mut rng = Lcg(0x11ab_cd22);
    let mut checked = 0u32;
    let mut clone = 0u32;
    for _ in 0..400u32 {
        let lt = MapLight {
            pos: [
                rng.f32_finite() % 200.0,
                rng.f32_finite() % 200.0,
                rng.f32_finite() % 200.0,
            ],
            color: [
                rng.f32_finite() % 2.0,
                rng.f32_finite() % 2.0,
                rng.f32_finite() % 2.0,
            ],
            start: (rng.next() % 50) as f32,
            end: (rng.next() % 100) as f32,
            falloff: 0.0,
            record: {
                let mut r = [0u32; 27];
                for w in r.iter_mut() {
                    *w = rng.next();
                }
                r
            },
        };
        emu.write_u32(light + 0xf0, lt.falloff.to_bits());
        for (i, w) in lt.record.iter().enumerate() {
            emu.write_u32(light + 0x7c + i as u64 * 4, *w);
        }
        for i in 0..0x30u64 {
            emu.write_u32(node + i * 4, 0xa5a5_a5a5);
        }
        emu.write_u32(chunk + 0x80, iv3 as u32);
        emu.write_u32(iv3 + 4, 0);
        emu.write_u32(link, 0);
        emu.call(0x6a7ac0, chunk as u32, 0, &[light as u32]);
        let ours = chunk_add_light([0.0; 3], &lt);
        assert!(ours.no_falloff, "falloff==0 -> clone path");
        clone += 1;
        for i in 0..27u64 {
            assert_eq!(
                emu.read_u32(node + 4 + i * 4),
                lt.record[i as usize],
                "clone word[{i}]"
            );
        }
        checked += 1;
    }
    assert!(clone > 0, "clone path fired");
    eprintln!("chunk_add_light: {checked} no-falloff clones bit-exact vs WoW.exe @ 0x6a7ac0");
}

/// An emulator for `0x6a7ac0` with its calls stubbed: the node, the list head, the context and
/// the light.
fn light_emu(wow: &Wow) -> (Emu<'_>, [u64; 4]) {
    let mut emu = Emu::new(wow);
    let node = emu.alloc(0x100);
    let head = emu.alloc(0x40);
    emu.stub(0x6a0ba0, node as u32, 0);
    emu.stub(0x6a93e0, 0, 0);
    emu.stub(0x71b6a0, 0, 1);
    emu.stub(0x71b780, 0, 1);
    emu.stub(0x71b620, 0, 1);
    let chunk = emu.alloc(0x200);
    let light = emu.alloc(0x100);
    (emu, [node, head, chunk, light])
}

/// `AddMapLight` (`0x6a7ac0`) with a falloff: called whole with the node `0x6a0ba0` gives and the
/// calls after it stubbed, whether a record is added and its colour at `+0x40`; and, when added,
/// run from the entry to the call at `0x6a7c15` for the distance at `[ebp-0x4]`, the intensity at
/// `[ebp-0x8]` and the direction, the call's arguments.
#[test]
fn chunk_add_light_falloff_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    // one emulator for the whole calls, another for the slices, each always stopping alike
    let (mut emu, [node, head, chunk, light]) = light_emu(&wow);
    let (mut slice, _) = light_emu(&wow);
    let mut rng = Lcg(0x6a7a_c0f0);
    let mut unit = move || f64::from(rng.next() >> 8) / f64::from(1u32 << 24);
    let (mut inside, mut faded, mut rejected) = (0u32, 0u32, 0u32);

    let mut case = |emu: &mut Emu, slice: &mut Emu, point: [f32; 3], lt: &MapLight, ctx: &str| {
        let set = |emu: &mut Emu| {
            for k in 0..3u64 {
                emu.write_u32(chunk + 0x24 + 4 * k, point[k as usize].to_bits());
                emu.write_u32(light + 0x88 + 4 * k, lt.pos[k as usize].to_bits());
                emu.write_u32(light + 0xb8 + 4 * k, lt.color[k as usize].to_bits());
            }
            emu.write_u32(light + 0xe8, lt.start.to_bits());
            emu.write_u32(light + 0xec, lt.end.to_bits());
            emu.write_u32(light + 0xf0, lt.falloff.to_bits());
            for i in 0..0x40u64 {
                emu.write_u32(node + i * 4, 0);
            }
            emu.write_u32(chunk + 0x7c, 0);
            emu.write_u32(chunk + 0x80, head as u32);
            emu.write_u32(head + 4, 0);
        };
        set(emu);
        emu.clear_calls();
        emu.call(0x6a7ac0, chunk as u32, 0, &[light as u32]);
        let added = emu.calls().iter().any(|c| c.addr == 0x6a0ba0);
        let ours = chunk_add_light(point, lt);
        assert!(!ours.no_falloff, "{ctx}");
        assert_eq!(added, ours.written, "{ctx}: added");
        if !added {
            rejected += 1;
            return;
        }
        for k in 0..3usize {
            assert_f32(
                emu.read_f32(node + 0x40 + 4 * k as u64),
                ours.color[k],
                &format!("{ctx} color[{k}]"),
            );
        }
        set(slice);
        let esp = 0x7008_0000u64;
        slice.write_u32(esp, 0xdead_beef);
        slice.write_u32(esp + 4, light as u32);
        slice.set_reg(RegisterX86::ESP, esp);
        slice.set_reg(RegisterX86::ECX, chunk);
        slice.run(0x6a7ac0, 0x6a7c15);
        let ebp = esp - 4;
        let top = slice.get_reg(RegisterX86::ESP);
        assert_f32(slice.read_f32(ebp - 4), ours.d, &format!("{ctx} d"));
        assert_f32(
            slice.read_f32(ebp - 8),
            ours.intensity,
            &format!("{ctx} intensity"),
        );
        for k in 0..3usize {
            assert_f32(
                slice.read_f32(top + 4 * k as u64),
                ours.dir[k],
                &format!("{ctx} dir[{k}]"),
            );
        }
        if ours.intensity == 1.0 {
            inside += 1;
        } else {
            faded += 1;
        }
    };

    let base = MapLight {
        pos: [10.0, 20.0, 30.0],
        color: [0.5, 0.75, 1.0],
        start: 5.0,
        end: 40.0,
        falloff: 0.05,
        record: [0; 27],
    };
    // at the start, at the end, and NaN bounds
    let at = |d: f32| [10.0 + d, 20.0, 30.0 - f32::from_bits(0x3f95_5555)];
    case(&mut emu, &mut slice, at(5.0), &base, "at start");
    case(&mut emu, &mut slice, at(40.0), &base, "at end");
    case(&mut emu, &mut slice, at(0.0), &base, "on the light");
    case(
        &mut emu,
        &mut slice,
        at(10.0),
        &MapLight {
            start: f32::NAN,
            ..base
        },
        "NaN start",
    );
    case(
        &mut emu,
        &mut slice,
        at(10.0),
        &MapLight {
            end: f32::NAN,
            ..base
        },
        "NaN end",
    );
    case(
        &mut emu,
        &mut slice,
        at(1.0),
        &MapLight {
            start: f32::NAN,
            ..base
        },
        "NaN start, near",
    );
    for i in 0..3000u32 {
        let point = [
            (unit() * 400.0 - 200.0) as f32,
            (unit() * 400.0 - 200.0) as f32,
            (unit() * 100.0 - 50.0) as f32,
        ];
        let reach = unit() * 120.0;
        let lt = MapLight {
            pos: [
                (f64::from(point[0]) + unit() * reach * 2.0 - reach) as f32,
                (f64::from(point[1]) + unit() * reach * 2.0 - reach) as f32,
                (f64::from(point[2]) + unit() * reach * 2.0 - reach) as f32,
            ],
            color: [unit() as f32, unit() as f32, unit() as f32],
            start: (unit() * 40.0) as f32,
            end: (40.0 + unit() * 80.0) as f32,
            falloff: (unit() * 0.1 + 0.001) as f32,
            record: [0; 27],
        };
        case(&mut emu, &mut slice, point, &lt, &format!("fuzz{i}"));
    }
    assert!(
        inside > 0 && faded > 0 && rejected > 0,
        "inside={inside} faded={faded} rejected={rejected}"
    );
}

/// `DrawEntityModels` (`0x683f80`) over a one-model list, the setters and the bounds pass stubbed:
/// whether `0x710cb0` is called and the alpha it is passed.
#[test]
fn doodad_fade_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x710cb0, 0, 0);
    emu.stub(0x710b90, 0, 0);
    emu.stub(0x710c50, 0, 0);
    emu.stub(0x710620, 0, 0);
    emu.stub(0x7134b0, 0, 0);
    emu.stub(0x672a20, 0, 0);
    // bit 0 of the render flags lets the models fade
    emu.write_u32(0xc7b2a4, 1);

    emu.write_u32(0xc7cf20, 0.0f32.to_bits());
    emu.write_u32(0xc7cf24, 0.0f32.to_bits());
    emu.write_u32(0xc7b498, 1000.0f32.to_bits());

    // the list's links sit at node + `[0xc7cae4]` + 4; an odd link ends it
    let node = emu.alloc(0x400);
    emu.write_u32(0xc7cae4, 0);
    emu.write_u32(0xc7caec, node as u32);
    emu.write_u32(node + 4, 1);
    emu.write_u32(node + 0x16c, 0);
    emu.write_u32(node + 0x174, 0);
    emu.write_u32(node + 0x88, 1);

    let cam = [0.0f32, 0.0];
    let farclip = 1000.0f32;
    let mut rng = Lcg(0x7a7a_5b5b);
    // each band and its edges, near and far, and a NaN radius
    let radii = [0.0f32, 0.25, 0.5, 1.0, 2.5, 4.0, 7.0, 8.0, 12.0];
    let mut checked = 0u32;
    let (mut shown, mut hidden, mut clamped) = (0u32, 0u32, 0u32);
    let mut cases: Vec<(f32, f32, f32)> = Vec::new();
    for &r in &radii {
        for _ in 0..40 {
            let x = (rng.next() % 1500) as f32;
            let y = (rng.next() % 1500) as f32;
            cases.push((x, y, r));
        }
    }
    cases.push((100.0, 100.0, f32::NAN));
    for (x, y, r) in cases {
        emu.write_u32(node + 0x5c, x.to_bits());
        emu.write_u32(node + 0x60, y.to_bits());
        emu.write_u32(node + 0x68, r.to_bits());
        emu.write_u32(0xc7caec, node as u32);
        emu.write_u32(node + 4, 1);
        emu.write_u32(0xc7b2a4, 1);
        emu.clear_calls();
        emu.call(0x683f80, 0, 0, &[]);
        let alpha_call = emu.calls().iter().find(|c| c.addr == 0x710cb0).copied();
        let ours = doodad_fade_decision(
            &FadeDoodad {
                xy: [x, y],
                radius: r,
            },
            cam,
            farclip,
        );
        match alpha_call {
            Some(c) => {
                let balpha = f32::from_bits(c.args[0]);
                assert!(
                    ours.visible,
                    "binary showed but ours says hidden (x={x} y={y} r={r})"
                );
                assert_f32(balpha, ours.alpha, &format!("fade alpha x={x} y={y} r={r}"));
                shown += 1;
                if balpha == 1.0 {
                    clamped += 1;
                }
            }
            None => {
                assert!(
                    !ours.visible,
                    "binary hid but ours says visible (x={x} y={y} r={r} a={})",
                    ours.alpha
                );
                hidden += 1;
            }
        }
        checked += 1;
    }
    assert!(
        shown > 0 && hidden > 0 && clamped > 0,
        "fade branches: shown={shown} hidden={hidden} clamped={clamped}"
    );
    eprintln!("doodad_fade: {checked} doodads bit-exact vs WoW.exe @ 0x683f80 (shown={shown} hidden={hidden} clamped={clamped})");
}
