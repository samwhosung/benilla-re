//! The polygon clippers: against the unit cube for the projector, and against the four portal
//! planes.

/// `[0x7ff9d8]`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);
/// `[0x7ffd74]`: 0.0.
const ZERO: f32 = f32::from_bits(0x0000_0000);
/// `[0x80c4f4]`: 1e-4 (`0x38d1b717`), the divisor `ClipToUnitCube` uses when an edge's difference is
/// exactly 0.
const DENOM_EPS: f32 = f32::from_bits(0x38d1_b717);
/// `[0x8029d0]`: 0.01 (`0x3c23d70a`), the top of `ClipPortalPolygon`'s band on a plane.
const POLY_EPS_POS: f32 = f32::from_bits(0x3c23_d70a);
/// `[0x810ccc]`: −0.01 (`0xbc23d70a`), its bottom.
const POLY_EPS_NEG: f32 = f32::from_bits(0xbc23_d70a);

/// `GetUnitCubeOutcode` (`0x686d20`): the six distances of `p` to the unit cube's faces and their
/// sign bits. `out` is `x, f32(1 − x), y, f32(1 − y), z, f32(1 − z)`, the coordinates copied as
/// dwords, then a word with the six signs at bits 31 to 26 in that order; a set bit is outside.
pub fn unitcell_outcode(p: &[f32; 3]) -> [f32; 7] {
    let mut out = [0.0f32; 7];
    out[0] = p[0];
    out[1] = (f64::from(ONE) - f64::from(p[0])) as f32;
    out[2] = p[1];
    out[3] = (f64::from(ONE) - f64::from(p[1])) as f32;
    out[4] = p[2];
    out[5] = (f64::from(ONE) - f64::from(p[2])) as f32;
    let packed = ((out[4].to_bits() >> 4) & 0x0800_0000)
        + ((out[1].to_bits() >> 1) & 0x4000_0000)
        + ((out[3].to_bits() >> 3) & 0x1000_0000)
        + ((out[5].to_bits() >> 5) & 0x0400_0000)
        + ((out[2].to_bits() >> 2) & 0x2000_0000)
        + (out[0].to_bits() & 0x8000_0000);
    out[6] = f32::from_bits(packed);
    out
}

/// The sign word of [`unitcell_outcode`], as the clipper reads it.
fn outcode_bits(p: &[f32; 3]) -> u32 {
    unitcell_outcode(p)[6].to_bits()
}

/// `ClipToUnitCube` (`0x686db0`): the polygon clipped to `[0, 1]³`, or `None` when empty. The AND
/// of every vertex's sign word nonzero rejects; their OR zero accepts unchanged. Otherwise each face
/// in bit order whose bit the OR has is clipped against: an edge whose two ends differ in that bit
/// gives `t = d0 / (d0 − d1)` in the register, the divisor 1e-4 when the difference is 0, and the
/// point from [`lerp_unitcube`]; a vertex inside the face is kept.
pub fn unitcube_clip(verts: &[[f32; 3]]) -> Option<Vec<[f32; 3]>> {
    let count = verts.len();
    if count == 0 {
        return None;
    }
    let mut codes: Vec<u32> = Vec::with_capacity(count);
    let mut and_all: u32 = 0xffff_ffff;
    let mut or_all: u32 = 0;
    for v in verts {
        let c = outcode_bits(v);
        codes.push(c);
        and_all &= c;
        or_all |= c;
    }
    if and_all != 0 {
        return None;
    }
    if or_all == 0 {
        return Some(verts.to_vec());
    }

    // The two vertex pools, `0xc7b678` and `0xc7d000`, with their codes.
    let mut cur_v: Vec<[f32; 3]> = verts.to_vec();
    let mut cur_c: Vec<u32> = codes;
    let mut next_v: Vec<[f32; 3]> = Vec::new();
    let mut next_c: Vec<u32> = Vec::new();

    // The OR of the input codes stays fixed; the face mask shifts right once a face.
    let mut mask: u32 = 0x8000_0000;
    for _face in 0..6 {
        if or_all & mask != 0 {
            next_v.clear();
            next_c.clear();
            let n = cur_v.len();
            let face = mask.leading_zeros() as usize;
            // Each vertex against the one before it, the first against the last.
            let mut prev_in = cur_c[n - 1] & mask;
            let mut prev_idx = n - 1;
            for i in 0..n {
                let cur_in = cur_c[i] & mask;
                if cur_in != prev_in {
                    let cur = &cur_v[prev_idx];
                    let next = &cur_v[i];
                    let d0 = unitcell_outcode(cur)[face];
                    let d1 = unitcell_outcode(next)[face];
                    let diff = f64::from(d0) - f64::from(d1);
                    let denom = if diff == f64::from(ZERO) {
                        f64::from(DENOM_EPS)
                    } else {
                        diff
                    };
                    let t = f64::from(d0) / denom;
                    let p = lerp_unitcube(cur, next, t);
                    next_c.push(outcode_bits(&p));
                    next_v.push(p);
                }
                if cur_in == 0 {
                    next_v.push(cur_v[i]);
                    next_c.push(cur_c[i]);
                }
                prev_in = cur_in;
                prev_idx = i;
            }
            if next_v.is_empty() {
                return None;
            }
            std::mem::swap(&mut cur_v, &mut next_v);
            std::mem::swap(&mut cur_c, &mut next_c);
        }
        mask >>= 1;
    }
    Some(cur_v)
}

/// `0x686f31` to `0x686fae`: the crossing, with its stores: `dz = f32(z1 − z0)` first, then `f32(dx·t)`
/// and `f32(dz·t)`, then `x = f32(dx·t + x0)`, `y = f32((y1 − y0)·t + y0)`, `z = f32(dz·t + z0)`.
fn lerp_unitcube(cur: &[f32; 3], next: &[f32; 3], tf: f64) -> [f32; 3] {
    let dz = (f64::from(next[2]) - f64::from(cur[2])) as f32; // [ebp-0x5c]
    let dxt = ((f64::from(next[0]) - f64::from(cur[0])) * tf) as f32; // [ebp-0x58]
    let dzt = (f64::from(dz) * tf) as f32; // [ebp-0x50]
    let x = (f64::from(dxt) + f64::from(cur[0])) as f32; // [ebp-0x34]
    let y = ((f64::from(next[1]) - f64::from(cur[1])) * tf + f64::from(cur[1])) as f32;
    let z = (f64::from(dzt) + f64::from(cur[2])) as f32; // [ebp-0x2c]
    [x, y, z]
}

/// `ClipPortalPolygon` (`0x6b4a80`): the polygon clipped against the four planes through the
/// origin at `0xc7d0dc` (16 bytes apart, the constant unused), which `0x680610` writes; `None` when
/// a pass empties it. A vertex with `d > 0.01` is inside (1), within ±0.01 on the plane (0), below
/// −0.01 behind (2); 0 and 1 are kept. A vertex of code 1 or 2 whose next has a nonzero different
/// code adds the crossing at `t = d0 / (d0 − d1)`, with no guard for a zero divisor; a vertex on the
/// plane adds none (`0x6b4bf0` jumps to `0x6b4c8f`).
pub fn poly_clip(verts: &[[f32; 3]], planes: &[[f32; 3]; 4]) -> Option<Vec<[f32; 3]>> {
    let mut cur: Vec<[f32; 3]> = verts.to_vec();
    for plane in planes.iter() {
        let n = cur.len();
        if n == 0 {
            return None;
        }
        // The distances and codes at `0xcbe0f0` and `0xcbe0b0`, the first repeated at the end.
        let mut dist: Vec<f32> = Vec::with_capacity(n + 1);
        let mut code: Vec<u8> = Vec::with_capacity(n + 1);
        for v in &cur {
            let d = (f64::from(plane[0]) * f64::from(v[0])
                + f64::from(v[1]) * f64::from(plane[1])
                + f64::from(v[2]) * f64::from(plane[2])) as f32;
            dist.push(d);
            code.push(classify(d));
        }
        dist.push(dist[0]);
        code.push(code[0]);

        let mut out: Vec<[f32; 3]> = Vec::new();
        for i in 0..n {
            let c = code[i];
            if c == 0 {
                out.push(cur[i]);
                continue;
            }
            if c == 1 {
                out.push(cur[i]);
            }
            let c_next = code[i + 1];
            if c_next != 0 && c_next != c {
                let j = (i + 1) % n;
                let t = f64::from(dist[i]) / (f64::from(dist[i]) - f64::from(dist[j]));
                out.push(lerp_poly(&cur[i], &cur[j], t));
            }
        }
        if out.is_empty() {
            return None;
        }
        cur = out;
    }
    Some(cur)
}

/// The code of a distance: 1 above 0.01, 0 within ±0.01, 2 below.
fn classify(d: f32) -> u8 {
    if d <= POLY_EPS_POS {
        if POLY_EPS_NEG <= d {
            0
        } else {
            2
        }
    } else {
        1
    }
}

/// `0x6b4c67` to `0x6b4c8a`: `f32((p1 − p0)·t + p0)` per component, the differences kept in the
/// register.
fn lerp_poly(cur: &[f32; 3], next: &[f32; 3], tf: f64) -> [f32; 3] {
    [
        ((f64::from(next[0]) - f64::from(cur[0])) * tf + f64::from(cur[0])) as f32,
        ((f64::from(next[1]) - f64::from(cur[1])) * tf + f64::from(cur[1])) as f32,
        ((f64::from(next[2]) - f64::from(cur[2])) * tf + f64::from(cur[2])) as f32,
    ]
}
