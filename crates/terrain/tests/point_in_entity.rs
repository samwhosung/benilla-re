//! `CWorld::IsPointInEntity` (`0x670a70`), `__fastcall(ecx = entity, edx = point)` returning `al`,
//! run whole with its callees: `PointInsidePlanes` (`0x6a4990`), `CM2Model::GetBoundingBox`
//! (`0x713640`), `InitPlaneUp` (`0x6376a0`) and `DotVector3` (`0x602630`) run as the client's.
//! `CM2Model::IsLoaded` (`0x7103d0`, `ret 8`) is a stub that answers a word the test sets: its
//! answer reaches the result only through `0x670ab7`'s zero test, which both answers exercise.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_terrain::entity::{is_point_in_entity, EntityVolume};
use common::{frand, next};

const IS_POINT_IN_ENTITY: u64 = 0x670a70;
const IS_LOADED: u64 = 0x7103d0;

struct Layout {
    entity: u64,
    point: u64,
    root: u64,
    planes: u64,
    model: u64,
    header: u64,
    loaded_word: u64,
}

fn write_f32s(e: &mut Emu, va: u64, v: &[f32]) {
    for (i, x) in v.iter().enumerate() {
        e.write_u32(va + 4 * i as u64, x.to_bits());
    }
}

fn run(e: &mut Emu, l: &Layout, flags: u32, volume: &EntityVolume<'_>, p: [f32; 3], ctx: &str) {
    e.write_u32(l.entity + 0x8, flags);
    e.write_u32(l.entity + 0x118, l.root as u32);
    match *volume {
        EntityVolume::MapObject { loaded, planes } => {
            e.write(l.root + 0x1d4, &[u8::from(loaded)]);
            e.write_u32(l.root + 0x194, planes.len() as u32);
            e.write_u32(l.root + 0x15c, l.planes as u32);
            for (k, plane) in planes.iter().enumerate() {
                write_f32s(e, l.planes + 0x10 * k as u64, plane);
            }
        }
        EntityVolume::Model {
            present,
            loaded,
            bbox,
        } => {
            e.write_u32(l.entity + 0x88, if present { l.model as u32 } else { 0 });
            e.write_u32(l.loaded_word, u32::from(loaded));
            write_f32s(e, l.header + 0xb4, &bbox);
        }
        EntityVolume::Other => {}
    }
    write_f32s(e, l.point, &p);
    let binary = e.call(IS_POINT_IN_ENTITY, l.entity as u32, l.point as u32, &[]) & 0xff;
    let ours = u32::from(is_point_in_entity(volume, p));
    assert_eq!(
        binary, ours,
        "{ctx}: flags={flags:#x} p={p:?} volume={volume:?}"
    );
}

/// A coordinate near a box face: the face itself, a float step either side, or anywhere.
fn near(s: &mut u32, face: f32) -> f32 {
    match next(s) % 5 {
        0 => face,
        1 => f32::from_bits(face.to_bits().wrapping_add(1)),
        2 => f32::from_bits(face.to_bits().wrapping_sub(1)),
        _ => face + frand(s, -4.0, 4.0),
    }
}

#[test]
fn point_in_entity_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let l = Layout {
        entity: e.alloc(0x200),
        point: e.alloc(0x10),
        root: e.alloc(0x200),
        planes: e.alloc(0x10 * 16),
        model: e.alloc(0x40),
        header: e.alloc(0x100),
        loaded_word: e.alloc(4),
    };
    // The model's shared record at `+0x30`, its header at `+0x130`; `+0x10` set so the box read
    // skips its lazy setup (`0x71364c`).
    let shared = e.alloc(0x200);
    e.write_u32(l.model + 0x10, 1);
    e.write_u32(l.model + 0x30, shared as u32);
    e.write_u32(shared + 0x130, l.header as u32);
    let word = l.loaded_word;
    e.stub_fn(IS_LOADED, 2, move |io| io.read_u32(word));

    let mut s = 0x670a_7000u32;
    let mut cases = 0u32;
    let unit_box = [-1.0f32, -2.0, -3.0, 4.0, 5.0, 6.0];

    // No tag bit, and tag bits the test does not read, answer true.
    for flags in [0u32, 0x20, 0x4, 0x2000, 0x80] {
        run(
            &mut e,
            &l,
            flags,
            &EntityVolume::Other,
            [1e30, -1e30, 0.0],
            "other",
        );
        cases += 1;
    }
    // A model: absent, unloaded, then each face of the box exactly, a step outside, inside, NaN.
    let model = |present, loaded, bbox| EntityVolume::Model {
        present,
        loaded,
        bbox,
    };
    for (present, loaded) in [(false, true), (true, false), (true, true)] {
        run(
            &mut e,
            &l,
            0x40,
            &model(present, loaded, unit_box),
            [0.0, 0.0, 0.0],
            "gates",
        );
        cases += 1;
    }
    for axis in 0..3 {
        for (face, sign) in [(unit_box[axis], -1i32), (unit_box[axis + 3], 1)] {
            for step in [-1i32, 0, 1] {
                let mut p = [0.0f32; 3];
                let bits = face.to_bits() as i32;
                // a step away from the box's centre is outside.
                let moved = if face == 0.0 {
                    face
                } else {
                    f32::from_bits((bits + step * sign * face.signum() as i32) as u32)
                };
                p[axis] = moved;
                run(&mut e, &l, 0x40, &model(true, true, unit_box), p, "face");
                cases += 1;
            }
        }
    }
    for p in [
        [f32::NAN, 100.0, 100.0],
        [100.0, f32::NAN, 0.0],
        [f32::INFINITY, 0.0, 0.0],
        [0.0, 0.0, f32::NEG_INFINITY],
        [-0.0, -0.0, -0.0],
    ] {
        run(&mut e, &l, 0x40, &model(true, true, unit_box), p, "special");
        // Bits beside 0x40 that the dispatch does not read.
        run(
            &mut e,
            &l,
            0x2060,
            &model(true, true, unit_box),
            p,
            "model with other bits",
        );
        cases += 2;
    }
    // A degenerate and an inverted box, and a box with a NaN corner.
    for bbox in [
        [0.0f32, 0.0, 0.0, 0.0, 0.0, 0.0],
        [1.0, 1.0, 1.0, -1.0, -1.0, -1.0],
        [f32::NAN, -1.0, -1.0, 1.0, 1.0, 1.0],
        [-0.0, -0.0, -0.0, 0.0, 0.0, 0.0],
    ] {
        for p in [[0.0f32, 0.0, 0.0], [-0.0, 0.0, -0.0], [0.5, 0.5, 0.5]] {
            run(&mut e, &l, 0x40, &model(true, true, bbox), p, "box shape");
            cases += 1;
        }
    }
    // Random boxes and points near their faces.
    for i in 0..3000 {
        let lo = [
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -500.0, 500.0),
            frand(&mut s, -50.0, 50.0),
        ];
        let size = [
            frand(&mut s, 0.0, 300.0),
            frand(&mut s, 0.0, 300.0),
            frand(&mut s, 0.0, 80.0),
        ];
        let bbox = [
            lo[0],
            lo[1],
            lo[2],
            lo[0] + size[0],
            lo[1] + size[1],
            lo[2] + size[2],
        ];
        let mut p = [0.0f32; 3];
        for (axis, c) in p.iter_mut().enumerate() {
            let face = if next(&mut s) & 1 == 0 {
                bbox[axis]
            } else {
                bbox[axis + 3]
            };
            *c = near(&mut s, face);
        }
        run(
            &mut e,
            &l,
            0x40,
            &model(true, true, bbox),
            p,
            &format!("model fuzz {i}"),
        );
        cases += 1;
    }

    // A map object: unloaded, no planes, and random convex plane sets.
    let up: [[f32; 4]; 2] = [[0.0, 0.0, 1.0, -1.0], [0.0, 0.0, -1.0, 0.0]];
    for loaded in [false, true] {
        let v = EntityVolume::MapObject {
            loaded,
            planes: &up,
        };
        for z in [-0.5f32, 0.0, 0.5, 1.0, 1.000_000_1, f32::NAN] {
            run(&mut e, &l, 0x8, &v, [0.0, 0.0, z], "map object slab");
            run(
                &mut e,
                &l,
                0x48,
                &v,
                [0.0, 0.0, z],
                "map object over model bit",
            );
            cases += 2;
        }
    }
    let none: [[f32; 4]; 0] = [];
    run(
        &mut e,
        &l,
        0x8,
        &EntityVolume::MapObject {
            loaded: true,
            planes: &none,
        },
        [1e30, 1e30, 1e30],
        "no planes",
    );
    cases += 1;
    for i in 0..2000 {
        let n = 1 + (next(&mut s) % 16) as usize;
        let mut planes = vec![[0.0f32; 4]; n];
        for plane in &mut planes {
            *plane = [
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -1.0, 1.0),
                frand(&mut s, -50.0, 10.0),
            ];
        }
        let p = [
            frand(&mut s, -40.0, 40.0),
            frand(&mut s, -40.0, 40.0),
            frand(&mut s, -40.0, 40.0),
        ];
        let v = EntityVolume::MapObject {
            loaded: true,
            planes: &planes,
        };
        run(&mut e, &l, 0x8, &v, p, &format!("map object fuzz {i}"));
        cases += 1;
    }
    eprintln!("point_in_entity: {cases} cases bit-exact vs WoW.exe @ 0x670a70");
}
