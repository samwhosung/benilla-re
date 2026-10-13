//! The cell walks, each run whole: `TraceTerrainSegment` (`0x69c320`) and `TraceTerrainFirstHit`
//! (`0x6aa3e0`) with their cell tests (`0x69c920`, `0x6aa600`) stubbed to miss, and the four walks
//! `0x69c4e0`, `0x69c570`, `0x69c600` and `0x69c780` on their own. The list each leaves at
//! `*0xc962c8` and its count at `0xc89f40` are compared.

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::dda::{
    ray_dda_chunk, ray_dda_map, CellIntersector, CellList, CellSeg, StepFloats,
};
use common::{assert_f32, next};

// The cell list's globals.
const CELL_PTR: u64 = 0xc96_2c8; // the list's buffer

const CELL_CNT: u64 = 0xc89_f40; // the count of integers

/// A cell test that always misses, like the stub.
struct MissIntersector;

impl CellIntersector for MissIntersector {
    fn intersect(
        &mut self,
        _list: &CellList,
        _ray: &benilla_re_terrain::dda::Ray6,
        _t_io: &mut f32,
    ) -> bool {
        false
    }
}

/// The exe's cell list: `*CELL_PTR`, `CELL_CNT` integers.
fn read_cell_list(emu: &Emu) -> Vec<i32> {
    let base = emu.read_u32(CELL_PTR) as u64;
    let n = emu.read_u32(CELL_CNT) as usize;
    (0..n)
        .map(|i| emu.read_u32(base + i as u64 * 4) as i32)
        .collect()
}

/// Zeroes the count: the traces do so themselves, the walks alone append to it.
fn reset_cell_list(emu: &mut Emu) {
    emu.write_u32(CELL_CNT, 0);
}

fn assert_cells(binary: &[i32], ours: &CellList, ctx: &str) {
    assert_eq!(
        binary.len(),
        ours.data.len(),
        "{ctx}: cell-int count binary={} ours={}",
        binary.len(),
        ours.data.len()
    );
    for (i, (b, o)) in binary.iter().zip(ours.data.iter()).enumerate() {
        assert_eq!(b, o, "{ctx}: cell int [{i}] binary={b} ours={o}");
    }
}

/// A segment as the callers cast them, `(A, B)`: the start anywhere in `[−2000, 36000]`, so cells
/// can be negative or past the grid, and the end within about 512 units of it per axis, so a walk
/// is about 100 cells at most, as the queries (`0x69bff0`, `0x6821f0`, `0x670250`) and the
/// 1000-unit ground ray of `0x6a8a20` cast them. Edge values fall on cell edges (1/0.24 units).
fn synth_axis(s: &mut u32) -> f32 {
    let r = next(s);
    if (r >> 8).is_multiple_of(8) {
        // multiples of a cell about the centre
        let edges = [
            17066.666,
            0.0,
            4.166_666_5,
            8.333_333,
            -4.166_666_5,
            17070.832,
            17062.5,
            33000.0,
            -1000.0,
            12.5,
            25.0,
            16000.0,
        ];
        edges[((r >> 11) as usize) % edges.len()]
    } else {
        let u = (r >> 8) as f64 / (1u32 << 24) as f64;
        (u * 38000.0 - 2000.0) as f32
    }
}

/// An offset of at most about 512 units per axis, often a small exact step, so short walks and
/// corners are common.
fn synth_offset(s: &mut u32) -> f32 {
    let r = next(s);
    if (r >> 8).is_multiple_of(6) {
        let small = [
            0.0,
            4.166_666_5,
            -4.166_666_5,
            2.083_333,
            -2.083_333,
            8.333_333,
            0.1,
            -0.1,
            50.0,
            -50.0,
        ];
        small[((r >> 11) as usize) % small.len()]
    } else {
        let u = (r >> 8) as f64 / (1u32 << 24) as f64;
        (u * 1024.0 - 512.0) as f32
    }
}

/// A segment: the start in the map, the end a bounded offset from it.
fn synth_seg(s: &mut u32) -> ((f32, f32), (f32, f32)) {
    let ax = synth_axis(s);
    let ay = synth_axis(s);
    let bx = (f64::from(ax) + f64::from(synth_offset(s))) as f32;
    let by = (f64::from(ay) + f64::from(synth_offset(s))) as f32;
    ((ax, ay), (bx, by))
}

/// Which way each case went, counted and printed, so a rare branch cannot go unseen at zero.
#[derive(Default)]
struct Cover {
    run_b: u32,    // |dy| < eps or ya == yb: 0x69c570
    run_a: u32,    // |dx| < eps or xa == xb: 0x69c4e0
    ymajor: u32,   // |dy| <= |dx|: 0x69c780
    xmajor: u32,   // otherwise: 0x69c600
    neg_dir: u32,  // a walk downward (start cell above end cell)
    corner: u32,   // a line walk listed a corner
    endpoint: u32, // a line walk added the end cell
}

const CELL_SCALE: f64 = 0.239_999_994_635_581_97; // 0x3e75c28f, for the classifier

const RECENTER: f64 = 17_066.666_015_625; // 0x46855555

fn classify(a: (f32, f32), b: (f32, f32)) -> (i32, i32, i32, i32, f32, f32) {
    let q = |w: f32| -> (f32, i32) {
        let r = (-(f64::from(w) - RECENTER)) as f32;
        let sc = (CELL_SCALE * f64::from(r)) as f32;
        let cell = (f64::from(sc) - 0.5).round_ties_even() as i32;
        (r, cell)
    };
    let (ya_r, ya_c) = q(a.1);
    let (xa_r, xa_c) = q(a.0);
    let (yb_r, yb_c) = q(b.1);
    let (xb_r, xb_c) = q(b.0);
    let dy = (f64::from(yb_r) - f64::from(ya_r)) as f32;
    let dx = (f64::from(xb_r) - f64::from(xa_r)) as f32;
    (ya_c, xa_c, yb_c, xb_c, dy, dx)
}

/// The two runs share one stream of 4000 cell rectangles, forward, backward and single; each test
/// runs its own function over it.
fn axis_run_env<'a>(wow: &Wow) -> (Emu<'a>, u64) {
    let mut emu = Emu::new(wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let buf = emu.alloc(0x10000);
    emu.write_u32(CELL_PTR, buf as u32);
    let pseg = emu.alloc(0x10); // the four cells
    (emu, pseg)
}

fn axis_run_seg(s: &mut u32) -> CellSeg {
    let xa = (next(s) >> 8) as i32 % 200 - 100;
    let ya = (next(s) >> 8) as i32 % 200 - 100;
    // spans forward, backward and single
    let xb = xa + ((next(s) >> 8) as i32 % 41 - 20);
    let yb = ya + ((next(s) >> 8) as i32 % 41 - 20);
    CellSeg { xa, ya, xb, yb }
}

fn write_seg(emu: &mut Emu, pseg: u64, seg: &CellSeg) {
    emu.write_u32(pseg, seg.xa as u32);
    emu.write_u32(pseg + 4, seg.ya as u32);
    emu.write_u32(pseg + 8, seg.xb as u32);
    emu.write_u32(pseg + 0xc, seg.yb as u32);
}

/// The two line walks share one stream of 6000 cases; the trace sends each to one of them
/// (`|dy| <= |dx|` to 0x69c780, otherwise 0x69c600), and each test checks the cases its function
/// takes, with its own corner and end counts.
struct MajorEnv<'a> {
    emu: Emu<'a>,
    p1: u64,
    p2: u64,
    pseg: u64,
}

fn major_axis_env<'a>(wow: &Wow) -> MajorEnv<'a> {
    let mut emu = Emu::new(wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    // the inputs first, the cell buffer last and large, so an overrun cannot reach them
    let p1 = emu.alloc(0x10);
    let p2 = emu.alloc(0x10);
    let pseg = emu.alloc(0x10);
    let buf = emu.alloc(0x40000);
    emu.write_u32(CELL_PTR, buf as u32);
    MajorEnv { emu, p1, p2, pseg }
}

/// One line-walk case: the coordinates about the centre, the cells they make, and the extents.
/// `None` for a case a run takes (a single cell, or a zero extent).
struct MajorCase {
    pf1: StepFloats,
    pf2: StepFloats,
    seg: CellSeg,
    ya_c: i32,
    xa_c: i32,
    yb_c: i32,
    xb_c: i32,
    abs_dy: f32,
    abs_dx: f32,
}

fn major_axis_case(s: &mut u32) -> Option<MajorCase> {
    let (a, b) = synth_seg(s);
    let (ya_c, xa_c, yb_c, xb_c, dy, dx) = classify(a, b);
    if dy == 0.0 || dx == 0.0 || ya_c == yb_c || xa_c == xb_c {
        return None;
    }
    // c0 = y, c1 = x
    let pf1 = StepFloats {
        c0: (-(f64::from(a.1) - RECENTER)) as f32,
        c1: (-(f64::from(a.0) - RECENTER)) as f32,
    };
    let pf2 = StepFloats {
        c0: (-(f64::from(b.1) - RECENTER)) as f32,
        c1: (-(f64::from(b.0) - RECENTER)) as f32,
    };
    let seg = CellSeg {
        xa: xa_c,
        ya: ya_c,
        xb: xb_c,
        yb: yb_c,
    };
    Some(MajorCase {
        pf1,
        pf2,
        seg,
        ya_c,
        xa_c,
        yb_c,
        xb_c,
        abs_dy: f64::from(dy).abs() as f32,
        abs_dx: f64::from(dx).abs() as f32,
    })
}

fn write_major(env: &mut MajorEnv<'_>, c: &MajorCase) {
    env.emu.write_u32(env.p1, c.pf1.c0.to_bits());
    env.emu.write_u32(env.p1 + 4, c.pf1.c1.to_bits());
    env.emu.write_u32(env.p2, c.pf2.c0.to_bits());
    env.emu.write_u32(env.p2 + 4, c.pf2.c1.to_bits());
    env.emu.write_u32(env.pseg, c.seg.xa as u32);
    env.emu.write_u32(env.pseg + 4, c.seg.ya as u32);
    env.emu.write_u32(env.pseg + 8, c.seg.xb as u32);
    env.emu.write_u32(env.pseg + 0xc, c.seg.yb as u32);
}

fn synth_z(s: &mut u32) -> f32 {
    let r = next(s);
    let u = (r >> 8) as f64 / (1u32 << 24) as f64;
    (u * 200.0 - 100.0) as f32
}

fn record_cover(cov: &mut Cover, a: (f32, f32), b: (f32, f32), list: &CellList) {
    let n_ints = list.data.len();
    let pairs = list.pairs();
    let (ya_c, xa_c, yb_c, xb_c, dy, dx) = classify(a, b);
    let eps = f32::from_bits(0x3480_0000);
    let abs_dy = f64::from(dy).abs() as f32;
    let abs_dx = f64::from(dx).abs() as f32;
    let _ = n_ints;
    // A line walk lists the first cell, then one pair per step for `start + dir ..= end + dir`, the
    // last one past the end; so the least is `span + 1` pairs, each corner adds one, and the end
    // fix adds `(yb, xb)`. That fix shows as the last pair `(yb, xb)` after a pair one past the
    // end, which an ending without it cannot have.
    let endpoint_fired = |xmajor: bool| -> bool {
        if pairs.len() < 2 {
            return false;
        }
        let last = pairs[pairs.len() - 1];
        let prev = pairs[pairs.len() - 2];
        if last != (yb_c, xb_c) {
            return false;
        }
        if xmajor {
            let dir = if ya_c < yb_c { 1 } else { -1 };
            prev.0 == yb_c + dir
        } else {
            let dir = if xa_c < xb_c { 1 } else { -1 };
            prev.1 == xb_c + dir
        }
    };
    if abs_dy < eps || ya_c == yb_c {
        cov.run_b += 1;
        if xa_c > xb_c {
            cov.neg_dir += 1;
        }
    } else if abs_dx < eps || xa_c == xb_c {
        cov.run_a += 1;
        if ya_c > yb_c {
            cov.neg_dir += 1;
        }
    } else if abs_dy <= abs_dx {
        cov.ymajor += 1;
        if xa_c > xb_c {
            cov.neg_dir += 1;
        }
        let e = endpoint_fired(false);
        if e {
            cov.endpoint += 1;
        }
        let span = (xb_c - xa_c).unsigned_abs() as usize + 1;
        if pairs.len() > span + 1 + usize::from(e) {
            cov.corner += 1;
        }
    } else {
        cov.xmajor += 1;
        if ya_c > yb_c {
            cov.neg_dir += 1;
        }
        let e = endpoint_fired(true);
        if e {
            cov.endpoint += 1;
        }
        let span = (yb_c - ya_c).unsigned_abs() as usize + 1;
        if pairs.len() > span + 1 + usize::from(e) {
            cov.corner += 1;
        }
    }
}

fn report_cover(name: &str, checked: u32, cov: &Cover) {
    eprintln!(
        "{name}: {checked} segments bit-exact | run_a={} run_b={} xmajor={} ymajor={} neg_dir={} corner={} endpoint={}",
        cov.run_a, cov.run_b, cov.xmajor, cov.ymajor, cov.neg_dir, cov.corner, cov.endpoint
    );
}

fn assert_cover(cov: &Cover) {
    assert!(cov.run_a > 0, "run_a (0x69c4e0) dispatch uncovered");
    assert!(cov.run_b > 0, "run_b (0x69c570) dispatch uncovered");
    assert!(cov.xmajor > 0, "xmajor (0x69c600) dispatch uncovered");
    assert!(cov.ymajor > 0, "ymajor (0x69c780) dispatch uncovered");
    assert!(cov.neg_dir > 0, "negative-direction walk uncovered");
    assert!(cov.corner > 0, "corner double-emit uncovered");
    assert!(cov.endpoint > 0, "endpoint fixup uncovered");
}

#[test]
fn ray_dda_map_driver() {
    // The four walks, downward walks, corners and end fixes are each counted and must occur; the
    // segments lie about the centre, edge values on cell edges.
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.stub(0x69c920, 0, 3); // the cell test misses: `ret 0xc`, 3 arguments

    // the inputs first, the cell buffer last and large (0x10000 integers)
    let pa = emu.alloc(0x10);
    let pb = emu.alloc(0x10);
    // the three arguments passed on to the stubbed cell test
    let arg3 = emu.alloc(0x10);
    let buf = emu.alloc(0x40000);
    emu.write_u32(CELL_PTR, buf as u32);

    let mut s: u32 = 0x0dda_0001;
    let mut cov = Cover::default();
    let mut checked = 0u32;
    for case in 0..6000usize {
        let (a, b) = synth_seg(&mut s);
        emu.write_u32(pa, a.0.to_bits());
        emu.write_u32(pa + 4, a.1.to_bits());
        emu.write_u32(pb, b.0.to_bits());
        emu.write_u32(pb + 4, b.1.to_bits());
        emu.write_u32(CELL_CNT, 0xdead_beef); // the trace must zero it

        // ecx = A, edx = B, 3 arguments, `ret 0xc`
        emu.call(
            0x69c320,
            pa as u32,
            pb as u32,
            &[arg3 as u32, arg3 as u32, arg3 as u32],
        );
        let bin = read_cell_list(&emu);

        let mut ours = CellList::default();
        // the walk reads x and y only; t passes to the stubbed test and comes back unchanged
        let mut t_dummy = 1.0e30f32;
        ray_dda_map(
            &[a.0, a.1, 0.0],
            &[b.0, b.1, 0.0],
            &mut t_dummy,
            &mut ours,
            &mut MissIntersector,
        );
        assert_eq!(
            t_dummy.to_bits(),
            1.0e30f32.to_bits(),
            "all-miss t unchanged"
        );
        assert_cells(&bin, &ours, &format!("ray_dda_map case {case}"));

        record_cover(&mut cov, a, b, &ours);
        checked += 1;
    }
    report_cover("ray_dda_map_driver", checked, &cov);
    assert_cover(&cov);
}

#[test]
fn ray_dda_chunk_driver() {
    // as for 0x69c320, and a miss leaves t unchanged
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.stub(0x6aa600, 0, 2); // the cell test misses: `ret 8`, 2 arguments

    // the inputs first, the cell buffer last and large
    let seg = emu.alloc(0x20); // A at 0..3, B at 3..6
    let t_io = emu.alloc(8);
    let arg = emu.alloc(0x10); // the 2 arguments passed on
    let buf = emu.alloc(0x40000);
    emu.write_u32(CELL_PTR, buf as u32);

    let mut s: u32 = 0x0dda_c000;
    let mut cov = Cover::default();
    let mut checked = 0u32;
    for case in 0..6000usize {
        // the z components near 1, read only by the stubbed test
        let (a, b) = synth_seg(&mut s);
        let az = synth_z(&mut s);
        let bz = synth_z(&mut s);
        let seg6 = [a.0, a.1, az, b.0, b.1, bz];
        for (i, &f) in seg6.iter().enumerate() {
            emu.write_u32(seg + i as u64 * 4, f.to_bits());
        }
        let t0 = synth_z(&mut s).abs() + 0.5; // the fraction t0
        emu.write_u32(t_io, t0.to_bits());
        emu.write_u32(CELL_CNT, 0xdead_beef);

        // ecx = seg, edx = &t, 2 arguments, `ret 8`
        let ret = emu.call(0x6aa3e0, seg as u32, t_io as u32, &[arg as u32, arg as u32]);
        let bin = read_cell_list(&emu);
        let bin_t = emu.read_f32(t_io);

        let mut ours = CellList::default();
        let mut t_ours = t0;
        let hit = ray_dda_chunk(&seg6, &mut t_ours, &mut ours, &mut MissIntersector);
        assert_cells(&bin, &ours, &format!("ray_dda_chunk case {case}"));
        // a miss: al = 0, t unchanged
        assert_eq!(ret & 0xff, 0, "case {case}: all-miss must return al=0");
        assert!(!hit, "case {case}: the model's miss flag");
        assert_f32(bin_t, t_ours, &format!("ray_dda_chunk case {case} t_io"));

        record_cover(&mut cov, a, b, &ours);
        checked += 1;
    }
    report_cover("ray_dda_chunk_driver", checked, &cov);
    assert_cover(&cov);
}

#[test]
fn dda_axis_run_a() {
    // forward, backward and single; x fixed, y from ya to yb
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut emu, pseg) = axis_run_env(&wow);
    let mut s: u32 = 0x57e9_0001;
    let (mut fwd, mut bwd, mut single) = (0u32, 0u32, 0u32);
    let mut checked = 0u32;
    for case in 0..4000usize {
        let seg = axis_run_seg(&mut s);
        write_seg(&mut emu, pseg, &seg);
        reset_cell_list(&mut emu);
        emu.call(0x69c4e0, pseg as u32, 0, &[]);
        let bin_a = read_cell_list(&emu);
        let mut ours_a = CellList::default();
        benilla_re_terrain::dda::dda_axis_run_a(&seg, &mut ours_a);
        assert_cells(&bin_a, &ours_a, &format!("axis_run_a case {case}"));
        match seg.ya.cmp(&seg.yb) {
            std::cmp::Ordering::Less => fwd += 1,
            std::cmp::Ordering::Greater => bwd += 1,
            std::cmp::Ordering::Equal => single += 1,
        }
        checked += 1;
    }
    eprintln!(
        "dda_axis_run_a: {checked} runs bit-exact vs 0x69c4e0 (y fwd={fwd} bwd={bwd} single={single})"
    );
    assert!(
        fwd > 0 && bwd > 0 && single > 0,
        "axis-run-a branches uncovered"
    );
}

#[test]
fn dda_axis_run_b() {
    // y fixed, x from xa to xb; the same stream
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut emu, pseg) = axis_run_env(&wow);
    let mut s: u32 = 0x57e9_0001;
    let (mut fwd, mut bwd, mut single) = (0u32, 0u32, 0u32);
    let mut checked = 0u32;
    for case in 0..4000usize {
        let seg = axis_run_seg(&mut s);
        write_seg(&mut emu, pseg, &seg);
        reset_cell_list(&mut emu);
        emu.call(0x69c570, pseg as u32, 0, &[]);
        let bin_b = read_cell_list(&emu);
        let mut ours_b = CellList::default();
        benilla_re_terrain::dda::dda_axis_run_b(&seg, &mut ours_b);
        assert_cells(&bin_b, &ours_b, &format!("axis_run_b case {case}"));
        match seg.xa.cmp(&seg.xb) {
            std::cmp::Ordering::Less => fwd += 1,
            std::cmp::Ordering::Greater => bwd += 1,
            std::cmp::Ordering::Equal => single += 1,
        }
        checked += 1;
    }
    eprintln!(
        "dda_axis_run_b: {checked} runs bit-exact vs 0x69c570 (x fwd={fwd} bwd={bwd} single={single})"
    );
    assert!(
        fwd > 0 && bwd > 0 && single > 0,
        "axis-run-b branches uncovered"
    );
}

#[test]
fn dda_step_ymajor() {
    // ecx = p1, edx = p2, &seg on the stack, `ret 4`; the cases with |dy| <= |dx|
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut env = major_axis_env(&wow);
    let mut s: u32 = 0x903a_0001;
    let (mut x_corner, mut x_end) = (0u32, 0u32);
    for case in 0..6000usize {
        let Some(c) = major_axis_case(&mut s) else {
            continue;
        };
        if c.abs_dy > c.abs_dx {
            continue; // the other walk's case
        }
        write_major(&mut env, &c);
        reset_cell_list(&mut env.emu);
        env.emu
            .call(0x69c780, env.p1 as u32, env.p2 as u32, &[env.pseg as u32]);
        let bin = read_cell_list(&env.emu);
        let mut ours = CellList::default();
        benilla_re_terrain::dda::dda_step_ymajor(&c.pf1, &c.pf2, &c.seg, &mut ours);
        assert_cells(&bin, &ours, &format!("ymajor case {case}"));
        // more than the least count of integers means a corner
        let span = (c.xb_c - c.xa_c).unsigned_abs() as usize + 1;
        if ours.data.len() > span * 2 {
            x_corner += 1;
        }
        x_end += 1;
    }
    eprintln!("dda_step_ymajor: {x_end} walks bit-exact vs 0x69c780 (corners={x_corner})");
    assert!(x_end > 0 && x_corner > 0, "x-major branch/corner gap");
}

#[test]
fn dda_step_xmajor() {
    // the cases with |dy| > |dx|; the same stream
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut env = major_axis_env(&wow);
    let mut s: u32 = 0x903a_0001;
    let (mut y_corner, mut y_end) = (0u32, 0u32);
    for case in 0..6000usize {
        let Some(c) = major_axis_case(&mut s) else {
            continue;
        };
        if c.abs_dy <= c.abs_dx {
            continue; // the other walk's case
        }
        write_major(&mut env, &c);
        reset_cell_list(&mut env.emu);
        env.emu
            .call(0x69c600, env.p1 as u32, env.p2 as u32, &[env.pseg as u32]);
        let bin = read_cell_list(&env.emu);
        let mut ours = CellList::default();
        benilla_re_terrain::dda::dda_step_xmajor(&c.pf1, &c.pf2, &c.seg, &mut ours);
        assert_cells(&bin, &ours, &format!("xmajor case {case}"));
        let span = (c.yb_c - c.ya_c).unsigned_abs() as usize + 1;
        if ours.data.len() > span * 2 {
            y_corner += 1;
        }
        y_end += 1;
    }
    eprintln!("dda_step_xmajor: {y_end} walks bit-exact vs 0x69c600 (corners={y_corner})");
    assert!(y_end > 0 && y_corner > 0, "y-major branch/corner gap");
}
