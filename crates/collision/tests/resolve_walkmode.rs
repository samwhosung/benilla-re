//! `CMovement::ResolveWalk` (`0x6367b0`) runs whole with `QueryWorldTriangles` (`0x6721b0`) stubbed
//! and the triangle list written by the test, and with `IsUnitPlayerDriven` (`0x5fa550`),
//! `CMovement::SetTransportFromSupport` (`0x6173b0`), `CMovement::UpdateAnchors` (`0x7c5cd0`),
//! `CMovement::GetFallDrop` (`0x7c6140`) and `CMovement::StartFalling2` (`0x7c61f0`) stubbed and
//! their calls recorded. The sweeps and `CMovement::ProbeFallAway` (`0x635f80`) run in the exe, and
//! the reimplementation is given their results.

use benilla_re_collision::resolve_clip::HitRecord;
use benilla_re_collision::resolve_multipass::SweepRes;
use benilla_re_collision::resolve_toolkit::Plane;
use benilla_re_collision::resolve_walkmode::{resolve_walk_mode, WalkSeam};
use benilla_re_difftest::{Emu, Wow};
use std::cell::RefCell;
use std::collections::BTreeMap;

const SET_TRANSPORT_FROM_SUPPORT: u64 = 0x6173b0;
const UPDATE_ANCHORS: u64 = 0x7c5cd0;

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
/// A cross product.
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A triangle around a point with a given normal.
fn tri(center: [f32; 3], normal: [f32; 3]) -> [[f32; 3]; 3] {
    let n = normalize(normal);
    let t1 = normalize(if n[0].abs() < 0.9 {
        cross(n, [1.0, 0.0, 0.0])
    } else {
        cross(n, [0.0, 1.0, 0.0])
    });
    let t2 = normalize(cross(n, t1));
    [
        core::array::from_fn(|k| center[k] + t1[k] * 5.0),
        core::array::from_fn(|k| center[k] - t1[k] * 4.0 + t2[k] * 5.0),
        core::array::from_fn(|k| center[k] - t1[k] * 4.0 - t2[k] * 5.0),
    ]
}

/// The addresses of the mover, the lists and the stubs' cells.
struct Vas {
    this: u64,
    recs: u64,
    svrows: u64,
    basepos: u64,
    dir: u64,
    mani: u64,
    idx: u64,
    cnt: u64,
    t: u64,
    faldir: u64,
    savedxy: u64,
    hcell: u64,
}

/// A sentinel for fields the function may leave alone.
const SENT: u32 = 0x7fc0_0001;

/// Maps the mover and the lists and installs the stubs.
fn setup(emu: &mut Emu) -> Vas {
    let this = emu.alloc(0x200);
    let unit = emu.alloc(0x1000);
    let desc = emu.alloc(0x20);
    let ptr2 = emu.alloc(0x20);
    let aux = emu.alloc(0x10);
    let recs = emu.alloc(0x1000);
    let svrows = emu.alloc(0x400);
    let basepos = emu.alloc(0x10);
    let dir = emu.alloc(0x10);
    let mani = emu.alloc(0x80);
    let idx = emu.alloc(8);
    let cnt = emu.alloc(8);
    let t = emu.alloc(8);
    let faldir = emu.alloc(0x10);
    let savedxy = emu.alloc(0x10);
    let hcell = emu.alloc(8);

    let mut patch = vec![0xd9u8, 0x05];
    patch.extend_from_slice(&(hcell as u32).to_le_bytes());
    patch.push(0xc3);
    emu.write(0x617430, &patch);

    emu.write_u32(this + 0x15c, unit as u32);
    emu.write_u32(unit + 8, desc as u32);
    emu.write_u32(unit + 0xe68, ptr2 as u32);
    emu.write_u32(desc + 8, 0x10);
    emu.write_u32(this + 0x30, aux as u32);
    emu.write_u32(this + 0x38, 0);
    emu.write_u32(this + 0x3c, 0);
    emu.write_u32(this + 0x20, 0);

    for k in 0..3u64 {
        emu.write_u32(0xc4e5a0 + k * 4, (-1.0e6f32).to_bits());
        emu.write_u32(0xc4e5ac + k * 4, (1.0e6f32).to_bits());
    }

    emu.write_u32(0xc4e544, svrows as u32);
    Vas {
        this,
        recs,
        svrows,
        basepos,
        dir,
        mani,
        idx,
        cnt,
        t,
        faldir,
        savedxy,
        hcell,
    }
}

/// Writes the mover and the world for one case.
#[allow(clippy::too_many_arguments)]
fn replant(
    emu: &mut Emu,
    v: &Vas,
    pos: [f32; 3],
    radius: f32,
    height: f32,
    flags: u32,
    h: f32,
    fa0: f32,
    f68: [f32; 2],
    swim_terminal: f32,
    hits: &[HitRecord],
    svrows: &[[f32; 2]],
) {
    for k in 0..3u64 {
        emu.write_u32(v.this + 0x10 + k * 4, pos[k as usize].to_bits());
    }
    emu.write_u32(v.this + 0x40, flags);
    emu.write_u32(v.this + 0x80, SENT);
    emu.write_u32(v.this + 0x84, fa0.to_bits());
    emu.write_u32(v.this + 0xa0, fa0.to_bits());
    emu.write_u32(v.this + 0xb0, radius.to_bits());
    emu.write_u32(v.this + 0xb4, height.to_bits());
    emu.write_u32(v.this + 0xb8, height.to_bits());
    emu.write_u32(v.this + 0x68, f68[0].to_bits());
    emu.write_u32(v.this + 0x6c, f68[1].to_bits());
    emu.write_u32(v.hcell, h.to_bits());
    emu.write_u32(0x87d898, swim_terminal.to_bits());

    for (i, hh) in hits.iter().enumerate() {
        let rv = v.recs + (i as u64) * 0x34;
        wv3(emu, rv, &hh.normal);
        emu.write_u32(rv + 0xc, hh.d.to_bits());
        for (r, vv) in hh.basis.iter().enumerate() {
            wv3(emu, rv + 0x10 + (r as u64) * 0xc, vv);
        }
    }
    let sentinel = v.recs + (hits.len() as u64) * 0x34;
    wv3(emu, sentinel, &[0.0, 0.0, 1.0]);
    emu.write_u32(sentinel + 0xc, 0.0f32.to_bits());
    for r in 0..3u64 {
        wv3(
            emu,
            sentinel + 0x10 + r * 0xc,
            &[pos[0], pos[1], pos[2] - 1.0],
        );
    }

    for (i, row) in svrows.iter().enumerate() {
        emu.write_u32(v.svrows + (i as u64) * 8, row[0].to_bits());
        emu.write_u32(v.svrows + (i as u64) * 8 + 4, row[1].to_bits());
    }
    emu.write_u32(0xc4e530, hits.len() as u32);
    emu.write_u32(0xc4e534, v.recs as u32);
    emu.write_u32(0xc4e544, v.svrows as u32);
    emu.write_u32(0xc4e568, 0);

    let aux = emu.read_u32(v.this + 0x30) as u64;
    emu.write_u32(aux, 0);
    emu.write_u32(aux + 4, 0);

    for i in 0..7u64 {
        emu.call(0x6376a0, (v.mani + i * 0x10) as u32, 0, &[]);
    }
}

/// The recorded calls as the reimplementation reports them.
fn seam_from_calls(emu: &Emu) -> (Option<[u32; 2]>, bool) {
    let mut facing = None;
    let mut begin = false;
    for c in emu.calls() {
        if c.addr == SET_TRANSPORT_FROM_SUPPORT {
            facing = Some([c.args[0], c.args[1]]);
        }
        if c.addr == UPDATE_ANCHORS {
            begin = true;
        }
    }
    (facing, begin)
}

/// Runs `0x6367b0` and the reimplementation on one case and compares them, recorded calls included.
#[allow(clippy::too_many_arguments)]
fn run_case(
    emu: &mut Emu,
    v: &Vas,
    pos: [f32; 3],
    radius: f32,
    height: f32,
    flags: u32,
    h: f32,
    fa0: f32,
    f68: [f32; 2],
    swim_terminal: f32,
    obj_pred: bool,
    budget: i32,
    dist: f32,
    dir: [f32; 2],
    hits: &[HitRecord],
    svrows: &[[f32; 2]],
    tally: &mut BTreeMap<String, u32>,
    ctx: &str,
) {
    let n_hits = hits.len() as u32;

    replant(
        emu,
        v,
        pos,
        radius,
        height,
        flags,
        h,
        fa0,
        f68,
        swim_terminal,
        hits,
        svrows,
    );
    wv3(emu, v.dir, &[dir[0], dir[1], 0.0]);
    emu.clear_calls();
    let oeax = emu.call(
        0x6367b0,
        v.this as u32,
        0,
        &[
            v.basepos as u32,
            budget as u32,
            dist.to_bits(),
            v.dir as u32,
        ],
    );
    let o_pos = rv3(emu, v.this + 0x10);
    let o_flags = emu.read_u32(v.this + 0x40);
    let o_floor = emu.read_f32(v.this + 0x80);
    let (o_facing, o_begin) = seam_from_calls(emu);

    replant(
        emu,
        v,
        pos,
        radius,
        height,
        flags,
        h,
        fa0,
        f68,
        swim_terminal,
        hits,
        svrows,
    );
    let mut r_pos = pos;
    let mut r_flags = flags;
    let mut r_floor = f32::from_bits(SENT);
    let mut seam = WalkSeam::default();
    let sweep_calls = std::cell::Cell::new(0u32);
    emu.clear_calls();

    let emu_cell = RefCell::new(&mut *emu);
    let reax = {
        let sweep = |dir: &[f32; 3], len: f32, p: &[f32; 3]| -> SweepRes {
            sweep_calls.set(sweep_calls.get() + 1);
            let mut e = emu_cell.borrow_mut();
            for k in 0..3u64 {
                e.write_u32(v.this + 0x10 + k * 4, p[k as usize].to_bits());
            }
            wv3(&mut e, v.dir, dir);
            e.write_u32(v.idx, 0xdead_beef);
            e.write_u32(v.cnt, 0);
            e.write_u32(v.t, SENT);
            let sret = e.call(
                0x632ba0,
                v.this as u32,
                0,
                &[
                    0xc4e52c,
                    v.dir as u32,
                    len.to_bits(),
                    v.idx as u32,
                    v.mani as u32,
                    v.cnt as u32,
                    v.t as u32,
                    0,
                ],
            );
            let mut manifold = [Plane::default(); 7];
            for (i, pl) in manifold.iter_mut().enumerate() {
                pl.n = rv3(&e, v.mani + (i as u64) * 0x10);
                pl.d = e.read_f32(v.mani + (i as u64) * 0x10 + 0xc);
            }
            SweepRes {
                contact: sret != 0,
                idx: e.read_u32(v.idx),
                t: e.read_f32(v.t),
                count: e.read_u32(v.cnt),
                manifold,
            }
        };
        let fall_land = |dir: &[f32; 2], saved_xy: &[f32; 2], drop: f32, p: &[f32; 3]| -> u32 {
            let mut e = emu_cell.borrow_mut();
            for k in 0..3u64 {
                e.write_u32(v.this + 0x10 + k * 4, p[k as usize].to_bits());
            }
            wv3(&mut e, v.faldir, &[dir[0], dir[1], 0.0]);
            e.write_u32(v.savedxy, saved_xy[0].to_bits());
            e.write_u32(v.savedxy + 4, saved_xy[1].to_bits());
            e.call(
                0x635f80,
                v.this as u32,
                0,
                &[v.faldir as u32, v.savedxy as u32, drop.to_bits()],
            ) & 0xff
        };
        resolve_walk_mode(
            &mut r_pos,
            radius,
            height,
            &mut r_flags,
            &mut r_floor,
            budget,
            dist,
            dir,
            h,
            obj_pred,
            swim_terminal,
            hits,
            n_hits,
            svrows,
            &mut seam,
            sweep,
            fall_land,
        )
    };

    assert_eq!(oeax, reax, "{ctx}: eax (binary={oeax} ours={reax})");
    for k in 0..3 {
        assert_eq!(
            o_pos[k].to_bits(),
            r_pos[k].to_bits(),
            "{ctx}: pos[{k}] binary={:#010x} ours={:#010x}",
            o_pos[k].to_bits(),
            r_pos[k].to_bits()
        );
    }
    assert_eq!(
        o_flags, r_flags,
        "{ctx}: flags binary={o_flags:#010x} ours={r_flags:#010x}"
    );
    assert_eq!(
        o_floor.to_bits(),
        r_floor.to_bits(),
        "{ctx}: floor binary={:#010x} ours={:#010x}",
        o_floor.to_bits(),
        r_floor.to_bits()
    );

    if flags & 0x4000_0000 == 0 {
        let r_facing = seam.set_facing.map(|s| [s[0].to_bits(), s[1].to_bits()]);
        assert_eq!(
            o_facing, r_facing,
            "{ctx}: SetFacing binary={o_facing:x?} ours={r_facing:x?}"
        );
        assert_eq!(
            o_begin, seam.begin_move,
            "{ctx}: BeginMove binary={o_begin} ours={}",
            seam.begin_move
        );
    }

    let label = format!(
        "sweeps={} eax={} water={} facing={}",
        sweep_calls.get(),
        if reax == budget as u32 { "B" } else { "0" },
        (flags & 0x4000_0000 != 0) as u32,
        seam.set_facing.is_some() as u32,
    );
    *tally.entry(label).or_insert(0) += 1;
}

/// The triangles of one regime around the mover.
#[allow(clippy::too_many_arguments)]
fn geometry(s: &mut u32, pos: [f32; 3], radius: f32, dir: [f32; 2], regime: u32) -> Vec<HitRecord> {
    let ch = fz(s).abs() * 0.5 + 0.2;
    let drop = fz(s).abs() * 0.5 + 0.2;

    let fwd_nz = match regime {
        2 => 0.9 + fz(s).abs() * 0.05,
        3 => 0.1 + fz(s).abs() * 0.2,
        4 => fz(s).abs() * 0.05,
        _ => fz(s).abs() * 0.95,
    };
    let fhxy = (1.0 - fwd_nz * fwd_nz).max(0.0).sqrt();
    let fang = (next(s) >> 8) as f32 / (1 << 24) as f32 * std::f32::consts::TAU;
    let fwd_n = [fhxy * fang.cos(), fhxy * fang.sin(), fwd_nz];
    let dn = normalize([dir[0], dir[1], 0.0]);
    vec![
        HitRecord {
            normal: [0.0, 0.0, -1.0],
            d: fz(s),
            basis: tri([pos[0], pos[1], pos[2] + ch], [0.0, 0.0, -1.0]),
        },
        HitRecord {
            normal: fwd_n,
            d: fz(s),
            basis: tri(
                [
                    pos[0] + dn[0] * (radius + 0.1),
                    pos[1] + dn[1] * (radius + 0.1),
                    pos[2] + ch * 0.5,
                ],
                fwd_n,
            ),
        },
        HitRecord {
            normal: [0.0, 0.0, 1.0],
            d: fz(s),
            basis: tri([pos[0], pos[1], pos[2] - drop], [0.0, 0.0, 1.0]),
        },
    ]
}

/// Random sweep results.
fn rand_svrows(s: &mut u32, n: usize) -> Vec<[f32; 2]> {
    (0..n).map(|_| [fz(s) * 0.5, fz(s) * 0.5]).collect()
}

/// Random cases across the regimes.
#[allow(clippy::too_many_arguments)]
fn fuzz(
    emu: &mut Emu,
    v: &Vas,
    obj_pred: bool,
    seed: u32,
    cases: u32,
    force_water: Option<bool>,
    tally: &mut BTreeMap<String, u32>,
    tag: &str,
) -> u32 {
    let mut s = seed;
    let mut checked = 0u32;
    for case in 0..cases {
        let regime = case % 5;
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = (fz(&mut s).abs() * 0.4) + 0.35;
        let height = (fz(&mut s).abs() * 0.5) + 0.7;
        let water = force_water.unwrap_or(next(&mut s) & 2 == 0);
        let grounded = next(&mut s) & 1 == 0;
        let mut flags = 0u32;
        if water {
            flags |= 0x4000_0000;
        }
        if grounded {
            flags |= 0x0400_0000;
        }
        let h = fz(&mut s).abs() * 0.8 + 0.3;
        let fa0 = fz(&mut s);
        let f68 = [fz(&mut s), fz(&mut s)];
        let swim_terminal = fz(&mut s);
        let budget = ((next(&mut s) % 200) + 1) as i32;
        let dist = fz(&mut s).abs() * 0.8 + 0.05;
        let dvec = normalize([fz(&mut s), fz(&mut s), 0.0]);
        let dir = [dvec[0], dvec[1]];
        let hits = geometry(&mut s, pos, radius, dir, regime);
        let svrows = rand_svrows(&mut s, hits.len());
        run_case(
            emu,
            v,
            pos,
            radius,
            height,
            flags,
            h,
            fa0,
            f68,
            swim_terminal,
            obj_pred,
            budget,
            dist,
            dir,
            &hits,
            &svrows,
            tally,
            &format!("{tag} R{regime} case {case}"),
        );
        checked += 1;
    }
    checked
}

/// Random moves across the regimes, with the cos 50° limit.
#[test]
fn walkmode_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 1, 2);
    emu.stub(0x7c61f0, 0, 1);
    emu.stub(0x7c6140, 0, 1);
    emu.stub(0x7c5cd0, 0, 1);
    emu.stub(0x6173b0, 0, 2);
    let v = setup(&mut emu);
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    let mut checked = fuzz(
        &mut emu,
        &v,
        true,
        0x6367_0001,
        800,
        Some(false),
        &mut tally,
        "F",
    );

    {
        let pos = [0.5f32, -0.3, 1.0];
        let hits = geometry(&mut 0x11u32, pos, 0.4, [1.0, 0.0], 0);
        let sv = vec![[0.0f32; 2]; hits.len()];
        run_case(
            &mut emu,
            &v,
            pos,
            0.4,
            0.8,
            0,
            0.5,
            0.0,
            [0.0, 0.0],
            0.0,
            true,
            50,
            1.0e-8,
            [1.0, 0.0],
            &hits,
            &sv,
            &mut tally,
            "P1 degenerate-dist",
        );
        checked += 1;
    }

    {
        let pos = [1.0f32, 2.0, 0.5];
        run_case(
            &mut emu,
            &v,
            pos,
            0.4,
            0.8,
            0,
            0.5,
            0.0,
            [0.0, 0.0],
            0.0,
            true,
            50,
            0.5,
            [1.0, 0.0],
            &[],
            &[],
            &mut tally,
            "P2 count0",
        );
        checked += 1;
    }

    eprintln!("walkmode: {checked} rounds bit-exact vs WoW.exe");
    eprintln!("walkmode branch coverage tally (sweeps / eax / water / facing):");
    for (k, n) in &tally {
        eprintln!("  {k:<34} {n}");
    }
    assert!(
        tally.keys().any(|k| k.contains("eax=B")),
        "fuzz failed to drive a budget-return leg"
    );
}

/// Moves with the cos 80° limit.
#[test]
fn walkmode_cos80_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 0, 0);
    emu.stub(0x6721b0, 1, 2);
    emu.stub(0x7c61f0, 0, 1);
    emu.stub(0x7c6140, 0, 1);
    emu.stub(0x7c5cd0, 0, 1);
    emu.stub(0x6173b0, 0, 2);
    let v = setup(&mut emu);
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    let checked = fuzz(
        &mut emu,
        &v,
        false,
        0x0c80_0001,
        300,
        Some(false),
        &mut tally,
        "C80",
    );
    eprintln!("walkmode_cos80: {checked} rounds bit-exact vs WoW.exe (obj_pred=false)");
    for (k, n) in &tally {
        eprintln!("  {k:<34} {n}");
    }
}

/// Sweeps that hit nothing.
#[test]
fn walkmode_sweep_miss_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 0, 2);
    emu.stub(0x7c61f0, 0, 1);
    emu.stub(0x7c6140, 0, 1);
    emu.stub(0x7c5cd0, 0, 1);
    emu.stub(0x6173b0, 0, 2);
    let v = setup(&mut emu);
    for k in 0..3u64 {
        emu.write_u32(0xc4e5a0 + k * 4, (-0.05f32).to_bits());
        emu.write_u32(0xc4e5ac + k * 4, (0.05f32).to_bits());
    }
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    let mut s = 0x5_0727u32;
    let mut checked = 0u32;
    for case in 0..120u32 {
        let pos = [fz(&mut s) * 5.0, fz(&mut s) * 5.0, fz(&mut s) * 5.0 + 20.0];
        let radius = (fz(&mut s).abs() * 0.4) + 0.35;
        let height = (fz(&mut s).abs() * 0.5) + 0.7;
        let water = case % 2 == 0;
        let flags = if water { 0x4000_0000 } else { 0 };
        let budget = ((next(&mut s) % 200) + 1) as i32;
        let dist = fz(&mut s).abs() * 0.8 + 0.05;
        let dvec = normalize([fz(&mut s), fz(&mut s), 0.0]);
        let dir = [dvec[0], dvec[1]];
        let hits = vec![HitRecord {
            normal: [0.0, 0.0, 1.0],
            d: 0.1,
            basis: tri([pos[0], pos[1], pos[2] - 0.5], [0.0, 0.0, 1.0]),
        }];
        let sv = vec![[fz(&mut s), fz(&mut s)]];
        run_case(
            &mut emu,
            &v,
            pos,
            radius,
            height,
            flags,
            fz(&mut s).abs() * 0.5 + 0.3,
            fz(&mut s),
            [fz(&mut s), fz(&mut s)],
            fz(&mut s),
            true,
            budget,
            dist,
            dir,
            &hits,
            &sv,
            &mut tally,
            &format!("miss case {case}"),
        );
        checked += 1;
    }
    eprintln!("walkmode_sweep_miss: {checked} rounds bit-exact vs WoW.exe");
    for (k, n) in &tally {
        eprintln!("  {k:<34} {n}");
    }
}
