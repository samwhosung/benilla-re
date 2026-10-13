//! The depth bucket of a point and the viewport gate of the scene walk
//! (`systems/terrain/cull.md#depth-buckets`, `systems/terrain/cull.md#scene-frame`).

/// `[0x810174]`: 0.03 (`0x3cf5c28f`), the bucket per unit of depth.
const BUCKET_FACTOR: f32 = f32::from_bits(0x3cf5_c28f);
/// `[0x86861c]`: 0.5, subtracted before `fistp`.
const BUCKET_BIAS: f32 = 0.5;
/// `[0x8029d0]`: 0.01 (`0x3c23d70a`), the viewport's least width.
const VIEWPORT_EPS: f32 = f32::from_bits(0x3c23_d70a);

/// `CMapChunk::BucketPoint` (`0x6818b0`), its key: `dist = ((c1·y + c2·z) + c0·x) + c3` at 53 bits
/// (`0x6818b6` to `0x6818d4`), `f32(0.03·dist)` stored at `0x6818e2`, then `fistp(m − 0.5)`. The
/// clamp and list insert after it are not modelled.
pub fn depth_bucket_key(point: [f32; 3], coef: [f32; 4]) -> i32 {
    let (x, y, z) = (point[0] as f64, point[1] as f64, point[2] as f64);
    let (c0, c1, c2, c3) = (
        coef[0] as f64,
        coef[1] as f64,
        coef[2] as f64,
        coef[3] as f64,
    );
    let s1 = (c1 * y) + (c2 * z);
    let s2 = s1 + (c0 * x);
    let dist = s2 + c3;
    let m = (BUCKET_FACTOR as f64) * dist;
    let mf = m as f32;
    let v = (mf as f64) - (BUCKET_BIAS as f64);
    v.round_ties_even() as i32
}

/// The condition codes `fcomp` sets, as `fnstsw` puts them in AH.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FcompCc {
    pub c3: bool,
    pub c2: bool,
    pub c0: bool,
}

impl FcompCc {
    /// AH: C0 at bit 0, C2 at bit 2, C3 at bit 6.
    pub fn ah(self) -> u8 {
        (u8::from(self.c0)) | (u8::from(self.c2) << 2) | (u8::from(self.c3) << 6)
    }
}

/// `fcomp`: greater clears all, less sets C0, equal sets C3, unordered sets all three.
fn fcomp(width: f64, eps: f64) -> FcompCc {
    if width.is_nan() || eps.is_nan() {
        FcompCc {
            c3: true,
            c2: true,
            c0: true,
        }
    } else if width > eps {
        FcompCc {
            c3: false,
            c2: false,
            c0: false,
        }
    } else if width < eps {
        FcompCc {
            c3: false,
            c2: false,
            c0: true,
        }
    } else {
        FcompCc {
            c3: true,
            c2: false,
            c0: false,
        }
    }
}

/// One side of the gate: the codes and whether `jnp` returns early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateDim {
    pub cc: FcompCc,
    /// `jnp 0x683034` taken: the walk returns.
    pub jnp_taken: bool,
    /// The side passes.
    pub passed: bool,
}

/// One side: `hi − lo` against 0.01, `test ah,5; jnp`. The jump is taken on an odd count of C0 and
/// C2, so only a width below 0.01 fails; equal and NaN pass.
fn gate_dim(hi: f32, lo: f32) -> GateDim {
    let width = (hi as f64) - (lo as f64);
    let cc = fcomp(width, VIEWPORT_EPS as f64);
    let masked = cc.ah() & 0x5;
    let pf_even = masked.count_ones().is_multiple_of(2);
    let jnp_taken = !pf_even;
    GateDim {
        cc,
        jnp_taken,
        passed: !jnp_taken,
    }
}

/// `WalkScene` (`0x682fa0`) from `0x682fa3` to `0x682fc6`: the viewport `[lo_x, lo_y, hi_x,
/// hi_y]` at `+0x0` to `+0xc` must be at least 0.01 wide on x, then on y, or the walk returns. The
/// y side runs only when x passes; it is computed here either way.
pub fn viewport_gate(rect: [f32; 4]) -> (GateDim, GateDim) {
    let dim0 = gate_dim(rect[2], rect[0]);
    let dim1 = gate_dim(rect[3], rect[1]);
    (dim0, dim1)
}
