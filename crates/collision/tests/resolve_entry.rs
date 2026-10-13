//! `CMovement::ResolveDisplacement` (`0x634040`) runs whole with `QueryWorldTriangles` (`0x6721b0`)
//! stubbed and the triangle list written by the test, and with the callees whose effects lie
//! outside the mover stubbed and their calls recorded and compared with the reimplementation's:
//! `CMovement::GetScaleRatio` (`0x617430`), `CMovement::GetFallDrop` (`0x7c6140`),
//! `IsUnitPlayerDriven` (`0x5fa550`), `CMovement::SkipTime` (`0x615ae0`),
//! `CMovement::BroadcastStateChange` (`0x61a820`), `CMovement::UpdateCurrentSpeed` (`0x7c5c20`),
//! `CMovement::RefreshGroundNormal` (`0x6332a0`), `CMovement::TransportToWorld` (`0x7c4930`),
//! `CMovement::StartFalling2` (`0x7c61f0`), `CMovement::UpdateAnchors` (`0x7c5cd0`) and
//! `CMovement::SetTransportFromSupport` (`0x6173b0`). The resolvers it dispatches to run in the
//! exe; the reimplementation is given the sweep results the exe computes.

#![allow(clippy::doc_lazy_continuation)]

use benilla_re_collision::resolve_clip::{earliest_contact, HitRecord, TraceWorld};
use benilla_re_collision::resolve_entry::{resolve_entry, Boundary, EntrySeam, EntryState};
use benilla_re_collision::resolve_integrator::{
    resolve_fall_land, IntegSeam, IntegState, SubstepSweep,
};
use benilla_re_collision::resolve_modes::TransportSweep;
use benilla_re_collision::resolve_multipass::SweepRes;
use benilla_re_collision::resolve_state::MaskSelectInput;
use benilla_re_collision::resolve_substep::SweepOut;
use benilla_re_collision::resolve_toolkit::{Aabb, Plane};
use benilla_re_difftest::{Emu, StubCall, Wow};

const QUERY_WORLD: u64 = 0x6721b0;
const GET_SCALE_RATIO: u64 = 0x617430;
const GET_FALL_DROP: u64 = 0x7c6140;
const IS_PLAYER_DRIVEN: u64 = 0x5fa550;
const SKIP_TIME: u64 = 0x615ae0;
const BROADCAST_STATE_CHANGE: u64 = 0x61a820;
const UPDATE_CURRENT_SPEED: u64 = 0x7c5c20;
const REFRESH_GROUND_NORMAL: u64 = 0x6332a0;
const TRANSPORT_TO_WORLD: u64 = 0x7c4930;
const START_FALLING2: u64 = 0x7c61f0;
const UPDATE_ANCHORS: u64 = 0x7c5cd0;
const SET_TRANSPORT_FROM_SUPPORT: u64 = 0x6173b0;

/// Writes three `f32`.
fn wv3(emu: &mut Emu, va: u64, v: &[f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, c.to_bits());
    }
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

/// An upright triangle around a point.
fn vtri(c: [f32; 3]) -> [[f32; 3]; 3] {
    [
        [c[0], c[1] + 3.5, c[2]],
        [c[0], c[1] - 2.5, c[2] + 3.5],
        [c[0], c[1] - 2.5, c[2] - 3.5],
    ]
}

/// A floor and a face ahead of the mover.
fn walk_geom(pos: [f32; 3], radius: f32, dir: [f32; 2], fwd_nz: f32) -> Vec<HitRecord> {
    let ch = 0.4f32;
    let drop = 0.4f32;
    let dn = normalize([dir[0], dir[1], 0.0]);
    let fhxy = (1.0 - fwd_nz * fwd_nz).max(0.0).sqrt();

    let fwd_n = [-dn[0] * fhxy, -dn[1] * fhxy, fwd_nz];
    vec![
        HitRecord {
            normal: [0.0, 0.0, -1.0],
            d: 0.1,
            basis: tri([pos[0], pos[1], pos[2] + ch], [0.0, 0.0, -1.0]),
        },
        HitRecord {
            normal: fwd_n,
            d: 0.1,
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
            d: 0.1,
            basis: tri([pos[0], pos[1], pos[2] - drop], [0.0, 0.0, 1.0]),
        },
    ]
}

/// One case: the mover, the move and the world.
#[derive(Clone)]
struct Case {
    name: &'static str,
    pos: [f32; 3],
    flags: u32,
    radius: f32,
    height: f32,
    floor: f32,
    f58: i32,
    f78: i32,
    f7c: f32,
    f84: f32,
    fa0: f32,
    support: [f32; 3],
    support_pt: [f32; 3],
    base_pos: i32,
    budget: i32,
    disp: [f32; 3],
    coll_height: f32,
    fall_dist: f32,
    obj_pred: bool,
    swim_terminal: f32,
    hits: Vec<HitRecord>,
    rows: Vec<[f32; 2]>,
}

/// The mover's fields after a run.
#[derive(Clone, Debug, PartialEq)]
struct Outs {
    eax: u32,
    pos: [u32; 3],
    flags: u32,
    f58: u32,
    f78: u32,
    f84: u32,
    floor: u32,
    support_pt: [u32; 3],
}

/// The addresses of the mover, the lists and the stubs' cells.
struct Vas {
    this: u64,
    unit: u64,
    desc: u64,
    ptr2: u64,
    aux: u64,
    recs: u64,
    rows: u64,
    disp: u64,
    hcell: u64,
    fcell: u64,
    pdir: u64,
    pmani: u64,
    pidx: u64,
    pcnt: u64,
    pt: u64,
    ftab: [u8; 0x20],
}

/// An emulator with the mover, the lists and the stubs.
fn setup(al: u32) -> (Emu<'static>, Vas) {
    let wow: &'static Wow = Box::leak(Box::new(Wow::load().expect("WoW.exe")));
    let mut emu = Emu::new(wow);

    emu.stub(QUERY_WORLD, al, 2);
    emu.stub(IS_PLAYER_DRIVEN, 0, 0);

    emu.stub(SKIP_TIME, 0, 1);
    emu.stub(BROADCAST_STATE_CHANGE, 0, 5);
    emu.stub(UPDATE_CURRENT_SPEED, 0, 1);
    emu.stub(REFRESH_GROUND_NORMAL, 0, 0);
    emu.stub(TRANSPORT_TO_WORLD, 0, 2);

    emu.stub(START_FALLING2, 0, 1);
    emu.stub(UPDATE_ANCHORS, 0, 1);
    emu.stub(SET_TRANSPORT_FROM_SUPPORT, 0, 2);

    let hcell = emu.alloc(8);
    let fcell = emu.alloc(8);

    let mut p = vec![0xd9u8, 0x05];
    p.extend_from_slice(&(hcell as u32).to_le_bytes());
    p.push(0xc3);
    emu.write(GET_SCALE_RATIO, &p);

    let mut p = vec![0xd9u8, 0x05];
    p.extend_from_slice(&(fcell as u32).to_le_bytes());
    p.extend_from_slice(&[0xc2, 0x04, 0x00]);
    emu.write(GET_FALL_DROP, &p);

    let this = emu.alloc(0x200);
    let unit = emu.alloc(0x1000);
    let desc = emu.alloc(0x20);
    let ptr2 = emu.alloc(0x20);
    let aux = emu.alloc(0x10);
    let recs = emu.alloc(0x800);
    let lrecs = emu.alloc(0x200);
    let rows = emu.alloc(0x200);
    let disp = emu.alloc(0x10);
    let pdir = emu.alloc(0x10);
    let pmani = emu.alloc(0x80);
    let pidx = emu.alloc(8);
    let pcnt = emu.alloc(8);
    let pt = emu.alloc(8);

    emu.write_u32(this + 0x15c, unit as u32);
    emu.write_u32(unit + 8, desc as u32);
    emu.write_u32(unit + 0xe68, ptr2 as u32);
    emu.write_u32(ptr2 + 8, 0);
    emu.write_u32(desc + 8, 0x10);
    emu.write_u32(this + 0x30, aux as u32);
    emu.write_u32(aux, 0);
    emu.write_u32(aux + 4, 0);

    emu.write_u32(0xc4e544, rows as u32);

    emu.write_u32(0xc4e56c, lrecs as u32);

    let ftab = capture_ftab(&mut emu, this);
    (
        emu,
        Vas {
            this,
            unit,
            desc,
            ptr2,
            aux,
            recs,
            rows,
            disp,
            hcell,
            fcell,
            pdir,
            pmani,
            pidx,
            pcnt,
            pt,
            ftab,
        },
    )
}

/// Runs `CMovement::SweepVolume` (`0x632ba0`) in the exe for the reimplementation's sweep.
fn probe_sweep(
    emu: &mut Emu,
    v: &Vas,
    fx: &Case,
    pos: &[f32; 3],
    dir: &[f32; 3],
    len: f32,
) -> (bool, u32, u32, f32) {
    wv3(emu, v.this + 0x10, pos);
    emu.write_u32(v.this + 0x40, fx.flags);
    emu.write_u32(v.this + 0xb0, fx.radius.to_bits());
    emu.write_u32(v.this + 0xb4, fx.height.to_bits());
    emu.write_u32(v.this + 0x38, 0);
    emu.write_u32(v.this + 0x3c, 0);
    emu.write_u32(v.this + 0x20, 0);
    wv3(emu, v.pdir, dir);
    for i in 0..7u64 {
        emu.call(0x6376a0, (v.pmani + i * 0x10) as u32, 0, &[]);
    }
    emu.write_u32(v.pidx, 0xdead_beef);
    emu.write_u32(v.pcnt, 0);
    emu.write_u32(v.pt, 0x7fc0_0001);
    let sret = emu.call(
        0x632ba0,
        v.this as u32,
        0,
        &[
            0xc4e52c,
            v.pdir as u32,
            len.to_bits(),
            v.pidx as u32,
            v.pmani as u32,
            v.pcnt as u32,
            v.pt as u32,
            0,
        ],
    );
    let idx = emu.read_u32(v.pidx);
    let hit_count = emu.read_u32(0xc4e530);
    let nz = if idx < hit_count {
        emu.read_f32(emu.read_u32(0xc4e534) as u64 + (idx as u64) * 0x34 + 8)
    } else {
        f32::NAN
    };
    (sret != 0, idx, hit_count, nz)
}

/// The first substep's displacement and length.
fn first_substep(disp: &[f32; 3], flatten_z: bool) -> ([f32; 3], f32) {
    let len = ((f64::from(disp[0]) * f64::from(disp[0]) + f64::from(disp[1]) * f64::from(disp[1]))
        + f64::from(disp[2]) * f64::from(disp[2]))
    .sqrt();
    let inv = 1.0 / len;
    let mut unit = [
        (inv * f64::from(disp[0])) as f32,
        (inv * f64::from(disp[1])) as f32,
        (inv * f64::from(disp[2])) as f32,
    ];
    if flatten_z {
        unit[2] = 0.0;
    }
    (unit, len as f32)
}

/// Reads the face table `CMovement::BuildVolume` (`0x631be0`) writes.
fn capture_ftab(emu: &mut Emu, this: u64) -> [u8; 0x20] {
    let pl = emu.alloc(0xa0);
    let vv = emu.alloc(0x80);
    let pp = emu.alloc(0x10);
    let ft = emu.alloc(0x80);
    let grab = |emu: &mut Emu, pos: &[f32; 3], radius: f32, height: f32| -> [u8; 0x20] {
        emu.write_u32(this + 0xb0, radius.to_bits());
        wv3(emu, pp, pos);
        emu.call(
            0x631be0,
            this as u32,
            0,
            &[pp as u32, height.to_bits(), pl as u32, vv as u32, ft as u32],
        );
        let mut t = [0u8; 0x20];
        for (i, b) in t.iter_mut().enumerate() {
            *b = (emu.read_u32(ft + (i as u64 & !3)) >> ((i % 4) * 8)) as u8;
        }
        t
    };
    let a = grab(emu, &[1.0, 2.0, 3.0], 0.5, 2.0);
    let b = grab(emu, &[-7.0, 4.0, 11.0], 0.83, 1.3);
    assert_eq!(
        a, b,
        "0x631be0 face-index table is not position/size-constant"
    );

    assert_eq!(
        a,
        benilla_re_collision::resolve_clip::PRISM_FACE_TABLE,
        "PRISM_FACE_TABLE const drifted from 0x631be0's captured table"
    );
    a
}

/// Runs `0x634040` and returns the mover and the recorded calls.
fn run_oracle(emu: &mut Emu, v: &Vas, fx: &Case) -> (Outs, Vec<StubCall>) {
    let _ = (v.unit, v.desc, v.ptr2, v.aux);

    wv3(emu, v.this + 0x10, &fx.pos);
    emu.write_u32(v.this + 0x40, fx.flags);
    emu.write_u32(v.this + 0xb0, fx.radius.to_bits());
    emu.write_u32(v.this + 0xb4, fx.height.to_bits());
    emu.write_u32(v.this + 0xb8, fx.height.to_bits());
    emu.write_u32(v.this + 0x80, fx.floor.to_bits());
    emu.write_u32(v.this + 0x58, fx.f58 as u32);
    emu.write_u32(v.this + 0x78, fx.f78 as u32);
    emu.write_u32(v.this + 0x7c, fx.f7c.to_bits());
    emu.write_u32(v.this + 0x84, fx.f84.to_bits());
    emu.write_u32(v.this + 0xa0, fx.fa0.to_bits());
    wv3(emu, v.this + 0x5c, &fx.support);
    wv3(emu, v.this + 0x24, &fx.support_pt);
    emu.write_u32(v.this + 0x38, 0);
    emu.write_u32(v.this + 0x3c, 0);
    emu.write_u32(v.this + 0xa4, 0);
    emu.write_u32(v.this + 0x20, 0);
    emu.write_u32(v.this + 0xc, 0);

    emu.write_u32(v.hcell, fx.coll_height.to_bits());
    emu.write_u32(v.fcell, fx.fall_dist.to_bits());

    emu.write_u32(0x87d898, fx.swim_terminal.to_bits());

    for (i, h) in fx.hits.iter().enumerate() {
        let rv = v.recs + (i as u64) * 0x34;
        wv3(emu, rv, &h.normal);
        emu.write_u32(rv + 0xc, h.d.to_bits());
        for (r, vv) in h.basis.iter().enumerate() {
            wv3(emu, rv + 0x10 + (r as u64) * 0xc, vv);
        }
    }
    emu.write_u32(0xc4e530, fx.hits.len() as u32);
    emu.write_u32(0xc4e534, v.recs as u32);
    emu.write_u32(0xc4e568, 0);

    for (i, row) in fx.rows.iter().enumerate() {
        emu.write_u32(v.rows + (i as u64) * 8, row[0].to_bits());
        emu.write_u32(v.rows + (i as u64) * 8 + 4, row[1].to_bits());
    }

    for k in 0..3u64 {
        emu.write_u32(0xc4e5a0 + k * 4, (-1.0e6f32).to_bits());
        emu.write_u32(0xc4e5ac + k * 4, (1.0e6f32).to_bits());
    }

    wv3(emu, v.disp, &fx.disp);

    emu.clear_calls();
    let eax = emu.call(
        0x634040,
        v.this as u32,
        0,
        &[fx.base_pos as u32, fx.budget as u32, v.disp as u32],
    );
    let calls = emu.calls().to_vec();
    let outs = Outs {
        eax,
        pos: core::array::from_fn(|k| emu.read_u32(v.this + 0x10 + (k as u64) * 4)),
        flags: emu.read_u32(v.this + 0x40),
        f58: emu.read_u32(v.this + 0x58),
        f78: emu.read_u32(v.this + 0x78),
        f84: emu.read_u32(v.this + 0x84),
        floor: emu.read_u32(v.this + 0x80),
        support_pt: core::array::from_fn(|k| emu.read_u32(v.this + 0x24 + (k as u64) * 4)),
    };
    (outs, calls)
}

/// Runs the reimplementation and returns the mover and its calls.
fn run_rust(fx: &Case, trace_hit: bool, ftab: &[u8; 0x20]) -> (Outs, EntrySeam) {
    let mut st = EntryState {
        integ: IntegState {
            pos: fx.pos,
            flags: fx.flags,
            f58: fx.f58,
            support: fx.support,
            f68: 0.0,
            f6c: 0.0,
            f78: fx.f78,
            f7c: fx.f7c,
            f84: fx.f84,
            fa0: fx.fa0,
            radius: fx.radius,
            vmax_land: 0.0,
            vmax_swim: 0.0,
        },
        height: fx.height,
        floor: fx.floor,
        guid_lo: 0,
        guid_hi: 0,
        support_pt: fx.support_pt,
        transport_obj_flags: None,
    };

    let radius = fx.radius;
    let height = fx.height;
    let flags0 = fx.flags;
    let hits = fx.hits.clone();
    let rows = fx.rows.clone();
    let hit_count = fx.hits.len() as u32;
    let mask = MaskSelectInput {
        flags: flags0,
        pitch: 0.0,
        pred: fx.obj_pred,
        unit_flag1: false,
        unit_flag2: false,
    };

    let do_sweep = |pos: &[f32; 3],
                    dir: &[f32; 3],
                    len: f32,
                    liquid_list: bool|
     -> (bool, f32, u32, [Plane; 5], u32) {
        let mut world = TraceWorld {
            cache_box: Aabb {
                min: [-1.0e6; 3],
                max: [1.0e6; 3],
            },
            solid: vec![],
            liquid: vec![],
            trace_ok: trace_hit,
        };
        let mut manifold = [Plane::default(); 5];
        let mut count = 0u32;
        let mut out_t = f32::from_bits(0x7fc0_0001);
        let mut rec_idx = hit_count;
        let contact = earliest_contact(
            pos,
            radius,
            height,
            flags0,
            liquid_list,
            dir,
            len,
            &mask,
            &hits,
            &mut world,
            ftab,
            &mut manifold,
            &mut count,
            &mut out_t,
            &mut rec_idx,
        );
        (contact, out_t, rec_idx, manifold, count)
    };

    let m7 = |m5: &[Plane; 5]| -> [Plane; 7] {
        let mut m = [Plane::default(); 7];
        m[..5].copy_from_slice(m5);
        m
    };

    let substep = |pos: &[f32; 3], dir: &[f32; 3], len: f32| -> SubstepSweep {
        let (contact, t, idx, m5, count) = do_sweep(pos, dir, len, false);
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
                count,
                hit_count,
            },
            manifold: m7(&m5),
            hit,
            row,
        }
    };

    let walk_sweep = |dir: &[f32; 3], len: f32, pos: &[f32; 3]| -> SweepRes {
        let (contact, t, idx, m5, count) = do_sweep(pos, dir, len, false);
        SweepRes {
            contact,
            idx,
            t,
            count,
            manifold: m7(&m5),
        }
    };

    let walk_fall_land = |d: &[f32; 2], saved: &[f32; 2], drop: f32, p: &[f32; 3]| -> u32 {
        let mut fst = IntegState {
            pos: *p,
            flags: flags0 & !0x2000,
            radius,
            ..Default::default()
        };
        let iseam = IntegSeam::default();
        resolve_fall_land(&mut fst, d, saved, drop, fx.obj_pred, &iseam, &substep)
    };
    let fall_sweep = |p: &[f32; 3], d: &[f32; 3], l: f32| -> SubstepSweep { substep(p, d, l) };
    let transport_sweep =
        |pos: &[f32; 3], dir: &[f32; 3], len: f32, liquid: bool| -> TransportSweep {
            let (contact, t, idx, _m5, _count) = do_sweep(pos, dir, len, liquid);
            let hit_normal = if (idx as usize) < hits.len() {
                hits[idx as usize].normal
            } else {
                [0.0; 3]
            };
            TransportSweep {
                contact,
                idx,
                t,
                hit_normal,
            }
        };

    let b = Boundary {
        trace_hit,
        coll_height: fx.coll_height,
        fall_dist: fx.fall_dist,
        obj_pred: fx.obj_pred,
        swim_terminal: fx.swim_terminal,
        transport: None,
        hit_count,
        liquid_count: 0,
        slide_vel_rows: &fx.rows,
        feas_this_c: 0,
        feas_pred: false,
    };
    let mut main_recs = fx.hits.clone();
    let mut liquid_recs: Vec<HitRecord> = vec![];
    let mut seam = EntrySeam::default();
    let eax = resolve_entry(
        &mut st,
        fx.base_pos,
        fx.budget,
        &fx.disp,
        &b,
        &mut main_recs,
        &mut liquid_recs,
        &mut seam,
        walk_sweep,
        walk_fall_land,
        fall_sweep,
        transport_sweep,
    );

    let outs = Outs {
        eax,
        pos: core::array::from_fn(|k| st.integ.pos[k].to_bits()),
        flags: st.integ.flags,
        f58: st.integ.f58 as u32,
        f78: st.integ.f78 as u32,
        f84: st.integ.f84.to_bits(),
        floor: st.floor.to_bits(),
        support_pt: core::array::from_fn(|k| st.support_pt[k].to_bits()),
    };
    (outs, seam)
}

/// Compares the recorded calls with the reimplementation's.
fn diff_seam(calls: &[StubCall], seam: &EntrySeam, ctx: &str, transport: bool) {
    let of = |a: u64| calls.iter().filter(|c| c.addr == a).count();

    let fa: Vec<i32> = calls
        .iter()
        .filter(|c| c.addr == SKIP_TIME)
        .map(|c| c.args[0] as i32)
        .collect();
    assert_eq!(
        fa, seam.free_advance,
        "{ctx}: free-advance(0x615ae0) sequence"
    );

    let bc: Vec<[u32; 5]> = calls
        .iter()
        .filter(|c| c.addr == BROADCAST_STATE_CHANGE)
        .map(|c| [c.args[0], c.args[1], c.args[2], c.args[3], c.args[4]])
        .collect();
    assert_eq!(
        bc, seam.broadcast,
        "{ctx}: broadcast(0x61a820) sequence+args"
    );

    assert_eq!(
        of(UPDATE_CURRENT_SPEED) as u32,
        seam.set_active_speed,
        "{ctx}: set-active-speed(0x7c5c20) count"
    );

    assert_eq!(
        of(REFRESH_GROUND_NORMAL) > 0,
        seam.support_update,
        "{ctx}: support(0x6332a0)"
    );
    assert_eq!(
        of(TRANSPORT_TO_WORLD) > 0,
        seam.commit,
        "{ctx}: commit(0x7c4930)"
    );

    if transport {
        return;
    }
    assert_eq!(
        of(START_FALLING2) > 0,
        seam.walk.start_falling,
        "{ctx}: StartFalling(0x7c61f0)"
    );
    assert_eq!(
        of(UPDATE_ANCHORS) > 0,
        seam.walk.begin_move,
        "{ctx}: BeginMove(0x7c5cd0)"
    );
    let osf = calls.iter().find(|c| c.addr == SET_TRANSPORT_FROM_SUPPORT);
    let rsf = seam.walk.set_facing;
    match (osf, rsf) {
        (Some(c), Some(s)) => {
            assert_eq!(
                [c.args[0], c.args[1]],
                [s[0].to_bits(), s[1].to_bits()],
                "{ctx}: SetFacing(0x6173b0) args"
            );
        }
        (None, None) => {}
        _ => panic!("{ctx}: SetFacing fired mismatch (oracle={osf:?} rust={rsf:?})"),
    }
}

/// Compares two movers' fields.
fn diff(o: &Outs, r: &Outs, ctx: &str) {
    assert_eq!(o.eax, r.eax, "{ctx}: eax");
    for k in 0..3 {
        assert_eq!(
            o.pos[k], r.pos[k],
            "{ctx}: pos[{k}] binary={:#010x} ours={:#010x}",
            o.pos[k], r.pos[k]
        );
    }
    assert_eq!(o.flags, r.flags, "{ctx}: flags");
    assert_eq!(o.f58, r.f58, "{ctx}: +0x58");
    assert_eq!(o.f78, r.f78, "{ctx}: +0x78");
    assert_eq!(o.f84, r.f84, "{ctx}: +0x84");
    assert_eq!(o.floor, r.floor, "{ctx}: +0x80 floor");
    for k in 0..3 {
        assert_eq!(
            o.support_pt[k], r.support_pt[k],
            "{ctx}: support_pt[{k}] binary={:#010x} ours={:#010x}",
            o.support_pt[k], r.support_pt[k]
        );
    }
}

/// Triangles too far to touch.
fn far_hits(n: usize) -> Vec<HitRecord> {
    (0..n)
        .map(|i| {
            let o = 700.0 + i as f32 * 13.0;
            HitRecord {
                normal: [0.0, 0.0, 1.0],
                d: -o,
                basis: [[o, o, o], [o + 4.0, o, o], [o, o + 4.0, o]],
            }
        })
        .collect()
}

/// Fixed walking, falling, water and transport cases, and random ones.
#[test]
fn resolve_entry_difftest() {
    let Some(_probe) = Wow::load() else {
        return;
    };

    let mut tally: std::collections::BTreeMap<&'static str, u32> =
        std::collections::BTreeMap::new();
    let mut checked = 0u32;

    {
        let (mut emu, v) = setup(1);
        let cases = vec![
            Case {
                name: "fall-freefall",
                pos: [1.0, 2.0, 30.0],
                flags: 0x2000,
                radius: 0.5,
                height: 2.0,
                floor: 0.0,
                f58: 0,
                f78: 100,
                f7c: 40.0,
                f84: 4.5,
                fa0: 3.0,
                support: [0.0, 0.0, 1.0],
                support_pt: [9.0, 9.0, 9.0],
                base_pos: 0,
                budget: 50,
                disp: [0.3, 0.4, -0.5],
                coll_height: 1.2,
                fall_dist: 0.8,
                obj_pred: false,
                swim_terminal: 7.0,
                hits: vec![],
                rows: vec![],
            },
            Case {
                name: "fall-degenerate",
                pos: [1.0, 2.0, 30.0],
                flags: 0x2000,
                radius: 0.5,
                height: 2.0,
                floor: 0.0,
                f58: 0,
                f78: 100,
                f7c: 40.0,
                f84: 4.5,
                fa0: 0.0,
                support: [0.0, 0.0, 1.0],
                support_pt: [9.0, 9.0, 9.0],
                base_pos: 0,
                budget: 50,
                disp: [1.0e-9, 0.0, 0.0],
                coll_height: 1.2,
                fall_dist: 0.0,
                obj_pred: false,
                swim_terminal: 7.0,
                hits: vec![],
                rows: vec![],
            },
            Case {
                name: "walk-nonblocking",
                pos: [0.0, 0.0, 5.0],
                flags: 0x1,
                radius: 0.5,
                height: 2.0,
                floor: 0.0,
                f58: 0,
                f78: 0,
                f7c: 0.0,
                f84: 4.5,
                fa0: 0.0,
                support: [0.0, 0.0, 1.0],
                support_pt: [9.0, 9.0, 9.0],
                base_pos: 0,
                budget: 100,
                disp: [0.4, 0.3, 0.0],
                coll_height: 1.2,
                fall_dist: 0.0,
                obj_pred: false,
                swim_terminal: 7.0,
                hits: far_hits(1),
                rows: vec![[0.0, 0.0]],
            },
            Case {
                name: "water-walk-nonblocking",
                pos: [0.0, 0.0, 5.0],
                flags: 0x1 | 0x4000_0000,
                radius: 0.5,
                height: 2.0,
                floor: 0.0,
                f58: 0,
                f78: 0,
                f7c: 0.0,
                f84: 4.5,
                fa0: 0.0,
                support: [0.0, 0.0, 1.0],
                support_pt: [9.0, 9.0, 9.0],
                base_pos: 0,
                budget: 100,
                disp: [0.4, 0.3, 0.0],
                coll_height: 1.2,
                fall_dist: 0.0,
                obj_pred: false,
                swim_terminal: 7.0,
                hits: far_hits(1),
                rows: vec![[0.0, 0.0]],
            },
            Case {
                name: "transport-noblock",
                pos: [0.0, 0.0, 5.0],
                flags: 0x1 | 0x0020_0800,
                radius: 0.5,
                height: 2.0,
                floor: 0.0,
                f58: 0,
                f78: 0,
                f7c: 0.0,
                f84: 4.5,
                fa0: 0.0,
                support: [0.0, 0.0, 1.0],
                support_pt: [9.0, 9.0, 9.0],
                base_pos: 0,
                budget: 100,
                disp: [0.4, 0.3, 0.0],
                coll_height: 1.2,
                fall_dist: 0.0,
                obj_pred: false,
                swim_terminal: 7.0,
                hits: far_hits(1),
                rows: vec![[0.0, 0.0]],
            },
            Case {
                name: "water-surface-snap",
                pos: [0.0, 0.0, 5.0],
                flags: 0x1 | 0x4000_0000,
                radius: 0.5,
                height: 2.0,
                floor: 0.0,
                f58: 0,
                f78: 0,
                f7c: 0.0,
                f84: 4.5,
                fa0: 0.0,
                support: [0.0, 0.0, 1.0],
                support_pt: [9.0, 9.0, 9.0],
                base_pos: 0,
                budget: 100,
                disp: [0.8, 0.6, 0.0],
                coll_height: 1.2,
                fall_dist: 0.0,
                obj_pred: false,
                swim_terminal: 7.0,

                hits: vec![HitRecord {
                    normal: [0.0, 0.0, 1.0],
                    d: 0.1,
                    basis: tri([0.0, 0.0, 5.0 - 0.6], [0.0, 0.0, 1.0]),
                }],
                rows: vec![[0.21, -0.13]],
            },
        ];
        for fx in &cases {
            let (o, calls) = run_oracle(&mut emu, &v, fx);
            let (r, seam) = run_rust(fx, true, &v.ftab);
            diff(&o, &r, fx.name);
            diff_seam(&calls, &seam, fx.name, fx.flags & 0x0020_0800 != 0);
            *tally.entry(fx.name).or_default() += 1;
            checked += 1;
        }
    }

    {
        let (mut emu, v) = setup(1);

        struct C {
            fx: Case,
            leg: &'static str,
            probe_dir: [f32; 3],
            probe_len: f32,
            steep: bool,
        }
        let base = |name, flags, pos: [f32; 3], disp: [f32; 3], hits, rows| Case {
            name,
            pos,
            flags,
            radius: 0.5,
            height: 2.0,
            floor: 0.0,
            f58: 0,
            f78: 0,
            f7c: 0.0,
            f84: 4.5,
            fa0: 0.0,
            support: [0.0, 0.0, 1.0],
            support_pt: [9.0, 9.0, 9.0],
            base_pos: 0,
            budget: 100,
            disp,
            coll_height: 1.2,
            fall_dist: 0.0,
            obj_pred: false,
            swim_terminal: 7.0,
            hits,
            rows,
        };
        let row3 = || vec![[0.21f32, -0.13], [0.07, 0.30], [-0.18, 0.05]];

        let p1 = [0.0f32, 0.0, 5.0];
        let d1 = [0.7f32, 0.0, 0.0];
        let (u1, l1) = first_substep(&d1, true);

        let p2 = [3.0f32, 0.0, 5.0];
        let d2 = [0.7f32, 0.0, 0.0];
        let (u2, l2) = first_substep(&d2, true);

        let p3 = [0.0f32, 8.0, 5.0];
        let d3 = [-0.8f32, 0.05, 0.0];
        let (u3, l3) = first_substep(&d3, false);
        let wall3 = {
            let dirn = u3;
            let center = [
                p3[0] + dirn[0] * l3 * 0.5,
                p3[1] + dirn[1] * l3 * 0.5,
                p3[2] + dirn[2] * l3 * 0.5,
            ];
            vec![HitRecord {
                normal: [1.0, 0.0, 0.0],
                d: 0.1,
                basis: vtri(center),
            }]
        };

        let cases = vec![
            C {
                fx: base(
                    "walk-walkable-commit",
                    0x1,
                    p1,
                    d1,
                    walk_geom(p1, 0.5, [u1[0], u1[1]], 0.9),
                    row3(),
                ),
                leg: "1 WALK walkable",
                probe_dir: [u1[0], u1[1], 0.0],
                probe_len: l1,
                steep: false,
            },
            C {
                fx: base(
                    "walk-nonwalkable-multipass",
                    0x1,
                    p2,
                    d2,
                    walk_geom(p2, 0.5, [u2[0], u2[1]], 0.02),
                    row3(),
                ),
                leg: "2 WALK non-walkable multipass",
                probe_dir: [u2[0], u2[1], 0.0],
                probe_len: l2,
                steep: true,
            },
            C {
                fx: base(
                    "transport-reflect-commit",
                    0x1 | 0x0020_0800,
                    p3,
                    d3,
                    wall3,
                    vec![[0.0, 0.0]],
                ),
                leg: "3 TRANSPORT reflect-slide",
                probe_dir: u3,
                probe_len: l3,
                steep: true,
            },
        ];

        for c in &cases {
            let (o, calls) = run_oracle(&mut emu, &v, &c.fx);
            let (r, seam) = run_rust(&c.fx, true, &v.ftab);
            diff(&o, &r, c.fx.name);
            diff_seam(&calls, &seam, c.fx.name, c.fx.flags & 0x0020_0800 != 0);

            let (contact, idx, hc, nz) =
                probe_sweep(&mut emu, &v, &c.fx, &c.fx.pos, &c.probe_dir, c.probe_len);
            assert!(
                contact && idx < hc,
                "{}: leg [{}] primary sweep did not commit (contact={contact} idx={idx} count={hc})",
                c.fx.name,
                c.leg
            );

            assert_eq!(
                f64::from(nz) <= f64::from(f32::from_bits(0x3e31_d0d4)),
                c.steep,
                "{}: committed normal slope (n.z={nz}) does not match expected steep={}",
                c.fx.name,
                c.steep
            );
            *tally.entry(c.fx.name).or_default() += 1;
            checked += 1;
        }
    }

    {
        let (mut emu, v) = setup(0);

        let fx = Case {
            name: "trace-clear-freeadvance",
            pos: [1.0, 2.0, 3.0],
            flags: 0x1,
            radius: 0.5,
            height: 2.0,
            floor: 0.0,
            f58: 0,
            f78: 0,
            f7c: 0.0,
            f84: 4.5,
            fa0: 0.0,
            support: [0.0, 0.0, 1.0],
            support_pt: [9.0, 9.0, 9.0],
            base_pos: 0,
            budget: 70,
            disp: [0.4, 0.3, 0.0],
            coll_height: 1.2,
            fall_dist: 0.0,
            obj_pred: false,
            swim_terminal: 7.0,
            hits: vec![],
            rows: vec![],
        };
        let (o, calls) = run_oracle(&mut emu, &v, &fx);
        let (r, seam) = run_rust(&fx, false, &v.ftab);
        diff(&o, &r, fx.name);
        diff_seam(&calls, &seam, fx.name, fx.flags & 0x0020_0800 != 0);
        *tally.entry(fx.name).or_default() += 1;
        checked += 1;

        let fx0 = Case {
            name: "budget-zero",
            budget: 0,
            ..fx.clone()
        };
        let (o, calls) = run_oracle(&mut emu, &v, &fx0);
        let (r, seam) = run_rust(&fx0, false, &v.ftab);
        diff(&o, &r, fx0.name);
        diff_seam(&calls, &seam, fx0.name, fx0.flags & 0x0020_0800 != 0);
        *tally.entry(fx0.name).or_default() += 1;
        checked += 1;
    }

    eprintln!("resolve_entry: {checked} whole-layer rounds bit-exact vs WoW.exe");
    for (k, n) in &tally {
        eprintln!("  {k}: {n}");
    }
}
