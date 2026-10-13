//! `CMovement::SweepAirborneStep` (`0x6351a0`) runs whole with `QueryWorldTriangles` (`0x6721b0`)
//! and `IsUnitPlayerDriven` (`0x5fa550`) stubbed and the triangle list written by the test; the
//! sweep it makes runs too, and the reimplementation is given the sweep's result as the exe
//! computes it (`CMovement::SweepVolume`, `0x632ba0`). `HasUpwardPlane` (`0x6353d0`) and
//! `HasBevelPlane` (`0x635410`) run whole.

use benilla_re_collision::resolve_clip::HitRecord;
use benilla_re_collision::resolve_substep::{
    any_plane_bevel, any_plane_up, walk_slide_substep, SweepOut,
};
use benilla_re_collision::resolve_toolkit::Plane;
use benilla_re_collision::resolve_walk::ground_at_point;
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
/// The move's direction.
fn unit_dir(mv: &[f32; 3], cd: f32) -> [f32; 3] {
    let inv = 1.0f64 / (cd as f64);
    core::array::from_fn(|k| (inv * (mv[k] as f64)) as f32)
}
/// A unit vector.
fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-6);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// The addresses of the mover, the lists and the outputs.
struct Vas {
    this: u64,
    recs: u64,
    rows: u64,
    dir: u64,
    mani: u64,
    idx: u64,
    cnt: u64,
    t: u64,
    mv: u64,
    cd: u64,
    oa: u64,
    ob: u64,
    fa: u64,
    fb: u64,
}

/// Maps the mover and the lists and installs the stubs, with the cached box.
fn setup(emu: &mut Emu, cache_min: [f32; 3], cache_max: [f32; 3]) -> Vas {
    let this = emu.alloc(0x200);
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
    let mv = emu.alloc(0x10);
    let cd = emu.alloc(8);
    let oa = emu.alloc(8);
    let ob = emu.alloc(8);
    let fa = emu.alloc(8);
    let fb = emu.alloc(8);

    emu.write_u32(this + 0x15c, unit as u32);
    emu.write_u32(unit + 8, desc as u32);
    emu.write_u32(unit + 0xe68, ptr2 as u32);
    emu.write_u32(desc + 8, 0x10);

    emu.write_u32(this + 0x30, aux as u32);
    emu.write_u32(aux, 0);
    emu.write_u32(aux + 4, 0);

    for k in 0..3u64 {
        emu.write_u32(0xc4e5a0 + k * 4, cache_min[k as usize].to_bits());
        emu.write_u32(0xc4e5ac + k * 4, cache_max[k as usize].to_bits());
    }

    emu.write_u32(0xc4e544, rows as u32);
    Vas {
        this,
        recs,
        rows,
        dir,
        mani,
        idx,
        cnt,
        t,
        mv,
        cd,
        oa,
        ob,
        fa,
        fb,
    }
}

/// Runs one step and compares every output.
#[allow(clippy::too_many_arguments)]
fn run_case(
    emu: &mut Emu,
    v: &Vas,
    pos: [f32; 3],
    radius: f32,
    height: f32,
    flags: u32,
    f84: f32,
    fa0: f32,
    t_param: f32,
    move_in: [f32; 3],
    cd_in: f32,
    hits: &[HitRecord],
    rows: &[[f32; 2]],
    tally: &mut BTreeMap<&'static str, u32>,
    ctx: &str,
) {
    let n_hits = hits.len() as u32;

    for k in 0..3u64 {
        emu.write_u32(v.this + 0x10 + k * 4, pos[k as usize].to_bits());
    }
    emu.write_u32(v.this + 0xb0, radius.to_bits());
    emu.write_u32(v.this + 0xb4, height.to_bits());
    emu.write_u32(v.this + 0x40, flags);
    emu.write_u32(v.this + 0x38, 0);
    emu.write_u32(v.this + 0x3c, 0);
    emu.write_u32(v.this + 0x20, 0);
    emu.write_u32(v.this + 0x84, f84.to_bits());
    emu.write_u32(v.this + 0xa0, fa0.to_bits());

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

    emu.write_u32(0xc4e530, n_hits);
    emu.write_u32(0xc4e534, v.recs as u32);
    emu.write_u32(0xc4e568, 0);

    let dir = unit_dir(&move_in, cd_in);
    wv3(emu, v.dir, &dir);

    for i in 0..7u64 {
        emu.call(0x6376a0, (v.mani + i * 0x10) as u32, 0, &[]);
    }

    emu.write_u32(v.idx, 0xdead_beef);
    emu.write_u32(v.cnt, 0);
    emu.write_u32(v.t, 0x7fc0_0001);
    let sret = emu.call(
        0x632ba0,
        v.this as u32,
        0,
        &[
            0xc4e52c,
            v.dir as u32,
            cd_in.to_bits(),
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
    let sweep = SweepOut {
        contact: sret != 0,
        idx: s_idx,
        t: s_t,
        count: s_cnt,
        hit_count: n_hits,
    };

    let sent = 0x7fc0_0001u32;
    let isent = 0x5a5a_5a5au32;
    wv3(emu, v.mv, &move_in);
    emu.write_u32(v.cd, cd_in.to_bits());
    emu.write_u32(v.oa, sent);
    emu.write_u32(v.oa + 4, sent);
    emu.write_u32(v.ob, sent);
    emu.write_u32(v.ob + 4, sent);
    emu.write_u32(v.fa, isent);
    emu.write_u32(v.fb, isent);
    let oeax = emu.call(
        0x6351a0,
        v.this as u32,
        0,
        &[
            t_param.to_bits(),
            v.mv as u32,
            v.cd as u32,
            v.oa as u32,
            v.ob as u32,
            v.fa as u32,
            v.fb as u32,
        ],
    );
    let o_move = rv3(emu, v.mv);
    let o_cd = emu.read_f32(v.cd);
    let o_oa = [emu.read_f32(v.oa), emu.read_f32(v.oa + 4)];
    let o_ob = [emu.read_f32(v.ob), emu.read_f32(v.ob + 4)];
    let o_fa = emu.read_u32(v.fa) as i32;
    let o_fb = emu.read_u32(v.fb) as i32;

    let idx = s_idx as usize;
    let hit = if idx < hits.len() {
        hits[idx]
    } else {
        HitRecord::default()
    };
    let row = if idx < rows.len() {
        rows[idx]
    } else {
        [0.0, 0.0]
    };
    let mut r_move = move_in;
    let mut r_cd = cd_in;
    let mut r_oa = [f32::from_bits(sent); 2];
    let mut r_ob = [f32::from_bits(sent); 2];
    let mut r_fa = isent as i32;
    let mut r_fb = isent as i32;
    let reax = walk_slide_substep(
        &pos,
        radius,
        f84,
        fa0,
        flags,
        true,
        t_param,
        &mut r_move,
        &mut r_cd,
        &mut r_oa,
        &mut r_ob,
        &mut r_fa,
        &mut r_fb,
        &sweep,
        &manifold,
        &hit,
        row,
    );

    assert_eq!(oeax, reax, "{ctx}: eax (binary={oeax} ours={reax})");
    for k in 0..3 {
        assert_eq!(
            o_move[k].to_bits(),
            r_move[k].to_bits(),
            "{ctx}: move[{k}] binary={:#010x} ours={:#010x}",
            o_move[k].to_bits(),
            r_move[k].to_bits()
        );
    }
    assert_eq!(
        o_cd.to_bits(),
        r_cd.to_bits(),
        "{ctx}: contactDist binary={:#010x} ours={:#010x}",
        o_cd.to_bits(),
        r_cd.to_bits()
    );
    for k in 0..2 {
        assert_eq!(
            o_oa[k].to_bits(),
            r_oa[k].to_bits(),
            "{ctx}: outVecA[{k}] binary={:#010x} ours={:#010x}",
            o_oa[k].to_bits(),
            r_oa[k].to_bits()
        );
        assert_eq!(
            o_ob[k].to_bits(),
            r_ob[k].to_bits(),
            "{ctx}: outVecB[{k}] binary={:#010x} ours={:#010x}",
            o_ob[k].to_bits(),
            r_ob[k].to_bits()
        );
    }
    assert_eq!(o_fa, r_fa, "{ctx}: flagA");
    assert_eq!(o_fb, r_fb, "{ctx}: flagB");

    let label = classify(
        cd_in, &sweep, &dir, &pos, &hit, &manifold, fa0, flags, t_param, f84,
    );
    *tally.entry(label).or_insert(0) += 1;
}

/// The regime a case falls in, for the tally.
#[allow(clippy::too_many_arguments)]
fn classify(
    cd_in: f32,
    sweep: &SweepOut,
    dir: &[f32; 3],
    pos: &[f32; 3],
    hit: &HitRecord,
    manifold: &[Plane],
    fa0: f32,
    flags: u32,
    t_param: f32,
    f84: f32,
) -> &'static str {
    use benilla_re_collision::resolve_fall::swim_fall_vel;
    let len_guard = 2.384_185_8e-7_f32 as f64;
    if (cd_in as f64).abs() < len_guard {
        return "1_degeneracy_ret0";
    }
    if !sweep.contact {
        return "2_sweep0_ret2";
    }
    if sweep.idx == sweep.hit_count {
        return "3_sentinel_ret0";
    }
    let t = sweep.t as f64;
    let offset = [
        (dir[0] as f64 * t) as f32,
        (dir[1] as f64 * t) as f32,
        (t * dir[2] as f64) as f32,
    ];
    if ground_at_point(pos, &offset, hit.normal[2], &hit.basis, true) {
        if any_plane_bevel(manifold, sweep.count) {
            "4_ground_bevel_hit"
        } else {
            "5_ground_bevel_miss"
        }
    } else if (dir[2] as f64) > 0.0 {
        let r = swim_fall_vel(fa0, flags) - (t_param as f64);
        if r >= 0.0 {
            let lhs = r * (f84 as f64);
            if lhs >= t {
                if any_plane_up(manifold, sweep.count) {
                    "9_nzup_hit_flagB"
                } else {
                    "A_nzup_miss_slide"
                }
            } else {
                "8_gate2_fail_slide"
            }
        } else {
            "7_gate1_fail_slide"
        }
    } else {
        "6_dirz_le0_slide"
    }
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

/// Random steps against level and upright triangles.
#[test]
fn walk_slide_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 1, 2);
    let v = setup(&mut emu, [-1.0e6, -1.0e6, -1.0e6], [1.0e6, 1.0e6, 1.0e6]);
    let mut s = 0x5a1d_2026u32;
    let mut tally: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut checked = 0u32;

    for case in 0..360u32 {
        let regime = case % 6;
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = (fz(&mut s).abs() * 0.4) + 0.35;
        let height = (fz(&mut s).abs() * 0.5) + 0.7;
        let flags = next(&mut s) & 0x000f_0000;
        let cd_in = (fz(&mut s).abs()) + 1.0;
        let reach = cd_in * 0.5;

        let (move_dir, normal, vertical, t_param, f84, fa0) = match regime {
            0 => {
                let d = normalize([
                    fz(&mut s) * 0.25,
                    fz(&mut s) * 0.25,
                    -(fz(&mut s).abs() + 0.6),
                ]);
                (
                    d,
                    [0.0f32, 0.0, 1.0],
                    false,
                    fz(&mut s),
                    fz(&mut s),
                    fz(&mut s),
                )
            }
            1 => {
                let d = normalize([
                    -(fz(&mut s).abs() + 0.5),
                    fz(&mut s) * 0.2,
                    -(fz(&mut s).abs() + 0.3),
                ]);
                (
                    d,
                    [1.0f32, 0.0, 0.0],
                    true,
                    fz(&mut s),
                    fz(&mut s),
                    fz(&mut s),
                )
            }
            2 => {
                let d = normalize([fz(&mut s) * 0.2, fz(&mut s) * 0.2, fz(&mut s).abs() + 0.7]);
                (
                    d,
                    [0.0f32, 0.0, -1.0],
                    false,
                    -(fz(&mut s).abs() + 5.0),
                    fz(&mut s).abs() + 8.0,
                    fz(&mut s),
                )
            }
            3 => {
                let d = normalize([fz(&mut s) * 0.2, fz(&mut s) * 0.2, fz(&mut s).abs() + 0.7]);
                (
                    d,
                    [0.0f32, 0.0, -1.0],
                    false,
                    fz(&mut s).abs() + 5.0,
                    fz(&mut s),
                    fz(&mut s),
                )
            }
            4 => {
                let d = normalize([fz(&mut s) * 0.2, fz(&mut s) * 0.2, fz(&mut s).abs() + 0.7]);
                (
                    d,
                    [0.0f32, 0.0, -1.0],
                    false,
                    -(fz(&mut s).abs() * 0.001),
                    fz(&mut s).abs() * 1e-4,
                    fz(&mut s),
                )
            }
            _ => {
                let d = normalize([
                    -(fz(&mut s).abs() + 0.7),
                    fz(&mut s) * 0.2,
                    fz(&mut s).abs() + 0.4,
                ]);
                (
                    d,
                    [1.0f32, 0.0, 0.0],
                    true,
                    -(fz(&mut s).abs() + 5.0),
                    fz(&mut s).abs() + 8.0,
                    fz(&mut s),
                )
            }
        };
        let move_in: [f32; 3] =
            core::array::from_fn(|k| (move_dir[k] as f64 * cd_in as f64) as f32);
        let center = [
            pos[0] + move_dir[0] * reach,
            pos[1] + move_dir[1] * reach,
            pos[2] + move_dir[2] * reach,
        ];
        let basis = if vertical { vtri(center) } else { htri(center) };
        let hits = vec![HitRecord {
            normal,
            d: fz(&mut s),
            basis,
        }];
        let rows = vec![[fz(&mut s) + 5.0, fz(&mut s) - 5.0], [0.0, 0.0]];
        run_case(
            &mut emu,
            &v,
            pos,
            radius,
            height,
            flags,
            f84,
            fa0,
            t_param,
            move_in,
            cd_in,
            &hits,
            &rows,
            &mut tally,
            &format!("R{regime} case {case}"),
        );
        checked += 1;
    }

    let pos = [0.5f32, -0.3, 1.0];
    let down = normalize([0.05, -0.02, -1.0]);
    let center = [
        pos[0] + down[0] * 0.5,
        pos[1] + down[1] * 0.5,
        pos[2] + down[2] * 0.5,
    ];
    let hits = vec![HitRecord {
        normal: [0.0, 0.0, 1.0],
        d: 0.25,
        basis: htri(center),
    }];
    let rows = vec![[1.5, -2.5], [0.0, 0.0]];

    {
        let cd = 1.0e-7f32;
        let mv: [f32; 3] = core::array::from_fn(|k| (down[k] as f64 * cd as f64) as f32);
        run_case(
            &mut emu,
            &v,
            pos,
            0.4,
            0.8,
            0,
            0.0,
            0.0,
            0.0,
            mv,
            cd,
            &hits,
            &rows,
            &mut tally,
            "P1 degeneracy",
        );
        checked += 1;
    }

    {
        let cd = 5.0e-7f32;
        let mv: [f32; 3] = core::array::from_fn(|k| (down[k] as f64 * cd as f64) as f32);
        run_case(
            &mut emu,
            &v,
            pos,
            0.4,
            0.8,
            0,
            0.0,
            0.0,
            0.0,
            mv,
            cd,
            &hits,
            &rows,
            &mut tally,
            "P2 sentinel",
        );
        checked += 1;
    }

    let cross = |a: [f32; 3], b: [f32; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    for case in 0..120u32 {
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = (fz(&mut s).abs() * 0.3) + 0.3;
        let height = (fz(&mut s).abs() * 0.4) + 0.6;
        let cd_in = (fz(&mut s).abs() * 0.5) + 1.2;
        let nz = 0.66 + (next(&mut s) >> 9) as f32 / (1 << 23) as f32 * 0.28;
        let ang = (next(&mut s) >> 8) as f32 / (1 << 24) as f32 * std::f32::consts::TAU;
        let hxy = (1.0 - nz * nz).max(0.0).sqrt();
        let normal = [hxy * ang.cos(), hxy * ang.sin(), nz];

        let move_dir = normalize([
            -normal[0] + fz(&mut s) * 0.15,
            -normal[1] + fz(&mut s) * 0.15,
            fz(&mut s) * 0.08,
        ]);
        let move_in: [f32; 3] =
            core::array::from_fn(|k| (move_dir[k] as f64 * cd_in as f64) as f32);
        let reach = cd_in * 0.5;
        let center = [
            pos[0] + move_dir[0] * reach,
            pos[1] + move_dir[1] * reach,
            pos[2] + move_dir[2] * reach,
        ];

        let t1 = normalize(if normal[0].abs() < 0.9 {
            cross(normal, [1.0, 0.0, 0.0])
        } else {
            cross(normal, [0.0, 1.0, 0.0])
        });
        let t2 = normalize(cross(normal, t1));
        let basis = [
            core::array::from_fn(|k| center[k] + t1[k] * 4.0),
            core::array::from_fn(|k| center[k] - t1[k] * 3.0 + t2[k] * 4.0),
            core::array::from_fn(|k| center[k] - t1[k] * 3.0 - t2[k] * 4.0),
        ];
        let hits = vec![HitRecord {
            normal,
            d: fz(&mut s),
            basis,
        }];
        let rows = vec![[fz(&mut s) + 5.0, fz(&mut s) - 5.0], [0.0, 0.0]];
        run_case(
            &mut emu,
            &v,
            pos,
            radius,
            height,
            0,
            fz(&mut s),
            fz(&mut s),
            fz(&mut s),
            move_in,
            cd_in,
            &hits,
            &rows,
            &mut tally,
            &format!("bevel-miss attempt {case}"),
        );
        checked += 1;
    }

    eprintln!("walk_slide: {checked} rounds bit-exact vs WoW.exe");
    eprintln!("walk_slide branch coverage tally:");
    for (k, n) in &tally {
        eprintln!("  {k:<24} {n}");
    }

    for b in [
        "1_degeneracy_ret0",
        "3_sentinel_ret0",
        "4_ground_bevel_hit",
        "6_dirz_le0_slide",
        "7_gate1_fail_slide",
        "8_gate2_fail_slide",
        "9_nzup_hit_flagB",
        "A_nzup_miss_slide",
    ] {
        if !tally.contains_key(b) {
            eprintln!("  WARNING: branch {b} not covered by the fuzz/probes");
        }
    }
}

/// A sweep that hits nothing.
#[test]
fn walk_slide_sweep_miss_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 0, 2);
    let v = setup(&mut emu, [-0.05, -0.05, -0.05], [0.05, 0.05, 0.05]);
    let mut s = 0x53e0_2026u32;
    let mut tally: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut checked = 0u32;
    for case in 0..40u32 {
        let pos = [fz(&mut s) * 5.0, fz(&mut s) * 5.0, fz(&mut s) * 5.0];
        let radius = (fz(&mut s).abs() * 0.4) + 0.35;
        let height = (fz(&mut s).abs() * 0.5) + 0.7;
        let cd_in = (fz(&mut s).abs()) + 1.0;
        let down = normalize([
            fz(&mut s) * 0.3,
            fz(&mut s) * 0.3,
            -(fz(&mut s).abs() + 0.5),
        ]);
        let move_in: [f32; 3] = core::array::from_fn(|k| (down[k] as f64 * cd_in as f64) as f32);
        let hits = vec![HitRecord {
            normal: [0.0, 0.0, 1.0],
            d: 0.1,
            basis: htri([pos[0], pos[1], pos[2] - 1.0]),
        }];
        let rows = vec![[1.0, 2.0], [0.0, 0.0]];
        run_case(
            &mut emu,
            &v,
            pos,
            radius,
            height,
            0,
            0.0,
            0.0,
            0.0,
            move_in,
            cd_in,
            &hits,
            &rows,
            &mut tally,
            &format!("sweep-miss case {case}"),
        );
        checked += 1;
    }
    eprintln!("walk_slide_sweep_miss: {checked} rounds bit-exact vs WoW.exe");
    for (k, n) in &tally {
        eprintln!("  {k:<24} {n}");
    }
    assert!(
        tally.contains_key("2_sweep0_ret2"),
        "sweep-miss test failed to drive the sweep-returns-0 leg"
    );
}

/// `0x6353d0` and `0x635410`, with normals at and around the two z values.
#[test]
fn leaf_predicates_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let pl_va = emu.alloc(0x100);
    let up_k = f32::from_bits(0x3f80_0000);
    let bevel_k = f32::from_bits(0xbef3_86a4);
    let gate = 9.536_743e-7_f32;
    let mut s = 0x1eaf_2026u32;
    let mut checked = 0u32;
    let mut hit_first = 0u32;
    let mut hit_later = 0u32;
    let mut all_miss = 0u32;

    let run = |emu: &mut Emu, planes: &[Plane], pl_va: u64| {
        for (i, pl) in planes.iter().enumerate() {
            wv3(emu, pl_va + (i as u64) * 0x10, &pl.n);
            emu.write_u32(pl_va + (i as u64) * 0x10 + 0xc, pl.d.to_bits());
        }
        let count = planes.len() as u32;
        let bu = emu.call(0x6353d0, pl_va as u32, count, &[]);
        let bb = emu.call(0x635410, pl_va as u32, count, &[]);
        (bu, bb)
    };

    for case in 0..500u32 {
        let count = (1 + (next(&mut s) % 7)) as usize;
        let mut planes = vec![
            Plane {
                n: [0.0, 0.0, 0.0],
                d: 0.0
            };
            count
        ];
        for pl in planes.iter_mut() {
            pl.n[0] = fz(&mut s);
            pl.n[1] = fz(&mut s);

            let mut nz = fz(&mut s);
            match (next(&mut s) >> 8) % 6 {
                0 => nz = up_k + fz(&mut s) * 3.0e-6,
                1 => nz = bevel_k + fz(&mut s) * 3.0e-6,
                2 => nz = up_k,
                3 => nz = bevel_k,
                _ => {}
            }
            pl.n[2] = nz;
            pl.d = fz(&mut s);
        }
        let (bu, bb) = run(&mut emu, &planes, pl_va);
        let ou = any_plane_up(&planes, count as u32);
        let ob = any_plane_bevel(&planes, count as u32);
        assert_eq!(bu & 0xff != 0, ou, "case {case}: 0x6353d0 (n.z≈1) verdict");
        assert_eq!(
            bb & 0xff != 0,
            ob,
            "case {case}: 0x635410 (n.z≈bevel) verdict"
        );
        if ou || ob {
            if planes[0].n[2] == up_k
                || (planes[0].n[2] as f64 - up_k as f64).abs() < gate as f64
                || (planes[0].n[2] as f64 - bevel_k as f64).abs() < gate as f64
            {
                hit_first += 1;
            } else {
                hit_later += 1;
            }
        } else {
            all_miss += 1;
        }
        checked += 1;
    }

    let probe1 = |emu: &mut Emu, nz: f32, ctx: &str| -> (bool, bool) {
        let pl = Plane {
            n: [0.1, 0.2, nz],
            d: 0.3,
        };
        wv3(emu, pl_va, &pl.n);
        emu.write_u32(pl_va + 0xc, pl.d.to_bits());
        let bu = emu.call(0x6353d0, pl_va as u32, 1, &[]) & 0xff != 0;
        let bb = emu.call(0x635410, pl_va as u32, 1, &[]) & 0xff != 0;
        let planes = [pl];
        assert_eq!(bu, any_plane_up(&planes, 1), "{ctx}: up verdict");
        assert_eq!(bb, any_plane_bevel(&planes, 1), "{ctx}: bevel verdict");
        (bu, bb)
    };

    let (mut up_hit, mut up_miss) = (0u32, 0u32);
    for off in [1u32, 4, 7, 8, 9, 16] {
        let nz = f32::from_bits(up_k.to_bits() + off);
        let (u, _) = probe1(&mut emu, nz, "up-boundary");
        if u {
            up_hit += 1;
        } else {
            up_miss += 1;
        }
    }

    let (mut bv_hit, mut bv_miss) = (0u32, 0u32);
    for off in [4u32, 16, 30, 32, 34, 64] {
        let nz = f32::from_bits(bevel_k.to_bits() + off);
        let (_, b) = probe1(&mut emu, nz, "bevel-boundary");
        if b {
            bv_hit += 1;
        } else {
            bv_miss += 1;
        }
    }
    assert!(
        up_hit > 0 && up_miss > 0,
        "up boundary probes must straddle the gate (hit={up_hit} miss={up_miss})"
    );
    assert!(
        bv_hit > 0 && bv_miss > 0,
        "bevel boundary probes must straddle the gate (hit={bv_hit} miss={bv_miss})"
    );

    let mk = |nz: f32| Plane {
        n: [0.1, 0.2, nz],
        d: 0.3,
    };
    for planes in [
        vec![mk(0.1), mk(0.2), mk(up_k)],
        vec![mk(0.1), mk(bevel_k)],
        vec![mk(0.0), mk(0.5), mk(-0.9)],
    ] {
        for (i, pl) in planes.iter().enumerate() {
            wv3(&mut emu, pl_va + (i as u64) * 0x10, &pl.n);
            emu.write_u32(pl_va + (i as u64) * 0x10 + 0xc, pl.d.to_bits());
        }
        let c = planes.len() as u32;
        let bu = emu.call(0x6353d0, pl_va as u32, c, &[]) & 0xff != 0;
        let bb = emu.call(0x635410, pl_va as u32, c, &[]) & 0xff != 0;
        assert_eq!(bu, any_plane_up(&planes, c), "later/miss: up");
        assert_eq!(bb, any_plane_bevel(&planes, c), "later/miss: bevel");
    }

    eprintln!(
        "leaf_predicates: {checked} fuzz rounds bit-exact (hit_first={hit_first} hit_later={hit_later} all_miss={all_miss}); boundary straddle up(hit={up_hit} miss={up_miss}) bevel(hit={bv_hit} miss={bv_miss})"
    );
}
