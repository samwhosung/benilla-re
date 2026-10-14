//! `0x5c4f30` against the exe, run whole: points mapped from the screen to the world, and the
//! rejection when `|w'| < 0.001`. The function reads the live device (`0xc0ed38`) through getters,
//! so the test plants one: `device[0]` is a vtable whose slot `0x5c` writes a chosen rectangle (the
//! `0x58a240` path); the viewport scalars sit at `+0xf38` (`0x58b060`); the top of the model-view
//! stack, index at `+0x1ac0`, matrix at `+0x1ac8` (`0x592730`); the view matrix at `+0xf50`
//! (`0x592690`), which `0x58b0c0` multiplies in; and the projection stack's top, index at `+0x1890`,
//! matrix at `+0x1898` (`0x5928e0`) for `0x58b280`. With the view matrix the identity, `0x58b0c0`
//! gives exactly the chosen model-view matrix, and `0x58b280` the chosen projection. The cases are
//! the guard at `0.0005`, `0.0009`, `0.001`, `0.0011` and `-0.0005`, and two fuzzes of matrices whose
//! `w'` stays above 8 in size. A NaN result matches any NaN.

mod common;

use benilla_re_difftest::{Emu, Wow};
use benilla_re_gxumath::project::{unproject_viewport, UnprojectViewport};
use common::{assert_arr, fuzz, fuzz_pos};

const UNPROJECT: u64 = 0x5c4f30;

const DEVICE_PTR: u64 = 0x00c0_ed38;

// device field offsets (read by the getter callees 0x592640 / 0x592690 / 0x592730 / 0x5928e0).
const D_VP: u64 = 0xf38; // ox, fx, oy, fy, z_near, z_far
const D_MATB: u64 = 0xf50; // device view matrix (0x592690)
const D_MATA_IDX: u64 = 0x1ac0; // modelview-stack top index
const D_MATA: u64 = 0x1ac8; // modelview-stack top matrix (0x592730)
const D_PROJ_IDX: u64 = 0x1890; // projection-stack #8 top index
const D_PROJ: u64 = 0x1898; // projection-stack #8 top matrix (0x5928e0)

const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

fn write_mat(e: &mut Emu, base: u64, m: &[f32; 16]) {
    for (i, &x) in m.iter().enumerate() {
        e.write_u32(base + 4 * i as u64, x.to_bits());
    }
}

/// Plant the synthetic `CGxDevice`: returns `(device_va, rect_payload_va)`. The vtable island
/// write-throughs the 16-byte `rect_payload` for the `0x58a240` getter; the caller rewrites it per case.
fn setup_device(e: &mut Emu) -> (u64, u64) {
    let dev = e.alloc(0x2000);
    let vt = e.alloc(0x100);
    let island = e.alloc(0x10);
    let rect = e.alloc(0x10);
    e.write_u32(DEVICE_PTR, dev as u32);
    e.write_u32(dev, vt as u32); // device[0] = vtable
    e.write_u32(vt + 0x5c, island as u32); // vtable[0x5c] = rect getter island
    e.stub_write_through(island, 0, rect, 16, 0, 1); // writes rect → arg0; vtable fn is `ret 4`
    e.write_u32(dev + D_MATA_IDX, 0); // modelview-stack top index = 0
    e.write_u32(dev + D_PROJ_IDX, 0); // projection-stack #8 top index = 0
    (dev, rect)
}

fn write_rect(e: &mut Emu, rect_va: u64, r: &[f32; 4]) {
    for (i, &x) in r.iter().enumerate() {
        e.write_u32(rect_va + 4 * i as u64, x.to_bits());
    }
}

fn write_points(e: &mut Emu, base: u64, pts: &[[f32; 3]]) {
    for (i, p) in pts.iter().enumerate() {
        for (k, &c) in p.iter().enumerate() {
            e.write_u32(base + ((i * 3 + k) * 4) as u64, c.to_bits());
        }
    }
}

fn read_points(e: &Emu, base: u64, n: usize) -> Vec<[f32; 3]> {
    (0..n)
        .map(|i| {
            [
                e.read_f32(base + ((i * 3) * 4) as u64),
                e.read_f32(base + ((i * 3 + 1) * 4) as u64),
                e.read_f32(base + ((i * 3 + 2) * 4) as u64),
            ]
        })
        .collect()
}

/// Run unproject for one case and assert the return flag + (on accept) the output points.
#[allow(clippy::too_many_arguments)]
fn check_unproject(
    e: &mut Emu,
    dev: u64,
    rect_va: u64,
    inbuf: u64,
    outbuf: u64,
    n: usize,
    cm: &[f32; 16],
    pj: &[f32; 16],
    vp: &UnprojectViewport,
    rect: &[f32; 4],
    pts: &[[f32; 3]],
    tag: &str,
) {
    // combined_m = mat4_mul(matA=cm, matB=I) = cm;  proj_inv = projection-stack top = pj.
    write_mat(e, dev + D_MATA, cm);
    write_mat(e, dev + D_MATB, &IDENTITY);
    write_mat(e, dev + D_PROJ, pj);
    e.write_u32(dev + D_VP + 16, vp.z_near.to_bits()); // [0xf48]
    e.write_u32(dev + D_VP + 20, vp.z_far.to_bits()); // [0xf4c]
    write_rect(e, rect_va, rect);
    write_points(e, inbuf, pts);
    e.write(outbuf, &vec![0u8; n * 3 * 4]);

    let eax = e.call(UNPROJECT, n as u32, inbuf as u32, &[outbuf as u32]);
    let ours = unproject_viewport(pts, cm, pj, vp, rect);
    match ours {
        Some(out) => {
            assert_eq!(eax, 1, "{tag}: binary rejected, ours accepted");
            let bin = read_points(e, outbuf, n);
            for (i, o) in out.iter().enumerate() {
                assert_arr(&bin[i], o, &format!("{tag} pt={i}"));
            }
        }
        None => assert_eq!(eax, 0, "{tag}: binary accepted, ours rejected"),
    }
}

#[test]
fn unproject_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let (dev, rect_va) = setup_device(&mut e);
    let n = 6usize;
    let inbuf = e.alloc((n * 3 * 4) as u64);
    let outbuf = e.alloc((n * 3 * 4) as u64);

    // ── explicit w-guard cases (combined_m = I; proj_inv row3 sets w' directly) ──
    let pts0: Vec<[f32; 3]> = (0..n).map(|_| [1.0, 2.0, 3.0]).collect();
    let vp0 = UnprojectViewport {
        z_near: 0.5,
        z_far: 1000.0,
    };
    let rect0 = [10.0, 20.0, 480.0, 640.0];
    // proj_inv with column-major row3 = (P[3],P[7],P[11],P[15]) = (0,0,0,w) → w' == w for any point.
    let pj_w = |w: f32| {
        let mut m = IDENTITY;
        m[15] = w;
        m
    };
    let guard_cases = [
        (0.0005f32, "reject w=0.0005"),
        (0.0009, "reject w=0.0009"),
        (0.001, "accept w=0.001 (==eps)"),
        (0.0011, "accept w=0.0011"),
        (-0.0005, "reject w=-0.0005"),
    ];
    for (w, tag) in guard_cases {
        check_unproject(
            &mut e,
            dev,
            rect_va,
            inbuf,
            outbuf,
            n,
            &IDENTITY,
            &pj_w(w),
            &vp0,
            &rect0,
            &pts0,
            tag,
        );
    }

    // safe matrix: tiny row3 weights + large constant → |w'| ≫ eps for bounded points.
    fn fuzz_mat_safe(s: &mut u32) -> [f32; 16] {
        let mut m = [0f32; 16];
        for x in m.iter_mut() {
            *x = fuzz(s, 4.0);
        }
        m[3] = fuzz(s, 0.05);
        m[7] = fuzz(s, 0.05);
        m[11] = fuzz(s, 0.05);
        m[15] = 8.0 + fuzz_pos(s, 8.0);
        m
    }

    // ── fuzz A: combined_m = I, vary proj_inv  →  M_total = proj_inv ──
    let mut s: u32 = 0x5c4f_0001;
    for case in 0..1500u32 {
        let pj = fuzz_mat_safe(&mut s);
        let vp = UnprojectViewport {
            z_near: fuzz(&mut s, 10.0),
            z_far: fuzz(&mut s, 1000.0),
        };
        let rect = [
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 2000.0),
            fuzz(&mut s, 2000.0),
        ];
        let pts: Vec<[f32; 3]> = (0..n)
            .map(|_| [fuzz(&mut s, 5.0), fuzz(&mut s, 5.0), fuzz(&mut s, 5.0)])
            .collect();
        check_unproject(
            &mut e,
            dev,
            rect_va,
            inbuf,
            outbuf,
            n,
            &IDENTITY,
            &pj,
            &vp,
            &rect,
            &pts,
            &format!("fuzzA[{case}]"),
        );
    }

    // ── fuzz B: vary combined_m, proj_inv = I  →  M_total = mat4_mul(I, combined_m) ──
    for case in 0..1500u32 {
        let cm = fuzz_mat_safe(&mut s);
        let vp = UnprojectViewport {
            z_near: fuzz(&mut s, 10.0),
            z_far: fuzz(&mut s, 1000.0),
        };
        let rect = [
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 100.0),
            fuzz(&mut s, 2000.0),
            fuzz(&mut s, 2000.0),
        ];
        let pts: Vec<[f32; 3]> = (0..n)
            .map(|_| [fuzz(&mut s, 5.0), fuzz(&mut s, 5.0), fuzz(&mut s, 5.0)])
            .collect();
        check_unproject(
            &mut e,
            dev,
            rect_va,
            inbuf,
            outbuf,
            n,
            &cm,
            &IDENTITY,
            &vp,
            &rect,
            &pts,
            &format!("fuzzB[{case}]"),
        );
    }
}
