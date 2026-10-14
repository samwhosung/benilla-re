//! The cell walks of the segment traces: `TraceTerrainSegment` (`0x69c320`) and
//! `TraceTerrainFirstHit` (`0x6aa3e0`), and the four walks they share
//! (`systems/terrain/trace.md#terrain-walk`, `#cell-walks`, `#terrain-first-hit`).
//!
//! Both ends of the segment are taken about 17066.666 and turned into cells, `fistp(f32(0.24·r) −
//! 0.5)`. A walk lists the cells from the start's to the end's, a corner crossing listing both
//! cells, as pairs `(y cell, x cell)` in the list at `0xc962c8`, whose count `0xc89f40` counts the
//! integers, twice the pairs. Each trace then tests the listed cells (`crate::intersect`).

/// World units to cells, 0.24 (`0x3e75c28f`): `[0x8107a4]` for `0x69c320`, `[0x810ae4]` for
/// `0x6aa3e0`, the same bits.
const CELL_X024: f32 = f32::from_bits(0x3e75_c28f);

/// `[0x7ffab4]`: the map's half extent, 17066.666 (`0x46855555`).
const RECENTER: f32 = f32::from_bits(0x4685_5555);

/// 0.5 before `fistp`: `[0x86a758]` and `[0x86aa2c]`.
const HALF: f32 = f32::from_bits(0x3f00_0000);

/// `[0x8029d4]`: 2^-22 (`0x34800000`), the extent below which a walk runs along one axis.
const DEGEN_EPS: f32 = f32::from_bits(0x3480_0000);

/// `[0x810678]`: 4.1666665 (`0x40855555`), a cell's size, the step of the line walks.
const STEP_4167: f32 = f32::from_bits(0x4085_5555);

/// `[0x7ff9d8]`: 1.0.
const ONE: f32 = f32::from_bits(0x3f80_0000);

/// The cell list: the integers at `*0xc962c8`, each cell a pair `(y cell, x cell)` written in that
/// order; its length is the count at `0xc89f40`. The traces zero the count before a walk.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CellList {
    pub data: Vec<i32>,
}

impl CellList {
    fn clear(&mut self) {
        self.data.clear();
    }
    #[inline]
    fn push(&mut self, v: i32) {
        self.data.push(v);
    }
    /// The count at `0xc89f40`, the integers written.
    pub fn count(&self) -> usize {
        self.data.len()
    }
    /// The `(y cell, x cell)` pairs in order.
    pub fn pairs(&self) -> Vec<(i32, i32)> {
        self.data
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| (c[0], c[1]))
            .collect()
    }
}

/// The four cells the traces pass a walk: the start's x and y cells, then the end's.
#[derive(Debug, Clone, Copy)]
pub struct CellSeg {
    /// `+0x0`: the start's x cell.
    pub xa: i32,
    /// `+0x4`: the start's y cell.
    pub ya: i32,
    /// `+0x8`: the end's x cell.
    pub xb: i32,
    /// `+0xc`: the end's y cell.
    pub yb: i32,
}

/// `ListCellRowRun` (`0x69c4e0`): the x cell fixed, y cells from `ya` to `yb` inclusive, up or
/// down; each cell `(y, xa)`.
pub fn dda_axis_run_a(seg: &CellSeg, list: &mut CellList) {
    let yb = seg.yb;
    let mut y = seg.ya;
    if y <= yb {
        loop {
            list.push(y);
            list.push(seg.xa);
            y += 1;
            if y > yb {
                break;
            }
        }
    } else {
        loop {
            list.push(y);
            list.push(seg.xa);
            y -= 1;
            if y < yb {
                break;
            }
        }
    }
}

/// `ListCellColumnRun` (`0x69c570`): the y cell fixed, x cells from `xa` to `xb` inclusive, up or
/// down; each cell `(ya, x)`.
pub fn dda_axis_run_b(seg: &CellSeg, list: &mut CellList) {
    let xb = seg.xb;
    let mut x = seg.xa;
    if x <= xb {
        loop {
            list.push(seg.ya);
            list.push(x);
            x += 1;
            if x > xb {
                break;
            }
        }
    } else {
        loop {
            list.push(seg.ya);
            list.push(x);
            x -= 1;
            if x < xb {
                break;
            }
        }
    }
}

/// An end of the segment as the line walks read it: its y, then its x, both taken about the
/// centre.
#[derive(Debug, Clone, Copy)]
pub struct StepFloats {
    /// `+0x0`: y.
    pub c0: f32,
    /// `+0x4`: x.
    pub c1: f32,
}

/// `ListCellsAlongRows` (`0x69c600`), when the y extent is the larger: the slope `m = (p2.c1 −
/// p1.c1) / (p2.c0 − p1.c0)` is kept in the register for `b = f32(p1.c1 − m·p1.c0)` and stored for
/// the loop (`fst [ebp-8]`). From the start cell, each y cell's x cell is `fistp(f32((r·m + b)·0.24)
/// − 0.5)` with `r` the stored coordinate `(y ± 1)·4.1666665`, advanced by a stored step; a change
/// of x cell lists `(y, new x)` before the next y cell's. The end cell is added when the last x
/// cell is not the end's.
pub fn dda_step_xmajor(p1: &StepFloats, p2: &StepFloats, seg: &CellSeg, list: &mut CellList) {
    let m_full = (f64::from(p2.c1) - f64::from(p1.c1)) / (f64::from(p2.c0) - f64::from(p1.c0));
    let m = m_full as f32;
    let p1x = p1.c0;
    let p1y = p1.c1;
    let b = ((-(m_full * f64::from(p1x))) + f64::from(p1y)) as f32;

    let mut y = seg.ya;
    let x_fixed = seg.xa;

    let (dir, fild_seed): (i32, i32) = if y < seg.yb { (1, y + 1) } else { (-1, y) };
    let mut r = (f64::from(fild_seed) * f64::from(STEP_4167)) as f32;

    list.push(y);
    list.push(x_fixed);

    let mut col = x_fixed;

    if y != seg.yb + dir {
        let step = (f64::from(dir) * f64::from(STEP_4167)) as f32;
        loop {
            let scaled =
                ((f64::from(r) * f64::from(m) + f64::from(b)) * f64::from(CELL_X024)) as f32;
            let new_col = crate::x87::fistp(f64::from(scaled) - f64::from(HALF));

            if new_col != col {
                list.push(y);
                list.push(new_col);
            }
            r = (f64::from(step) + f64::from(r)) as f32;
            y += dir;
            list.push(y);
            list.push(new_col);
            col = new_col;
            if y == seg.yb + dir {
                break;
            }
        }
    }

    if col != seg.xb {
        list.push(seg.yb);
        list.push(seg.xb);
    }
}

/// `ListCellsAlongColumns` (`0x69c780`), when the x extent is at least the y extent: `b = f32(p1.c1
/// − m·p1.c0)` and `inv = f32(1/m)`; each x cell's y cell is `fistp(f32(((r − b)·inv)·0.24) − 0.5)`,
/// with the same corner and end rules.
pub fn dda_step_ymajor(p1: &StepFloats, p2: &StepFloats, seg: &CellSeg, list: &mut CellList) {
    let m = (f64::from(p2.c1) - f64::from(p1.c1)) / (f64::from(p2.c0) - f64::from(p1.c0));
    let p1x = p1.c0;
    let p1y = p1.c1;

    let b = ((-(m * f64::from(p1x))) + f64::from(p1y)) as f32;
    let inv = (f64::from(ONE) / m) as f32;

    let mut x = seg.xa;
    let y_fixed = seg.ya;

    let (dir, fild_seed): (i32, i32) = if x < seg.xb { (1, x + 1) } else { (-1, x) };
    let mut r = (f64::from(fild_seed) * f64::from(STEP_4167)) as f32;

    list.push(y_fixed);
    list.push(x);

    let mut row = y_fixed;

    if x != seg.xb + dir {
        let step = (f64::from(dir) * f64::from(STEP_4167)) as f32;
        loop {
            let scaled =
                (((f64::from(r) - f64::from(b)) * f64::from(inv)) * f64::from(CELL_X024)) as f32;
            let new_row = crate::x87::fistp(f64::from(scaled) - f64::from(HALF));

            if new_row != row {
                list.push(new_row);
                list.push(x);
            }
            r = (f64::from(step) + f64::from(r)) as f32;
            x += dir;
            list.push(new_row);
            list.push(x);
            row = new_row;
            if x == seg.xb + dir {
                break;
            }
        }
    }

    if row != seg.yb {
        list.push(seg.yb);
        list.push(seg.xb);
    }
}

/// One end's coordinates `f32(−(c − 17066.666))` and cells `fistp(f32(0.24·r) − 0.5)`, y first.
fn recenter_quantize(wx: f32, wy: f32) -> (f32, f32, i32, i32) {
    let yr = (-(f64::from(wy) - f64::from(RECENTER))) as f32;
    let xr = (-(f64::from(wx) - f64::from(RECENTER))) as f32;
    let y_scaled = (f64::from(CELL_X024) * f64::from(yr)) as f32;
    let y_cell = crate::x87::fistp(f64::from(y_scaled) - f64::from(HALF));
    let x_scaled = (f64::from(CELL_X024) * f64::from(xr)) as f32;
    let x_cell = crate::x87::fistp(f64::from(x_scaled) - f64::from(HALF));
    (yr, xr, y_cell, x_cell)
}

/// The walk both traces run (`0x69c3d5` to `0x69c4b8`, `0x6aa498` to `0x6aa57e`): with the stored
/// extents, a y extent below 2^-22 or one y cell runs `0x69c570`; else an x extent below 2^-22 or
/// one x cell runs `0x69c4e0`; else the y extent at most the x extent runs `0x69c780`, and above it
/// `0x69c600`. A NaN extent fails the `<` and goes on to the cell test.
fn dda_walk(ax: f32, ay: f32, bx: f32, by: f32, list: &mut CellList) {
    let (ya_r, xa_r, ya_cell, xa_cell) = recenter_quantize(ax, ay);
    let (yb_r, xb_r, yb_cell, xb_cell) = recenter_quantize(bx, by);

    let dy = (f64::from(yb_r) - f64::from(ya_r)) as f32;
    let dx = (f64::from(xb_r) - f64::from(xa_r)) as f32;

    let seg = CellSeg {
        xa: xa_cell,
        ya: ya_cell,
        xb: xb_cell,
        yb: yb_cell,
    };

    list.clear();

    let abs_dy = f64::from(dy).abs() as f32;
    let p1 = StepFloats { c0: ya_r, c1: xa_r };
    let p2 = StepFloats { c0: yb_r, c1: xb_r };

    if abs_dy < DEGEN_EPS || ya_cell == yb_cell {
        dda_axis_run_b(&seg, list);
    } else {
        let abs_dx = f64::from(dx).abs() as f32;
        if abs_dx < DEGEN_EPS || xa_cell == xb_cell {
            dda_axis_run_a(&seg, list);
        } else if abs_dy <= abs_dx {
            dda_step_ymajor(&p1, &p2, &seg, list);
        } else {
            dda_step_xmajor(&p1, &p2, &seg, list);
        }
    }
}

/// The test of the listed cells a trace runs after its walk, given the trace's ray; on a hit it
/// writes `t` through `t_io`. `crate::intersect` has the two the client uses.
pub trait CellIntersector {
    /// Tests the listed cells against `ray`; true on a hit.
    fn intersect(&mut self, list: &CellList, ray: &Ray6, t_io: &mut f32) -> bool;
}

/// `TraceTerrainSegment` (`0x69c320`, `systems/terrain/trace.md#terrain-walk`): the walk from `a`
/// to `b` (x and y), then `TraceListedCells` (`0x69c920`) given `a`, `b` and its own three stack
/// arguments. The result is the intersector's.
pub fn ray_dda_map(
    a: &[f32; 3],
    b: &[f32; 3],
    t_io: &mut f32,
    list: &mut CellList,
    isect: &mut impl CellIntersector,
) {
    dda_walk(a[0], a[1], b[0], b[1], list);
    isect.intersect(
        list,
        &Ray6 {
            origin: *a,
            endpoint: *b,
        },
        t_io,
    );
}

/// A ray as `0x6aa3e0` passes it: the start, and the start plus `(end − start)·t0`.
#[derive(Debug, Clone, Copy)]
pub struct Ray6 {
    pub origin: [f32; 3],
    pub endpoint: [f32; 3],
}

/// `TraceTerrainFirstHit` (`0x6aa3e0`, `systems/terrain/trace.md#terrain-first-hit`): the start at
/// `seg[0..3]` and the end at `seg[3..6]`, `t_io` the fraction `t0`. The same walk, then
/// `TraceListedCellsFirstHit` (`0x6aa600`) on the ray from the start to `start + (end − start)·t0`
/// (`0x4841f0`, `0x5f8cf0`, `0x621160`); on a hit the fraction becomes `f32(t·t0)`. Returns the hit.
pub fn ray_dda_chunk(
    seg: &[f32; 6],
    t_io: &mut f32,
    list: &mut CellList,
    isect: &mut impl CellIntersector,
) -> bool {
    let a = [seg[0], seg[1], seg[2]];
    let b = [seg[3], seg[4], seg[5]];

    dda_walk(a[0], a[1], b[0], b[1], list);

    let t0 = *t_io;
    let dir = vec3_sub(&b, &a);
    let scaled = vec3_scale(&dir, t0);
    let endpoint = vec3_add(&a, &scaled);
    let ray = Ray6 {
        origin: a,
        endpoint,
    };

    let hit = isect.intersect(list, &ray, t_io);
    if hit {
        *t_io = (f64::from(*t_io) * f64::from(t0)) as f32;
    }
    hit
}

/// `0x4841f0`: `a − b`, each component stored as `f32`.
fn vec3_sub(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        (f64::from(a[0]) - f64::from(b[0])) as f32,
        (f64::from(a[1]) - f64::from(b[1])) as f32,
        (f64::from(a[2]) - f64::from(b[2])) as f32,
    ]
}

/// `0x5f8cf0`: `a·s`, each component stored as `f32`.
fn vec3_scale(a: &[f32; 3], s: f32) -> [f32; 3] {
    [
        (f64::from(s) * f64::from(a[0])) as f32,
        (f64::from(s) * f64::from(a[1])) as f32,
        (f64::from(s) * f64::from(a[2])) as f32,
    ]
}

/// `0x621160`: `a + b`, each component stored as `f32`.
fn vec3_add(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        (f64::from(a[0]) + f64::from(b[0])) as f32,
        (f64::from(a[1]) + f64::from(b[1])) as f32,
        (f64::from(a[2]) + f64::from(b[2])) as f32,
    ]
}
