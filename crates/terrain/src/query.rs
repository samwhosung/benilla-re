//! The point queries: the chunk height and its vector helpers, the liquid height, the liquid
//! status and the ground effect (`systems/terrain/queries.md`).

use crate::mesh::MAP_CENTER;
use benilla_re_objectlayer::spline_anchor::dot3;

/// `CrossVector3` (`0x672130`, `systems/terrain/queries.md#chunk-height`): `a × b`, each component
/// stored as `f32`.
pub fn vec3_cross_world(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        (f64::from(a[1]) * f64::from(b[2]) - f64::from(a[2]) * f64::from(b[1])) as f32,
        (f64::from(a[2]) * f64::from(b[0]) - f64::from(a[0]) * f64::from(b[2])) as f32,
        (f64::from(a[0]) * f64::from(b[1]) - f64::from(a[1]) * f64::from(b[0])) as f32,
    ]
}

/// `NormalizeVector3` (`0x6720f0`, `systems/terrain/queries.md#chunk-height`): `s = 1/√((x² + y²) +
/// z²)`, kept at 53 bits, then each component `f32(s·v[i])`.
pub fn vec3_normalize_world(v: &mut [f32; 3]) {
    let s = 1.0f64
        / (f64::from(v[0]) * f64::from(v[0])
            + f64::from(v[1]) * f64::from(v[1])
            + f64::from(v[2]) * f64::from(v[2]))
        .sqrt();
    v[0] = (s * f64::from(v[0])) as f32;
    v[1] = (s * f64::from(v[1])) as f32;
    v[2] = (s * f64::from(v[2])) as f32;
}

/// The hole bits, `0x8103f8`: cell `(x, y)` is bit `(x >> 1) + 4·(y >> 1)`.
const HOLES_TABLE: [u16; 16] = [
    0x0001, 0x0002, 0x0004, 0x0008, 0x0010, 0x0020, 0x0040, 0x0080, 0x0100, 0x0200, 0x0400, 0x0800,
    0x1000, 0x2000, 0x4000, 0x8000,
];

/// `CMapChunk::GetHeight` (`0x6b72a0`, `systems/terrain/queries.md#chunk-height`), the path
/// without SSE (`0xc9e388` clear): the height at `(px, py)` in cell `(cellx, celly)`, or `None` in
/// a hole. Two signed tests against the diagonals, each true at `<= 0`, pick the triangle around
/// the inner vertex from `0x81047c` and `0x810480`; the plane is the normalized cross of its two
/// edges from the centre, solved at the point.
#[allow(clippy::too_many_arguments)]
pub fn chunk_get_height(
    verts: &[[f32; 3]; 145],
    holes: u16,
    pos: [f32; 3],
    cellx: u32,
    celly: u32,
    px: f32,
    py: f32,
) -> Option<f32> {
    let cx = (cellx & 7) as usize;
    let cy = (celly & 7) as usize;
    if u32::from(HOLES_TABLE[(cx >> 1) + (cy >> 1) * 4]) & u32::from(holes) != 0 {
        return None;
    }
    let dx = (f64::from(px) - f64::from(pos[0])) as f32;
    let dy = (f64::from(py) - f64::from(pos[1])) as f32;
    let base = cy * 0x11 + cx; // the cell's top-left outer vertex
    let v = |k: usize| -> &[f32; 3] { &verts[base + k] };
    let center = v(9);
    // `0x6b7326` and `0x6b7358`: `test ah,0x41; jp` counts zero as true, NaN as false.
    let t1 = (f64::from(v(18)[1]) - f64::from(dy)) * (f64::from(v(18)[0]) - f64::from(v(0)[0]))
        - (f64::from(v(18)[0]) - f64::from(dx)) * (f64::from(v(18)[1]) - f64::from(v(0)[1]));
    let mut q = usize::from(t1 <= 0.0);
    let t2 = (f64::from(v(17)[1]) - f64::from(dy)) * (f64::from(v(17)[0]) - f64::from(v(1)[0]))
        - (f64::from(v(17)[0]) - f64::from(dx)) * (f64::from(v(17)[1]) - f64::from(v(1)[1]));
    if t2 <= 0.0 {
        q += 2;
    }
    const LUT_A: [usize; 4] = [17, 0, 18, 1];
    const LUT_B: [usize; 4] = [0, 1, 17, 18];
    let c1 = v(LUT_A[q]);
    let c2 = v(LUT_B[q]);
    let sub = |a: &[f32; 3], b: &[f32; 3]| -> [f32; 3] {
        [
            (f64::from(a[0]) - f64::from(b[0])) as f32,
            (f64::from(a[1]) - f64::from(b[1])) as f32,
            (f64::from(a[2]) - f64::from(b[2])) as f32,
        ]
    };
    let t_b = sub(c2, center);
    let t_a = sub(c1, center);
    let mut n = vec3_cross_world(&t_a, &t_b);
    vec3_normalize_world(&mut n);
    // `d = n·C` (`0x602630` at `0x6b74cd`), negated; `z = f32(pos.z − ((n.x·dx + n.y·dy) − d) / n.z)`.
    let d = dot3(n, *center);
    let h = (f64::from(pos[2])
        - ((f64::from(n[0]) * f64::from(dx) + f64::from(n[1]) * f64::from(dy)) + -d)
            / f64::from(n[2])) as f32;
    Some(h)
}

/// `CMapChunk::GetLiquidHeight` (`0x6b7500`, `systems/terrain/queries.md#liquid-height`), its
/// height: the cell is `round_even(c − 0.5) & 7` per axis and the fractions are taken against that
/// rounding; the first slot whose cell is not 0xf decides, a type with `flags & 3 == 1` giving 0;
/// otherwise the bilinear of the 9×9 heights with one store at the end. `None` when no slot has
/// the cell. The flow the function computes for a submerged point goes to stack locals that
/// nothing reads.
pub fn liquid_height_sample(
    slots: [Option<(&[u8; 64], &[f32; 81])>; 4], // per slot, its cell flags and heights
    x: f32,
    y: f32,
) -> Option<f32> {
    let rx = f64::from(x) - 0.5;
    let ry = f64::from(y) - 0.5;
    let ix = rx.round_ties_even() as i32;
    let iy = ry.round_ties_even() as i32;
    let fx = (f64::from(x) - f64::from(ix as f32)) as f32;
    let fy = (f64::from(y) - f64::from(iy as f32)) as f32;
    let cx = (ix & 7) as usize;
    let cy = (iy & 7) as usize;
    let mut found: Option<(u8, &[f32; 81])> = None;
    for (flags, heights) in slots.iter().flatten() {
        let b = flags[cy * 8 + cx];
        if b & 0xf != 0xf {
            found = Some((b, heights));
            break;
        }
    }
    let (b, heights) = found?;
    if b & 3 == 1 {
        return Some(0.0);
    }
    let i = cx + cy * 9;
    let h00 = heights[i];
    let h01 = heights[i + 1];
    let h10 = heights[i + 9];
    let h11 = heights[i + 10];
    let t1 = (f64::from(h01) - f64::from(h00)) * f64::from(fx) + f64::from(h00);
    let t2 = (f64::from(h11) - f64::from(h10)) * f64::from(fx) + f64::from(h10);
    let h = ((t2 - t1) * f64::from(fy) + t1) as f32;
    Some(h)
}

/// `[0x81073c]`: world units to cells, 0.24 (`8/33.33`).
pub const CELL_SCALE: f32 = f32::from_bits(0x3e75_c28f);

/// One flow zone of the object at a liquid slot's `+0x18`: zone 0 at its `+0x0`, zone 1 at
/// `+0x28`; within a zone the centre at `+0x0`, radius `+0xc`, direction `+0x10`, scale `+0x1c`.
#[derive(Clone, Copy, Debug, Default)]
pub struct SoundZone {
    pub center: [f32; 3],
    pub radius: f32,
    pub dir: [f32; 3],
    pub scale: f32,
}

/// One liquid slot as [`liquid_status`] reads it.
pub struct LiquidSlotData<'a> {
    pub flags: &'a [u8; 64],
    pub heights: &'a [f32; 81],
    /// `+0x14`: how many zones give the flow, 0, 1 or 2.
    pub flow_count: u32,
    /// The two zones at `+0x18`.
    pub zones: [SoundZone; 2],
}

/// What `0x69b6d0` writes, one field per out-pointer; `None` where it writes nothing.
#[derive(Default, PartialEq, Debug)]
pub struct LiquidStatusOut {
    pub hit: bool,
    pub fishable: Option<u32>,
    pub liquid_type: Option<u32>,
    pub height: Option<f32>,
    pub flow: Option<[f32; 3]>,
}

/// `0x69b84a` to `0x69ba1b`: the flow of a submerged point by the zone count. 0 gives zero; 1
/// gives zone 0's `scale·dir` (`0x5f8cf0`) inside it; 2 gives, inside both, the normalized sum of
/// the directions (`0x621160`, `0x6720f0`) times `f32((scale1 + scale0)·0.5)` (`0x5132f0`), inside
/// one that zone's `scale·dir`. Inside is `dist² <= radius²` at 53 bits (`0x69be70`). A count of 3
/// or more writes nothing (`0x69b85e`).
fn liquid_sound_flow(zones: &[SoundZone; 2], flow_count: u32, pos: [f32; 3]) -> Option<[f32; 3]> {
    let inside = |z: &SoundZone| {
        crate::leaves::vec3_dist2(&z.center, &pos) <= f64::from(z.radius) * f64::from(z.radius)
    };
    let scale_dir = |z: &SoundZone| -> [f32; 3] {
        core::array::from_fn(|k| (f64::from(z.scale) * f64::from(z.dir[k])) as f32)
    };
    match flow_count {
        0 => Some([0.0; 3]),
        1 => Some(if inside(&zones[0]) {
            scale_dir(&zones[0])
        } else {
            [0.0; 3]
        }),
        2 => Some(match (inside(&zones[0]), inside(&zones[1])) {
            (true, true) => {
                let mut f: [f32; 3] = core::array::from_fn(|k| {
                    (f64::from(zones[0].dir[k]) + f64::from(zones[1].dir[k])) as f32
                });
                vec3_normalize_world(&mut f);
                let avg = ((f64::from(zones[1].scale) + f64::from(zones[0].scale)) * 0.5) as f32;
                core::array::from_fn(|k| (f64::from(f[k]) * f64::from(avg)) as f32)
            }
            (true, false) => scale_dir(&zones[0]),
            (false, true) => scale_dir(&zones[1]),
            (false, false) => [0.0; 3],
        }),
        _ => None,
    }
}

/// `GetLiquidStatus` (`0x69b6d0`, `systems/terrain/queries.md#liquid-status`), the terrain part
/// (the map object query `0x69b520` before it is not modelled): the scaled cell coordinates
/// `f32(−(p − 17066.666)·0.24)`, rounded by `fistp(s − 0.5)`, split into tile, chunk and cell; per
/// slot the fishable bit `b >> 7` is written, a cell of 0xf goes on, a type 1 cell below 0.01
/// answers height 0 and zero flow, any other the bilinear height, the flow when submerged, and a
/// hit when `z < h + 0.01`.
pub fn liquid_status<'a>(
    pos: [f32; 3],
    lookup: impl FnOnce(usize, usize) -> Option<[Option<LiquidSlotData<'a>>; 4]>,
) -> LiquidStatusOut {
    let mut out = LiquidStatusOut::default();
    let wxs = (-(f64::from(pos[1]) - f64::from(MAP_CENTER)) * f64::from(CELL_SCALE)) as f32;
    let wys = (-(f64::from(pos[0]) - f64::from(MAP_CENTER)) * f64::from(CELL_SCALE)) as f32;
    let u3 = (f64::from(wxs) - 0.5).round_ties_even() as i32;
    let u4 = (f64::from(wys) - 0.5).round_ties_even() as i32;
    let area_idx = (((u4 >> 7) & 0x3f) * 0x40 + ((u3 >> 7) & 0x3f)) as usize;
    let chunk_idx = (((u4 >> 3) & 0xf) * 0x10 + ((u3 >> 3) & 0xf)) as usize;
    let Some(slots) = lookup(area_idx, chunk_idx) else {
        return out;
    };
    let fx = (f64::from(wxs) - f64::from(u3 as f32)) as f32;
    let fy = (f64::from(wys) - f64::from(u4 as f32)) as f32;
    let cx = (u3 & 7) as usize;
    let cy = (u4 & 7) as usize;
    for slot in slots.into_iter().flatten() {
        let b = slot.flags[cy * 8 + cx];
        out.fishable = Some(u32::from(b >> 7));
        if b & 0xf == 0xf {
            continue;
        }
        if b & 3 == 1 {
            if f64::from(pos[2]) < f64::from(f32::from_bits(0x3c23_d70a)) {
                out.height = Some(0.0);
                out.flow = Some([0.0; 3]);
                out.liquid_type = Some(u32::from(b & 3));
                out.hit = true;
                return out;
            }
            continue;
        }
        let i = cx + cy * 9;
        let (h00, h01, h10, h11) = (
            slot.heights[i],
            slot.heights[i + 1],
            slot.heights[i + 9],
            slot.heights[i + 10],
        );
        let t1 = (f64::from(h01) - f64::from(h00)) * f64::from(fx) + f64::from(h00);
        let t2 = (f64::from(h11) - f64::from(h10)) * f64::from(fx) + f64::from(h10);
        let h = ((t2 - t1) * f64::from(fy) + t1) as f32;
        out.height = Some(h);
        if f64::from(pos[2]) < f64::from(h) {
            out.flow = liquid_sound_flow(&slot.zones, slot.flow_count, pos);
        }
        if f64::from(pos[2]) < f64::from(h) + f64::from(f32::from_bits(0x3c23_d70a)) {
            out.liquid_type = Some(u32::from(b & 3));
            out.hit = true;
            return out;
        }
    }
    out
}

/// What [`ground_effect_lookup`] reads from the chunk.
pub struct GroundChunk<'a> {
    pub holes: u16,
    pub layer_count: u32,
    /// `+0xf0c`: the low-resolution layer map, eight `u16`.
    pub lowres: &'a [u16; 8],
    /// The four layers' effect ids, each layer's `+0xa`.
    pub layer_effects: [u16; 4],
}

/// `GetGroundEffect` (`0x69b1c0`): world `(x, y)` to the ground effect row's `+0x18`. The flipped
/// coordinates must lie in `[0, 34133.332)`; scaled by 0.24 and rounded by `fistp(c − 0.5)`, split
/// into tile, chunk and cell; a hole or no layers gives none; the cell's 2-bit layer index (masks
/// `0x8104b4`, shifts `0x8104c4`) picks the layer, whose effect id must be neither 0xffff nor above
/// the table's count.
pub fn ground_effect_lookup<'a>(
    pos: [f32; 2],
    table_count: i32,
    chunk: impl FnOnce(usize, usize) -> Option<GroundChunk<'a>>,
    effect_row: impl FnOnce(u16) -> Option<u32>,
) -> Option<u32> {
    const BOUND: f32 = f32::from_bits(0x4705_5555); // [0x7ffab0], 34133.332
    const MASK: [u16; 8] = [0x3, 0xc, 0x30, 0xc0, 0x300, 0xc00, 0x3000, 0xc000];
    const SHIFT: [u32; 8] = [0, 2, 4, 6, 8, 10, 12, 14];
    let wx = (-(f64::from(pos[1]) - f64::from(MAP_CENTER))) as f32;
    let wy = (-(f64::from(pos[0]) - f64::from(MAP_CENTER))) as f32;
    if !(wx >= 0.0 && wy >= 0.0 && wx < BOUND && wy < BOUND) {
        return None;
    }
    let u7 = (f64::from(CELL_SCALE) * f64::from(wx) - 0.5).round_ties_even() as i32;
    let u5 = (f64::from(CELL_SCALE) * f64::from(wy) - 0.5).round_ties_even() as i32;
    let area_idx = (((u5 >> 7) & 0x3f) * 0x40 + ((u7 >> 7) & 0x3f)) as usize;
    let chunk_idx = (((u5 >> 3) & 0xf) * 0x10 + ((u7 >> 3) & 0xf)) as usize;
    let ch = chunk(area_idx, chunk_idx)?;
    let cx = (u7 & 7) as usize;
    let cy = (u5 & 7) as usize;
    if u32::from(HOLES_TABLE[(cx >> 1) + (cy >> 1) * 4]) & u32::from(ch.holes) != 0 {
        return None;
    }
    if ch.layer_count == 0 {
        return None;
    }
    let idx = usize::from((ch.lowres[cy] & MASK[cx]) >> SHIFT[cx]);
    let effect = ch.layer_effects[idx];
    if effect == 0xffff || i32::from(effect) > table_count {
        return None;
    }
    effect_row(effect)
}
