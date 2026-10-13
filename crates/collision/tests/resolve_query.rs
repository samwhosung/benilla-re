//! `CMovement::QuerySweptBox` (`0x633840`) runs whole with `QueryWorldTriangles` (`0x6721b0`) and
//! `IsUnitPlayerDriven` (`0x5fa550`) stubbed, the stubbed query returning the triangle list the
//! test writes; the transport cases plant the matrices, the inverse from `InvertAffineMatrix`
//! (`0x7bd820`). `IsVector3Finite` (`0x6337d0`) runs whole.

use benilla_re_collision::resolve_clip::HitRecord;
use benilla_re_collision::resolve_query::{pos_finite_validate, swept_world_query, Transport};
use benilla_re_collision::resolve_toolkit::Aabb;
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use std::collections::BTreeMap;

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// A random `f32` in `[-4, 4)`.
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}

/// The return address that ends a run.
const HALT: u64 = 0x6f6f_6f6f;
/// Where the arguments go on the stack.
const ARGBASE: u64 = 0x7008_0000;

/// Reads a triangle record.
fn read_rec(emu: &Emu, va: u64) -> HitRecord {
    let rf = |o: u64| emu.read_f32(va + o);
    HitRecord {
        normal: [rf(0), rf(4), rf(8)],
        d: rf(0xc),
        basis: [
            [rf(0x10), rf(0x14), rf(0x18)],
            [rf(0x1c), rf(0x20), rf(0x24)],
            [rf(0x28), rf(0x2c), rf(0x30)],
        ],
    }
}
/// Writes a triangle record.
fn write_rec(emu: &mut Emu, va: u64, r: &HitRecord) {
    let mut wf = |o: u64, x: f32| emu.write_u32(va + o, x.to_bits());
    wf(0, r.normal[0]);
    wf(4, r.normal[1]);
    wf(8, r.normal[2]);
    wf(0xc, r.d);
    for (i, row) in r.basis.iter().enumerate() {
        let b = 0x10 + (i as u64) * 0xc;
        wf(b, row[0]);
        wf(b + 4, row[1]);
        wf(b + 8, row[2]);
    }
}
/// A random triangle record.
fn rand_rec(s: &mut u32) -> HitRecord {
    HitRecord {
        normal: [fz(s), fz(s), fz(s)],
        d: fz(s),
        basis: [
            [fz(s), fz(s), fz(s)],
            [fz(s), fz(s), fz(s)],
            [fz(s), fz(s), fz(s)],
        ],
    }
}
/// A record as its words, for comparing.
fn rec_words(r: &HitRecord) -> Vec<u32> {
    let mut w = vec![
        r.normal[0].to_bits(),
        r.normal[1].to_bits(),
        r.normal[2].to_bits(),
        r.d.to_bits(),
    ];
    for row in &r.basis {
        for c in row {
            w.push(c.to_bits());
        }
    }
    w
}
/// Names the movement mode the flags select.
fn mode_name(flags: u32) -> &'static str {
    if flags & 0x2000 != 0 {
        "FALL"
    } else if flags & 0x20_0000 != 0 {
        "SWIM"
    } else {
        "WALK"
    }
}

/// The addresses of the mover, the lists and the stubs' results.
struct Vas {
    this: u64,
    main: u64,
    liquid: u64,
    hcell: u64,
    fcell: u64,
    mbuf: u64,
    trsbuf: u64,
}

/// Maps the mover and the lists and installs the stubs.
fn setup(emu: &mut Emu) -> Vas {
    let this = emu.alloc(0x200);
    let unit = emu.alloc(0x1000);
    let desc = emu.alloc(0x20);
    let ptr2 = emu.alloc(0x20);
    let main = emu.alloc(0x800);
    let liquid = emu.alloc(0x800);
    let hcell = emu.alloc(8);
    let fcell = emu.alloc(8);
    let mbuf = emu.alloc(0x40);
    let trsbuf = emu.alloc(0x40);

    let mut p = vec![0xd9u8, 0x05];
    p.extend_from_slice(&(hcell as u32).to_le_bytes());
    p.push(0xc3);
    emu.write(0x617430, &p);

    let mut p = vec![0xd9u8, 0x05];
    p.extend_from_slice(&(fcell as u32).to_le_bytes());
    p.extend_from_slice(&[0xc2, 0x04, 0x00]);
    emu.write(0x7c6140, &p);

    let mut p = vec![0x8bu8, 0xf9, 0xbe];
    p.extend_from_slice(&(mbuf as u32).to_le_bytes());
    p.extend_from_slice(&[0xb9, 0x10, 0x00, 0x00, 0x00, 0xf3, 0xa5, 0xc2, 0x08, 0x00]);
    emu.write(0x630ac0, &p);

    emu.write_u32(this + 0x15c, unit as u32);
    emu.write_u32(unit + 8, desc as u32);
    emu.write_u32(unit + 0xe68, ptr2 as u32);
    emu.write_u32(desc + 8, 0x10);
    emu.write_u32(this + 0x20, 0);
    Vas {
        this,
        main,
        liquid,
        hcell,
        fcell,
        mbuf,
        trsbuf,
    }
}

/// Writes a transport's matrix and its inverse and returns them.
fn plant_transport(emu: &mut Emu, v: &Vas, m: &[[f32; 3]; 4]) -> Transport {
    for c in 0..4u64 {
        for r in 0..3u64 {
            emu.write_u32(
                v.mbuf + c * 0x10 + r * 4,
                m[c as usize][r as usize].to_bits(),
            );
        }
        emu.write_u32(v.mbuf + c * 0x10 + 0xc, 0);
    }

    emu.call(0x7bd820, v.mbuf as u32, 0, &[v.trsbuf as u32, 0x3f80_0000]);
    let ext = |buf: u64| -> [[f32; 3]; 4] {
        core::array::from_fn(|c| {
            core::array::from_fn(|r| emu.read_f32(buf + (c as u64) * 0x10 + (r as u64) * 4))
        })
    };
    Transport {
        m: *m,
        trs: ext(v.trsbuf),
    }
}

/// Runs one query and compares the box, the list and the bases with the reimplementation's.
#[allow(clippy::too_many_arguments)]
fn run_case(
    emu: &mut Emu,
    v: &Vas,
    pos: [f32; 3],
    radius: f32,
    height: f32,
    flags: u32,
    dist: f32,
    fall_dist: f32,
    coll_height: f32,
    dir_in: [f32; 3],
    transport: Option<Transport>,
    main_recs: &[HitRecord],
    liquid_recs: &[HitRecord],
    trace_hit: bool,
    tally: &mut BTreeMap<String, u32>,
    ctx: &str,
) {
    let guid_zero = transport.is_none();

    for k in 0..3u64 {
        emu.write_u32(v.this + 0x10 + k * 4, pos[k as usize].to_bits());
    }
    let guid = if guid_zero { 0 } else { 0x1234_5678u32 };
    emu.write_u32(v.this + 0x38, guid);
    emu.write_u32(v.this + 0x3c, 0);
    emu.write_u32(v.this + 0x40, flags);
    emu.write_u32(v.this + 0x78, 0);
    emu.write_u32(v.this + 0xb0, radius.to_bits());
    emu.write_u32(v.this + 0xb4, height.to_bits());
    emu.write_u32(v.hcell, coll_height.to_bits());
    emu.write_u32(v.fcell, fall_dist.to_bits());
    for (i, r) in main_recs.iter().enumerate() {
        write_rec(emu, v.main + (i as u64) * 0x34, r);
    }
    for (i, r) in liquid_recs.iter().enumerate() {
        write_rec(emu, v.liquid + (i as u64) * 0x34, r);
    }
    emu.write_u32(0xc4e530, main_recs.len() as u32);
    emu.write_u32(0xc4e534, v.main as u32);
    emu.write_u32(0xc4e568, liquid_recs.len() as u32);
    emu.write_u32(0xc4e56c, v.liquid as u32);
    if let Some(tp) = &transport {
        plant_transport(emu, v, &tp.m);
    }

    emu.write_u32(ARGBASE, HALT as u32);
    emu.write_u32(ARGBASE + 4, dist.to_bits());
    emu.write_u32(ARGBASE + 8, 0);
    emu.write_u32(ARGBASE + 0xc, dir_in[0].to_bits());
    emu.write_u32(ARGBASE + 0x10, dir_in[1].to_bits());
    emu.write_u32(ARGBASE + 0x14, dir_in[2].to_bits());
    emu.set_reg(RegisterX86::ESP, ARGBASE);
    emu.set_reg(RegisterX86::ECX, v.this);
    emu.run(0x633840, HALT);

    let o_al = (emu.get_reg(RegisterX86::EAX) as u32) & 0xff;
    let o_scr = Aabb {
        min: core::array::from_fn(|k| emu.read_f32(0xc4e5a0 + (k as u64) * 4)),
        max: core::array::from_fn(|k| emu.read_f32(0xc4e5ac + (k as u64) * 4)),
    };
    let o_dir: [f32; 3] = core::array::from_fn(|k| emu.read_f32(ARGBASE + 0xc + (k as u64) * 4));
    let o_main: Vec<HitRecord> = (0..main_recs.len())
        .map(|i| read_rec(emu, v.main + (i as u64) * 0x34))
        .collect();
    let o_liquid: Vec<HitRecord> = (0..liquid_recs.len())
        .map(|i| read_rec(emu, v.liquid + (i as u64) * 0x34))
        .collect();

    let mut r_dir = dir_in;
    let mut r_main = main_recs.to_vec();
    let mut r_liquid = liquid_recs.to_vec();
    let (r_hits, r_scr) = swept_world_query(
        pos,
        radius,
        height,
        flags,
        guid_zero,
        dist,
        fall_dist,
        coll_height,
        &mut r_dir,
        trace_hit,
        transport.as_ref(),
        &mut r_main,
        &mut r_liquid,
    );
    let r_al = u32::from(r_hits);

    assert_eq!(o_al, r_al, "{ctx}: al (binary={o_al} ours={r_al})");
    for k in 0..3 {
        assert_eq!(
            o_scr.min[k].to_bits(),
            r_scr.min[k].to_bits(),
            "{ctx}: scratch.min[{k}] binary={:#010x} ours={:#010x}",
            o_scr.min[k].to_bits(),
            r_scr.min[k].to_bits()
        );
        assert_eq!(
            o_scr.max[k].to_bits(),
            r_scr.max[k].to_bits(),
            "{ctx}: scratch.max[{k}] binary={:#010x} ours={:#010x}",
            o_scr.max[k].to_bits(),
            r_scr.max[k].to_bits()
        );
        assert_eq!(
            o_dir[k].to_bits(),
            r_dir[k].to_bits(),
            "{ctx}: dir[{k}] binary={:#010x} ours={:#010x}",
            o_dir[k].to_bits(),
            r_dir[k].to_bits()
        );
    }
    let diff_recs = |label: &str, ob: &[HitRecord], rb: &[HitRecord]| {
        for (i, (o, r)) in ob.iter().zip(rb.iter()).enumerate() {
            for (w, (a, b)) in rec_words(o).iter().zip(rec_words(r).iter()).enumerate() {
                assert_eq!(
                    a, b,
                    "{ctx}: {label}[{i}] word {w} binary={a:#010x} ours={b:#010x}"
                );
            }
        }
    };
    diff_recs("main", &o_main, &r_main);
    diff_recs("liquid", &o_liquid, &r_liquid);

    let label = format!(
        "mode={} water={} guid={} liq={} al={}",
        mode_name(flags),
        u32::from(flags & 0x4000_0000 != 0),
        u32::from(!guid_zero),
        u32::from(!liquid_recs.is_empty() && flags & 0x20_0000 != 0),
        r_al
    );
    *tally.entry(label).or_insert(0) += 1;
}

/// A transport turned about z, for the transport cases.
const ROT_YAW: [[f32; 3]; 4] = [
    [0.8f32, 0.6, 0.0],
    [-0.6, 0.8, 0.0],
    [0.0, 0.0, 1.0],
    [3.0, -2.0, 0.5],
];

/// Walking, falling and swimming movers, on and off a transport.
#[test]
fn query_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 1, 2);
    let v = setup(&mut emu);
    let mut s = 0x6338_4001u32;
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    let mut checked = 0u32;

    for case in 0..900u32 {
        let regime = case % 9;
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let radius = fz(&mut s).abs() * 0.5 + 0.3;
        let height = fz(&mut s).abs() * 0.6 + 0.5;
        let dist = fz(&mut s).abs() * 1.5 + 0.05;
        let coll_height = fz(&mut s).abs() * 0.8 + 0.1;

        let fall_dist = fz(&mut s) * 1.2;
        let dir_in = [fz(&mut s), fz(&mut s), fz(&mut s)];

        let mut flags = 0u32;
        match regime {
            0 | 1 => flags |= 0x2000,
            2 | 3 => flags |= 0x20_0000,
            _ => {}
        }
        if next(&mut s) & 1 == 0 {
            flags |= 0x4000_0000;
        }

        let transport = if regime % 3 == 0 {
            Some(plant_transport(&mut emu, &v, &ROT_YAW))
        } else {
            None
        };

        let nmain = 1 + (next(&mut s) % 4) as usize;
        let main_recs: Vec<HitRecord> = (0..nmain).map(|_| rand_rec(&mut s)).collect();
        let nliq = if flags & 0x20_0000 != 0 {
            (next(&mut s) % 4) as usize
        } else {
            0
        };
        let liquid_recs: Vec<HitRecord> = (0..nliq).map(|_| rand_rec(&mut s)).collect();

        run_case(
            &mut emu,
            &v,
            pos,
            radius,
            height,
            flags,
            dist,
            fall_dist,
            coll_height,
            dir_in,
            transport,
            &main_recs,
            &liquid_recs,
            true,
            &mut tally,
            &format!("R{regime} case {case}"),
        );
        checked += 1;
    }

    {
        let r = 0.5f32;
        let rt2 = (f64::from(r) * f64::from(f32::from_bits(0x3fb5_04f3))) as f32;
        run_case(
            &mut emu,
            &v,
            [0.2, -0.4, 1.0],
            r,
            rt2,
            0x20_0000,
            0.8,
            0.0,
            0.5,
            [0.3, 0.4, 0.2],
            None,
            &[rand_rec(&mut s)],
            &[],
            true,
            &mut tally,
            "P1 swim h≈r√2",
        );
        checked += 1;
    }

    {
        let r = 0.5f32;
        let tan50 = f32::from_bits(0x3f98_8b62);
        let skin = f32::from_bits(0x3ab6_0b61);
        let h = ((f64::from(r) + f64::from(skin)) / f64::from(tan50)) as f32;
        run_case(
            &mut emu,
            &v,
            [1.0, 1.0, 0.5],
            r,
            0.8,
            0,
            0.6,
            0.0,
            h,
            [0.5, 0.5, 0.1],
            None,
            &[rand_rec(&mut s)],
            &[],
            true,
            &mut tally,
            "P2 walk steprise edge",
        );
        checked += 1;
    }

    {
        let ch = 0.4f32;
        run_case(
            &mut emu,
            &v,
            [-1.0, 0.5, 0.2],
            0.45,
            0.7,
            0x4000_0000,
            (f64::from(ch) * 2.0) as f32,
            0.0,
            ch,
            [0.2, -0.3, 0.4],
            None,
            &[rand_rec(&mut s)],
            &[],
            true,
            &mut tally,
            "P3 walk zext edge",
        );
        checked += 1;
    }

    {
        run_case(
            &mut emu,
            &v,
            [0.0, 0.0, 5.0],
            0.5,
            0.9,
            0x2000,
            1.0,
            -2.0,
            0.5,
            [0.1, 0.1, 0.9],
            None,
            &[rand_rec(&mut s)],
            &[],
            true,
            &mut tally,
            "P4 fall tilt skip",
        );
        checked += 1;
    }

    {
        let tp = plant_transport(&mut emu, &v, &ROT_YAW);
        run_case(
            &mut emu,
            &v,
            [2.0, -1.0, 3.0],
            0.55,
            0.8,
            0x20_0000 | 0x4000_0000,
            0.7,
            0.0,
            0.5,
            [0.6, -0.2, 0.5],
            Some(tp),
            &[rand_rec(&mut s), rand_rec(&mut s)],
            &[rand_rec(&mut s), rand_rec(&mut s)],
            true,
            &mut tally,
            "P5 transport+swim+liquid",
        );
        checked += 1;
    }

    eprintln!("query: {checked} rounds bit-exact vs WoW.exe");
    eprintln!("query branch coverage (mode/water/guid/liquid/al):");
    for (k, n) in &tally {
        eprintln!("  {k:<46} {n}");
    }
    for m in ["FALL", "SWIM", "WALK"] {
        assert!(
            tally.keys().any(|k| k.contains(&format!("mode={m}"))),
            "fuzz failed to drive the {m} arm"
        );
    }
    assert!(
        tally.keys().any(|k| k.contains("guid=1")),
        "fuzz failed to drive the transport rotate"
    );
    assert!(
        tally.keys().any(|k| k.contains("liq=1")),
        "fuzz failed to drive the swim liquid pass"
    );
}

/// A query that finds nothing.
#[test]
fn query_clear_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x5fa550, 1, 0);
    emu.stub(0x6721b0, 0, 2);
    let v = setup(&mut emu);
    let mut s = 0x0c1e_a201u32;
    let mut tally: BTreeMap<String, u32> = BTreeMap::new();
    let mut checked = 0u32;
    for case in 0..240u32 {
        let regime = case % 4;
        let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let mut flags = 0u32;
        match regime {
            0 => flags |= 0x2000,
            1 => flags |= 0x20_0000,
            _ => {}
        }
        if next(&mut s) & 1 == 0 {
            flags |= 0x4000_0000;
        }
        let transport = if case % 3 == 0 {
            Some(plant_transport(&mut emu, &v, &ROT_YAW))
        } else {
            None
        };
        run_case(
            &mut emu,
            &v,
            pos,
            fz(&mut s).abs() * 0.5 + 0.3,
            fz(&mut s).abs() * 0.6 + 0.5,
            flags,
            fz(&mut s).abs() * 1.5 + 0.05,
            fz(&mut s) * 1.2,
            fz(&mut s).abs() * 0.8 + 0.1,
            [fz(&mut s), fz(&mut s), fz(&mut s)],
            transport,
            &[rand_rec(&mut s)],
            &[],
            false,
            &mut tally,
            &format!("clear R{regime} case {case}"),
        );
        checked += 1;
    }
    eprintln!("query_clear: {checked} rounds bit-exact vs WoW.exe");
    for (k, n) in &tally {
        eprintln!("  {k:<46} {n}");
    }
    assert!(
        tally.keys().any(|k| k.ends_with("al=0")),
        "clear test failed to drive the al=0 leg"
    );
}

/// `0x6337d0`.
#[test]
fn pos_finite_validate_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let vec = emu.alloc(0x10);
    let mut s = 0x6337_d001u32;
    let mut checked = 0u32;
    let specials = [
        0.0f32,
        -0.0,
        1.5,
        -3.25,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::from_bits(0x7f80_0001),
    ];
    for case in 0..400u32 {
        let pick = |s: &mut u32| -> f32 {
            if next(s) & 7 == 0 {
                specials[(next(s) as usize) % specials.len()]
            } else {
                fz(s)
            }
        };
        let p = [pick(&mut s), pick(&mut s), pick(&mut s)];
        for (k, c) in p.iter().enumerate() {
            emu.write_u32(vec + (k as u64) * 4, c.to_bits());
        }
        let o = emu.call(0x6337d0, 0, 0, &[vec as u32]) & 0xff;
        let r = pos_finite_validate(&p);
        assert_eq!(o, r, "0x6337d0 case {case}: pos={p:?} binary={o} ours={r}");
        checked += 1;
    }
    eprintln!("pos_finite_validate: {checked} rounds bit-exact vs WoW.exe");
}
