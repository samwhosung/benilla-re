//! Ripples, map lights, detail doodads, the liquid level-of-detail meshes and the nearest liquid,
//! and the distance fade of entity models: `AgeRipple` (`0x68a1a0`), `AddMapLight` (`0x6a7ac0`),
//! `FillDetailDoodadVertices` (`0x6b2600`), `BuildLiquidLodMesh` (`0x68a400`),
//! `FindNearestChunkLiquid` (`0x68b0d0`) and `DrawDoodadModels` (`0x683f80`).

use crate::x87::fistp;

/// `[0x7ff9d8]`: 1.0.
const ONE: f32 = 1.0;
/// `[0x7ffab4]`: 17066.666 (`0x46855555`).
const C_POS: f32 = f32::from_bits(0x4685_5555);
/// `[0x869ffc]`: 0.5, subtracted before `fistp`.
const HALF_BIAS: f32 = 0.5;

/// The fields of a ripple `0x68a1a0` reads and writes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ripple {
    /// `+0xc`, the age.
    pub t: f32,
    /// `+0x30`, the lifetime.
    pub duration: f32,
    /// `+0x38`, the growth per second.
    pub rate: f32,
    /// `+0x2c`, the inner radius's inset.
    pub inset: f32,
    /// `+0x1c`, the fade per second.
    pub decay: f32,
    /// `+0x14`, `t·rate`.
    pub size: f32,
    /// `+0x10`, `size − inset`.
    pub inner: f32,
    /// `+0x8`, `1 − t·decay`.
    pub alpha: f32,
}

/// `AgeRipple` (`0x68a1a0`, ecx = the ripple, `dt` on the stack): `t = dt + t` stored with `fst` and
/// kept; `fcom [+0x30]; test ah,0x41; jne` lives on when `t` is at or below the lifetime or either is
/// NaN, else returns 0 with the rest untouched. Living: `size = t·rate` stored with `fst` and kept,
/// `inner = size − inset` from the unrounded size, `alpha = 1 − t·decay`; returns 1.
pub fn ripple_age(r: &Ripple, dt: f32) -> (bool, Ripple) {
    let t = f64::from(dt) + f64::from(r.t);
    let mut out = *r;
    out.t = t as f32;
    if t > f64::from(r.duration) {
        return (false, out);
    }
    let size = t * f64::from(r.rate);
    out.size = size as f32;
    out.inner = (size - f64::from(r.inset)) as f32;
    out.alpha = (f64::from(ONE) - t * f64::from(r.decay)) as f32;
    (true, out)
}

/// `[0x810a70]`: 1.1666666 (`0x3f955555`), added to the point's z.
const Z_OFFSET: f32 = f32::from_bits(0x3f95_5555);

/// The fields of a map light `0x6a7ac0` reads.
#[derive(Clone, Copy, Debug)]
pub struct MapLight {
    /// `+0x88`, the position.
    pub pos: [f32; 3],
    /// `+0xb8`, the colour.
    pub color: [f32; 3],
    /// `+0xe8`, where the falloff starts.
    pub start: f32,
    /// `+0xec`, where the light ends.
    pub end: f32,
    /// `+0xf0`, the falloff per unit; 0.0 copies the light unchanged.
    pub falloff: f32,
    /// `+0x7c`, the 27 dwords copied into the context's record.
    pub record: [u32; 27],
}

/// What `0x6a7ac0` computes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightRecord {
    /// Whether a record is added.
    pub written: bool,
    /// The falloff is 0.0: the 27 dwords copied, nothing computed.
    pub no_falloff: bool,
    /// `[ebp-0x8]`: 1.0 before the falloff starts, else `1 − (d − start)·falloff`.
    pub intensity: f32,
    /// `[ebp-0x4]`: the stored distance.
    pub d: f32,
    /// The direction from the light, passed to `0x71b6a0`.
    pub dir: [f32; 3],
    /// The colour times the intensity, at `+0x40` of the record.
    pub color: [f32; 3],
}

/// `AddMapLight` (`0x6a7ac0`, ecx = the context, the light on the stack, the point at `this+0x24`):
/// a falloff of 0.0 (`test ah,0x44; jnp`) copies the light. Otherwise the point, its z plus
/// 1.1666666, less the light's position, each stored; `d = √((dz² + dy²) + dx²)` stored with `fst`
/// and compared unrounded with the start: below it the intensity is 1.0. Otherwise the stored `d`
/// at or past the end, or NaN, adds nothing (`0x6a7b80`); the intensity is `f32(1 − (d − start)·
/// falloff)`. The direction is each component times `1/d` kept in the register; the colour each
/// component times the stored intensity.
pub fn chunk_add_light(chunk_point: [f32; 3], light: &MapLight) -> LightRecord {
    if light.falloff == 0.0 {
        return LightRecord {
            written: true,
            no_falloff: true,
            intensity: 1.0,
            d: 0.0,
            dir: [0.0; 3],
            color: light.color,
        };
    }
    let pz = (f64::from(chunk_point[2]) + f64::from(Z_OFFSET)) as f32;
    let point = [chunk_point[0], chunk_point[1], pz];
    let [dx, dy, dz] = std::array::from_fn::<f32, 3, _>(|k| {
        (f64::from(point[k]) - f64::from(light.pos[k])) as f32
    })
    .map(f64::from);
    let d_live = ((dz * dz + dy * dy) + dx * dx).sqrt();
    let d = d_live as f32;
    let intensity = if d_live < f64::from(light.start) {
        1.0f32
    } else {
        let before_end = f64::from(d) < f64::from(light.end);
        if !before_end {
            return LightRecord {
                written: false,
                no_falloff: false,
                intensity: 0.0,
                d,
                dir: [0.0; 3],
                color: [0.0; 3],
            };
        }
        (f64::from(ONE) - (f64::from(d) - f64::from(light.start)) * f64::from(light.falloff)) as f32
    };
    let inv = f64::from(ONE) / f64::from(d);
    let dir = [(dx * inv) as f32, (dy * inv) as f32, (dz * inv) as f32];
    let color = light
        .color
        .map(|c| (f64::from(c) * f64::from(intensity)) as f32);
    LightRecord {
        written: true,
        no_falloff: false,
        intensity,
        d,
        dir,
        color,
    }
}

/// The fields of a detail doodad placement `0x6b2600` reads, 0x2c bytes apart.
#[derive(Clone, Copy, Debug)]
pub struct DetailPlacement {
    /// `+0x0`: bit 0 picks the second colour.
    pub flag: u8,
    /// `+0x8`, the position.
    pub origin: [f32; 3],
    /// `+0x14`, the sine of the facing.
    pub sin: f32,
    /// `+0x18`, its cosine.
    pub cos: f32,
    /// `+0x1c`, the scale.
    pub scale: f32,
    /// `+0x20`, the normal every vertex takes.
    pub normal: [f32; 3],
}

/// A vertex of the model, 0x30 bytes apart: the position at `+0x0`, the texture coordinates at
/// `+0x20`.
#[derive(Clone, Copy, Debug)]
pub struct TemplateVert {
    pub pos: [f32; 3],
    pub uv: [f32; 2],
}

/// A vertex written, nine dwords: position, normal, colour, texture coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstanceVert {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub color: u32,
    pub uv: [f32; 2],
}

/// `FillDetailDoodadVertices` (`0x6b2600`) for one placement, from `0x6b26f8`: each model vertex
/// turned and scaled, `x = (p.x·cos − sin·p.y)·s + o.x` and `y = (p.x·sin + cos·p.y)·s + o.y` and
/// `z = p.z·s + o.z`, each in the register until its store; the colour is `colors[flag & 1]`.
pub fn detail_doodad_verts(
    placement: &DetailPlacement,
    template: &[TemplateVert],
    colors: [u32; 2],
) -> Vec<InstanceVert> {
    let color = colors[usize::from(placement.flag & 1 != 0)];
    let f = f64::from;
    let s = f(placement.scale);
    template
        .iter()
        .map(|p| InstanceVert {
            pos: [
                ((f(p.pos[0]) * f(placement.cos) - f(placement.sin) * f(p.pos[1])) * s
                    + f(placement.origin[0])) as f32,
                ((f(p.pos[0]) * f(placement.sin) + f(placement.cos) * f(p.pos[1])) * s
                    + f(placement.origin[1])) as f32,
                (f(p.pos[2]) * s + f(placement.origin[2])) as f32,
            ],
            normal: placement.normal,
            color,
            uv: p.uv,
        })
        .collect()
}

/// `[0x810350]`: −4.1666665 (`0xc0855555`), a liquid cell's width negated.
const CELL_NEG: f32 = f32::from_bits(0xc085_5555);

/// One liquid level-of-detail mesh of `0x68a400`.
#[derive(Clone, Debug, PartialEq)]
pub struct LiquidLodMesh {
    /// `+0x8`: the `(n+2)²` positions.
    pub positions: Vec<[f32; 2]>,
    /// `+0x18`: the texture coordinates.
    pub texcoords: Vec<[f32; 2]>,
    /// `+0x28`: the strip, `(n+1)(n+2)·2 + 2` indices.
    pub indices: Vec<i16>,
}

/// `BuildLiquidLodMesh` (`0x68a400`, ecx = the mesh, `n` on the stack, 1, 3, 7 or 15): the vertices
/// row by row, even rows from `u = 0` up and odd rows down, each `tex = f32(u/(n+1))` and `pos =
/// f32(−4.1666665·tex)`; then the strip that zigzags each row pair, ending in two repeats. The
/// strip the function builds at `+0x3c` (through `0x68a260` for n above 1) is not modelled.
pub fn liquid_lod_mesh(n: u32) -> LiquidLodMesh {
    let np1 = n + 1;
    let np2 = n + 2;
    let fdiv = np1 as f32;
    let mut positions: Vec<[f32; 2]> = Vec::with_capacity((np2 * np2) as usize);
    let mut texcoords: Vec<[f32; 2]> = Vec::with_capacity((np2 * np2) as usize);
    let mut push = |u: u32, v: u32| {
        let tu = (f64::from(u as f32) / f64::from(fdiv)) as f32;
        let tv = (f64::from(v as f32) / f64::from(fdiv)) as f32;
        positions.push([
            (f64::from(CELL_NEG) * f64::from(tu)) as f32,
            (f64::from(CELL_NEG) * f64::from(tv)) as f32,
        ]);
        texcoords.push([tu, tv]);
    };
    let mut v = 0u32;
    while v < np2 {
        for u in 0..np2 {
            push(u, v);
        }
        if v + 1 >= np2 {
            break;
        }
        for u in (0..np2).rev() {
            push(u, v + 1);
        }
        v += 2;
    }

    let count = (np1 * np2 * 2 + 2) as usize;
    let mut indices = vec![0i16; count];
    let stride = (np2 * 2) as i32 - 1;
    let mut lo = 0i32;
    let mut hi = stride;
    let mut write = 0usize;
    let mut last = 0i16;
    for _ in 0..np1 {
        for _ in 0..np2 {
            indices[write] = lo as i16;
            lo += 1;
            indices[write + 1] = hi as i16;
            hi -= 1;
            write += 2;
        }
        lo = hi + 1;
        last = lo as i16;
        hi = stride + lo;
    }
    indices[write] = last;
    let mut tail = last;
    if np1 & 1 != 0 {
        tail = tail.wrapping_add(np1 as i16);
    }
    indices[write + 1] = tail;

    LiquidLodMesh {
        positions,
        texcoords,
        indices,
    }
}

/// `[0x8102a4]`: 0.03 (`0x3cf5c28f`), units to chunks.
const CHUNK_SCALE: f32 = f32::from_bits(0x3cf5_c28f);
/// `[0x810290]`: 4.1666665 (`0x40855555`), a liquid cell's width.
const TILE_EXTENT: f32 = f32::from_bits(0x4085_5555);
/// `[0x7ffa24]`: 0.5.
const MID_HALF: f32 = 0.5;

/// The area `0x68b0d0` reads.
pub trait LiquidWorld {
    /// `[area+0x278 + (x·16 + y)·4]`, a chunk or 0.
    fn chunk_ptr(&self, cell_index: usize) -> u32;
    /// `[chunk+0x118 + layer·4]`, a liquid or 0.
    fn liquid_layer(&self, chunk: u32, layer: usize) -> u32;
    /// The byte at `[layer+0x10] + row·8 + col`.
    fn tile_nibble(&self, layer: u32, tx: usize, ty: usize) -> u8;
    /// `[chunk+0x6c]` and `[chunk+0x70]`.
    fn chunk_origin(&self, chunk: u32) -> [f32; 2];
    /// `[layer+0x8]` and `[layer+0x4]`, the liquid's top and bottom.
    fn layer_z(&self, layer: u32) -> [f32; 2];
}

/// What `0x68b0d0` writes, per liquid type.
#[derive(Clone, Debug, PartialEq)]
pub struct NearestLiquid {
    /// 1 for each type seen.
    pub flags: [u32; 16],
    /// The offset to the nearest cell of each type.
    pub vecs: [[f32; 3]; 16],
    /// Its squared distance; a type with no nearer cell keeps its value.
    pub dist2: [f32; 16],
}

/// `FindNearestChunkLiquid` (`0x68b0d0`, ecx = the area, then the point, the radius and the three
/// outputs): the point's chunk, `fistp(f32(−(c − 17066.666)·0.03) − 0.5) & 0xf` per axis, and the
/// chunks `window` either side of it within 0 to 15. `window` is the caller's: the client takes it
/// from `0x73fdf5` of `radius·0.24` (`[0x810354]`) plus 512.0 stored, `>> 14 & 0xff`. For each cell
/// of each liquid whose byte's low nibble `t` is not 0xf, from `0x68b22e`: `flags[t] = 1`; the
/// offset `(f32(f32(ox − row·4.1666665) − p.x), f32((oy − col·4.1666665) − p.y), f32((top +
/// bottom)·0.5 − p.z))`, the products and the second difference in the register; and the squared
/// length `(dy² + dx²) + dz²`, compared unrounded (`test ah,5; jp` at `0x68b29d`), replaces type
/// `t`'s when below.
pub fn nearest_liquid(
    world: &dyn LiquidWorld,
    pos: [f32; 3],
    window: i32,
    init_dist2: [f32; 16],
) -> NearestLiquid {
    let ny = (-(f64::from(pos[1]) - f64::from(C_POS))) as f32;
    let cy =
        fistp(f64::from((f64::from(CHUNK_SCALE) * f64::from(ny)) as f32) - f64::from(HALF_BIAS))
            & 0xf;
    let sx = (-(f64::from(pos[0]) - f64::from(C_POS)) * f64::from(CHUNK_SCALE)) as f32;
    let cx = fistp(f64::from(sx) - f64::from(HALF_BIAS)) & 0xf;

    let lo = |v: i32| if v > 0 { v } else { 0 };
    let hi = |v: i32| if v >= 0xf { 0xf } else { v };
    let (x_lo, x_hi) = (lo(cx - window), hi(cx + window));
    let (y_lo, y_hi) = (lo(cy - window), hi(cy + window));

    let mut out = NearestLiquid {
        flags: [0u32; 16],
        vecs: [[0f32; 3]; 16],
        dist2: init_dist2,
    };
    for ox in x_lo..=x_hi {
        for iy in y_lo..=y_hi {
            let chunk = world.chunk_ptr((ox * 0x10 + iy) as usize);
            if chunk == 0 {
                continue;
            }
            let origin = world.chunk_origin(chunk);
            for layer in 0..4usize {
                let lobj = world.liquid_layer(chunk, layer);
                if lobj == 0 {
                    continue;
                }
                let [top, bottom] = world.layer_z(lobj);
                for ty in 0..8usize {
                    for tx in 0..8usize {
                        let nib = world.tile_nibble(lobj, tx, ty) & 0xf;
                        if nib == 0xf {
                            continue;
                        }
                        let t = nib as usize;
                        out.flags[t] = 1;
                        let row = f64::from(ty as i32) * f64::from(TILE_EXTENT);
                        let col = f64::from(tx as i32) * f64::from(TILE_EXTENT);
                        let dx0 = (f64::from(origin[0]) - row) as f32;
                        let dx = (f64::from(dx0) - f64::from(pos[0])) as f32;
                        let dy = ((f64::from(origin[1]) - col) - f64::from(pos[1])) as f32;
                        let dz = ((f64::from(top) + f64::from(bottom)) * f64::from(MID_HALF)
                            - f64::from(pos[2])) as f32;
                        let [fx, fy, fz] = [dx, dy, dz].map(f64::from);
                        let d2 = (fy * fy + fx * fx) + fz * fz;
                        if d2 < f64::from(out.dist2[t]) {
                            out.dist2[t] = d2 as f32;
                            out.vecs[t] = [dx, dy, dz];
                        }
                    }
                }
            }
        }
    }
    out
}

/// `[0x810188]`: 0.5, the radius of the near band.
const BAND0: f32 = 0.5;
/// `[0x81018c]`: 2.5.
const BAND1: f32 = 2.5;
/// `[0x810190]`: 7.0; larger models do not fade.
const BAND2: f32 = 7.0;
/// `[0x810194]`: 10.0, the near band's fade length.
const RANGE0: f32 = 10.0;
/// `[0x810198]`: 25.0.
const RANGE1: f32 = 25.0;
/// `[0x81019c]`: 50.0.
const RANGE2: f32 = 50.0;
/// `[0x8101a0]`: 50.0, the near band's distance.
const FAR0: f32 = 50.0;
/// `[0x8101a4]`: 125.0.
const FAR1: f32 = 125.0;
/// `[0x8101a8]`: 200.0, capped by the far clip.
const FAR2: f32 = 200.0;

/// One model's fields `0x683f80` reads.
#[derive(Clone, Copy, Debug)]
pub struct FadeDoodad {
    /// `+0x5c`, the position.
    pub xy: [f32; 2],
    /// `+0x68`, the radius.
    pub radius: f32,
}

/// The alpha `0x683f80` passes to `0x710cb0`, and whether it shows the model.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FadeDecision {
    pub alpha: f32,
    pub visible: bool,
}

/// `DrawDoodadModels` (`0x683f80`) for one model: the band ends `f32(50 − 10)`, `f32(125 − 25)` and
/// `f32(min(farclip, 200) − 50)` (a far clip at or above 200, or NaN, keeps 200); the distance
/// `√(dy² + dx²) − radius` from the camera in the register. A radius above 7.0, or NaN (`test
/// ah,0x41; jp` at `0x6840c2`), shows at 1.0; above 2.5 the alpha is `1 − (dist − end2)/50`, above
/// 0.5 `1 − (dist − end1)/25`, else `1 − (dist − end0)/10`, stored. Above 1.0 it is 1.0; it shows
/// when above 0.0.
pub fn doodad_fade_decision(d: &FadeDoodad, cam: [f32; 2], farclip: f32) -> FadeDecision {
    let f5 = (f64::from(FAR0) - f64::from(RANGE0)) as f32;
    let f6 = (f64::from(FAR1) - f64::from(RANGE1)) as f32;
    let far2 = if farclip < FAR2 { farclip } else { FAR2 };
    let local_c = (f64::from(far2) - f64::from(RANGE2)) as f32;

    let ex = f64::from(d.xy[0]) - f64::from(cam[0]);
    let ey = f64::from(d.xy[1]) - f64::from(cam[1]);
    let dist = (ey * ey + ex * ex).sqrt() - f64::from(d.radius);

    let r = d.radius;
    if r > BAND2 || r.is_nan() {
        return FadeDecision {
            alpha: 1.0,
            visible: true,
        };
    }
    let term = if r > BAND0 {
        if r > BAND1 {
            (dist - f64::from(local_c)) / f64::from(RANGE2)
        } else {
            (dist - f64::from(f6)) / f64::from(RANGE1)
        }
    } else {
        (dist - f64::from(f5)) / f64::from(RANGE0)
    };
    let alpha = (f64::from(ONE) - term) as f32;
    if alpha > ONE {
        return FadeDecision {
            alpha: 1.0,
            visible: true,
        };
    }
    FadeDecision {
        alpha,
        visible: alpha > 0.0,
    }
}
