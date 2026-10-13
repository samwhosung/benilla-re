//! `CMovement::ResolveFall` (`0x635b00`) and `CMovement::ResolveFreeMove` (`0x634640`) run whole
//! with `QueryWorldTriangles` (`0x6721b0`) stubbed and the triangle lists written by the test, and
//! with the callees whose effects lie outside the mover stubbed: `IsUnitPlayerDriven` (`0x5fa550`),
//! `CMovement::SetTransportFromSupport` (`0x6173b0`) and `CMovement::UpdateAnchors` (`0x7c5cd0`),
//! and for `0x635b00` also `CMovement::SkipTime` (`0x615ae0`), `CMovement::ReportOpcode2ca`
//! (`0x61a800`), `CMovement::ApplyLatchedDirections` (`0x61ab20`), `CMovement::RestartFallAtRest`
//! (`0x633220`) and `CMovement::StopFalling` (`0x7c6290`). The sweeps run in the exe, and the
//! reimplementation is given their results.

#![allow(clippy::too_many_arguments)]

use benilla_re_collision::resolve_clip::HitRecord;
use benilla_re_collision::resolve_integrator::{IntegSeam, IntegState, SubstepSweep};
use benilla_re_collision::resolve_modes::{
    fall_mode_resolve, transport_mode_resolve, TransportSweep,
};
use benilla_re_collision::resolve_substep::SweepOut;
use benilla_re_collision::resolve_toolkit::Plane;
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};

/// The number of free-move sweeps run, for the tally.
static TSWEEPS: AtomicU32 = AtomicU32::new(0);
/// The number of liquid sweeps run, for the tally.
static LIQUID_SWEEPS: AtomicU32 = AtomicU32::new(0);

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// A random `f32` in `[-4, 4)`.
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}
/// Writes three `f32`.
fn wv3(emu: &mut Emu, va: u64, v: &[f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, c.to_bits());
    }
}
/// Reads three `f32`.
fn rv3(emu: &Emu, va: u64) -> [f32; 3] {
    core::array::from_fn(|i| emu.read_f32(va + (i as u64) * 4))
}
/// A unit vector.
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / l, v[1] / l, v[2] / l]
}
/// A level triangle around a point.
fn htri(c: [f32; 3]) -> [[f32; 3]; 3] {
    [
        [c[0] + 3.5, c[1], c[2]],
        [c[0] - 2.5, c[1] + 3.5, c[2]],
        [c[0] - 2.5, c[1] - 3.5, c[2]],
    ]
}
/// An upright triangle around a point.
fn vtri(c: [f32; 3]) -> [[f32; 3]; 3] {
    [
        [c[0], c[1] + 3.5, c[2]],
        [c[0], c[1] - 2.5, c[2] + 3.5],
        [c[0], c[1] - 2.5, c[2] - 3.5],
    ]
}

/// The addresses of the mover, the lists and the stubs' cells.
struct Vas {
    master: u64,
    sweep: u64,
    recs: u64,
    lrecs: u64,
    rows: u64,
    dir: u64,
    mani: u64,
    idx: u64,
    cnt: u64,
    t: u64,
    dirvec: u64,
    obj: u64,
    hcell: u64,
}

/// A sentinel for the support normal.
const SUPPORT_SENT: [f32; 3] = [
    f32::from_bits(0x1111_1111),
    f32::from_bits(0x2222_2222),
    f32::from_bits(0x3333_3333),
];

/// Maps the mover and the lists and installs the stubs, with the cached box.
fn setup(emu: &mut Emu, cache_min: [f32; 3], cache_max: [f32; 3]) -> Vas {
    let master = emu.alloc(0x200);
    let sweep = emu.alloc(0x200);
    let unit = emu.alloc(0x1000);
    let desc = emu.alloc(0x20);
    let ptr2 = emu.alloc(0x20);
    let aux = emu.alloc(0x10);
    let recs = emu.alloc(0x400);
    let lrecs = emu.alloc(0x400);
    let rows = emu.alloc(0x100);
    let dir = emu.alloc(0x10);
    let mani = emu.alloc(0x80);
    let idx = emu.alloc(8);
    let cnt = emu.alloc(8);
    let t = emu.alloc(8);
    let dirvec = emu.alloc(0x10);
    let obj = emu.alloc(0x20);
    let hcell = emu.alloc(8);
    for this in [master, sweep] {
        emu.write_u32(this + 0x15c, unit as u32);
        emu.write_u32(this + 0x30, aux as u32);
    }
    emu.write_u32(unit + 8, desc as u32);
    emu.write_u32(unit + 0xe68, ptr2 as u32);
    emu.write_u32(desc + 8, 0x10);
    emu.write_u32(aux, 0);
    emu.write_u32(aux + 4, 0);
    for k in 0..3u64 {
        emu.write_u32(0xc4e5a0 + k * 4, cache_min[k as usize].to_bits());
        emu.write_u32(0xc4e5ac + k * 4, cache_max[k as usize].to_bits());
    }
    emu.write_u32(0xc4e544, rows as u32);

    let mut patch = vec![0xd9u8, 0x05];
    patch.extend_from_slice(&(hcell as u32).to_le_bytes());
    patch.extend_from_slice(&[0xc2, 0x04, 0x00]);
    emu.write(0x7c6140, &patch);
    Vas {
        master,
        sweep,
        recs,
        lrecs,
        rows,
        dir,
        mani,
        idx,
        cnt,
        t,
        dirvec,
        obj,
        hcell,
    }
}

/// Writes the mover's fields.
fn plant_this(emu: &mut Emu, this_va: u64, st: &IntegState, height: f32, guid: u32) {
    wv3(emu, this_va + 0x10, &st.pos);
    emu.write_u32(this_va + 0x40, st.flags);
    emu.write_u32(this_va + 0x58, st.f58 as u32);
    wv3(emu, this_va + 0x5c, &st.support);
    emu.write_u32(this_va + 0x68, st.f68.to_bits());
    emu.write_u32(this_va + 0x6c, st.f6c.to_bits());
    emu.write_u32(this_va + 0x78, st.f78 as u32);
    emu.write_u32(this_va + 0x7c, st.f7c.to_bits());
    emu.write_u32(this_va + 0x84, st.f84.to_bits());
    emu.write_u32(this_va + 0xa0, st.fa0.to_bits());
    emu.write_u32(this_va + 0xb0, st.radius.to_bits());
    emu.write_u32(this_va + 0xb4, height.to_bits());
    emu.write_u32(this_va + 0x38, guid);
    emu.write_u32(this_va + 0x3c, 0);
    emu.write_u32(this_va + 0x20, 0);
    wv3(emu, this_va + 0x24, &SUPPORT_SENT);
}

/// Writes the triangle list the stubbed query returns.
fn plant_world(emu: &mut Emu, v: &Vas, hits: &[HitRecord], rows: &[[f32; 2]]) {
    for (i, h) in hits.iter().enumerate() {
        let rv = v.recs + (i as u64) * 0x34;
        wv3(emu, rv, &h.normal);
        emu.write_u32(rv + 0xc, h.d.to_bits());
        for (r, vv) in h.basis.iter().enumerate() {
            wv3(emu, rv + 0x10 + (r as u64) * 0xc, vv);
        }
    }
    for (i, row) in rows.iter().enumerate() {
        emu.write_u32(v.rows + (i as u64) * 8, row[0].to_bits());
        emu.write_u32(v.rows + (i as u64) * 8 + 4, row[1].to_bits());
    }
    emu.write_u32(0xc4e530, hits.len() as u32);
    emu.write_u32(0xc4e534, v.recs as u32);
    emu.write_u32(0xc4e568, 0);
    emu.write_u32(0xc4e56c, v.lrecs as u32);
}

/// Writes the liquid list.
fn plant_liquid(emu: &mut Emu, v: &Vas, hits: &[HitRecord]) {
    for (i, h) in hits.iter().enumerate() {
        let rv = v.lrecs + (i as u64) * 0x34;
        wv3(emu, rv, &h.normal);
        emu.write_u32(rv + 0xc, h.d.to_bits());
        for (r, vv) in h.basis.iter().enumerate() {
            wv3(emu, rv + 0x10 + (r as u64) * 0xc, vv);
        }
    }
    emu.write_u32(0xc4e568, hits.len() as u32);
    emu.write_u32(0xc4e56c, v.lrecs as u32);
}

/// Runs `CMovement::SweepVolume` (`0x632ba0`) in the exe for the reimplementation's sweep.
fn do_sweep(
    emu: &mut Emu,
    v: &Vas,
    pos: &[f32; 3],
    dir: &[f32; 3],
    len: f32,
    liquid: bool,
) -> (bool, u32, f32, [Plane; 7]) {
    wv3(emu, v.sweep + 0x10, pos);
    wv3(emu, v.dir, dir);
    for i in 0..7u64 {
        emu.call(0x6376a0, (v.mani + i * 0x10) as u32, 0, &[]);
    }
    emu.write_u32(v.idx, 0xdead_beef);
    emu.write_u32(v.cnt, 0);
    emu.write_u32(v.t, 0x7fc0_0001);
    let block = if liquid { 0xc4e564u32 } else { 0xc4e52cu32 };
    let sret = emu.call(
        0x632ba0,
        v.sweep as u32,
        0,
        &[
            block,
            v.dir as u32,
            len.to_bits(),
            v.idx as u32,
            v.mani as u32,
            v.cnt as u32,
            v.t as u32,
            0,
        ],
    );
    let s_idx = emu.read_u32(v.idx);
    let s_t = emu.read_f32(v.t);
    let mut manifold = [Plane::default(); 7];
    for (i, pl) in manifold.iter_mut().enumerate() {
        pl.n = rv3(emu, v.mani + (i as u64) * 0x10);
        pl.d = emu.read_f32(v.mani + (i as u64) * 0x10 + 0xc);
    }
    (sret != 0, s_idx, s_t, manifold)
}

/// One fall substep's sweep for the reimplementation.
fn fall_substep(
    emu: &mut Emu,
    v: &Vas,
    pos: &[f32; 3],
    dir: &[f32; 3],
    remaining: f32,
    n_hits: u32,
    hits: &[HitRecord],
    rows: &[[f32; 2]],
) -> SubstepSweep {
    let (contact, idx, t, manifold) = do_sweep(emu, v, pos, dir, remaining, false);
    let cnt = emu.read_u32(v.cnt);
    let hit = if (idx as usize) < hits.len() {
        hits[idx as usize]
    } else {
        HitRecord::default()
    };
    let row = if (idx as usize) < rows.len() {
        rows[idx as usize]
    } else {
        [0.0, 0.0]
    };
    SubstepSweep {
        sweep: SweepOut {
            contact,
            idx,
            t,
            count: cnt,
            hit_count: n_hits,
        },
        manifold,
        hit,
        row,
    }
}

/// Reads the mover's fields after a run.
fn read_outs(emu: &Emu, this: u64) -> IntegState {
    IntegState {
        pos: rv3(emu, this + 0x10),
        flags: emu.read_u32(this + 0x40),
        f58: emu.read_u32(this + 0x58) as i32,
        support: rv3(emu, this + 0x5c),
        f68: emu.read_f32(this + 0x68),
        f6c: emu.read_f32(this + 0x6c),
        f78: emu.read_u32(this + 0x78) as i32,
        f7c: emu.read_f32(this + 0x7c),
        f84: emu.read_f32(this + 0x84),
        fa0: emu.read_f32(this + 0xa0),
        radius: emu.read_f32(this + 0xb0),
        vmax_land: 0.0,
        vmax_swim: 0.0,
    }
}

/// Compares two movers' fields.
fn diff_state(a: &IntegState, b: &IntegState, ctx: &str) {
    for k in 0..3 {
        assert_eq!(
            a.pos[k].to_bits(),
            b.pos[k].to_bits(),
            "{ctx}: pos[{k}] bin={:#010x} ours={:#010x}",
            a.pos[k].to_bits(),
            b.pos[k].to_bits()
        );
        assert_eq!(
            a.support[k].to_bits(),
            b.support[k].to_bits(),
            "{ctx}: support[{k}]",
        );
    }
    assert_eq!(a.flags, b.flags, "{ctx}: flags");
    assert_eq!(a.f58, b.f58, "{ctx}: f58");
    assert_eq!(a.f68.to_bits(), b.f68.to_bits(), "{ctx}: f68");
    assert_eq!(a.f6c.to_bits(), b.f6c.to_bits(), "{ctx}: f6c");
    assert_eq!(a.f78, b.f78, "{ctx}: f78");
    assert_eq!(a.f84.to_bits(), b.f84.to_bits(), "{ctx}: f84");
}

/// Installs the stubs `0x635b00` needs.
fn fall_stubs(emu: &mut Emu) {
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 1, 2);
    emu.stub(0x615ae0, 0, 1);
    emu.stub(0x7c6290, 0, 0);
    emu.stub(0x633220, 0, 0);
    emu.stub(0x61a800, 0, 1);
    emu.stub(0x6173b0, 1, 2);
    emu.stub(0x61ab20, 1, 0);
    emu.stub(0x7c5cd0, 0, 1);
}

/// Runs `0x635b00` and the reimplementation on one case and compares them.
#[allow(clippy::too_many_arguments)]
fn run_fall(
    emu: &mut Emu,
    v: &Vas,
    init: &IntegState,
    height: f32,
    guid: u32,
    feas_this_c: u32,
    base: i32,
    dt: i32,
    substep: f32,
    dir: [f32; 3],
    fall_dist: f32,
    hits: &[HitRecord],
    rows: &[[f32; 2]],
    ctx: &str,
) -> u32 {
    let n_hits = hits.len() as u32;

    plant_this(emu, v.master, init, height, guid);
    plant_this(emu, v.sweep, init, height, guid);
    emu.write_u32(v.master + 0xc, feas_this_c);
    plant_world(emu, v, hits, rows);
    emu.write_u32(v.hcell, fall_dist.to_bits());
    wv3(emu, v.dirvec, &dir);
    let oeax = emu.call(
        0x635b00,
        v.master as u32,
        0,
        &[base as u32, dt as u32, substep.to_bits(), v.dirvec as u32],
    );
    let o_out = read_outs(emu, v.master);
    let o_support: [f32; 3] = rv3(emu, v.master + 0x24);

    let mut st = *init;
    let mut support = SUPPORT_SENT;
    let seam = IntegSeam { not_falling_ret: 1 };
    let guid_zero = guid == 0;
    let feas_pred = true;
    let reax = fall_mode_resolve(
        &mut st,
        &mut support,
        guid_zero,
        feas_this_c,
        feas_pred,
        n_hits,
        base,
        dt,
        substep,
        &dir,
        fall_dist,
        true,
        &seam,
        |pos, d, rem| fall_substep(emu, v, pos, d, rem, n_hits, hits, rows),
    );

    assert_eq!(oeax, reax, "{ctx}: eax (bin={oeax} ours={reax})");
    diff_state(&o_out, &st, ctx);
    for k in 0..3 {
        assert_eq!(
            o_support[k].to_bits(),
            support[k].to_bits(),
            "{ctx}: support_pt[{k}] bin={:#010x} ours={:#010x}",
            o_support[k].to_bits(),
            support[k].to_bits()
        );
    }
    reax
}

/// A fall that meets a triangle.
#[allow(clippy::type_complexity)]
fn fall_collide_case(
    s: &mut u32,
    base: &IntegState,
    height: &mut f32,
) -> (
    IntegState,
    f32,
    [f32; 3],
    f32,
    Vec<HitRecord>,
    Vec<[f32; 2]>,
) {
    let mut st = *base;
    st.pos = [fz(s), fz(s), fz(s)];
    st.radius = (fz(s).abs() * 0.4) + 0.35;
    *height = (fz(s).abs() * 0.5) + 0.7;
    st.f84 = fz(s);
    st.fa0 = -(fz(s).abs() * 8.0);
    st.f7c = st.pos[2] + fz(s).abs() * 2.0;
    let dir = normalize([fz(s) * 0.25, fz(s) * 0.25, 0.0]);
    let substep = fz(s).abs() + 0.5;
    let fall_dist = fz(s).abs() + 0.6;
    let delta = [
        (substep as f64 * dir[0] as f64) as f32,
        (substep as f64 * dir[1] as f64) as f32,
        -fall_dist,
    ];
    let center = [
        st.pos[0] + delta[0] * 0.5,
        st.pos[1] + delta[1] * 0.5,
        st.pos[2] + delta[2] * 0.5,
    ];
    let hits = vec![HitRecord {
        normal: [0.0, 0.0, 1.0],
        d: fz(s),
        basis: htri(center),
    }];
    let rows = vec![[fz(s) + 5.0, fz(s) - 5.0], [0.0, 0.0]];
    (st, substep, dir, fall_dist, hits, rows)
}

/// `0x635b00`: free falls and falls that meet floors and walls.
#[test]
fn fall_mode_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    fall_stubs(&mut emu);
    let vmax_land = emu.read_f32(0x87d894);
    let vmax_swim = emu.read_f32(0x87d898);
    let v = setup(&mut emu, [-1.0e6, -1.0e6, -1.0e6], [1.0e6, 1.0e6, 1.0e6]);

    let base = IntegState {
        flags: 0x2000,
        f68: 0.1,
        f6c: -0.2,
        f7c: 0.0,
        vmax_land,
        vmax_swim,
        ..Default::default()
    };

    let mut s = 0x635b_2026u32;
    let mut tally: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut checked = 0u32;
    let mut collide_partial = 0u32;

    for case in 0..220u32 {
        let mut height = 1.0f32;
        let (init, substep, dir, fall_dist, hits, rows) =
            fall_collide_case(&mut s, &base, &mut height);
        let dt = 5 + (next(&mut s) % 200) as i32;
        let eax = run_fall(
            &mut emu,
            &v,
            &init,
            height,
            0,
            0,
            0,
            dt,
            substep,
            dir,
            fall_dist,
            &hits,
            &rows,
            &format!("collide {case}"),
        );
        *tally.entry("collide").or_insert(0) += 1;
        if eax < dt as u32 {
            collide_partial += 1;
        }
        checked += 1;
    }

    for case in 0..60u32 {
        let mut init = base;
        init.pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        init.radius = (fz(&mut s).abs() * 0.4) + 0.35;
        init.fa0 = -(fz(&mut s).abs() * 8.0);
        init.f78 = (next(&mut s) % 400) as i32;
        init.f7c = init.pos[2] + fz(&mut s).abs() * 2.0;
        let height = 0.9;
        let dir = normalize([fz(&mut s) * 0.3, fz(&mut s) * 0.3, 0.0]);
        let substep = fz(&mut s).abs() + 0.5;
        let fall_dist = fz(&mut s).abs() + 0.4;
        let dt = 5 + (next(&mut s) % 200) as i32;
        run_fall(
            &mut emu,
            &v,
            &init,
            height,
            0,
            0,
            0,
            dt,
            substep,
            dir,
            fall_dist,
            &[],
            &[],
            &format!("freefall {case}"),
        );
        *tally.entry("freefall").or_insert(0) += 1;
        checked += 1;
    }

    for case in 0..20u32 {
        let mut init = base;
        init.pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        init.f78 = (next(&mut s) % 400) as i32;
        let dir = [fz(&mut s), fz(&mut s), 0.0];
        let dt = 5 + (next(&mut s) % 200) as i32;

        run_fall(
            &mut emu,
            &v,
            &init,
            0.9,
            0,
            0,
            0,
            dt,
            0.0,
            dir,
            0.0,
            &[],
            &[],
            &format!("degenerate {case}"),
        );
        *tally.entry("degenerate").or_insert(0) += 1;
        checked += 1;
    }

    {
        let mut init = base;
        init.pos = [1.0, 2.0, 3.0];

        run_fall(
            &mut emu,
            &v,
            &init,
            0.9,
            0,
            0,
            0,
            0,
            1.0,
            [1.0, 0.0, 0.0],
            1.0,
            &[],
            &[],
            "dt0",
        );
        *tally.entry("dt0").or_insert(0) += 1;
        checked += 1;

        run_fall(
            &mut emu,
            &v,
            &init,
            0.9,
            0x1234,
            0,
            0,
            60,
            0.7,
            normalize([0.3, 0.2, 0.0]),
            0.5,
            &[],
            &[],
            "guid_feasible",
        );
        *tally.entry("guid_feasible").or_insert(0) += 1;
        checked += 1;

        run_fall(
            &mut emu,
            &v,
            &init,
            0.9,
            0x1234,
            0x55,
            0,
            60,
            0.7,
            normalize([0.3, 0.2, 0.0]),
            0.5,
            &[],
            &[],
            "guid_infeasible",
        );
        *tally.entry("guid_infeasible").or_insert(0) += 1;
        checked += 1;
    }

    eprintln!("fall_mode: {checked} cases bit-exact vs WoW.exe");
    for (k, n) in &tally {
        eprintln!("    {k:<16} {n}");
    }
    eprintln!("  collide cases with eax<dt (partial-consume): {collide_partial}");
    assert!(
        collide_partial > 0,
        "no collide case partially consumed — the integrator leg never sub-stepped"
    );
}

/// One free-move sweep for the reimplementation.
fn transport_sweep(
    emu: &mut Emu,
    v: &Vas,
    pos: &[f32; 3],
    dir: &[f32; 3],
    len: f32,
    liquid: bool,
    pcount: u32,
    lcount: u32,
    phits: &[HitRecord],
    lhits: &[HitRecord],
) -> TransportSweep {
    TSWEEPS.fetch_add(1, Ordering::Relaxed);
    if liquid {
        LIQUID_SWEEPS.fetch_add(1, Ordering::Relaxed);
    }
    let (contact, idx, t, _mani) = do_sweep(emu, v, pos, dir, len, liquid);
    let (recs, cnt): (&[HitRecord], u32) = if liquid {
        (lhits, lcount)
    } else {
        (phits, pcount)
    };
    let hit_normal = if (idx as usize) < recs.len() && idx < cnt {
        recs[idx as usize].normal
    } else {
        [0.0, 0.0, 0.0]
    };
    TransportSweep {
        contact,
        idx,
        t,
        hit_normal,
    }
}

/// Runs `0x634640` and the reimplementation on one case and compares them.
#[allow(clippy::too_many_arguments)]
fn run_transport(
    emu: &mut Emu,
    v: &Vas,
    pos0: [f32; 3],
    flags: u32,
    radius: f32,
    height: f32,
    obj_flags: Option<u32>,
    guid: u32,
    feas_this_c: u32,
    budget: i32,
    dist: f32,
    dir0: [f32; 3],
    phits: &[HitRecord],
    lhits: &[HitRecord],
    ctx: &str,
) -> u32 {
    let pcount = phits.len() as u32;
    let lcount = lhits.len() as u32;

    for this in [v.master, v.sweep] {
        wv3(emu, this + 0x10, &pos0);
        emu.write_u32(this + 0x40, flags);
        emu.write_u32(this + 0xb0, radius.to_bits());
        emu.write_u32(this + 0xb4, height.to_bits());
        emu.write_u32(this + 0x38, guid);
        emu.write_u32(this + 0x3c, 0);
        emu.write_u32(this + 0x20, 0);
        emu.write_u32(this + 0xa4, obj_flags.map(|_| v.obj as u32).unwrap_or(0));
    }
    if let Some(of) = obj_flags {
        emu.write_u32(v.obj + 0x18, of);
    }
    emu.write_u32(v.master + 0xc, feas_this_c);
    let rows = [[0.0f32, 0.0]];
    plant_world(emu, v, phits, &rows);
    plant_liquid(emu, v, lhits);

    let oeax = emu.call(
        0x634640,
        v.master as u32,
        0,
        &[
            0,
            budget as u32,
            dist.to_bits(),
            dir0[0].to_bits(),
            dir0[1].to_bits(),
            dir0[2].to_bits(),
        ],
    );
    let o_pos = rv3(emu, v.master + 0x10);

    let esp = emu.get_reg(RegisterX86::ESP);
    let o_dir: [f32; 3] = [
        emu.read_f32(esp - 0xc),
        emu.read_f32(esp - 8),
        emu.read_f32(esp - 4),
    ];

    let mut pos = pos0;
    let mut dir = dir0;
    let guid_zero = guid == 0;
    let reax = transport_mode_resolve(
        &mut pos,
        flags,
        obj_flags,
        guid_zero,
        feas_this_c,
        true,
        pcount,
        lcount,
        budget,
        dist,
        &mut dir,
        |p, d, len, liquid| {
            transport_sweep(emu, v, p, d, len, liquid, pcount, lcount, phits, lhits)
        },
    );

    assert_eq!(oeax, reax, "{ctx}: eax (bin={oeax} ours={reax})");
    for k in 0..3 {
        assert_eq!(
            o_pos[k].to_bits(),
            pos[k].to_bits(),
            "{ctx}: pos[{k}] bin={:#010x} ours={:#010x}",
            o_pos[k].to_bits(),
            pos[k].to_bits()
        );
        assert_eq!(
            o_dir[k].to_bits(),
            dir[k].to_bits(),
            "{ctx}: dir[{k}] bin={:#010x} ours={:#010x}",
            o_dir[k].to_bits(),
            dir[k].to_bits()
        );
    }
    reax
}

/// A wall across the move.
fn wall_geom(s: &mut u32) -> ([f32; 3], f32, f32, f32, [f32; 3], Vec<HitRecord>) {
    let pos = [fz(s), fz(s), fz(s)];
    let radius = (fz(s).abs() * 0.4) + 0.35;
    let height = (fz(s).abs() * 0.5) + 0.7;
    let dist = fz(s).abs() + 1.0;
    let dir = normalize([-(fz(s).abs() + 0.6), fz(s) * 0.2, fz(s) * 0.15]);
    let center = [
        pos[0] + dir[0] * dist * 0.5,
        pos[1] + dir[1] * dist * 0.5,
        pos[2] + dir[2] * dist * 0.5,
    ];
    let hits = vec![HitRecord {
        normal: [1.0, 0.0, 0.0],
        d: fz(s),
        basis: vtri(center),
    }];
    (pos, radius, height, dist, dir, hits)
}

/// `0x634640` against walls: with the liquid list, with flag 0x800, and with an object skipped.
#[test]
fn transport_mode_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 1, 2);
    emu.stub(0x6173b0, 1, 2);
    emu.stub(0x7c5cd0, 0, 1);
    let v = setup(&mut emu, [-1.0e6, -1.0e6, -1.0e6], [1.0e6, 1.0e6, 1.0e6]);

    let mut s = 0x6346_2026u32;
    let mut tally: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut checked = 0u32;

    let lhit_for = |c: [f32; 3]| {
        vec![HitRecord {
            normal: [1.0, 0.0, 0.0],
            d: 0.0,
            basis: vtri(c),
        }]
    };

    for case in 0..360u32 {
        let regime = case % 3;
        let (pos, radius, height, dist, dir, phits) = wall_geom(&mut s);
        let lcenter = [pos[0] + dir[0] * dist, pos[1] + dir[1] * dist, pos[2]];
        let lhits = lhit_for(lcenter);
        let budget = 5 + (next(&mut s) % 200) as i32;
        let (label, flags, obj_flags) = match regime {
            0 => ("wall_liquid", 0x2000u32, None),
            1 => ("wall_noblocker", 0x2000 | 0x800, None),
            _ => ("wall_objskip", 0x2000u32, Some(0x200u32)),
        };
        run_transport(
            &mut emu,
            &v,
            pos,
            flags,
            radius,
            height,
            obj_flags,
            0,
            0,
            budget,
            dist,
            dir,
            &phits,
            &lhits,
            &format!("{label} {case}"),
        );
        *tally.entry(label).or_insert(0) += 1;
        checked += 1;
    }

    let mut liquid_found = false;
    for _ in 0..2000u32 {
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = (fz(&mut s).abs() * 0.4) + 0.35;
        let height = (fz(&mut s).abs() * 0.5) + 0.7;
        let off = radius + fz(&mut s).abs() * 0.3;
        let cx = pos[0] + off;
        let phits = vec![HitRecord {
            normal: [1.0, 0.0, 0.0],
            d: -cx,
            basis: vtri([cx, pos[1], pos[2]]),
        }];
        let dir = normalize([fz(&mut s) * 0.05, 1.0, fz(&mut s) * 0.1]);
        let dist = fz(&mut s).abs() + 1.0;

        emu.write_u32(v.sweep + 0xb0, radius.to_bits());
        emu.write_u32(v.sweep + 0xb4, height.to_bits());
        emu.write_u32(v.sweep + 0x40, 0x2000);
        plant_world(&mut emu, &v, &phits, &[[0.0f32, 0.0]]);
        let (contact, idx, _, _) = do_sweep(&mut emu, &v, &pos, &dir, dist, false);
        if contact && idx >= phits.len() as u32 {
            let budget = 80;
            run_transport(
                &mut emu,
                &v,
                pos,
                0x2000,
                radius,
                height,
                None,
                0,
                0,
                budget,
                dist,
                dir,
                &phits,
                &phits,
                "liquid_sentinel",
            );
            *tally.entry("liquid_sentinel").or_insert(0) += 1;
            checked += 1;
            liquid_found = true;
            break;
        }
    }

    {
        let (pos, radius, height, dist, dir, phits) = wall_geom(&mut s);
        let lhits = lhit_for(pos);

        run_transport(
            &mut emu, &v, pos, 0x2000, radius, height, None, 0, 0, 0, dist, dir, &phits, &lhits,
            "budget0",
        );
        *tally.entry("budget0").or_insert(0) += 1;
        checked += 1;

        run_transport(
            &mut emu,
            &v,
            pos,
            0x2000,
            radius,
            height,
            None,
            0,
            0,
            80,
            0.0,
            dir,
            &phits,
            &lhits,
            "dist_degenerate",
        );
        *tally.entry("dist_degenerate").or_insert(0) += 1;
        checked += 1;

        run_transport(
            &mut emu,
            &v,
            pos,
            0x2000,
            radius,
            height,
            None,
            0x99,
            0x55,
            80,
            dist,
            dir,
            &phits,
            &lhits,
            "guid_infeasible",
        );
        *tally.entry("guid_infeasible").or_insert(0) += 1;
        checked += 1;
    }

    {
        let mut emu2 = Emu::new(&wow);
        emu2.stub(0x5fa550, 1, 0);
        emu2.stub(0x6721b0, 0, 2);
        emu2.stub(0x6173b0, 1, 2);
        emu2.stub(0x7c5cd0, 0, 1);
        let v2 = setup(&mut emu2, [-0.02, -0.02, -0.02], [0.02, 0.02, 0.02]);
        let phits = vec![HitRecord {
            normal: [1.0, 0.0, 0.0],
            d: 0.0,
            basis: vtri([8.0, 6.0, 7.0]),
        }];
        let lhits = vec![HitRecord {
            normal: [1.0, 0.0, 0.0],
            d: 0.0,
            basis: vtri([8.0, 6.0, 7.0]),
        }];
        run_transport(
            &mut emu2,
            &v2,
            [5.0, 6.0, 7.0],
            0x2000,
            0.5,
            0.9,
            None,
            0,
            0,
            80,
            1.5,
            normalize([1.0, 0.2, 0.0]),
            &phits,
            &lhits,
            "no_hit",
        );
        *tally.entry("no_hit").or_insert(0) += 1;
        checked += 1;
    }

    eprintln!("transport_mode: {checked} cases bit-exact vs WoW.exe");
    for (k, n) in &tally {
        eprintln!("    {k:<16} {n}");
    }
    let tsweeps = TSWEEPS.load(Ordering::Relaxed);
    let liquid = LIQUID_SWEEPS.load(Ordering::Relaxed);

    eprintln!("  total sweeps={tsweeps} (of which liquid second-passes={liquid})");
    if liquid_found {
        assert!(
            liquid > 0,
            "a first-sweep sentinel was found but the liquid second-pass (0xc4e564) never fired"
        );
    } else {
        eprintln!(
            "  NOTE: no first-sweep sentinel synthesized — liquid-sweep invocation uncovered"
        );
    }
    assert!(
        tsweeps > checked,
        "no case ran more than its first sweep — the reflect/liquid loop never fired"
    );
}

/// `0x634640` where the sweep's hit index differs from the list's count.
#[test]
#[allow(clippy::type_complexity)]
fn transport_mode_hitindex_mismatch_probe() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 1, 2);
    emu.stub(0x6173b0, 1, 2);
    emu.stub(0x7c5cd0, 0, 1);
    let v = setup(&mut emu, [-1.0e6, -1.0e6, -1.0e6], [1.0e6, 1.0e6, 1.0e6]);

    let mut s = 0x634A_C0DEu32;

    let mut found: Option<([f32; 3], f32, f32, f32, [f32; 3], Vec<HitRecord>, u32)> = None;
    for _ in 0..4000u32 {
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = (fz(&mut s).abs() * 0.4) + 0.35;
        let height = (fz(&mut s).abs() * 0.5) + 0.7;
        let dist = fz(&mut s).abs() + 1.0;

        let dir = [1.0f32, 0.0, 0.0];

        let f = 0.3 + fz(&mut s).abs() * 0.15;
        let cx = pos[0] + dir[0] * dist * f;
        let center = [cx, pos[1], pos[2]];
        let hits = vec![HitRecord {
            normal: [-1.0, 0.0, 0.0],
            d: cx,
            basis: vtri(center),
        }];

        emu.write_u32(v.sweep + 0xb0, radius.to_bits());
        emu.write_u32(v.sweep + 0xb4, height.to_bits());
        emu.write_u32(v.sweep + 0x40, 0x2000);
        plant_world(&mut emu, &v, &hits, &[[0.0f32, 0.0]]);
        let (contact, idx, _, _) = do_sweep(&mut emu, &v, &pos, &dir, dist, false);
        if contact && (idx as usize) < hits.len() {
            found = Some((pos, radius, height, dist, dir, hits, idx));
            break;
        }
    }

    let (pos, radius, height, dist, dir, phits, probe_idx) =
        found.expect("no head-on geometry produced a committed valid hit index (idx < count)");

    assert!(
        (probe_idx as usize) < phits.len(),
        "probe must commit idx<count to take 0x6347ac (got idx={probe_idx}, count={})",
        phits.len()
    );

    let lhits = vec![HitRecord {
        normal: [-1.0, 0.0, 0.0],
        d: pos[0],
        basis: vtri(pos),
    }];
    let budget = 80;
    run_transport(
        &mut emu,
        &v,
        pos,
        0x2000,
        radius,
        height,
        None,
        0,
        0,
        budget,
        dist,
        dir,
        &phits,
        &lhits,
        "hitindex_mismatch",
    );
    eprintln!(
        "transport_mode hitindex-mismatch probe: 1 case bit-exact vs WoW.exe \
         (forced the 0x6347ac dir-projection twin: committed idx={probe_idx} < primary_count={}, \
         so loop head 0x634703 took jb 0x6347a5 and fell through to the 0x6347ac process twin)",
        phits.len()
    );
}
