//! The moving transport's path, whole, against the exe: `0x5f4cc0` builds a path from synthetic
//! `TaxiPathNode` rows, then `0x5f50a0` places the ship on it at times across its period
//! (`systems/objectlayer/transports.md`, The ship path, Leg control points, Moving the ship).
//!
//! Both functions run whole, with their callees as the client's bytes: the curve set-up, the 20-chord
//! segment lengths, the span timing with `__ftol`, the speed curve, the arc-length frame and the
//! heading. Stubbed: the allocator (`0x6462e0`, `0x646320`, `0x646430`, the harness's arena), the
//! map object's entity creation `0x5f4970`, the transport list `0x6308b0`, the clock `0x42c010`, the
//! handler's own tick (vtable slot `+0x74`) during the build, and in the tick the current map
//! `0x468580`, the camera `0x4818f0`, the frame matrix (`0x7bdc40`, `0x7bdd60`, which only the
//! passengers read; the list is empty) and the entity calls (`0x6709a0`, `0x670950`, `0x670a30`,
//! `0x6709f0`), whose arguments carry the animation back. The basis tables are filled by their
//! initializers `0x453fa0` and `0x454080` first.
//!
//! The rows keep to the inputs whose result the client fully writes: every leg has at least four
//! nodes, no stop lies on a leg's last node, and no two neighbouring nodes coincide.
//!
//! The facing passes through `fpatan` at `0x5f528e`, which the emulator computes as the host's
//! `atan2` in double; the comparison of the facing rests on that, and the reimplementation calls
//! the host's `atan2` too.

mod common;

use benilla_re_difftest::{data_dir, Emu, Wow};
use benilla_re_objectlayer::ship_path::{
    build, pose, ShipPath, TaxiNode, CATMULL_ROM, CATMULL_ROM_DERIVATIVE,
};
use common::next;

const BUILD: u64 = 0x5f4cc0;
const TICK: u64 = 0x5f50a0;
const STORE: u64 = 0xc0d6a4;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SMEM_FREE: u64 = 0x646430;
const ARENA: u64 = 0x0800_0000;
const ARENA_SIZE: u64 = 0x0100_0000;

struct Rig {
    handler: u64,
    go: u64,
    map: u64,
}

fn rand_range(s: &mut u32, lo: u32, hi: u32) -> u32 {
    lo + (next(s) >> 8) % (hi - lo)
}

fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    common::frand(s, lo, hi)
}

/// A path of 1 to 4 legs, each of 4 to 11 nodes, between rows of other paths. A leg after the
/// first opens on a map change or after its predecessor's last node carries flag 0x1. Interior
/// nodes and first nodes are stops a third of the time; flag 0x4 rides along as noise.
fn make_rows(s: &mut u32, path: u32) -> Vec<TaxiNode> {
    let mut rows = Vec::new();
    for _ in 0..rand_range(s, 0, 3) {
        rows.push(TaxiNode {
            path: path - 1,
            map: 0,
            position: [frand(s, -100.0, 100.0), 0.0, 0.0],
            flags: 2,
            delay: 5,
        });
    }
    let legs = rand_range(s, 1, 5);
    let mut map = rand_range(s, 0, 2);
    let mut pos = [
        frand(s, -9000.0, 9000.0),
        frand(s, -9000.0, 9000.0),
        frand(s, -50.0, 50.0),
    ];
    for leg in 0..legs {
        if leg > 0 {
            let teleport = next(s) >> 31 == 1;
            if teleport {
                rows.last_mut().unwrap().flags |= 1;
            } else {
                map ^= 1;
            }
        }
        let n = rand_range(s, 4, 12);
        for k in 0..n {
            let step = frand(s, 20.0, 400.0);
            let angle = frand(s, 0.0, std::f32::consts::TAU);
            pos[0] += step * angle.cos();
            pos[1] += step * angle.sin();
            pos[2] += frand(s, -5.0, 5.0);
            let mut flags = 0;
            if k + 1 < n && next(s).is_multiple_of(3) {
                flags |= 2;
            }
            if next(s).is_multiple_of(5) {
                flags |= 4;
            }
            rows.push(TaxiNode {
                path,
                map,
                position: pos,
                flags,
                delay: rand_range(s, 0, 90),
            });
        }
    }
    rows.push(TaxiNode {
        path: path + 1,
        map: 0,
        position: [0.0; 3],
        flags: 2,
        delay: 7,
    });
    if next(s) >> 31 == 1 {
        rows.push(TaxiNode {
            path,
            map: 0,
            position: [1.0, 2.0, 3.0],
            flags: 2,
            delay: 9,
        });
    }
    rows
}

fn write_rows(e: &mut Emu, rows: &[TaxiNode]) {
    let va = e.alloc(0x24 * rows.len() as u64);
    for (i, r) in rows.iter().enumerate() {
        let b = va + 0x24 * i as u64;
        e.write_u32(b, i as u32 + 1);
        e.write_u32(b + 4, r.path);
        e.write_u32(b + 8, i as u32);
        e.write_u32(b + 0xc, r.map);
        for k in 0..3 {
            e.write_u32(b + 0x10 + 4 * k as u64, r.position[k].to_bits());
        }
        e.write_u32(b + 0x1c, r.flags);
        e.write_u32(b + 0x20, r.delay);
    }
    e.write_u32(STORE, va as u32);
    e.write_u32(STORE + 4, rows.len() as u32);
}

/// The game object, its record's data columns (path, speed, acceleration for type 15, read by
/// `0x621b00` and `0x5f8150` at `GO+0x214`), and the handler with a vtable whose slot `+0x74` is a
/// stub, so the build stops before its first tick.
fn rig(e: &mut Emu, path: u32, speed: u32, accel: u32) -> Rig {
    let go = e.alloc(0x300);
    let fields = e.alloc(0x100);
    e.write_u32(go + 0x110, fields as u32);
    e.write_u32(fields + 0x3c, 15);
    let data = e.alloc(0x100);
    e.write_u32(go + 0x214, data as u32);
    e.write_u32(data + 0x1c, path);
    e.write_u32(data + 0x20, speed);
    e.write_u32(data + 0x24, accel);
    let vtable = e.alloc(0x100);
    let tick_stub = e.alloc(0x10);
    e.write_u32(vtable + 0x74, tick_stub as u32);
    e.stub(tick_stub, 0, 2);
    let handler = e.alloc(0x100);
    e.write_u32(handler, vtable as u32);
    e.write_u32(handler + 4, go as u32);
    e.write_u32(handler + 0x1c, 1); // an empty passenger list
    let entity = e.alloc(0x40);
    e.write_u32(handler + 0x10, entity as u32);
    let map = e.alloc(0x10);
    Rig { handler, go, map }
}

fn read_f32s(e: &Emu, va: u64, n: usize) -> Vec<u32> {
    (0..n).map(|i| e.read_u32(va + 4 * i as u64)).collect()
}

/// The path the exe built, read back into the reimplementation's shape.
fn read_path(e: &Emu, handler: u64) -> ShipPath {
    use benilla_re_objectlayer::ship_path::{Leg, Stop};
    let p = handler + 0x20;
    let count = e.read_u32(p + 0xc);
    let legs_va = u64::from(e.read_u32(p + 0x10));
    let legs = (0..u64::from(count))
        .map(|i| {
            let l = legs_va + 0x4c * i;
            let n = e.read_u32(l + 4 + 0xc) as usize;
            let pts = u64::from(e.read_u32(l + 4 + 0x10));
            let points = (0..n)
                .map(|k| {
                    let v = read_f32s(e, pts + 12 * k as u64, 3);
                    [
                        f32::from_bits(v[0]),
                        f32::from_bits(v[1]),
                        f32::from_bits(v[2]),
                    ]
                })
                .collect();
            let lens = u64::from(e.read_u32(l + 4 + 0x20));
            let lengths = read_f32s(e, lens, n.saturating_sub(3))
                .into_iter()
                .map(f32::from_bits)
                .collect();
            let m = e.read_u32(l + 0x40) as usize;
            let st = u64::from(e.read_u32(l + 0x44));
            let stops = (0..m)
                .map(|j| {
                    let b = st + 12 * j as u64;
                    Stop {
                        time: e.read_u32(b),
                        distance: f32::from_bits(e.read_u32(b + 4)),
                        wait: e.read_u32(b + 8),
                    }
                })
                .collect();
            Leg {
                map: e.read_u32(l),
                points,
                lengths,
                length: f32::from_bits(e.read_u32(l + 8)),
                start: e.read_u32(l + 0x30),
                travel: e.read_u32(l + 0x34),
                end: e.read_u32(l + 0x38),
                stops,
            }
        })
        .collect();
    ShipPath {
        speed: f32::from_bits(e.read_u32(p + 0x18)),
        accel: f32::from_bits(e.read_u32(p + 0x20)),
        legs,
        period: e.read_u32(p + 0x1c),
    }
}

fn setup(wow: &Wow) -> Emu<'_> {
    let mut e = Emu::new(wow);
    e.map(ARENA, ARENA_SIZE);
    e.stub_alloc_sized(SMEM_ALLOC, ARENA, ARENA + ARENA_SIZE, 0, 4);
    e.stub_realloc_sized(SMEM_REALLOC, 5);
    e.stub(SMEM_FREE, 1, 4);
    e.stub(0x5f4970, 0, 0);
    e.stub(0x6308b0, 0, 0);
    e.stub(0x42c010, 0, 0);
    e.call(0x453fa0, 0, 0, &[]);
    e.call(0x454080, 0, 0, &[]);
    let cr: Vec<u32> = read_f32s(&e, 0xb05e10, 16);
    assert_eq!(cr, CATMULL_ROM.map(f32::to_bits), "the basis at 0xb05e10");
    let d = read_f32s(&e, 0xb05dd8, 12);
    let ours: Vec<u32> = CATMULL_ROM_DERIVATIVE
        .chunks(4)
        .flat_map(|r| r[..3].iter().map(|v| v.to_bits()))
        .collect();
    assert_eq!(d, ours, "the derivative basis at 0xb05dd8");
    e
}

fn check_build(e: &mut Emu, rows: &[TaxiNode], path: u32, speed: u32, accel: u32) -> Rig {
    write_rows(e, rows);
    let r = rig(e, path, speed, accel);
    e.call(BUILD, r.handler as u32, 0, &[]);
    let got = read_path(e, r.handler);
    let ours = build(rows, path, speed, accel);
    assert_eq!(
        e.read_u32(r.handler + 0x24),
        path,
        "taxiPathID at path +0x04"
    );
    assert_eq!(got.legs.len(), ours.legs.len(), "leg count, path {path}");
    for (k, (g, o)) in got.legs.iter().zip(&ours.legs).enumerate() {
        let ctx = format!("path {path} leg {k}");
        assert_eq!(g.map, o.map, "{ctx} map");
        let bits =
            |v: &[[f32; 3]]| -> Vec<u32> { v.iter().flatten().map(|f| f.to_bits()).collect() };
        assert_eq!(bits(&g.points), bits(&o.points), "{ctx} control points");
        let lb = |v: &[f32]| -> Vec<u32> { v.iter().map(|f| f.to_bits()).collect() };
        assert_eq!(lb(&g.lengths), lb(&o.lengths), "{ctx} segment lengths");
        assert_eq!(g.length.to_bits(), o.length.to_bits(), "{ctx} length");
        assert_eq!(
            (g.start, g.travel, g.end),
            (o.start, o.travel, o.end),
            "{ctx} start, travel, end"
        );
        assert_eq!(g.stops.len(), o.stops.len(), "{ctx} stop count");
        for (j, (gs, os)) in g.stops.iter().zip(&o.stops).enumerate() {
            assert_eq!(
                (gs.time, gs.distance.to_bits(), gs.wait),
                (os.time, os.distance.to_bits(), os.wait),
                "{ctx} stop {j}"
            );
        }
    }
    assert_eq!(
        (got.speed.to_bits(), got.accel.to_bits(), got.period),
        (ours.speed.to_bits(), ours.accel.to_bits(), ours.period),
        "path {path} speed, acceleration, period"
    );
    r
}

/// The client's builder against `build` on 300 random paths: the legs' maps, control points,
/// segment lengths, lengths, start, travel and end times, the stop tables and the period.
#[test]
fn ship_path_build_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x5f4c_c0a1u32;
    for case in 0..300u32 {
        let mut e = setup(&wow);
        let path = 200 + case;
        let rows = make_rows(&mut s, path);
        let speed = rand_range(&mut s, 1, 60);
        let accel = rand_range(&mut s, 1, 12);
        check_build(&mut e, &rows, path, speed, accel);
    }
}

/// The client's tick against `pose` on 120 random paths, 60 times each, spread over the period and
/// probed at every stop's arrival and departure and every leg's ends: the leg it stores at `+0x58`
/// and, when the leg's map is the current one, the position `+0x44`, the facing `+0x50` and the
/// animation it plays. A quarter of the probes run on another map, where nothing but `+0x58` moves.
#[test]
fn ship_path_tick_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x5f50_a0b2u32;
    let mut seen = [0u32; 5];
    for case in 0..120u32 {
        let mut e = setup(&wow);
        let path = 500 + case;
        let rows = make_rows(&mut s, path);
        let speed = rand_range(&mut s, 1, 60);
        let accel = rand_range(&mut s, 1, 12);
        let r = check_build(&mut e, &rows, path, speed, accel);
        let ours = build(&rows, path, speed, accel);

        let map_va = r.map;
        e.stub_fn(0x468580, 0, move |io| io.read_u32(map_va));
        let camera = e.alloc(0x100);
        e.stub(0x4818f0, camera as u32, 0);
        e.stub(0x7bdc40, 0, 1);
        e.stub(0x7bdd60, 0, 3);
        e.stub(0x6709a0, 0, 2);
        e.stub(0x670950, 1, 0);
        e.stub(0x670a30, 0, 2);
        e.stub(0x6709f0, 0, 1);

        let mut times: Vec<u32> = Vec::new();
        for leg in &ours.legs {
            for t in [leg.start, leg.end.wrapping_sub(1)] {
                times.push(t);
            }
            for st in &leg.stops {
                for t in [
                    st.time,
                    st.time.wrapping_sub(1),
                    st.time.wrapping_add(st.wait),
                ] {
                    times.push(t);
                }
            }
        }
        for _ in 0..60 {
            times.push(next(&mut s) % ours.period.max(1));
        }
        for t in times {
            let offset = next(&mut s);
            let now = t
                .wrapping_sub(offset)
                .wrapping_add(ours.period.wrapping_mul(next(&mut s) % 3));
            let want = pose(&ours, now, offset);
            let on_map = !next(&mut s).is_multiple_of(4);
            let map = match want.map {
                Some(m) if on_map => m,
                Some(m) => m + 7,
                None => 99,
            };
            e.write_u32(r.map, map);
            e.write_u32(r.go + 0x278, offset);
            for k in 0..4 {
                e.write_u32(r.handler + 0x44 + 4 * k, 0x7fa5_a5a5);
            }
            e.write_u32(r.handler + 0x54, 0xfff0);
            e.write_u32(r.handler + 0x58, 0xfff1);
            e.clear_calls();
            e.call(TICK, r.handler as u32, 0, &[now, 0]);
            let ctx = format!("path {path} t {t} now {now} offset {offset}");
            assert_eq!(e.read_u32(r.handler + 0x58), want.leg, "{ctx}: leg");
            let pos = read_f32s(&e, r.handler + 0x44, 3);
            let facing = e.read_u32(r.handler + 0x50);
            if want.map.is_some() && on_map {
                assert_eq!(pos, want.position.map(f32::to_bits), "{ctx}: position");
                assert_eq!(facing, want.facing.to_bits(), "{ctx}: facing");
                let play = e
                    .calls()
                    .iter()
                    .find(|c| c.addr == 0x6709f0)
                    .expect("the animation plays");
                assert_eq!(e.read_u32(r.handler + 0x54), want.anim, "{ctx}: animation");
                assert_eq!(
                    play.args[0], 0,
                    "{ctx}: PlayEntityAnimation's stack argument"
                );
                seen[match want.anim {
                    0 => 0,
                    0xa2 => 1,
                    0xa3 => 2,
                    _ => 3,
                }] += 1;
            } else {
                assert_eq!(pos, vec![0x7fa5_a5a5; 3], "{ctx}: no move off the map");
                assert_eq!(facing, 0x7fa5_a5a5, "{ctx}: no turn off the map");
                seen[4] += 1;
            }
        }
    }
    eprintln!("at a stop, speeding up, cruising, slowing down, off the map: {seen:?}");
    assert!(
        seen.iter().all(|&n| n > 0),
        "every regime is probed: {seen:?}"
    );
}

/// The install's `TaxiPathNode.dbc` through both: the nine ship paths a server sails, at the
/// speeds and accelerations it gives them (30 and 1, and 1 and 1 for path 436), built by the client
/// and by `build`, and each ship placed at 200 times. Reads the install, so it skips without it.
#[test]
fn ship_path_install_paths() {
    let Some(dir) = data_dir() else {
        return;
    };
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut vfs = benilla_re_storm::Vfs::mount_data_dir(&dir).expect("mount the install");
    let dbc = vfs
        .read(r"DBFilesClient\TaxiPathNode.dbc")
        .expect("read TaxiPathNode.dbc");
    assert_eq!(&dbc[0..4], b"WDBC");
    let count = u32::from_le_bytes(dbc[4..8].try_into().unwrap()) as usize;
    assert_eq!(u32::from_le_bytes(dbc[12..16].try_into().unwrap()), 36);
    let word = |i: usize, k: usize| -> u32 {
        let o = 20 + 36 * i + 4 * k;
        u32::from_le_bytes(dbc[o..o + 4].try_into().unwrap())
    };
    let rows: Vec<TaxiNode> = (0..count)
        .map(|i| TaxiNode {
            path: word(i, 1),
            map: word(i, 3),
            position: [4, 5, 6].map(|k| f32::from_bits(word(i, k))),
            flags: word(i, 7),
            delay: word(i, 8),
        })
        .collect();
    let mut s = 0x5f4c_d0c3u32;
    for (path, speed, accel) in [
        (241, 30, 1),
        (285, 30, 1),
        (292, 30, 1),
        (293, 30, 1),
        (295, 30, 1),
        (301, 30, 1),
        (302, 30, 1),
        (303, 30, 1),
        (436, 1, 1),
    ] {
        let mut e = setup(&wow);
        let r = check_build(&mut e, &rows, path, speed, accel);
        let ours = build(&rows, path, speed, accel);
        let shape: Vec<(u32, usize, usize)> = ours
            .legs
            .iter()
            .map(|l| (l.map, l.points.len(), l.stops.len() - 1))
            .collect();
        eprintln!(
            "path {path}: period {} ms, legs (map, nodes, stops) {shape:?}",
            ours.period
        );
        let map_va = r.map;
        e.stub_fn(0x468580, 0, move |io| io.read_u32(map_va));
        let camera = e.alloc(0x100);
        e.stub(0x4818f0, camera as u32, 0);
        e.stub(0x7bdc40, 0, 1);
        e.stub(0x7bdd60, 0, 3);
        e.stub(0x6709a0, 0, 2);
        e.stub(0x670950, 1, 0);
        e.stub(0x670a30, 0, 2);
        e.stub(0x6709f0, 0, 1);
        for _ in 0..200 {
            let now = next(&mut s);
            let want = pose(&ours, now, 0);
            e.write_u32(r.map, want.map.unwrap());
            e.write_u32(r.go + 0x278, 0);
            e.write_u32(r.handler + 0x54, 0xfff0);
            e.call(TICK, r.handler as u32, 0, &[now, 0]);
            let ctx = format!("path {path} now {now}");
            assert_eq!(e.read_u32(r.handler + 0x58), want.leg, "{ctx}: leg");
            let pos = read_f32s(&e, r.handler + 0x44, 3);
            assert_eq!(pos, want.position.map(f32::to_bits), "{ctx}: position");
            assert_eq!(
                e.read_u32(r.handler + 0x50),
                want.facing.to_bits(),
                "{ctx}: facing"
            );
            assert_eq!(e.read_u32(r.handler + 0x54), want.anim, "{ctx}: animation");
        }
    }
}
