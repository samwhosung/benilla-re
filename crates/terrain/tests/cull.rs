//! The cull volumes and the portal rectangle, each run whole: `BuildFrustumVolume` (`0x682490`)
//! and `BuildPortalVolume` (`0x682930`) with the scratch reserve (`0x4078e0`) and the `qsort`
//! (`0x4044b0`) stubbed, through `0x6865f0` and `0x686640`; `TestOrientedBoxPlanes` (`0x6869c0`);
//! and `ProjectPortal` (`0x6b46f0`) on its x87 path, through `TransformVector4` (`0x7bcb40`),
//! `ClipPortalPolygon` (`0x6b4a80`), `0x7bf680` and `IsPointInPolygon` (`0x7c23e0`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_terrain::cull::{
    frustum_push_rect, frustum_root_rect, obb_plane_cull, portal_screen_rect, Obb, PortalRect,
};
use common::assert_f32;

/// The LCG over finite floats near 1; choices from the high bits.
struct Lcg(u32);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0
    }
    /// A finite float in [-mag, mag) from the high bits.
    fn coord(&mut self, mag: f32) -> f32 {
        let u = (self.next() >> 8) as f32 / (1u32 << 24) as f32; // [0, 1)
        u * 2.0 * mag - mag
    }
    fn unit(&mut self) -> f32 {
        (self.next() >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// Where the corners land: the pooled volume `0xc7d308 + 0xfc·[0xc7cfe0]`, index 0 here, its
/// `+0x60`.
const FRUSTUM_THIS: u64 = 0x00c7_d308;

const CORNERS_OFF: u64 = 0x60;

const POOL_INDEX_VA: u64 = 0x00c7_cfe0;

/// The corner rays at `0xc7bcd8`, eight vectors.
const RAYS_VA: u64 = 0x00c7_bcd8;

fn write_rays(emu: &mut Emu, rays: &[[f32; 3]; 8]) {
    for (i, r) in rays.iter().enumerate() {
        for (c, &f) in r.iter().enumerate() {
            emu.write_u32(RAYS_VA + (i * 3 + c) as u64 * 4, f.to_bits());
        }
    }
}

fn read_committed_corners(emu: &Emu) -> [[f32; 3]; 8] {
    let mut out = [[0.0f32; 3]; 8];
    for (i, corner) in out.iter_mut().enumerate() {
        let base = FRUSTUM_THIS + CORNERS_OFF + i as u64 * 0xc;
        for (c, slot) in corner.iter_mut().enumerate() {
            *slot = emu.read_f32(base + c as u64 * 4);
        }
    }
    out
}

/// Both builders: the scratch reserve `0x4078e0` (eight pushes of NaN) and the `qsort` `0x4044b0`
/// of the unread copy do nothing, and the pool index is 0.
fn setup_corner_emu(emu: &mut Emu) {
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.write_u32(POOL_INDEX_VA, 0); // so this = 0xc7d308
    emu.stub(0x4078e0, 0, 1); // the scratch reserve
    emu.stub(0x4044b0, 0, 4); // the qsort, 4 `__cdecl` arguments
}

const VIEW_PROJ_VA: u64 = 0x00cb_e2d8;

const GUARD_PLANES_VA: u64 = 0x00c7_d0dc; // plane 0's ny, where `ClipPortalPolygon` starts its [esi]

const EYE_VA: u64 = 0x00ca_7edc;

const RSQRT_GATE_VA: u64 = 0x00c9_e388;

fn write_view_proj(emu: &mut Emu, m: &[f32; 16]) {
    for (i, &f) in m.iter().enumerate() {
        emu.write_u32(VIEW_PROJ_VA + i as u64 * 4, f.to_bits());
    }
}

/// Writes the four planes: plane k's nx at `0xc7d0d8 + 0x10·k`, as in `tests/clip.rs`.
fn write_guard_planes(emu: &mut Emu, planes: &[[f32; 3]; 4]) {
    let base = GUARD_PLANES_VA - 4; // plane 0's nx
    for (k, pl) in planes.iter().enumerate() {
        let b = base + k as u64 * 0x10;
        emu.write_u32(b, pl[0].to_bits());
        emu.write_u32(b + 4, pl[1].to_bits());
        emu.write_u32(b + 8, pl[2].to_bits());
        emu.write_u32(b + 0xc, 0);
    }
}

#[test]
fn frustum_root_rect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    setup_corner_emu(&mut emu);
    let rect_ptr = emu.alloc(0x10);
    let mut rng = Lcg(0x1357_9bdf);
    let mut checked = 0u32;
    for _ in 0..3000 {
        // rays from near the eye to tips about 10 units out
        let mut rays = [[0.0f32; 3]; 8];
        for r in rays.iter_mut() {
            *r = [rng.coord(10.0), rng.coord(10.0), rng.coord(10.0)];
        }
        // the rectangle [x0, y1, x2, y3], fractions in [0, 1]
        let rect = [rng.unit(), rng.unit(), rng.unit(), rng.unit()];
        write_rays(&mut emu, &rays);
        for (i, &f) in rect.iter().enumerate() {
            emu.write_u32(rect_ptr + i as u64 * 4, f.to_bits());
        }
        // the target cleared, so a missing write shows
        for i in 0..24 {
            emu.write_u32(FRUSTUM_THIS + CORNERS_OFF + i * 4, 0xdead_beef);
        }
        // ecx = the rectangle
        emu.call(0x682490, rect_ptr as u32, 0, &[]);
        let bin = read_committed_corners(&emu);
        let ours = frustum_root_rect(&rays, &rect);
        for (ci, (b, o)) in bin.iter().zip(ours.iter()).enumerate() {
            for c in 0..3 {
                assert_f32(b[c], o[c], &format!("root_rect corner[{ci}].{c}"));
            }
        }
        checked += 1;
    }
    eprintln!("frustum_root_rect: {checked} cull volumes bit-exact vs WoW.exe @ 0x682490");
}

#[test]
fn frustum_push_rect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    setup_corner_emu(&mut emu);
    let rect_ptr = emu.alloc(0x10);
    let mut rng = Lcg(0x2468_ace0);
    let mut checked = 0u32;
    for _ in 0..3000 {
        let mut rays = [[0.0f32; 3]; 8];
        for r in rays.iter_mut() {
            *r = [rng.coord(10.0), rng.coord(10.0), rng.coord(10.0)];
        }
        let rect = [rng.unit(), rng.unit(), rng.unit(), rng.unit()];
        write_rays(&mut emu, &rays);
        for (i, &f) in rect.iter().enumerate() {
            emu.write_u32(rect_ptr + i as u64 * 4, f.to_bits());
        }
        for i in 0..24 {
            emu.write_u32(FRUSTUM_THIS + CORNERS_OFF + i * 4, 0xdead_beef);
        }
        // ecx = the rays (0xc7bcd8), edx = the rectangle
        emu.call(0x682930, RAYS_VA as u32, rect_ptr as u32, &[]);
        let bin = read_committed_corners(&emu);
        let ours = frustum_push_rect(&rays, &rect);
        for (ci, (b, o)) in bin.iter().zip(ours.iter()).enumerate() {
            for c in 0..3 {
                assert_f32(b[c], o[c], &format!("push_rect corner[{ci}].{c}"));
            }
        }
        checked += 1;
    }
    eprintln!("frustum_push_rect: {checked} cull volumes bit-exact vs WoW.exe @ 0x682930");
}

#[test]
fn obb_plane_cull_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let planes_ptr = emu.alloc(6 * 0x10);
    let corners_ptr = emu.alloc(0x18); // minimum, maximum
    let axes_ptr = emu.alloc(0x24); // 9 floats
    let center_ptr = emu.alloc(0xc);
    let mut rng = Lcg(0x0bad_f00d);

    let mut checked = 0u32;
    let (mut hit_out, mut hit_in) = (0u32, 0u32);
    let (mut pick_min, mut pick_max) = (0u32, 0u32);

    for case in 0..6000usize {
        // Six planes with normals near 1 and offsets moving the box in and out, some sets tight and
        // some loose.
        let mut planes = [[0.0f32; 4]; 6];
        for p in planes.iter_mut() {
            *p = [
                rng.coord(1.0),
                rng.coord(1.0),
                rng.coord(1.0),
                rng.coord(8.0),
            ];
        }
        // a box with min below max, its axes a rotation in some cases and the identity in others
        let cx = rng.coord(5.0);
        let cy = rng.coord(5.0);
        let cz = rng.coord(5.0);
        let hx = rng.unit() * 2.0 + 0.1;
        let hy = rng.unit() * 2.0 + 0.1;
        let hz = rng.unit() * 2.0 + 0.1;
        let min = [-hx, -hy, -hz];
        let max = [hx, hy, hz];
        let axes = if case % 3 == 0 {
            [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
        } else {
            // any finite basis: the test is defined for any 3×3
            let mut a = [0.0f32; 9];
            for v in a.iter_mut() {
                *v = rng.coord(1.0);
            }
            a
        };
        let center = [cx, cy, cz];

        for (i, p) in planes.iter().enumerate() {
            for (c, &f) in p.iter().enumerate() {
                emu.write_u32(planes_ptr + (i * 4 + c) as u64 * 4, f.to_bits());
            }
        }
        for (c, &f) in min.iter().chain(max.iter()).enumerate() {
            emu.write_u32(corners_ptr + c as u64 * 4, f.to_bits());
        }
        for (c, &f) in axes.iter().enumerate() {
            emu.write_u32(axes_ptr + c as u64 * 4, f.to_bits());
        }
        for (c, &f) in center.iter().enumerate() {
            emu.write_u32(center_ptr + c as u64 * 4, f.to_bits());
        }

        // ecx = planes; the corners, axes and centre on the stack
        let ret = emu.call(
            0x6869c0,
            planes_ptr as u32,
            0,
            &[corners_ptr as u32, axes_ptr as u32, center_ptr as u32],
        );
        let obb = Obb {
            corners: [min, max],
            axes,
            center,
        };
        let ours = obb_plane_cull(&planes, &obb);
        assert_eq!(
            ret, ours,
            "obb_plane_cull case {case}: ret binary={ret} ours={ours}"
        );
        if ours == 0 {
            hit_out += 1;
        } else {
            hit_in += 1;
        }
        // count the corners picked each way, from the reimplementation
        if case % 3 == 0 {
            // identity axes: the local normal is the normal
            for p in &planes {
                if p[0].to_bits() >> 31 == 1 {
                    pick_min += 1;
                } else {
                    pick_max += 1;
                }
            }
        }
        checked += 1;
    }
    assert!(hit_out > 0, "no OBB reject case fired");
    assert!(hit_in > 0, "no OBB accept case fired");
    assert!(
        pick_min > 0 && pick_max > 0,
        "support-corner sign branches not both hit"
    );
    eprintln!(
        "obb_plane_cull: {checked} cases bit-exact vs WoW.exe @ 0x6869c0 \
         (reject {hit_out}, accept {hit_in}; pick min {pick_min}/max {pick_max})"
    );
}

#[test]
fn portal_screen_rect_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.write_u32(RSQRT_GATE_VA, 0); // the x87 path
                                     // The axis table at 0xcf5c7c and 0xcf5c80 that 0x7c23e0 reads is zero in the image until
                                     // 0x7c18d0 runs: per axis k, [1, 2], [2, 0], [0, 1].
    let lut = [(1u32, 2u32), (2, 0), (0, 1)];
    for (k, (u, v)) in lut.iter().enumerate() {
        emu.write_u32(0xcf5c7c + k as u64 * 8, *u);
        emu.write_u32(0xcf5c80 + k as u64 * 8, *v);
    }
    // `_atexit` (0x409aef) is `__cdecl` with no `ret N`, so its stub pops nothing; `0x6b4a80`
    // calls it before `push ebx`, and popping more would move the saved ebx.
    emu.stub(0x409aef, 0, 0);

    // the pyramid ±1/√2 about +z that 0x680610 builds
    let inv = 1.0f32 / 2.0f32.sqrt();
    let guard: [[f32; 3]; 4] = [
        [inv, 0.0, inv],
        [-inv, 0.0, inv],
        [0.0, inv, inv],
        [0.0, -inv, inv],
    ];
    write_guard_planes(&mut emu, &guard);

    // a perspective of sorts: the identity with w taken from z
    let view_proj: [f32; 16] = [
        1.0, 0.0, 0.0, 0.0, //
        0.0, 1.0, 0.0, 0.0, //
        0.0, 0.0, 1.0, 1.0, // z to w
        0.0, 0.0, 0.0, 1.0,
    ];
    write_view_proj(&mut emu, &view_proj);

    let eye_pos = [0.0f32, 0.0, 0.0];
    for (i, &f) in eye_pos.iter().enumerate() {
        emu.write_u32(EYE_VA + i as u64 * 4, f.to_bits());
    }

    // the owner: only its +0x134, the vertices, is read
    let owner = emu.alloc(0x200);
    let vert_pool = emu.alloc(0x400); // up to about 85 vertices, 12 bytes apart
    emu.write_u32(owner + 0x134, vert_pool as u32);
    let portal_desc = emu.alloc(8); // two u16, start and count
    let out = emu.alloc(0x20);

    let mut rng = Lcg(0x5eed_1234);
    let mut checked = 0u32;
    let (mut culled_n, mut visible_n) = (0u32, 0u32);
    let (mut eye_on, mut wclamp) = (0u32, 0u32);

    // The eye on the portal's plane: portals on the plane y = −z through the origin, with the eye's
    // projection inside (the whole screen) and outside (the bounds).
    let eye_probes: [(&[[f32; 3]], &str); 2] = [
        // a quad about the origin's projection on y = −z: the eye inside
        (
            &[
                [-1.0, -1.0, 1.0],
                [1.0, -1.0, 1.0],
                [1.0, -2.0, 2.0],
                [-1.0, -2.0, 2.0],
            ],
            "inside",
        ),
        // the same plane, moved along +x: the eye outside
        (
            &[
                [3.0, -1.0, 1.0],
                [5.0, -1.0, 1.0],
                [5.0, -2.0, 2.0],
                [3.0, -2.0, 2.0],
            ],
            "outside",
        ),
    ];
    // The eye at (0, −1.5, 1.5), on y = −z; along z it projects to (0, −1.5), inside the first
    // quad's x in [−1, 1], y in [−2, −1], and outside the second's x in [3, 5].
    let eye_on_plane = [0.0f32, -1.5, 1.5];
    for (i, &f) in eye_on_plane.iter().enumerate() {
        emu.write_u32(EYE_VA + i as u64 * 4, f.to_bits());
    }
    for (probe, tag) in eye_probes {
        let verts: Vec<[f32; 3]> = probe.to_vec();
        for (i, v) in verts.iter().enumerate() {
            for (c, &f) in v.iter().enumerate() {
                emu.write_u32(vert_pool + (i * 3 + c) as u64 * 4, f.to_bits());
            }
        }
        emu.write_u32(portal_desc, (verts.len() as u32) << 16);
        for i in 0..8 {
            emu.write_u32(out + i * 4, 0);
        }
        emu.call(0x6b46f0, owner as u32, 0, &[portal_desc as u32, out as u32]);
        let bin_culled = emu.read_u32(out) & 1 != 0;
        let bin_rect = [
            emu.read_f32(out + 8),
            emu.read_f32(out + 4),
            emu.read_f32(out + 0x10),
            emu.read_f32(out + 0xc),
        ];
        let ours = portal_screen_rect(&verts, &view_proj, &guard, &eye_on_plane, 0);
        assert_eq!(bin_culled, ours.culled, "eye-probe {tag}: culled");
        for (c, (b, o)) in bin_rect.iter().zip(ours.rect.iter()).enumerate() {
            assert_f32(*b, *o, &format!("eye-probe {tag} rect[{c}]"));
        }
        if ours.rect == [-1.0, -1.0, 1.0, 1.0] && !ours.culled {
            eye_on += 1;
        }
        checked += 1;
    }
    // The threshold: a portal in the plane x = 0, so n = (±1, 0, 0) and |d| = |ex|; the eye in
    // its y and z inside the quad. ex = 0.01f is on the plane (`<=`), one ulp up is not, one down is.
    {
        let quad: [[f32; 3]; 4] = [
            [0.0, -1.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 1.0, 2.0],
            [0.0, -1.0, 2.0],
        ];
        for (i, v) in quad.iter().enumerate() {
            for (c, &f) in v.iter().enumerate() {
                emu.write_u32(vert_pool + (i * 3 + c) as u64 * 4, f.to_bits());
            }
        }
        emu.write_u32(portal_desc, (quad.len() as u32) << 16);
        let eps = 0.01f32;
        for (ex, on) in [
            (f32::from_bits(eps.to_bits() - 1), true),
            (eps, true),
            (f32::from_bits(eps.to_bits() + 1), false),
        ] {
            let eye = [ex, 0.0f32, 1.5];
            for (i, &f) in eye.iter().enumerate() {
                emu.write_u32(EYE_VA + i as u64 * 4, f.to_bits());
            }
            for i in 0..8 {
                emu.write_u32(out + i * 4, 0);
            }
            emu.call(0x6b46f0, owner as u32, 0, &[portal_desc as u32, out as u32]);
            let bin_rect = [
                emu.read_f32(out + 8),
                emu.read_f32(out + 4),
                emu.read_f32(out + 0x10),
                emu.read_f32(out + 0xc),
            ];
            let ours = portal_screen_rect(&quad, &view_proj, &guard, &eye, 0);
            assert_eq!(
                bin_rect == [-1.0, -1.0, 1.0, 1.0],
                on,
                "threshold ex bits {:#x}: binary",
                ex.to_bits()
            );
            assert_eq!(
                ours.rect == [-1.0, -1.0, 1.0, 1.0],
                on,
                "threshold ex bits {:#x}: ours",
                ex.to_bits()
            );
            for (c, (b, o)) in bin_rect.iter().zip(ours.rect.iter()).enumerate() {
                assert_f32(*b, *o, &format!("threshold {:#x} rect[{c}]", ex.to_bits()));
            }
            checked += 1;
        }
    }
    // the eye back at the origin for the random cases
    for (i, &f) in eye_pos.iter().enumerate() {
        emu.write_u32(EYE_VA + i as u64 * 4, f.to_bits());
    }

    // the fixed cases, then random ones
    for case in 0..4000usize {
        // 3 to 6 vertices in front of the eye (z > 0), near 1
        let count = 3 + (case % 4);
        let mut verts: Vec<[f32; 3]> = Vec::with_capacity(count);
        // a flat portal (constant z) in some cases, for the eye on its plane
        let planar = case % 5 == 0;
        // every seventh portal far to one side, so the pyramid clips it away and it is culled
        let off_screen = case % 7 == 3;
        let pz = rng.unit() * 2.0 + 0.5;
        for _ in 0..count {
            let z = if planar { pz } else { rng.unit() * 3.0 + 0.3 };
            if off_screen {
                // x in [10, 14]; the plane x <= z drops every vertex
                verts.push([rng.unit() * 4.0 + 10.0, rng.coord(0.5), z]);
            } else {
                verts.push([rng.coord(2.0), rng.coord(2.0), z]);
            }
        }
        // the vertices from index 0
        for (i, v) in verts.iter().enumerate() {
            for (c, &f) in v.iter().enumerate() {
                emu.write_u32(vert_pool + (i * 3 + c) as u64 * 4, f.to_bits());
            }
        }
        emu.write_u32(portal_desc, (count as u32) << 16); // start 0, then the count
        for i in 0..8 {
            emu.write_u32(out + i * 4, 0);
        }

        // ecx = owner, the descriptor and out on the stack
        emu.call(0x6b46f0, owner as u32, 0, &[portal_desc as u32, out as u32]);

        // bit 0 of the first byte is culled; out+8 min x, +4 min y, +0x10 max x, +0xc max y
        let flags = emu.read_u32(out) & 0xff;
        let bin_culled = flags & 1 != 0;
        let bin_rect = [
            emu.read_f32(out + 8),
            emu.read_f32(out + 4),
            emu.read_f32(out + 0x10),
            emu.read_f32(out + 0xc),
        ];

        let ours: PortalRect = portal_screen_rect(&verts, &view_proj, &guard, &eye_pos, 0);

        let ours_culled = ours.culled;
        assert_eq!(
            bin_culled, ours_culled,
            "portal case {case}: culled binary={bin_culled} ours={ours_culled}"
        );
        if ours.culled {
            culled_n += 1;
        } else {
            visible_n += 1;
            for (c, (b, o)) in bin_rect.iter().zip(ours.rect.iter()).enumerate() {
                assert_f32(*b, *o, &format!("portal case {case} rect[{c}]"));
            }
            if ours.rect == [-1.0, -1.0, 1.0, 1.0] {
                eye_on += 1;
            }
            // a tiny clipped w is counted
            if planar {
                wclamp += 1; // flat portals often give a w near zero
            }
        }
        checked += 1;
    }
    assert!(culled_n > 0, "no portal cull (clip-empty) case fired");
    assert!(visible_n > 0, "no portal visible case fired");
    assert!(
        eye_on > 0,
        "eye-on-plane fills-screen branch (eye_in_polygon TRUE) never fired"
    );
    let _ = wclamp;
    eprintln!(
        "portal_screen_rect: {checked} portals bit-exact vs WoW.exe @ 0x6b46f0 \
         (culled {culled_n}, visible {visible_n}, eye-on-plane {eye_on})"
    );
}
