//! The chunk and liquid vertex builders and the vector selects they fold their bounds with
//! (`systems/terrain/chunk.md#chunk-vertices`, `systems/terrain/liquid.md#liquid-vertices`).

/// `[0x80fee4]`: the chunk edge, 33.333332 (`0x42055555`).
pub const CHUNK_SIZE: f32 = f32::from_bits(0x4205_5555);
/// `[0x7ffab4]`: the map's half extent, 17066.666 (`0x46855555`).
pub const MAP_CENTER: f32 = f32::from_bits(0x4685_5555);
/// `[0x807b10]`: 8.0.
const EIGHT: f64 = 8.0;
/// `[0x801628]`: 2.0.
const TWO: f64 = 2.0;

/// `MinVector3` (`0x699250`): the per-component minimum; z and y take `b` when `b <= a`, x takes
/// `a` when `a < b`.
pub fn vec3_min(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    let z = if b[2] <= a[2] { b[2] } else { a[2] };
    let y = if b[1] <= a[1] { b[1] } else { a[1] };
    let x = if a[0] < b[0] { a[0] } else { b[0] };
    [x, y, z]
}

/// `MaxVector3` (`0x6992c0`): the per-component maximum; z and y take `b` when `a <= b`, x takes
/// `a` when `b < a`.
pub fn vec3_max(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    let z = if a[2] <= b[2] { b[2] } else { a[2] };
    let y = if a[1] <= b[1] { b[1] } else { a[1] };
    let x = if b[0] < a[0] { a[0] } else { b[0] };
    [x, y, z]
}

/// `MaxVector3InPlace` (`0x6b11f0`): `dst[i] = src[i]` where `dst[i] <= src[i]`.
pub fn vec3_max_inplace(dst: &mut [f32; 3], src: &[f32; 3]) {
    let z = if dst[2] <= src[2] { src[2] } else { dst[2] };
    let y = if dst[1] <= src[1] { src[1] } else { dst[1] };
    let x = if dst[0] <= src[0] { src[0] } else { dst[0] };
    dst[0] = x;
    dst[1] = y;
    dst[2] = z;
}

/// The vertex step of chunk index `i`, with the stores of `0x6b0e50`'s head: `prod = f32(i·C)`,
/// `origin = f32(MAP_CENTER − prod)`, `d = f32(origin(i) − origin(i+1))`, `step = f32(−d/8)`. At
/// `i = 512` the product is the centre exactly and `513·C` rounds up to 17100.0, so the stores
/// decide the step.
pub(crate) fn grid_step(i: i32) -> f32 {
    let prod = |i: i32| -> f32 { (f64::from(i as f32) * f64::from(CHUNK_SIZE)) as f32 };
    let origin = |i: i32| -> f32 { (-f64::from(prod(i)) + f64::from(MAP_CENTER)) as f32 };
    let d = (f64::from(origin(i)) - f64::from(origin(i + 1))) as f32;
    (f64::from(-d) / EIGHT) as f32
}

/// What `0x6b0e50` writes into the chunk.
pub struct ChunkMesh {
    /// The 145 vertices at `+0x83c`: the 9×9 outer grid with the 8×8 inner grid between its rows,
    /// in the file's order.
    pub verts: [[f32; 3]; 145],
    /// The box minimum at `+0x44`, after the position is added.
    pub aabb_min: [f32; 3],
    /// The box maximum at `+0x50`.
    pub aabb_max: [f32; 3],
    /// The sphere centre at `+0x5c`, `(min + max) / 2`.
    pub center: [f32; 3],
    /// The sphere radius at `+0x68`, `√‖max − center‖²`.
    pub radius: f32,
    /// The copy of minimum and maximum at `+0x94`.
    pub bounds_copy: [f32; 6],
}

/// `CMapChunk::BuildVertices` (`0x6b0e50`): the 145 vertices from the relative heights, and the
/// box and sphere. `ix`, `iy` are the chunk's indices (`+0x8c`, `+0x90`) and `pos` its position
/// (`+0x6c`); heights are copied as raw dwords, and each coordinate is `f32(f32(k)·step)`, the
/// inner rows offset by the stored half steps.
pub fn mcvt_world_verts(ix: i32, iy: i32, pos: [f32; 3], heights: &[f32; 145]) -> ChunkMesh {
    let step_x = grid_step(ix); // the column axis, vert[1]
    let step_y = grid_step(iy); // the row axis, vert[0]
    let half_x = (f64::from(step_x) / TWO) as f32;
    let half_y = (f64::from(step_y) / TWO) as f32;
    let mut verts = [[0f32; 3]; 145];
    let mut min = [f32::MAX; 3];
    let mut max = [-f32::MAX; 3];
    let mut k = 0usize;
    let mut v = 0usize;
    for row in 0..9i32 {
        let row_coord = (f64::from(row as f32) * f64::from(step_y)) as f32;
        for col in 0..9i32 {
            let vert = [
                row_coord,
                (f64::from(col as f32) * f64::from(step_x)) as f32,
                heights[k],
            ];
            min = vec3_min(&min, &vert);
            vec3_max_inplace(&mut max, &vert);
            verts[v] = vert;
            k += 1;
            v += 1;
        }
        if row < 8 {
            for col in 0..8i32 {
                let vert = [
                    (f64::from(row_coord) + f64::from(half_y)) as f32,
                    (f64::from(col as f32) * f64::from(step_x) + f64::from(half_x)) as f32,
                    heights[k],
                ];
                min = vec3_min(&min, &vert);
                vec3_max_inplace(&mut max, &vert);
                verts[v] = vert;
                k += 1;
                v += 1;
            }
        }
    }
    for i in 0..3 {
        min[i] = (f64::from(pos[i]) + f64::from(min[i])) as f32;
        max[i] = (f64::from(max[i]) + f64::from(pos[i])) as f32;
    }
    // The sums are stored before each is halved.
    let sum_z = (f64::from(min[2]) + f64::from(max[2])) as f32;
    let sum_y = (f64::from(min[1]) + f64::from(max[1])) as f32;
    let sum_x = (f64::from(min[0]) + f64::from(max[0])) as f32;
    let center = [
        (f64::from(sum_x) / TWO) as f32,
        (f64::from(sum_y) / TWO) as f32,
        (f64::from(sum_z) / TWO) as f32,
    ];
    // The deltas and the sum of squares are stored; `fsqrt` at 53 bits.
    let dz = (f64::from(max[2]) - f64::from(center[2])) as f32;
    let dy = (f64::from(max[1]) - f64::from(center[1])) as f32;
    let dx = (f64::from(max[0]) - f64::from(center[0])) as f32;
    let sq = (f64::from(dx) * f64::from(dx)
        + f64::from(dy) * f64::from(dy)
        + f64::from(dz) * f64::from(dz)) as f32;
    let radius = f64::from(sq).sqrt() as f32;
    let bounds_copy = [min[0], min[1], min[2], max[0], max[1], max[2]];
    ChunkMesh {
        verts,
        aabb_min: min,
        aabb_max: max,
        center,
        radius,
        bounds_copy,
    }
}

/// `CChunkLiquid::BuildVertices` (`0x68d540`): the 81 liquid vertices `(row·step_y, col·step_x,
/// h − base)`, `h` the float at `+4` of each 8-byte cell and `base` the chunk's `+0x74`.
pub fn liquid_grid_verts(
    ix: i32,
    iy: i32,
    height_base: f32,
    heights: &[f32; 81],
) -> [[f32; 3]; 81] {
    let step_x = grid_step(ix);
    let step_y = grid_step(iy);
    let mut out = [[0f32; 3]; 81];
    for row in 0..9i32 {
        for col in 0..9i32 {
            let k = (row * 9 + col) as usize;
            out[k] = [
                (f64::from(row as f32) * f64::from(step_y)) as f32,
                (f64::from(col as f32) * f64::from(step_x)) as f32,
                (f64::from(heights[k]) - f64::from(height_base)) as f32,
            ];
        }
    }
    out
}

#[cfg(test)]
mod sanity {
    #[test]
    fn grid_step_bits() {
        // The step at index 0 is -4.166748046875.
        assert_eq!(crate::mesh::grid_step(0).to_bits(), 0xc085_5600, "step(0)");
        eprintln!(
            "step(512) bits = {:#010x}",
            crate::mesh::grid_step(512).to_bits()
        );
    }
}
