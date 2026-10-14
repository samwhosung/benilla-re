//! Small functions of the terrain's files: the squared distance, a plane from a normal and a point,
//! the colour lerps and fade, a 3×3 scale and the area id.
//!
//! The colour functions work on bytes, with the wrapping of the client's `mov bl` and `add bl,cl`.

/// `[0x7ffab4]`: the map's half extent, 17066.666 (`0x46855555`).
const C_POS: f32 = f32::from_bits(0x4685_5555);
/// `[0x810740]`: world units to area cells, 0.03 (`0x3cf5c28f`).
const S_SCALE: f32 = f32::from_bits(0x3cf5_c28f);
/// `[0x86a750]`: 0.5, subtracted before `fistp`.
const H_HALF: f32 = f32::from_bits(0x3f00_0000);
/// `[0x7ffe58]`: 255.0.
const MUL_255: f32 = f32::from_bits(0x437f_0000);
/// `[0x86b6b4]`: 0.5, subtracted before the fade's `fistp`.
const SUB_FADE: f32 = f32::from_bits(0x3f00_0000);
/// `[0x8029cc]`: 512.0, added so that `bits >> 14` of the sum is `t·255` as a byte.
const ADD_ALPHA: f32 = f32::from_bits(0x4400_0000);

/// `0xc96318`: the area table, 64×64 tile pointers.
pub const AREA_TABLE_VA: u32 = 0x00c9_6318;

/// `DistanceSquared` (`0x69be70`, `systems/terrain/queries.md#liquid-height`): `dy² + dz² + dx²`,
/// left in `st0` (`ret 4`); the caller's store rounds it.
pub fn vec3_dist2(a: &[f32; 3], b: &[f32; 3]) -> f64 {
    let dx = f64::from(b[0]) - f64::from(a[0]);
    let dy = f64::from(b[1]) - f64::from(a[1]);
    let dz = f64::from(b[2]) - f64::from(a[2]);
    dy * dy + dz * dz + dx * dx
}

/// `SetPlaneFromNormal` (`0x6acde0`, `systems/terrain/collide.md#impassable-walls`): the plane
/// `[n, d]` with `n` copied as dwords and `d = f32(−(nz·pz + (ny·py + nx·px)))`.
pub fn plane_from_np(n: &[f32; 3], p: &[f32; 3]) -> [f32; 4] {
    let dot = f64::from(n[2]) * f64::from(p[2])
        + (f64::from(n[1]) * f64::from(p[1]) + f64::from(n[0]) * f64::from(p[0]));
    [n[0], n[1], n[2], (-dot) as f32]
}

/// `LerpColor` (`0x690ae0`): each colour byte of `dst` moved toward `src` by `t` of 255, alpha
/// kept. `t == 0xff` takes `src`'s three bytes; otherwise each byte is `d + ((s − d)·t >> 8)`,
/// the difference in 32 bits, the shift logical and the sum a byte.
pub fn color_lerp_i(dst: u32, t: u32, src: u32) -> u32 {
    if t == 0xff {
        return ((src ^ dst) & 0x00ff_ffff) ^ dst;
    }
    let chan = |sh: u32| -> u8 {
        let s = (src >> sh) & 0xff;
        let d = (dst >> sh) & 0xff;
        let prod = (s.wrapping_sub(d)).wrapping_mul(t) >> 8;
        (prod as u8).wrapping_add(d as u8)
    };
    let b = chan(0);
    let g = chan(8);
    let r = chan(16);
    (dst & 0xff00_0000) | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

/// The integer fade of `ScaleSelfLitMaterials` (`0x6b4090`): `fistp(f32(fade·255) − 0.5)`, the product
/// stored at `0x6b40e3`. The fade is `+0x1ac` of the object `0x6d48b0` returns.
pub fn fade_to_i(fade: f32) -> i32 {
    let scaled = (f64::from(fade) * f64::from(MUL_255)) as f32;
    let biased = f64::from(scaled) - f64::from(SUB_FADE);
    round_ties_even_i32(biased)
}

/// One colour of `ScaleSelfLitMaterials` (`0x6b4090`): each of the three bytes times `i`, shifted right
/// by 8; the alpha is dropped. The function does this to every 0x40-byte element of `+0x1d8` (count
/// `+0x1dc`) whose first byte has 0x10, after copying its `+0x10` to `+0x14`.
pub fn color_fade_modulate_i(rgb: u32, i: i32) -> u32 {
    let iu = i as u32; // `imul` in 32 bits, the shift logical
    let b = ((rgb & 0xff).wrapping_mul(iu) >> 8) & 0xff;
    let g = (((rgb >> 8) & 0xff).wrapping_mul(iu) >> 8) & 0xff;
    let r = (((rgb >> 16) & 0xff).wrapping_mul(iu) >> 8) & 0xff;
    (r << 16) | (g << 8) | b
}

/// A fog record: two floats and a packed colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorRec {
    pub x: f32,
    pub y: f32,
    pub rgba: u32,
}

/// `LerpFogRecord` (`0x69efd0`): `dst` moved toward `src` by `t`. The floats are `f32(d + (s −
/// d)·t)`; the colour factor is `i = bits(f32(t·255 + 512)) >> 14 & 0xff`, and `i == 0` keeps the
/// colour, `i == 0xff` takes `src`'s three bytes, otherwise each byte moves as in `LerpColor`.
pub fn lerp_color_rec(dst: ColorRec, src: ColorRec, t: f32) -> ColorRec {
    let x = ((f64::from(src.x) - f64::from(dst.x)) * f64::from(t) + f64::from(dst.x)) as f32;
    let y = ((f64::from(src.y) - f64::from(dst.y)) * f64::from(t) + f64::from(dst.y)) as f32;
    let mag = ((f64::from(t) * f64::from(MUL_255)) + f64::from(ADD_ALPHA)) as f32;
    let i = (mag.to_bits() >> 14) & 0xff;
    let rgba = if i == 0 {
        dst.rgba
    } else if i == 0xff {
        ((dst.rgba ^ src.rgba) & 0x00ff_ffff) ^ dst.rgba
    } else {
        let chan = |sh: u32| -> u8 {
            let s = (src.rgba >> sh) & 0xff;
            let d = (dst.rgba >> sh) & 0xff;
            let prod = (s.wrapping_sub(d)).wrapping_mul(i) >> 8;
            (prod as u8).wrapping_add(d as u8)
        };
        let b = chan(0);
        let g = chan(8);
        let r = chan(16);
        (dst.rgba & 0xff00_0000) | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
    };
    ColorRec { x, y, rgba }
}

/// `ScaleMatrix3` (`0x672d80`): each of nine floats `f32(s·m[i])`.
pub fn mat3_scale_world(src: &[f32; 9], scalar: f32) -> [f32; 9] {
    let mut out = [0f32; 9];
    for (o, &s) in out.iter_mut().zip(src.iter()) {
        *o = (f64::from(scalar) * f64::from(s)) as f32;
    }
    out
}

/// The area table as [`area_id_lookup`] reads it.
pub trait AreaWorld {
    /// The tile pointer at `0xc96318 + 4·index`, 0 when absent.
    fn tile_ptr(&self, grid_index: usize) -> u32;
    /// The tile's `+0x278 + 4·index`, the chunk pointer, 0 when absent.
    fn chunk_ptr(&self, tile: u32, chunk_index: usize) -> u32;
    /// The chunk's `+0x16c`, its area id.
    fn area_id(&self, chunk: u32) -> u32;
}

/// `GetAreaId` (`0x69b120`, `systems/terrain/queries.md#area-id`): the area id at `(x, y)`, 0
/// where the tile or chunk is absent. `iy = fistp(f32(−(y − C)·0.03) − 0.5)` and `ix =
/// fistp(f32(0.03·f32(−(x − C))) − 0.5)`, with different stores; the tile is `(ix >> 4 & 0x3f)·64
/// + (iy >> 4 & 0x3f)` and the chunk `(ix & 0xf)·16 + (iy & 0xf)`.
pub fn area_id_lookup(world: &dyn AreaWorld, x: f32, y: f32) -> u32 {
    let sy = (-(f64::from(y) - f64::from(C_POS)) * f64::from(S_SCALE)) as f32;
    let iy = round_ties_even_i32(f64::from(sy) - f64::from(H_HALF));
    let nx = (-(f64::from(x) - f64::from(C_POS))) as f32;
    let sx = (f64::from(S_SCALE) * f64::from(nx)) as f32;
    let ix = round_ties_even_i32(f64::from(sx) - f64::from(H_HALF));

    let grid_index = (((ix >> 4) & 0x3f) * 0x40 + ((iy >> 4) & 0x3f)) as usize;
    let tile = world.tile_ptr(grid_index);
    if tile == 0 {
        return 0;
    }
    let chunk_index = (((ix & 0xf) * 0x10) + (iy & 0xf)) as usize;
    let chunk = world.chunk_ptr(tile, chunk_index);
    if chunk == 0 {
        return 0;
    }
    world.area_id(chunk)
}

/// `fistp` under the client's control word: round to nearest, ties to even.
fn round_ties_even_i32(v: f64) -> i32 {
    v.round_ties_even() as i32
}
