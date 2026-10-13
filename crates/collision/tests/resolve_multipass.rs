//! `CMovement::ProbeStepUp` (`0x636100`) runs whole with `QueryWorldTriangles` (`0x6721b0`) stubbed
//! and the triangle list written by the test, and with `IsUnitPlayerDriven` (`0x5fa550`),
//! `CMovement::SetTransportFromSupport` (`0x6173b0`), `CMovement::UpdateAnchors` (`0x7c5cd0`),
//! `CMovement::GetFallDrop` (`0x7c6140`) and `CMovement::StartFalling2` (`0x7c61f0`) stubbed. The
//! sweeps and `CMovement::ProbeFallAway` (`0x635f80`) run in the exe, and the reimplementation is
//! given their results.

use benilla_re_collision::resolve_clip::HitRecord;
use benilla_re_collision::resolve_multipass::{step_up_resolve, SweepRes};
use benilla_re_collision::resolve_toolkit::Plane;
use benilla_re_difftest::{Emu, Wow};
use std::cell::RefCell;
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
    indir: u64,
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
    let recs = emu.alloc(0x800);
    let indir = emu.alloc(0x10);
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
    Vas {
        this,
        recs,
        indir,
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
    hits: &[HitRecord],
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
    emu.write_u32(0xc4e530, hits.len() as u32);
    emu.write_u32(0xc4e534, v.recs as u32);
    emu.write_u32(0xc4e568, 0);

    let aux = emu.read_u32(v.this + 0x30) as u64;
    emu.write_u32(aux, 0);
    emu.write_u32(aux + 4, 0);

    for i in 0..7u64 {
        emu.call(0x6376a0, (v.mani + i * 0x10) as u32, 0, &[]);
    }
}

/// Runs `0x636100` and the reimplementation on one case and compares them.
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
    input_dir: [f32; 2],
    vel: [f32; 3],
    hits: &[HitRecord],
    tally: &mut BTreeMap<String, u32>,
    ctx: &str,
) {
    let n_hits = hits.len() as u32;

    replant(emu, v, pos, radius, height, flags, h, fa0, f68, hits);
    wv3(emu, v.indir, &[input_dir[0], input_dir[1], 0.0]);
    let oeax = emu.call(
        0x636100,
        v.this as u32,
        0,
        &[
            v.indir as u32,
            vel[0].to_bits(),
            vel[1].to_bits(),
            vel[2].to_bits(),
        ],
    );
    let o_pos = rv3(emu, v.this + 0x10);
    let o_flags = emu.read_u32(v.this + 0x40);
    let o_floor = emu.read_f32(v.this + 0x80);

    replant(emu, v, pos, radius, height, flags, h, fa0, f68, hits);
    let mut r_pos = pos;
    let mut r_flags = flags;
    let mut r_floor = f32::from_bits(SENT);
    let mut r_vel = vel;
    let sweep_calls = std::cell::Cell::new(0u32);
    let fall_calls = std::cell::Cell::new(0u32);

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
            fall_calls.set(fall_calls.get() + 1);
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
        step_up_resolve(
            &mut r_pos,
            radius,
            height,
            &mut r_flags,
            &mut r_floor,
            h,
            true,
            input_dir,
            &mut r_vel,
            hits,
            n_hits,
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

    let label = format!(
        "sweeps={} fall={} eax={}",
        sweep_calls.get(),
        fall_calls.get(),
        reax
    );
    *tally.entry(label).or_insert(0) += 1;
}

/// Random steps against faces of varied slope.
#[test]
fn multipass_difftest() {
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
    let mut s = 0x6361_0001u32;
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    let mut checked = 0u32;

    for case in 0..600u32 {
        let regime = case % 6;
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = (fz(&mut s).abs() * 0.4) + 0.35;
        let height = (fz(&mut s).abs() * 0.5) + 0.7;

        let flags = if next(&mut s) & 1 == 0 {
            0x0400_0000
        } else {
            0
        };
        let h = fz(&mut s).abs() * 0.8 + 0.3;
        let fa0 = fz(&mut s);
        let f68 = [fz(&mut s), fz(&mut s)];

        let cos50 = 0.6427876f32;
        let vz = match regime {
            1 => cos50 + fz(&mut s).abs() + 0.1,
            2 => -(fz(&mut s).abs() + 0.1),
            _ => (fz(&mut s).abs() % cos50) * 0.9,
        };
        let vxy = normalize([fz(&mut s), fz(&mut s), 0.0]);
        let vel = [
            vxy[0] * (fz(&mut s).abs() + 0.3),
            vxy[1] * (fz(&mut s).abs() + 0.3),
            vz,
        ];
        let input_dir = [fz(&mut s) * 0.5, fz(&mut s) * 0.5];

        let ch = fz(&mut s).abs() * 0.5 + 0.2;
        let drop = fz(&mut s).abs() * 0.5 + 0.2;
        let fwd_nz = match regime {
            3 => 0.9 + fz(&mut s).abs() * 0.05,
            4 => 0.2 + fz(&mut s).abs() * 0.2,
            _ => fz(&mut s).abs() * 0.95,
        };
        let fhxy = (1.0 - fwd_nz * fwd_nz).max(0.0).sqrt();
        let fang = (next(&mut s) >> 8) as f32 / (1 << 24) as f32 * std::f32::consts::TAU;
        let fwd_n = [fhxy * fang.cos(), fhxy * fang.sin(), fwd_nz];

        let mut hits = vec![
            HitRecord {
                normal: [0.0, 0.0, -1.0],
                d: fz(&mut s),
                basis: tri([pos[0], pos[1], pos[2] + ch], [0.0, 0.0, -1.0]),
            },
            HitRecord {
                normal: fwd_n,
                d: fz(&mut s),
                basis: tri(
                    [
                        pos[0] + vxy[0] * (radius + 0.1),
                        pos[1] + vxy[1] * (radius + 0.1),
                        pos[2] + ch * 0.5,
                    ],
                    fwd_n,
                ),
            },
            HitRecord {
                normal: [0.0, 0.0, 1.0],
                d: fz(&mut s),
                basis: tri([pos[0], pos[1], pos[2] - drop], [0.0, 0.0, 1.0]),
            },
        ];

        if regime == 5 {
            hits.remove(0);
        }
        run_case(
            &mut emu,
            &v,
            pos,
            radius,
            height,
            flags,
            h,
            fa0,
            f68,
            input_dir,
            vel,
            &hits,
            &mut tally,
            &format!("R{regime} case {case}"),
        );
        checked += 1;
    }

    {
        let pos = [0.5f32, -0.3, 1.0];
        let hits = vec![HitRecord {
            normal: [0.0, 0.0, 1.0],
            d: 0.1,
            basis: tri([pos[0], pos[1], pos[2] - 0.5], [0.0, 0.0, 1.0]),
        }];
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
            [0.0, 0.0],
            [0.3, 0.2, 0.6427876],
            &hits,
            &mut tally,
            "P1 vz=cos50",
        );
        checked += 1;
    }

    {
        let pos = [-1.0f32, 2.0, 0.5];
        let hits = vec![
            HitRecord {
                normal: [0.0, 0.0, -1.0],
                d: 0.2,
                basis: tri([pos[0], pos[1], pos[2] + 0.3], [0.0, 0.0, -1.0]),
            },
            HitRecord {
                normal: [0.0, 0.0, 1.0],
                d: 0.1,
                basis: tri([pos[0], pos[1], pos[2] - 0.4], [0.0, 0.0, 1.0]),
            },
        ];
        run_case(
            &mut emu,
            &v,
            pos,
            0.45,
            0.9,
            0x0400_0000,
            0.6,
            -0.5,
            [0.3, -0.2],
            [0.7, -0.7],
            [0.001, -0.002, 0.2],
            &hits,
            &mut tally,
            "P2 tiny-horiz",
        );
        checked += 1;
    }

    eprintln!("multipass: {checked} rounds bit-exact vs WoW.exe");
    eprintln!("multipass branch coverage tally (sweeps reached / fall-land / eax):");
    for (k, n) in &tally {
        eprintln!("  {k:<28} {n}");
    }

    assert!(
        tally.keys().any(|k| k.ends_with("eax=1")),
        "fuzz failed to drive the vector-slide return-1 leg"
    );
    assert!(
        tally.keys().any(|k| k.ends_with("eax=2")),
        "fuzz failed to drive the landed-walkable return-2 leg"
    );
    assert!(
        tally.keys().any(|k| k.contains("fall=1")),
        "fuzz failed to drive the 0x635f80 fall/land leg"
    );
}

/// Sweeps that hit nothing.
#[test]
fn multipass_sweep_miss_difftest() {
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
    let mut s = 0x5_0727u32;
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    let mut checked = 0u32;
    let cos50 = 0.6427876f32;
    for case in 0..60u32 {
        let pos = [fz(&mut s) * 5.0, fz(&mut s) * 5.0, fz(&mut s) * 5.0 + 20.0];
        let radius = (fz(&mut s).abs() * 0.4) + 0.35;
        let height = (fz(&mut s).abs() * 0.5) + 0.7;
        let flags = if next(&mut s) & 1 == 0 {
            0x0400_0000
        } else {
            0
        };
        let vz = if case % 2 == 0 {
            (fz(&mut s).abs() % cos50) * 0.9
        } else {
            cos50 + fz(&mut s).abs() + 0.1
        };
        let vxy = normalize([fz(&mut s), fz(&mut s), 0.0]);
        let vel = [
            vxy[0] * (fz(&mut s).abs() + 0.3),
            vxy[1] * (fz(&mut s).abs() + 0.3),
            vz,
        ];
        let hits = vec![HitRecord {
            normal: [0.0, 0.0, 1.0],
            d: 0.1,
            basis: tri([pos[0], pos[1], pos[2] - 0.5], [0.0, 0.0, 1.0]),
        }];
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
            [fz(&mut s) * 0.5, fz(&mut s) * 0.5],
            vel,
            &hits,
            &mut tally,
            &format!("miss case {case}"),
        );
        checked += 1;
    }
    eprintln!("multipass_sweep_miss: {checked} rounds bit-exact vs WoW.exe");
    for (k, n) in &tally {
        eprintln!("  {k:<28} {n}");
    }
    assert!(
        tally.keys().any(|k| k.ends_with("eax=0")),
        "sweep-miss test failed to drive the abort return-0 leg"
    );
}
