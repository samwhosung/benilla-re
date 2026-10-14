//! `CMovement::ResolveAirborneMove` (`0x635600`) and `CMovement::ProbeFallAway` (`0x635f80`) run
//! whole with `QueryWorldTriangles` (`0x6721b0`) stubbed and the triangle list written by the test,
//! and with the callees whose effects lie outside the mover stubbed and their calls recorded:
//! `IsUnitPlayerDriven` (`0x5fa550`), `CMovement::SkipTime` (`0x615ae0`),
//! `CMovement::SetTransportFromSupport` (`0x6173b0`), `CMovement::ReportOpcode2ca` (`0x61a800`),
//! `CMovement::ApplyLatchedDirections` (`0x61ab20`), `CMovement::RestartFallAtRest` (`0x633220`),
//! `CMovement::UpdateAnchors` (`0x7c5cd0`) and `CMovement::StopFalling` (`0x7c6290`), and for
//! `0x635f80` also `SplatVector3` (`0x4078e0`), `CMovement::GetFallDrop` (`0x7c6140`) and
//! `CMovement::StartFalling2` (`0x7c61f0`). The sweeps run in the exe, and the reimplementation is
//! given their results.

#![allow(clippy::too_many_arguments)]

use benilla_re_collision::resolve_clip::HitRecord;
use benilla_re_collision::resolve_integrator::{
    fall_integrator, resolve_fall_land, IntegSeam, IntegState, SubstepSweep,
};
use benilla_re_collision::resolve_substep::SweepOut;
use benilla_re_collision::resolve_toolkit::Plane;
use benilla_re_difftest::{Emu, Wow};
use std::collections::BTreeMap;

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
    rows: u64,
    dir: u64,
    mani: u64,
    idx: u64,
    cnt: u64,
    t: u64,
    dirvec: u64,
    saved: u64,
}

/// Maps the mover and the lists and installs the stubs, with the cached box.
fn setup(emu: &mut Emu, cache_min: [f32; 3], cache_max: [f32; 3]) -> Vas {
    let master = emu.alloc(0x200);
    let sweep = emu.alloc(0x200);
    let unit = emu.alloc(0x1000);
    let desc = emu.alloc(0x20);
    let ptr2 = emu.alloc(0x20);
    let aux = emu.alloc(0x10);
    let recs = emu.alloc(0x400);
    let rows = emu.alloc(0x100);
    let dir = emu.alloc(0x10);
    let mani = emu.alloc(0x80);
    let idx = emu.alloc(8);
    let cnt = emu.alloc(8);
    let t = emu.alloc(8);
    let dirvec = emu.alloc(0x10);
    let saved = emu.alloc(0x10);

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
    Vas {
        master,
        sweep,
        recs,
        rows,
        dir,
        mani,
        idx,
        cnt,
        t,
        dirvec,
        saved,
    }
}

/// Writes the mover's fields.
fn plant_this(emu: &mut Emu, this_va: u64, st: &IntegState, height: f32) {
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

    emu.write_u32(this_va + 0x38, 0);
    emu.write_u32(this_va + 0x3c, 0);
    emu.write_u32(this_va + 0x20, 0);
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
}

/// Runs `CMovement::SweepVolume` (`0x632ba0`) in the exe for the reimplementation's sweep.
fn do_sweep(
    emu: &mut Emu,
    v: &Vas,
    pos: &[f32; 3],
    dir: &[f32; 3],
    remaining: f32,
    n_hits: u32,
    hits: &[HitRecord],
    rows: &[[f32; 2]],
) -> SubstepSweep {
    wv3(emu, v.sweep + 0x10, pos);
    wv3(emu, v.dir, dir);
    for i in 0..7u64 {
        emu.call(0x6376a0, (v.mani + i * 0x10) as u32, 0, &[]);
    }
    emu.write_u32(v.idx, 0xdead_beef);
    emu.write_u32(v.cnt, 0);
    emu.write_u32(v.t, 0x7fc0_0001);
    let sret = emu.call(
        0x632ba0,
        v.sweep as u32,
        0,
        &[
            0xc4e52c,
            v.dir as u32,
            remaining.to_bits(),
            v.idx as u32,
            v.mani as u32,
            v.cnt as u32,
            v.t as u32,
            0,
        ],
    );
    let s_idx = emu.read_u32(v.idx);
    let s_t = emu.read_f32(v.t);
    let s_cnt = emu.read_u32(v.cnt);
    let mut manifold = [Plane::default(); 7];
    for (i, pl) in manifold.iter_mut().enumerate() {
        pl.n = rv3(emu, v.mani + (i as u64) * 0x10);
        pl.d = emu.read_f32(v.mani + (i as u64) * 0x10 + 0xc);
    }
    let hit = if (s_idx as usize) < hits.len() {
        hits[s_idx as usize]
    } else {
        HitRecord::default()
    };
    let row = if (s_idx as usize) < rows.len() {
        rows[s_idx as usize]
    } else {
        [0.0, 0.0]
    };
    SubstepSweep {
        sweep: SweepOut {
            contact: sret != 0,
            idx: s_idx,
            t: s_t,
            count: s_cnt,
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
            "{ctx}: pos[{k}] binary={:#010x} ours={:#010x}",
            a.pos[k].to_bits(),
            b.pos[k].to_bits()
        );
        assert_eq!(
            a.support[k].to_bits(),
            b.support[k].to_bits(),
            "{ctx}: support[{k}] binary={:#010x} ours={:#010x}",
            a.support[k].to_bits(),
            b.support[k].to_bits()
        );
    }
    assert_eq!(a.flags, b.flags, "{ctx}: flags");
    assert_eq!(a.f58, b.f58, "{ctx}: f58");
    assert_eq!(
        a.f68.to_bits(),
        b.f68.to_bits(),
        "{ctx}: f68 binary={:#010x} ours={:#010x}",
        a.f68.to_bits(),
        b.f68.to_bits()
    );
    assert_eq!(
        a.f6c.to_bits(),
        b.f6c.to_bits(),
        "{ctx}: f6c binary={:#010x} ours={:#010x}",
        a.f6c.to_bits(),
        b.f6c.to_bits()
    );
    assert_eq!(a.f78, b.f78, "{ctx}: f78");
    assert_eq!(
        a.f84.to_bits(),
        b.f84.to_bits(),
        "{ctx}: f84 binary={:#010x} ours={:#010x}",
        a.f84.to_bits(),
        b.f84.to_bits()
    );
}

/// Runs `0x635600` and the reimplementation on one case and compares them, recorded calls included.
fn run_integrator(
    emu: &mut Emu,
    v: &Vas,
    init: &IntegState,
    height: f32,
    base: i32,
    dt_ms: i32,
    delta: [f32; 3],
    flag: i32,
    not_falling: u32,
    hits: &[HitRecord],
    rows: &[[f32; 2]],
    ctx: &str,
) -> u32 {
    let n_hits = hits.len() as u32;

    plant_this(emu, v.master, init, height);
    plant_this(emu, v.sweep, init, height);
    plant_world(emu, v, hits, rows);
    let oeax = emu.call(
        0x635600,
        v.master as u32,
        0,
        &[
            base as u32,
            dt_ms as u32,
            delta[0].to_bits(),
            delta[1].to_bits(),
            delta[2].to_bits(),
            flag as u32,
        ],
    );
    let o_out = read_outs(emu, v.master);

    let mut st = *init;
    let seam = IntegSeam {
        not_falling_ret: not_falling,
    };
    let reax = fall_integrator(
        &mut st,
        base,
        dt_ms,
        delta,
        flag,
        true,
        &seam,
        |pos, dir, remaining| do_sweep(emu, v, pos, dir, remaining, n_hits, hits, rows),
    );

    assert_eq!(oeax, reax, "{ctx}: eax (binary={oeax} ours={reax})");
    diff_state(&o_out, &st, ctx);
    reax
}

/// A floor under the fall.
fn floor_case(
    s: &mut u32,
    init_base: &IntegState,
    height: &mut f32,
) -> (IntegState, [f32; 3], Vec<HitRecord>, Vec<[f32; 2]>) {
    let mut st = *init_base;
    st.pos = [fz(s), fz(s), fz(s)];
    st.radius = (fz(s).abs() * 0.4) + 0.35;
    *height = (fz(s).abs() * 0.5) + 0.7;
    st.f84 = fz(s);
    st.fa0 = -(fz(s).abs() * 8.0);
    st.f7c = st.pos[2] + fz(s).abs() * 2.0;
    let cd = (fz(s).abs()) + 1.0;
    let mdir = normalize([fz(s) * 0.25, fz(s) * 0.25, -(fz(s).abs() + 0.6)]);
    let delta: [f32; 3] = core::array::from_fn(|k| (mdir[k] as f64 * cd as f64) as f32);
    let center = [
        st.pos[0] + mdir[0] * cd * 0.5,
        st.pos[1] + mdir[1] * cd * 0.5,
        st.pos[2] + mdir[2] * cd * 0.5,
    ];
    let hits = vec![HitRecord {
        normal: [0.0, 0.0, 1.0],
        d: fz(s),
        basis: htri(center),
    }];
    let rows = vec![[fz(s) + 5.0, fz(s) - 5.0], [0.0, 0.0]];
    (st, delta, hits, rows)
}

/// A wall across the fall.
fn wall_case(
    s: &mut u32,
    init_base: &IntegState,
    height: &mut f32,
) -> (IntegState, [f32; 3], Vec<HitRecord>, Vec<[f32; 2]>) {
    let mut st = *init_base;
    st.pos = [fz(s), fz(s), fz(s)];
    st.radius = (fz(s).abs() * 0.4) + 0.35;
    *height = (fz(s).abs() * 0.5) + 0.7;
    st.f84 = fz(s).abs() + 0.2;
    st.fa0 = fz(s);
    st.f7c = st.pos[2] + fz(s).abs() * 4.0;
    st.f68 = fz(s) * 0.3;
    st.f6c = fz(s) * 0.3;
    let cd = (fz(s).abs()) + 1.0;
    let mdir = normalize([-(fz(s).abs() + 0.6), fz(s) * 0.2, fz(s) * 0.15]);
    let delta: [f32; 3] = core::array::from_fn(|k| (mdir[k] as f64 * cd as f64) as f32);
    let center = [
        st.pos[0] + mdir[0] * cd * 0.5,
        st.pos[1] + mdir[1] * cd * 0.5,
        st.pos[2] + mdir[2] * cd * 0.5,
    ];
    let hits = vec![HitRecord {
        normal: [1.0, 0.0, 0.0],
        d: fz(s),
        basis: vtri(center),
    }];
    let rows = vec![[fz(s) + 5.0, fz(s) - 5.0], [0.0, 0.0]];
    (st, delta, hits, rows)
}

/// A rising fall.
fn up_case(
    s: &mut u32,
    init_base: &IntegState,
    height: &mut f32,
) -> (IntegState, [f32; 3], Vec<HitRecord>, Vec<[f32; 2]>) {
    let mut st = *init_base;
    st.pos = [fz(s), fz(s), fz(s)];
    st.radius = (fz(s).abs() * 0.4) + 0.3;
    *height = (fz(s).abs() * 0.4) + 0.6;
    st.f84 = fz(s) * 0.5;
    st.fa0 = -(fz(s).abs() * 8.0);
    st.f7c = st.pos[2] + fz(s).abs() * 2.0;
    st.f68 = fz(s) * 0.4;
    st.f6c = -(fz(s).abs() * 0.4 + 0.1);

    let delta = [st.f68, st.f6c, f32::from_bits(0x8000_0000)];
    let hits = vec![HitRecord {
        normal: [0.0, 0.0, 1.0],
        d: fz(s),
        basis: htri([st.pos[0], st.pos[1], st.pos[2] - 0.5]),
    }];
    let rows = vec![[fz(s) + 5.0, fz(s) - 5.0], [0.0, 0.0]];
    (st, delta, hits, rows)
}

/// Falls onto floors and walls and rising falls.
#[test]
fn fall_integrator_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 1, 2);

    emu.stub(0x615ae0, 0, 1);
    emu.stub(0x7c6290, 0, 0);
    emu.stub(0x633220, 0, 0);
    emu.stub(0x61a800, 0, 1);
    emu.stub(0x6173b0, 0, 2);
    emu.stub(0x61ab20, 1, 0);
    emu.stub(0x7c5cd0, 0, 1);
    let vmax_land = emu.read_f32(0x87d894);
    let vmax_swim = emu.read_f32(0x87d898);
    let v = setup(&mut emu, [-1.0e6, -1.0e6, -1.0e6], [1.0e6, 1.0e6, 1.0e6]);

    let base = IntegState {
        flags: 0x2000,
        f58: 0,
        f68: 0.1,
        f6c: -0.2,
        f78: 0,
        f7c: 0.0,
        vmax_land,
        vmax_swim,
        ..Default::default()
    };

    let mut s = 0x6356_2026u32;
    let mut tally: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut checked = 0u32;
    let mut floor_eax_lt = 0u32;

    for case in 0..400u32 {
        let mut height = 1.0f32;
        let regime = case % 4;
        let (label, flag, init, delta, hits, rows) = match regime {
            3 => {
                let (i, d, h, r) = wall_case(&mut s, &base, &mut height);
                ("wall_slide", 1, i, d, h, r)
            }
            2 => {
                let (i, d, h, r) = up_case(&mut s, &base, &mut height);
                ("up_floor", 0, i, d, h, r)
            }
            _ => {
                let (i, d, h, r) = floor_case(&mut s, &base, &mut height);
                ("floor", 1, i, d, h, r)
            }
        };
        // `dt_ms` is an integer, so the `__ftol` on `dt_ms · 0.001 · 1000` is always in range:
        // NaN and out-of-range inputs reach only the drop-time conversion (`resolve_fall_land`).
        let dt_ms = 5 + (next(&mut s) % 200) as i32;
        *tally.entry(label).or_insert(0) += 1;
        let eax = run_integrator(
            &mut emu,
            &v,
            &init,
            height,
            0,
            dt_ms,
            delta,
            flag,
            1,
            &hits,
            &rows,
            &format!("R{regime} case {case}"),
        );
        if label == "floor" && eax < dt_ms as u32 {
            floor_eax_lt += 1;
        }
        checked += 1;
    }

    {
        let mut height = 1.0f32;
        let (init, delta, hits, rows) = floor_case(&mut s, &base, &mut height);
        run_integrator(
            &mut emu, &v, &init, height, 0, 80, delta, 0, 1, &hits, &rows, "P1 flag0",
        );
        *tally.entry("flag0").or_insert(0) += 1;
        checked += 1;
    }

    {
        let mut height = 1.0f32;
        let (mut init, delta, hits, rows) = floor_case(&mut s, &base, &mut height);
        init.flags = 0;
        run_integrator(
            &mut emu,
            &v,
            &init,
            height,
            0,
            80,
            delta,
            1,
            1,
            &hits,
            &rows,
            "P2 notfalling",
        );
        *tally.entry("notfalling").or_insert(0) += 1;
        checked += 1;
    }

    {
        let mut height = 1.0f32;
        let (mut init, delta, hits, rows) = floor_case(&mut s, &base, &mut height);
        init.flags = 0x2000 | 0x5;
        run_integrator(
            &mut emu,
            &v,
            &init,
            height,
            0,
            120,
            delta,
            1,
            1,
            &hits,
            &rows,
            "P3 restore",
        );
        *tally.entry("restore").or_insert(0) += 1;
        checked += 1;
    }

    eprintln!("fall_integrator: {checked} cases bit-exact vs WoW.exe");
    eprintln!("  branch/scenario tally:");
    for (k, n) in &tally {
        eprintln!("    {k:<14} {n}");
    }
    eprintln!("  floor cases with eax<dt (partial-consume, the recursion driver): {floor_eax_lt}");
    assert!(
        floor_eax_lt > 0,
        "no floor case partially consumed — the 0x635f80 multi-chunk driver would not loop"
    );
}

/// Falls that hit nothing.
#[test]
fn fall_integrator_freefly_damage_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 0, 2);
    emu.stub(0x615ae0, 0, 1);
    emu.stub(0x7c6290, 0, 0);
    emu.stub(0x633220, 0, 0);
    emu.stub(0x61a800, 0, 1);
    emu.stub(0x6173b0, 0, 2);
    emu.stub(0x61ab20, 1, 0);
    emu.stub(0x7c5cd0, 0, 1);
    let vmax_land = emu.read_f32(0x87d894);
    let vmax_swim = emu.read_f32(0x87d898);
    let v = setup(&mut emu, [-0.05, -0.05, -0.05], [0.05, 0.05, 0.05]);

    let base = IntegState {
        flags: 0x2000,
        f68: 0.1,
        f6c: -0.2,
        f7c: 0.0,
        vmax_land,
        vmax_swim,
        ..Default::default()
    };
    let mut s = 0x5eef_2026u32;
    let mut checked = 0u32;
    for case in 0..40u32 {
        let mut st = base;
        st.pos = [fz(&mut s) * 5.0, fz(&mut s) * 5.0, fz(&mut s) * 5.0];
        st.radius = (fz(&mut s).abs() * 0.4) + 0.35;
        st.f58 = (next(&mut s) % 500) as i32;
        st.f78 = (next(&mut s) % 500) as i32;
        st.fa0 = -(fz(&mut s).abs() * 8.0);
        st.f7c = st.pos[2] + fz(&mut s).abs() * 4.0;
        let height = (fz(&mut s).abs() * 0.5) + 0.7;
        let cd = (fz(&mut s).abs()) + 1.0;
        let mdir = normalize([
            fz(&mut s) * 0.3,
            fz(&mut s) * 0.3,
            -(fz(&mut s).abs() + 0.5),
        ]);
        let delta: [f32; 3] = core::array::from_fn(|k| (mdir[k] as f64 * cd as f64) as f32);
        let hits = vec![HitRecord {
            normal: [0.0, 0.0, 1.0],
            d: 0.1,
            basis: htri([st.pos[0], st.pos[1], st.pos[2] - 1.0]),
        }];
        let rows = vec![[1.0, 2.0], [0.0, 0.0]];
        // `dt_ms` is an integer, so the `__ftol` on `dt_ms · 0.001 · 1000` is always in range:
        // NaN and out-of-range inputs reach only the drop-time conversion (`resolve_fall_land`).
        let dt_ms = 5 + (next(&mut s) % 200) as i32;
        run_integrator(
            &mut emu,
            &v,
            &st,
            height,
            0,
            dt_ms,
            delta,
            1,
            1,
            &hits,
            &rows,
            &format!("freefly case {case}"),
        );
        checked += 1;
    }
    eprintln!("fall_integrator_freefly: {checked} damage-leg cases bit-exact vs WoW.exe");
}

/// `0x635f80`.
#[test]
fn resolve_fall_land_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 1, 2);
    emu.stub(0x615ae0, 0, 1);
    emu.stub(0x7c6290, 0, 0);
    emu.stub(0x633220, 0, 0);
    emu.stub(0x61a800, 0, 1);
    emu.stub(0x6173b0, 0, 2);
    emu.stub(0x61ab20, 0, 0);
    emu.stub(0x7c5cd0, 0, 1);
    emu.stub(0x7c61f0, 0, 1);
    emu.stub(0x7c6140, 0, 1);
    emu.stub(0x4078e0, 0, 1);
    let vmax_land = emu.read_f32(0x87d894);
    let vmax_swim = emu.read_f32(0x87d898);
    let v = setup(&mut emu, [-1.0e6, -1.0e6, -1.0e6], [1.0e6, 1.0e6, 1.0e6]);

    let fpz = emu.alloc(0x20);
    emu.write(fpz, &[0xd9u8, 0xee].repeat(8));

    let mut s = 0x635f_2026u32;
    let mut checked = 0u32;
    let mut accept = 0u32;
    let mut reject = 0u32;
    let mut flag_exit = 0u32;

    // Drop heights that make the time in milliseconds NaN, past `i32` and past `i64`, where
    // `__ftol` returns a low dword or 0.
    const DROPS: [f32; 6] = [f32::NAN, f32::INFINITY, 1.0e16, 1.0e30, 1.0e38, f32::MAX];
    for case in 0..200u32 + DROPS.len() as u32 {
        let mut st = IntegState {
            flags: if case % 7 == 0 { 0 } else { 0x2000 },
            f58: (next(&mut s) % 400) as i32,
            f68: fz(&mut s) * 0.4,
            f6c: -(fz(&mut s).abs() * 0.4 + 0.1),
            f78: (next(&mut s) % 300) as i32,
            f7c: 0.0,
            radius: (fz(&mut s).abs() * 0.4) + 0.3,
            fa0: -(fz(&mut s).abs() * 8.0),
            vmax_land,
            vmax_swim,
            ..Default::default()
        };
        st.pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        st.f7c = st.pos[2] + fz(&mut s).abs() * 2.0;
        let mut drop_h = fz(&mut s).abs() * 0.8 + 0.05;
        if let Some(&d) = DROPS.get((case as usize).wrapping_sub(200)) {
            drop_h = d;
        }
        let hits = vec![HitRecord {
            normal: [0.0, 0.0, 1.0],
            d: fz(&mut s),
            basis: htri([st.pos[0], st.pos[1], st.pos[2] - 0.05]),
        }];
        let rows = vec![[fz(&mut s) + 5.0, fz(&mut s) - 5.0], [0.0, 0.0]];
        let dir = [normalize([st.f68, st.f6c, 0.0])[0], st.f6c];
        let saved = [st.pos[0], st.pos[1]];

        plant_this(&mut emu, v.master, &st, 0.9);
        plant_this(&mut emu, v.sweep, &st, 0.9);
        plant_world(&mut emu, &v, &hits, &rows);
        emu.write_u32(v.dirvec, dir[0].to_bits());
        emu.write_u32(v.dirvec + 4, dir[1].to_bits());
        emu.write_u32(v.saved, saved[0].to_bits());
        emu.write_u32(v.saved + 4, saved[1].to_bits());
        emu.run(fpz, fpz + 16);
        let oeax = emu.call(
            0x635f80,
            v.master as u32,
            0,
            &[v.dirvec as u32, v.saved as u32, drop_h.to_bits()],
        );
        let o_out = read_outs(&emu, v.master);

        let mut rst = st;
        let seam = IntegSeam { not_falling_ret: 0 };
        let n_hits = hits.len() as u32;
        let reax = resolve_fall_land(
            &mut rst,
            &dir,
            &saved,
            drop_h,
            true,
            &seam,
            |pos, d, rem| do_sweep(&mut emu, &v, pos, d, rem, n_hits, &hits, &rows),
        );

        assert_eq!(
            oeax, reax,
            "fall_land case {case}: eax (bin={oeax} ours={reax})"
        );
        diff_state(&o_out, &rst, &format!("fall_land case {case}"));
        if reax == 1 {
            accept += 1;
        } else {
            reject += 1;
        }
        if st.flags & 0x2000 == 0 {
            flag_exit += 1;
        }
        checked += 1;
    }
    eprintln!(
        "resolve_fall_land: {checked} cases bit-exact vs WoW.exe (accept={accept} reject={reject} \
         flag_exit={flag_exit})"
    );
    assert!(accept > 0 && reject > 0, "fall_land verdict not both-sided");
    assert!(
        flag_exit > 0,
        "the FALLING-clear 0-chunk exit was not covered"
    );
}
