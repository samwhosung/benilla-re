//! The walking helpers run whole: `ComputeWallPushOut` (`0x635d80`), `SelectBevelContactNormal`
//! (`0x636610`), `CMovement::LimitFaceRise` (`0x635c00`), `IsListedTriangleSteep` (`0x6336a0`),
//! `CMovement::LandsOnTriangle` (`0x633720`) and `CMovement::ComputeWallPush` (`0x635090`), with
//! `IsUnitPlayerDriven` (`0x5fa550`) stubbed where it is called.

use benilla_re_collision::resolve_toolkit::{capsule_kdop_planes, Plane};
use benilla_re_collision::resolve_walk::{feature_classify, vector_slide};
use benilla_re_difftest::{Emu, Wow};

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

/// `0x635d80` and `0x636610`.
#[test]
fn walk_helpers_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let pl_va = emu.alloc(0x60);
    let out_va = emu.alloc(0x10);
    let v_va = emu.alloc(0x30);
    let mut s = 0xa11_c0deu32;
    let mut checked = 0u32;
    for case in 0..300u32 {
        let center = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let kd = capsule_kdop_planes(&center, fz(&mut s).abs() + 0.5, fz(&mut s).abs() + 0.3);
        let count = 1 + (case % 4);
        let picks: Vec<Plane> = match count {
            1 => vec![kd[5 + (case as usize % 4)]],
            2 => vec![kd[5], kd[7 + (case as usize % 2)]],
            3 => vec![kd[5], kd[6], kd[7 + (case as usize % 2)]],
            _ => vec![kd[5], kd[6], kd[7], kd[8]],
        };

        let mut picks = picks;
        if case % 7 == 0 {
            picks[0].n[2] = fz(&mut s);
        }
        for (i, pl) in picks.iter().enumerate() {
            wv3(&mut emu, pl_va + (i as u64) * 0x10, &pl.n);
            emu.write_u32(pl_va + (i as u64) * 0x10 + 0xc, pl.d.to_bits());
        }
        let sent = 0x7fc0_0001u32;
        for k in 0..3u64 {
            emu.write_u32(out_va + k * 4, sent);
        }
        let eax = emu.call(0x636610, pl_va as u32, count, &[out_va as u32]);
        let mut ours = [f32::from_bits(sent); 3];
        let ok = feature_classify(&picks, count, &mut ours);
        assert_eq!(
            eax & 0xff != 0,
            ok,
            "case {case}: classify verdict (count {count})"
        );
        if ok {
            let bin = rv3(&emu, out_va);
            for k in 0..3 {
                assert_eq!(
                    bin[k].to_bits(),
                    ours[k].to_bits(),
                    "case {case}: classify[{k}] binary={:#010x} ours={:#010x}",
                    bin[k].to_bits(),
                    ours[k].to_bits()
                );
            }
        }

        let w = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let speed = fz(&mut s);
        let mut n = [fz(&mut s), fz(&mut s), fz(&mut s)];
        if case % 5 == 0 {
            n = [0.0, 0.0, -1.0];
        }
        if case % 6 == 0 {
            n = [0.0, 0.0, 0.7];
        }
        wv3(&mut emu, v_va, &w);
        wv3(&mut emu, v_va + 0xc, &n);
        for k in 0..3u64 {
            emu.write_u32(out_va + k * 4, sent);
        }
        emu.call(
            0x635d80,
            0,
            0,
            &[
                out_va as u32,
                v_va as u32,
                speed.to_bits(),
                (v_va + 0xc) as u32,
            ],
        );
        let mut ours = [f32::from_bits(sent); 3];
        vector_slide(&w, speed, &n, &mut ours);
        let bin = rv3(&emu, out_va);
        for k in 0..3 {
            assert_eq!(
                bin[k].to_bits(),
                ours[k].to_bits(),
                "case {case}: slide[{k}] binary={:#010x} ours={:#010x}",
                bin[k].to_bits(),
                ours[k].to_bits()
            );
        }
        checked += 1;
    }
    eprintln!("walk_helpers: {checked} rounds bit-exact vs WoW.exe (2 functions)");
}

/// `0x635c00`.
#[test]
fn speed_select_difftest() {
    use benilla_re_collision::resolve_walk::speed_select;
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);

    let cell = emu.alloc(8);
    let mut patch = vec![0xd9u8, 0x05];
    patch.extend_from_slice(&(cell as u32).to_le_bytes());
    patch.push(0xc3);
    emu.write(0x617430, &patch);
    let this_va = emu.alloc(0x100);
    let out_va = emu.alloc(0x10);
    let w_va = emu.alloc(0x10);
    let sp_va = emu.alloc(8);
    let v_va = emu.alloc(0x10);
    let mut s = 0x57ee_d001u32;
    let mut checked = 0u32;
    for case in 0..300u32 {
        let flags = if case % 2 == 0 { 0x0400_0000 } else { 0 };
        let h = fz(&mut s).abs() + 0.1;
        let w = [fz(&mut s), fz(&mut s), fz(&mut s)];
        let speed0 = fz(&mut s);
        let mut dir = [fz(&mut s), fz(&mut s), fz(&mut s)];
        if case % 7 == 0 {
            dir[2] = 0.0;
        }
        let flag = (case % 3 != 1) as u32;
        let v = [fz(&mut s), fz(&mut s), fz(&mut s)];
        emu.write_u32(cell, h.to_bits());
        emu.write_u32(this_va + 0x40, flags);
        wv3(&mut emu, w_va, &w);
        emu.write_u32(sp_va, speed0.to_bits());
        wv3(&mut emu, v_va, &v);
        let sent = 0x7fc0_0001u32;
        for k in 0..3u64 {
            emu.write_u32(out_va + k * 4, sent);
        }
        emu.call(
            0x635c00,
            this_va as u32,
            0,
            &[
                out_va as u32,
                w_va as u32,
                sp_va as u32,
                dir[0].to_bits(),
                dir[1].to_bits(),
                dir[2].to_bits(),
                flag,
                v_va as u32,
            ],
        );
        let mut speed = speed0;
        let mut dir2 = dir;
        let mut out = [f32::from_bits(sent); 3];
        speed_select(flags, h, &w, &mut speed, &mut dir2, flag, &v, &mut out);
        let bs = emu.read_f32(sp_va);
        assert_eq!(
            bs.to_bits(),
            speed.to_bits(),
            "case {case}: speed binary={:#010x} ours={:#010x}",
            bs.to_bits(),
            speed.to_bits()
        );
        let bo = rv3(&emu, out_va);
        for k in 0..3 {
            assert_eq!(
                bo[k].to_bits(),
                out[k].to_bits(),
                "case {case}: out[{k}] binary={:#010x} ours={:#010x}",
                bo[k].to_bits(),
                out[k].to_bits()
            );
        }
        checked += 1;
    }
    eprintln!("speed_select: {checked} rounds bit-exact vs WoW.exe");
}

/// `0x6336a0`, `0x633720` and `0x635090`.
#[test]
fn ground_slide_difftest() {
    use benilla_re_collision::resolve_toolkit::Plane;
    use benilla_re_collision::resolve_walk::{ground_at_point, slide_response, wall_test};
    let Some(wow) = Wow::load() else { return };
    for pred in [false, true] {
        let mut emu = Emu::new(&wow);
        emu.stub(0x5fa550, pred as u32, 0);
        let this_va = emu.alloc(0x200);
        let hits = emu.alloc(0x200);
        let off_va = emu.alloc(0x10);
        let out_va = emu.alloc(0x10);
        let dir_va = emu.alloc(0x10);
        let feat_va = emu.alloc(0x40);
        emu.write_u32(0xc4e534, hits as u32);
        let unit2_va = emu.alloc(0x20);
        emu.write_u32(this_va + 0x15c, unit2_va as u32);
        let mut s = 0x6e0d_0001u32 ^ pred as u32;
        for case in 0..200u32 {
            let pos = [fz(&mut s), fz(&mut s), fz(&mut s)];
            let offset = [fz(&mut s) * 0.5, fz(&mut s) * 0.5, fz(&mut s) * 0.5];
            let idx = (next(&mut s) % 4) as u64;
            let mut n = {
                let v = [fz(&mut s) * 0.4, fz(&mut s) * 0.4, fz(&mut s) + 0.3];
                let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(0.01);
                [v[0] / l, v[1] / l, v[2] / l]
            };
            if case % 6 == 0 {
                n = [1.0, 0.0, 0.0];
            }
            let basis: [[f32; 3]; 3] = core::array::from_fn(|_| {
                [
                    pos[0] + fz(&mut s),
                    pos[1] + fz(&mut s),
                    pos[2] + fz(&mut s),
                ]
            });
            for (k, c) in pos.iter().enumerate() {
                emu.write_u32(this_va + 0x10 + (k as u64) * 4, c.to_bits());
            }
            let rv = hits + idx * 0x34;
            for (k, c) in n.iter().enumerate() {
                emu.write_u32(rv + (k as u64) * 4, c.to_bits());
            }
            emu.write_u32(rv + 0xc, fz(&mut s).to_bits());
            for (r, v) in basis.iter().enumerate() {
                for (k, c) in v.iter().enumerate() {
                    emu.write_u32(rv + 0x10 + (r as u64) * 0xc + (k as u64) * 4, c.to_bits());
                }
            }
            for (k, c) in offset.iter().enumerate() {
                emu.write_u32(off_va + (k as u64) * 4, c.to_bits());
            }

            let eax = emu.call(0x633720, this_va as u32, 0, &[idx as u32, off_va as u32]);
            let ours = ground_at_point(&pos, &offset, n[2], &basis, pred);
            assert_eq!(eax & 0xff != 0, ours, "p{pred} case {case}: ground");

            let eax = emu.call(0x6336a0, 0, 0, &[idx as u32]);
            assert_eq!(
                eax & 0xff != 0,
                wall_test(n[2]),
                "p{pred} case {case}: wall"
            );

            let dir = {
                let v = [fz(&mut s), fz(&mut s), fz(&mut s)];
                let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(0.01);
                [v[0] / l, v[1] / l, v[2] / l]
            };
            let (t, len) = (fz(&mut s).abs() * 0.5, fz(&mut s).abs() + 0.5);
            let f0 = Plane {
                n: [fz(&mut s), fz(&mut s), fz(&mut s)],
                d: fz(&mut s),
            };
            let f1 = Plane {
                n: [fz(&mut s), fz(&mut s), fz(&mut s)],
                d: fz(&mut s),
            };
            let ftype = [0u32, 2, 3, 4][(case % 4) as usize];
            let radius = fz(&mut s).abs() + 0.3;
            emu.write_u32(this_va + 0xb0, radius.to_bits());
            for (k, c) in dir.iter().enumerate() {
                emu.write_u32(dir_va + (k as u64) * 4, c.to_bits());
            }
            for (i, pl) in [&f0, &f1].iter().enumerate() {
                for (k, c) in pl.n.iter().enumerate() {
                    emu.write_u32(feat_va + (i as u64) * 0x10 + (k as u64) * 4, c.to_bits());
                }
                emu.write_u32(feat_va + (i as u64) * 0x10 + 0xc, pl.d.to_bits());
            }
            let sent = 0x7fc0_0001u32;
            emu.write_u32(out_va, sent);
            emu.write_u32(out_va + 4, sent);
            emu.call(
                0x635090,
                this_va as u32,
                0,
                &[
                    out_va as u32,
                    dir_va as u32,
                    t.to_bits(),
                    len.to_bits(),
                    idx as u32,
                    feat_va as u32,
                    ftype,
                ],
            );
            let mut out = [f32::from_bits(sent); 2];
            slide_response(
                &pos,
                radius,
                &dir,
                t,
                len,
                n[2],
                &n,
                emu.read_f32(rv + 0xc),
                &basis,
                &[f0, f1],
                ftype,
                &mut out,
            );
            for k in 0..2u64 {
                let b = emu.read_f32(out_va + k * 4);
                assert_eq!(
                    b.to_bits(),
                    out[k as usize].to_bits(),
                    "p{pred} case {case}: slide[{k}] binary={:#010x} ours={:#010x}",
                    b.to_bits(),
                    out[k as usize].to_bits()
                );
            }
        }
    }
    eprintln!("ground_slide: 400 rounds bit-exact vs WoW.exe (3 functions)");
}
