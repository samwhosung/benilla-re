//! The mover's arithmetic run whole against the exe. Each callee the function makes is stubbed: a
//! callee whose result matters is replaced by `mov eax,[cell]; ret N` or `fld dword [cell]; ret N`
//! with the value the case wants, and the rest return without effect. The stubbed callees are
//! `0x7c5490` and `0x7c5f30` for `0x616af0`, `0x634040` for `0x616cb0`, `0x7c6290` for `0x616d60`,
//! `0x631670` for `0x617460`, `0x7c4930` and `0x634030` for the three that store a position or a
//! size, and `0x6158e0`, `0x7c5360` and `0x618920` for `0x619090` and `0x6191c0`.

#![allow(
    clippy::neg_cmp_op_on_partial_ord,
    clippy::too_many_arguments,
    clippy::needless_range_loop
)]

use benilla_re_collision::mover::{
    angle_wrap, anti_warp, build_delta, dead_band, facing_interp, param_set, pos_lerp, pos_valid,
    reconcile_gate, turn_clamp, WrapNode,
};
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// A random `f32` in `[-4, 4)`.
fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}
fn wv3(emu: &mut Emu, va: u64, v: &[f32; 3]) {
    for (i, c) in v.iter().enumerate() {
        emu.write_u32(va + (i as u64) * 4, c.to_bits());
    }
}
fn rv3(emu: &Emu, va: u64) -> [f32; 3] {
    core::array::from_fn(|i| emu.read_f32(va + (i as u64) * 4))
}

/// Replaces the callee at `addr` with `mov eax,[cell]; ret N`.
fn patch_eax(emu: &mut Emu, addr: u64, cell: u64, ret_n: u16) {
    let mut code = vec![0xa1u8];
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    if ret_n == 0 {
        code.push(0xc3);
    } else {
        code.push(0xc2);
        code.extend_from_slice(&ret_n.to_le_bytes());
    }
    emu.write(addr, &code);
}

/// Replaces the callee at `addr` with `fld dword [cell]; ret N`.
fn patch_fld(emu: &mut Emu, addr: u64, cell: u64, ret_n: u16) {
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    if ret_n == 0 {
        code.push(0xc3);
    } else {
        code.push(0xc2);
        code.extend_from_slice(&ret_n.to_le_bytes());
    }
    emu.write(addr, &code);
}

/// `0x616af0`: `eax`, the position and `+0x7c`.
#[test]
fn anti_warp_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cell5490 = emu.alloc(8);
    let cell7c = emu.alloc(8);
    patch_eax(&mut emu, 0x7c5490, cell5490, 8);
    patch_fld(&mut emu, 0x7c5f30, cell7c, 4);
    emu.stub(0x7c4930, 0, 2);
    emu.stub(0x634030, 0, 1);
    let this_va = emu.alloc(0x200);
    let mi_va = emu.alloc(0x40);
    let delta_va = emu.alloc(0x10);
    emu.write_u32(this_va + 0xa4, mi_va as u32);

    #[derive(Clone, Copy)]
    struct C {
        ok: bool,
        flags: u32,
        mi: u32,
        base: [f32; 3],
        delta: [f32; 3],
        pos: [f32; 3],
        dt: u32,
        fzp: f32,
        cfz: f32,
    }
    let z = [0.0f32; 3];
    let mut cases: Vec<C> = vec![
        C {
            ok: false,
            flags: 0x1,
            mi: 0,
            base: z,
            delta: z,
            pos: z,
            dt: 10,
            fzp: 7.0,
            cfz: 3.0,
        },
        C {
            ok: true,
            flags: 0,
            mi: 0,
            base: z,
            delta: z,
            pos: z,
            dt: 10,
            fzp: 7.0,
            cfz: 3.0,
        },
        C {
            ok: true,
            flags: 0x1,
            mi: 0x200,
            base: z,
            delta: z,
            pos: z,
            dt: 10,
            fzp: 7.0,
            cfz: 3.0,
        },
        C {
            ok: true,
            flags: 0x1,
            mi: 0,
            base: [0.0, 0.0, 0.0],
            delta: [0.5, 0.0, 0.0],
            pos: z,
            dt: 200,
            fzp: 7.0,
            cfz: 3.0,
        },
        C {
            ok: true,
            flags: 0x1,
            mi: 0,
            base: [0.0, 0.0, 0.0],
            delta: [2.0, 2.0, 0.0],
            pos: z,
            dt: 1,
            fzp: 7.0,
            cfz: 3.0,
        },
        C {
            ok: true,
            flags: 0x1,
            mi: 0,
            base: [0.0, 0.0, 0.0],
            delta: [4.0, 0.0, 0.0],
            pos: z,
            dt: 200,
            fzp: 7.0,
            cfz: 3.0,
        },
        C {
            ok: true,
            flags: 0x2000,
            mi: 0,
            base: [0.0, 0.0, 0.0],
            delta: [4.0, 0.0, 0.0],
            pos: z,
            dt: 200,
            fzp: 11.5,
            cfz: 3.0,
        },
        C {
            ok: true,
            flags: 0x1,
            mi: 0,
            base: [0.0, 0.0, 0.0],
            delta: [f32::NAN, 0.0, 0.0],
            pos: z,
            dt: 200,
            fzp: 7.0,
            cfz: 3.0,
        },
        C {
            ok: true,
            flags: 0x2000,
            mi: 0,
            base: [0.0, 0.0, 0.0],
            delta: [0.5, 0.0, 0.0],
            pos: z,
            dt: 0,
            fzp: 1.0,
            cfz: 3.0,
        },
    ];
    let mut s = 0x0a17_1f00u32;
    for case in 0..900u32 {
        let ok = next(&mut s) & 0x70 != 0;
        let flags = match case % 5 {
            0 => 0,
            1 => 0x1,
            2 => 0x2000,
            3 => 0x2001,
            _ => 0x8,
        };
        let mi = if next(&mut s) & 0x300 == 0 { 0x200 } else { 0 };
        let base = [fz(&mut s) * 2.0, fz(&mut s) * 2.0, fz(&mut s)];
        let delta = match case % 7 {
            0 => [fz(&mut s) * 0.1, fz(&mut s) * 0.1, fz(&mut s) * 0.1],
            1 => [fz(&mut s), fz(&mut s), fz(&mut s)],
            6 => [f32::NAN, fz(&mut s), 0.0],
            _ => [fz(&mut s) * 5.0, fz(&mut s) * 5.0, fz(&mut s)],
        };
        let pos = [
            base[0] + fz(&mut s) * 0.5,
            base[1] + fz(&mut s) * 0.5,
            base[2],
        ];
        let dt = match case % 6 {
            0 => 0,
            1 => 1,
            2 => 2,
            5 => 250,
            _ => (next(&mut s) % 250) + 1,
        };
        cases.push(C {
            ok,
            flags,
            mi,
            base,
            delta,
            pos,
            dt,
            fzp: fz(&mut s) * 4.0,
            cfz: fz(&mut s),
        });
    }

    let mut tally = [0u32; 6];
    let mut checked = 0u32;
    for (ci, c) in cases.iter().enumerate() {
        emu.write_u32(cell5490, c.ok as u32);
        emu.write_u32(cell7c, c.fzp.to_bits());
        emu.write_u32(this_va + 0x40, c.flags);
        emu.write_u32(mi_va + 0x18, c.mi);
        wv3(&mut emu, this_va + 0x44, &c.base);
        wv3(&mut emu, this_va + 0x10, &c.pos);
        emu.write_u32(this_va + 0x7c, c.cfz.to_bits());
        wv3(&mut emu, delta_va, &c.delta);
        let eax = emu.call(0x616af0, this_va as u32, 0, &[0, c.dt, delta_va as u32]);
        let pos_bin = rv3(&emu, this_va + 0x10);
        let fz_bin = emu.read_f32(this_va + 0x7c);

        let o = anti_warp(
            c.ok, c.flags, c.mi, c.base, c.delta, c.pos, c.dt, c.fzp, c.cfz,
        );
        assert_eq!(eax, o.eax, "case {ci}: eax {eax:#x} vs {:#x}", o.eax);
        for k in 0..3 {
            assert_eq!(
                pos_bin[k].to_bits(),
                o.pos[k].to_bits(),
                "case {ci}: pos[{k}] {:#010x} vs {:#010x}",
                pos_bin[k].to_bits(),
                o.pos[k].to_bits()
            );
        }
        assert_eq!(
            fz_bin.to_bits(),
            o.fall_z.to_bits(),
            "case {ci}: fall_z {:#010x} vs {:#010x}",
            fz_bin.to_bits(),
            o.fall_z.to_bits()
        );

        if !c.ok {
            tally[0] += 1;
        } else if c.flags & 0x200f == 0 {
            tally[1] += 1;
        } else if c.mi & 0x200 != 0 {
            tally[2] += 1;
        } else if o.eax == 1 {
            tally[3] += 1;
        } else if c.flags & 0x2000 != 0 {
            tally[5] += 1;
        } else {
            tally[4] += 1;
        }
        checked += 1;
    }
    eprintln!(
        "anti_warp: {checked} bit-exact; [ifail={} notmove={} swim={} skip={} snap={} snap_fall={}]",
        tally[0], tally[1], tally[2], tally[3], tally[4], tally[5]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "anti_warp leg {i} never hit");
    }
}

/// `0x616bf0`: the verdict.
#[test]
fn pos_valid_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    let this_va = emu.alloc(0x40);
    let half = f32::from_bits(0x4685_5555);
    let mut probes: Vec<[f32; 3]> = vec![
        [0.0, 0.0, 0.0],
        [100.0, -100.0, 50.0],
        [17064.666, 17064.0, 0.0],
        [-17064.0, 0.0, 0.0],
        [f32::NAN, 0.0, 0.0],
        [0.0, f32::INFINITY, 0.0],
        [0.0, 0.0, f32::NAN],
        [half - 1.0, 0.0, 0.0],
        [0.0, half - 1.0, 0.0],
        [half - 40000.0, 0.0, 0.0],
        [0.0, half - 40000.0, 0.0],
        [0.0, 0.0, 1e30],
    ];
    let mut s = 0xb0fd_0001u32;
    for case in 0..1500u32 {
        let span = match case % 4 {
            0 => 17070.0,
            1 => 16000.0,
            _ => fz(&mut s).abs() * 9000.0,
        };
        let mk = |s: &mut u32| match next(s) % 6 {
            0 => f32::NAN,
            1 => f32::INFINITY,
            2 => f32::NEG_INFINITY,
            _ => fz(s) / 4.0 * span,
        };
        probes.push([mk(&mut s), mk(&mut s), mk(&mut s)]);
    }
    let mut tally = [0u32; 3];
    let mut checked = 0u32;
    for (ci, p) in probes.iter().enumerate() {
        wv3(&mut emu, this_va + 0x10, p);
        let eax = emu.call(0x616bf0, this_va as u32, 0, &[]);
        let ours = pos_valid(*p);
        assert_eq!(
            eax != 0,
            ours,
            "case {ci}: pos {p:?} binary={eax} ours={ours}"
        );
        if ours {
            tally[0] += 1;
        } else if !p[0].is_finite() || !p[1].is_finite() || !p[2].is_finite() {
            tally[1] += 1;
        } else {
            tally[2] += 1;
        }
        checked += 1;
    }
    eprintln!(
        "pos_valid: {checked} bit-exact; [valid={} nonfinite={} bounds={}]",
        tally[0], tally[1], tally[2]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "pos_valid leg {i} never hit");
    }
}

/// `0x616cb0`: the displacement passed to `0x634040`.
#[test]
fn build_delta_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    emu.stub(0x634040, 0, 3);
    let this_va = emu.alloc(0x200);
    let mi_va = emu.alloc(0x40);
    let off_va = emu.alloc(0x10);

    #[derive(Clone, Copy)]
    struct C {
        base: [f32; 3],
        off: [f32; 3],
        pos: [f32; 3],
        mi_present: bool,
        mi: u32,
        flags: u32,
    }
    let mut cases: Vec<C> = vec![
        C {
            base: [1.0, 2.0, 3.0],
            off: [0.1, 0.2, 0.3],
            pos: [0.0, 0.0, 0.0],
            mi_present: true,
            mi: 0x200,
            flags: 0,
        },
        C {
            base: [1.0, 2.0, 3.0],
            off: [0.1, 0.2, 0.3],
            pos: [0.0, 0.0, 0.0],
            mi_present: true,
            mi: 0x204,
            flags: 0,
        },
        C {
            base: [1.0, 2.0, 3.0],
            off: [0.1, 0.2, 0.3],
            pos: [0.0, 0.0, 0.0],
            mi_present: true,
            mi: 0,
            flags: 0x800,
        },
        C {
            base: [1.0, 2.0, 3.0],
            off: [0.1, 0.2, 0.3],
            pos: [0.0, 0.0, 0.0],
            mi_present: true,
            mi: 0,
            flags: 0x200000,
        },
        C {
            base: [1.0, 2.0, 3.0],
            off: [0.1, 0.2, 0.3],
            pos: [0.0, 0.0, 0.0],
            mi_present: false,
            mi: 0,
            flags: 0,
        },
    ];
    let mut s = 0xbd_e17a01u32;
    for case in 0..900u32 {
        let mi_present = next(&mut s) & 0x40 != 0;
        let mi = match case % 4 {
            0 => 0x200,
            1 => 0x204,
            2 => 0,
            _ => 0x4,
        };
        let flags = match case % 5 {
            0 => 0,
            1 => 0x800,
            2 => 0x200000,
            3 => 0x200800,
            _ => 0x10,
        };
        cases.push(C {
            base: [fz(&mut s) * 4.0, fz(&mut s) * 4.0, fz(&mut s) * 4.0],
            off: [fz(&mut s), fz(&mut s), fz(&mut s)],
            pos: [fz(&mut s) * 4.0, fz(&mut s) * 4.0, fz(&mut s) * 4.0],
            mi_present,
            mi,
            flags,
        });
    }

    let mut tally = [0u32; 2];
    let mut checked = 0u32;
    for (ci, c) in cases.iter().enumerate() {
        wv3(&mut emu, this_va + 0x44, &c.base);
        wv3(&mut emu, this_va + 0x10, &c.pos);
        emu.write_u32(this_va + 0x40, c.flags);
        emu.write_u32(this_va + 0xa4, if c.mi_present { mi_va as u32 } else { 0 });
        emu.write_u32(mi_va + 0x18, c.mi);
        wv3(&mut emu, off_va, &c.off);
        emu.clear_calls();
        emu.call(0x616cb0, this_va as u32, 0, &[0, 0, off_va as u32]);
        let dva = emu
            .calls()
            .iter()
            .rev()
            .find(|c| c.addr == 0x634040)
            .expect("0x634040 called")
            .args[2] as u64;
        let delta_bin = rv3(&emu, dva);

        let o = build_delta(c.base, c.off, c.pos, c.mi_present, c.mi, c.flags);
        for k in 0..3 {
            assert_eq!(
                delta_bin[k].to_bits(),
                o[k].to_bits(),
                "case {ci}: delta[{k}] {:#010x} vs {:#010x}",
                delta_bin[k].to_bits(),
                o[k].to_bits()
            );
        }
        if o[2].to_bits() == 0 && !(c.base[2] + c.off[2] - c.pos[2] == 0.0) {
            tally[1] += 1;
        } else {
            tally[0] += 1;
        }
        checked += 1;
    }
    eprintln!(
        "build_delta: {checked} bit-exact; [keep≈{} zero≈{}]",
        tally[0], tally[1]
    );
    assert!(tally[1] > 0, "Δz-zero leg never hit");
}

/// `0x616d60`: the position.
#[test]
fn dead_band_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    emu.stub(0x7c6290, 0, 0);
    emu.stub(0x7c4930, 0, 2);
    emu.stub(0x634030, 0, 1);
    let this_va = emu.alloc(0x200);
    let np_va = emu.alloc(0x10);

    let mut cases: Vec<([f32; 3], [f32; 3])> = vec![
        ([0.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([1.0, 1.0, 1.0], [0.0, 0.0, 0.0]),
        ([3.0, 0.0, 0.0], [0.0, 0.0, 0.0]),
        ([5.0, 5.0, 5.0], [0.0, 0.0, 0.0]),
        ([f32::NAN, 0.0, 0.0], [0.0, 0.0, 0.0]),
    ];
    let mut s = 0xdead_ba90u32;
    for case in 0..900u32 {
        let pos = [fz(&mut s) * 4.0, fz(&mut s) * 4.0, fz(&mut s) * 4.0];
        let np = match case % 4 {
            0 => [
                pos[0] + fz(&mut s) * 0.5,
                pos[1] + fz(&mut s) * 0.5,
                pos[2] + fz(&mut s) * 0.5,
            ],
            3 => [f32::NAN, pos[1], pos[2]],
            _ => [
                pos[0] + fz(&mut s) * 4.0,
                pos[1] + fz(&mut s) * 4.0,
                pos[2] + fz(&mut s) * 4.0,
            ],
        };
        cases.push((np, pos));
    }
    let mut tally = [0u32; 2];
    let mut checked = 0u32;
    for (ci, (np, pos)) in cases.iter().enumerate() {
        wv3(&mut emu, this_va + 0x10, pos);
        wv3(&mut emu, np_va, np);
        emu.call(0x616d60, this_va as u32, 0, &[np_va as u32]);
        let pos_bin = rv3(&emu, this_va + 0x10);
        let (commit, o) = dead_band(*np, *pos);
        for k in 0..3 {
            assert_eq!(
                pos_bin[k].to_bits(),
                o[k].to_bits(),
                "case {ci}: pos[{k}] {:#010x} vs {:#010x}",
                pos_bin[k].to_bits(),
                o[k].to_bits()
            );
        }
        tally[commit as usize] += 1;
        checked += 1;
    }
    eprintln!(
        "dead_band: {checked} bit-exact; [skip={} commit={}]",
        tally[0], tally[1]
    );
    assert!(tally[0] > 0 && tally[1] > 0, "dead_band not both-sided");
}

/// `0x617080`: each queued event's facing.
#[test]
fn angle_wrap_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    let this_va = emu.alloc(0x200);
    let nodes_base = emu.alloc(0x400);
    let node_stride = 0x50u64;
    let two_pi = f32::from_bits(0x40c9_0fdb);

    let mut s = 0xa6_19e000u32;
    let mut tally = [0u32; 4];
    let mut checked = 0u32;
    let mut empty_lists = 0u32;
    for case in 0..600u32 {
        let n = (case % 7) as usize;
        let mut nodes: Vec<WrapNode> = Vec::new();
        for i in 0..n {
            let typ = if next(&mut s) & 0x100 == 0 {
                0x10
            } else {
                next(&mut s) & 0xff
            };
            let active = if next(&mut s) & 0x80 == 0 { 0u8 } else { 1u8 };

            let facing = match (case + i as u32) % 4 {
                0 => fz(&mut s) * 0.5 + two_pi,
                1 => -(fz(&mut s).abs()) - 0.1,
                2 => fz(&mut s).abs() * 0.3,
                _ => fz(&mut s) * 8.0,
            };
            nodes.push(WrapNode {
                typ,
                active,
                facing,
            });
        }
        let arg = fz(&mut s) * 4.0;

        if n == 0 {
            emu.write_u32(this_va + 0x158, if case % 2 == 0 { 0 } else { 1 });
            empty_lists += 1;
        } else {
            emu.write_u32(this_va + 0x158, (nodes_base) as u32);
            for (i, nd) in nodes.iter().enumerate() {
                let nv = nodes_base + (i as u64) * node_stride;
                emu.write_u32(nv + 0xc, nd.typ);
                emu.write_u32(nv + 0x4c, nd.active as u32);
                emu.write_u32(nv + 0x1c, nd.facing.to_bits());
                let nxt = if i + 1 < n {
                    nodes_base + ((i + 1) as u64) * node_stride
                } else {
                    0
                };
                emu.write_u32(nv + 4, nxt as u32);
            }
        }
        emu.call(0x617080, this_va as u32, 0, &[arg.to_bits()]);

        let mut ours = nodes.clone();
        angle_wrap(arg, &mut ours);
        for (i, nd) in ours.iter().enumerate() {
            let nv = nodes_base + (i as u64) * node_stride;
            let bin = emu.read_f32(nv + 0x1c);
            assert_eq!(
                bin.to_bits(),
                nd.facing.to_bits(),
                "case {case}: node[{i}] facing {:#010x} vs {:#010x}",
                bin.to_bits(),
                nd.facing.to_bits()
            );

            let processed = nodes[i].typ == 0x10 || nodes[i].active != 0;
            if !processed {
                tally[3] += 1;
            } else {
                let a = f64::from(arg) + f64::from(nodes[i].facing);
                if a > f64::from(two_pi) {
                    tally[0] += 1;
                } else if !(a < 0.0) {
                    tally[1] += 1;
                } else {
                    tally[2] += 1;
                }
            }
        }
        checked += 1;
    }
    eprintln!(
        "angle_wrap: {checked} list-walks bit-exact ({empty_lists} empty/tagged heads); \
         legs [sub={} asis={} add={} skipped={}]",
        tally[0], tally[1], tally[2], tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "angle_wrap leg {i} never hit");
    }
}

/// `0x617460`: the pair passed to `0x631670`.
#[test]
fn turn_clamp_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    emu.stub(0x631670, 0, 2);
    let mut cases: Vec<[f32; 3]> = vec![
        [1.0, 1.0, 1.0],
        [10.0, 1.0, 1.0],
        [f32::NAN, 1.0, 1.0],
        [0.0, 0.0, 0.0],
    ];
    let mut s = 0x7c_1a3001u32;
    for _ in 0..900u32 {
        cases.push([fz(&mut s) * 4.0, fz(&mut s) * 4.0, fz(&mut s) * 2.0]);
    }
    let slope = f64::from(f32::from_bits(0x3fec_b91b));
    let half = f64::from(f32::from_bits(0x3f00_0000));
    let mut tally = [0u32; 2];
    let mut checked = 0u32;
    for (ci, a) in cases.iter().enumerate() {
        emu.clear_calls();
        emu.call(
            0x617460,
            0,
            0,
            &[a[0].to_bits(), a[1].to_bits(), a[2].to_bits()],
        );
        let call = *emu
            .calls()
            .iter()
            .rev()
            .find(|c| c.addr == 0x631670)
            .expect("0x631670");
        let a1p_bin = call.args[0];
        let a0p_bin = call.args[1];

        let (a0p, a1p) = turn_clamp(a[0], a[1], a[2]);
        assert_eq!(
            a0p_bin,
            a0p.to_bits(),
            "case {ci}: a0' {a0p_bin:#010x} vs {:#010x}",
            a0p.to_bits()
        );
        assert_eq!(
            a1p_bin,
            a1p.to_bits(),
            "case {ci}: a1' {a1p_bin:#010x} vs {:#010x}",
            a1p.to_bits()
        );

        let a0_scaled = ((f64::from(a[0]) * f64::from(a[2])) * half) as f32;
        if f64::from(a0_scaled) * slope > f64::from(a[2]) {
            tally[1] += 1;
        } else {
            tally[0] += 1;
        }
        checked += 1;
    }
    eprintln!(
        "turn_clamp: {checked} bit-exact; [noclamp={} clamp={}]",
        tally[0], tally[1]
    );
    assert!(tally[0] > 0 && tally[1] > 0, "turn_clamp not both-sided");
}

/// `0x6174b0`: `+0xb0`, `+0xb4` and `+0xb8`.
#[test]
fn param_set_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    emu.stub(0x7c4930, 0, 2);
    emu.stub(0x634030, 0, 1);
    let this_va = emu.alloc(0x200);
    let sent = 0x7fc0_0001u32;

    let mut cases: Vec<[f32; 4]> = vec![
        [2.0, 3.0, 4.0, 5.0],
        [2.0, 3.0, 4.0, 1e-9],
        [2.0, 3.0, 4.0, 0.0],
        [1.0, 1.0, 1.0, f32::NAN],
    ];
    let mut s = 0x5e_77b001u32;
    for case in 0..900u32 {
        let a3 = match case % 3 {
            0 => 1e-9,
            _ => fz(&mut s) * 4.0,
        };
        cases.push([fz(&mut s) * 4.0, fz(&mut s) * 4.0, fz(&mut s) * 4.0, a3]);
    }
    let mut tally = [0u32; 2];
    let mut checked = 0u32;
    for (ci, a) in cases.iter().enumerate() {
        for off in [0xb0u64, 0xb4, 0xb8] {
            emu.write_u32(this_va + off, sent);
        }
        emu.call(
            0x6174b0,
            this_va as u32,
            0,
            &[
                a[0].to_bits(),
                a[1].to_bits(),
                a[2].to_bits(),
                a[3].to_bits(),
            ],
        );
        let bin = [
            emu.read_f32(this_va + 0xb0),
            emu.read_f32(this_va + 0xb4),
            emu.read_f32(this_va + 0xb8),
        ];
        let o = param_set(a[0], a[1], a[2], a[3]);
        for k in 0..3 {
            assert_eq!(
                bin[k].to_bits(),
                o[k].to_bits(),
                "case {ci}: [b0,b4,b8][{k}] {:#010x} vs {:#010x}",
                bin[k].to_bits(),
                o[k].to_bits()
            );
        }
        if f64::from(a[3]).abs() < f64::from(f32::from_bits(0x3480_0000)) {
            tally[0] += 1;
        } else {
            tally[1] += 1;
        }
        checked += 1;
    }
    eprintln!(
        "param_set: {checked} bit-exact; [keep1.0={} divide={}]",
        tally[0], tally[1]
    );
    assert!(tally[0] > 0 && tally[1] > 0, "param_set not both-sided");
}

/// `0x618f80`: `+0x144` and `+0x148`.
#[test]
fn facing_interp_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    let this_va = emu.alloc(0x200);
    let sent = 0x7fc0_0001u32;

    #[derive(Clone, Copy)]
    struct C {
        t0: u32,
        h0: f32,
        t1: u32,
        h1: f32,
        cur: f32,
    }
    let mut cases: Vec<C> = vec![
        C {
            t0: 1000,
            h0: 0.0,
            t1: 1100,
            h1: 1.0,
            cur: 5.0,
        },
        C {
            t0: 1000,
            h0: 0.0,
            t1: 1000,
            h1: 1.0,
            cur: 5.0,
        },
        C {
            t0: 1000,
            h0: 0.0,
            t1: 1000,
            h1: 1.0,
            cur: 1e-9,
        },
        C {
            t0: 1000,
            h0: 0.0,
            t1: 1100,
            h1: 6.0,
            cur: 5.0,
        },
        C {
            t0: 1000,
            h0: 6.0,
            t1: 1100,
            h1: 0.0,
            cur: 5.0,
        },
        C {
            t0: 1000,
            h0: 1.0,
            t1: 100000,
            h1: 1.0000001,
            cur: 5.0,
        },
    ];
    let mut s = 0xfa_c19000u32;
    for case in 0..900u32 {
        let t0 = next(&mut s) % 100000;
        let dt = match case % 5 {
            0 => 0,
            1 => 1,
            _ => (next(&mut s) % 5000) + 1,
        };
        cases.push(C {
            t0,
            h0: fz(&mut s) * 4.0,
            t1: t0.wrapping_add(dt),
            h1: fz(&mut s) * 4.0,
            cur: if case % 6 == 0 {
                fz(&mut s) * 1e-7
            } else {
                fz(&mut s) * 3.0
            },
        });
    }
    let mut tally = [0u32; 4];
    let mut deadzone = 0u32;
    let mut checked = 0u32;
    for (ci, c) in cases.iter().enumerate() {
        emu.write_u32(this_va + 0x144, c.cur.to_bits());
        emu.write_u32(this_va + 0x148, sent);
        emu.call(
            0x618f80,
            this_va as u32,
            0,
            &[c.t0, c.h0.to_bits(), c.t1, c.h1.to_bits()],
        );
        let f144 = emu.read_f32(this_va + 0x144);
        let f148 = emu.read_f32(this_va + 0x148);
        let o = facing_interp(c.t0, c.h0, c.t1, c.h1, c.cur);
        assert_eq!(
            f144.to_bits(),
            o.f144.to_bits(),
            "case {ci}: +0x144 {:#010x} vs {:#010x}",
            f144.to_bits(),
            o.f144.to_bits()
        );
        assert_eq!(
            f148.to_bits(),
            o.f148.to_bits(),
            "case {ci}: +0x148 {:#010x} vs {:#010x}",
            f148.to_bits(),
            o.f148.to_bits()
        );

        let dt = c.t1.wrapping_sub(c.t0);
        let dt_s = (i64::from(dt) as f64) * f64::from(f32::from_bits(0x3a83_126f));
        let dt_s = if dt_s.abs() < f64::from(f32::from_bits(0x3480_0000)) {
            0.0
        } else {
            dt_s
        };
        if dt_s == 0.0 {
            tally[0] += 1;
        } else {
            let dh = f64::from(c.h1) - f64::from(c.h0);
            if dh > f64::from(f32::from_bits(0x4049_0fdb)) {
                tally[1] += 1;
            } else if !(dh < f64::from(f32::from_bits(0xc049_0fdb))) {
                tally[3] += 1;
            } else {
                tally[2] += 1;
            }
        }
        if o.f148.to_bits() == 0 {
            deadzone += 1;
        }
        checked += 1;
    }
    eprintln!(
        "facing_interp: {checked} bit-exact; legs [skip={} sub2π={} add2π={} asis={}] zero148={deadzone}",
        tally[0], tally[1], tally[2], tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "facing_interp leg {i} never hit");
    }
    assert!(deadzone > 0, "dead-zone +0x148 zero never hit");
}

/// `0x619090`: `+0x14c`.
#[test]
fn reconcile_gate_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    let cell_head = emu.alloc(8);
    let cell_ok = emu.alloc(8);
    patch_eax(&mut emu, 0x6158e0, cell_head, 0);
    patch_eax(&mut emu, 0x7c5360, cell_ok, 8);
    emu.stub(0x618920, 1, 1);
    let this_va = emu.alloc(0x200);
    let node_va = emu.alloc(0x60);
    let sent: i32 = 0x7fc0_0001;

    #[derive(Clone, Copy)]
    struct C {
        head_null: bool,
        active: u8,
        opcode: u32,
        yaw_s: u32,
        yaw_e: u32,
        pitch_s: u32,
        pitch_e: u32,
        ok: bool,
        flag3: bool,
        base: [f32; 3],
        ev: [f32; 3],
        now: i32,
        ev_ts: i32,
    }
    let mk = |base: [f32; 3], ev: [f32; 3], flag3: bool| C {
        head_null: false,
        active: 1,
        opcode: 0,
        yaw_s: 0x1111,
        yaw_e: 0x1111,
        pitch_s: 0x2222,
        pitch_e: 0x2222,
        ok: true,
        flag3,
        base,
        ev,
        now: 1000,
        ev_ts: 1300,
    };
    let mut cases: Vec<C> = vec![
        C {
            head_null: true,
            ..mk([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], false)
        },
        C {
            active: 0,
            ..mk([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], false)
        },
        C {
            opcode: 0x26,
            ..mk([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], false)
        },
        C {
            yaw_e: 0x9999,
            ..mk([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], false)
        },
        C {
            pitch_e: 0x9999,
            ..mk([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], false)
        },
        C {
            ok: false,
            ..mk([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], false)
        },
        mk([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], false),
        mk([10.0, 20.0, 3.0], [1.0, 2.0, 3.0], false),
        mk([1.0, 2.0, 3.0], [1.0, 2.0, 30.0], true),
        mk([1.0, 2.0, 3.0], [1.0, 2.0, 30.0], false),
    ];
    let mut s = 0x1e_c04001u32;
    for case in 0..900u32 {
        let base = [fz(&mut s) * 4.0, fz(&mut s) * 4.0, fz(&mut s) * 4.0];
        let jitter = match case % 3 {
            0 => 0.0,
            1 => fz(&mut s) * 0.001,
            _ => fz(&mut s) * 2.0,
        };
        let ev = [base[0] + jitter, base[1] + jitter, base[2] + jitter * 3.0];
        cases.push(C {
            head_null: next(&mut s) & 0x700 == 0,
            active: if next(&mut s) & 0x40 == 0 { 0 } else { 1 },
            opcode: if next(&mut s) & 0x40 == 0 { 0x26 } else { 0 },
            yaw_s: 0x1111,
            yaw_e: if next(&mut s) & 0x40 == 0 {
                0x9999
            } else {
                0x1111
            },
            pitch_s: 0x2222,
            pitch_e: 0x2222,
            ok: next(&mut s) & 0x70 != 0,
            flag3: next(&mut s) & 0x80 != 0,
            base,
            ev,
            now: 1000,
            ev_ts: 1000 + ((next(&mut s) % 600) as i32),
        });
    }
    let mut tally = [0u32; 3];
    let mut axis = [0u32; 2];
    let mut checked = 0u32;
    for (ci, c) in cases.iter().enumerate() {
        emu.write_u32(cell_head, if c.head_null { 0 } else { node_va as u32 });
        emu.write_u32(cell_ok, c.ok as u32);
        emu.write_u32(node_va + 0x4c, c.active as u32);
        emu.write_u32(node_va + 0xc, c.opcode);
        emu.write_u32(node_va + 0x28, c.yaw_e);
        emu.write_u32(node_va + 0x2c, c.pitch_e);
        emu.write_u32(node_va + 8, c.ev_ts as u32);
        wv3(&mut emu, node_va + 0x10, &c.ev);
        emu.write_u32(this_va + 0x38, c.yaw_s);
        emu.write_u32(this_va + 0x3c, c.pitch_s);
        emu.write_u32(this_va + 0x40, if c.flag3 { 0x200000 } else { 0 });
        wv3(&mut emu, this_va + 0x44, &c.base);
        emu.write_u32(this_va + 0x58, 0);
        emu.write_u32(this_va + 0x14c, sent as u32);
        emu.call(0x619090, this_va as u32, 0, &[c.now as u32]);
        let f14c = emu.read_u32(this_va + 0x14c) as i32;

        let window = c.ev_ts.wrapping_sub(c.now);
        let o = reconcile_gate(
            c.head_null,
            c.active,
            c.opcode,
            c.yaw_s,
            c.yaw_e,
            c.pitch_s,
            c.pitch_e,
            c.ok,
            [0.0, 0.0, 0.0],
            c.flag3,
            c.base,
            c.ev,
            window,
        );
        assert_eq!(f14c, o, "case {ci}: +0x14c {f14c:#x} vs {o:#x}");

        let reject = c.head_null
            || c.active == 0
            || c.opcode == 0x26
            || c.yaw_s != c.yaw_e
            || c.pitch_s != c.pitch_e
            || !c.ok;
        if reject {
            tally[0] += 1;
        } else {
            axis[c.flag3 as usize] += 1;
            if o == 0 {
                tally[2] += 1;
            } else {
                tally[1] += 1;
            }
        }
        checked += 1;
    }
    eprintln!(
        "reconcile_gate: {checked} bit-exact; [reject={} arm={} clear={}] axis[2d={} 3d={}]",
        tally[0], tally[1], tally[2], axis[0], axis[1]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "reconcile_gate leg {i} never hit");
    }
    assert!(axis[0] > 0 && axis[1] > 0, "2-axis/3-axis not both hit");
}

/// `0x6191c0`: `eax`, the output and `+0x14c`.
#[test]
fn pos_lerp_difftest() {
    let Some(wow) = Wow::load() else { return };
    let mut emu = Emu::new(&wow);
    let cell_head = emu.alloc(8);
    patch_eax(&mut emu, 0x6158e0, cell_head, 0);
    let this_va = emu.alloc(0x200);
    let node_va = emu.alloc(0x60);
    let out_va = emu.alloc(0x10);
    let sent = 0x7fc0_0001u32;

    #[derive(Clone, Copy)]
    struct C {
        head_null: bool,
        ev_ts: i32,
        now: i32,
        win: i32,
        cur: [f32; 3],
        tgt: [f32; 3],
    }
    let mut cases: Vec<C> = vec![
        C {
            head_null: true,
            ev_ts: 0,
            now: 0,
            win: 100,
            cur: [1.0, 2.0, 3.0],
            tgt: [0.0, 0.0, 0.0],
        },
        C {
            head_null: false,
            ev_ts: 900,
            now: 1000,
            win: 100,
            cur: [1.0, 2.0, 3.0],
            tgt: [0.0, 0.0, 0.0],
        },
        C {
            head_null: false,
            ev_ts: 1050,
            now: 1000,
            win: 100,
            cur: [4.0, 6.0, 8.0],
            tgt: [0.0, 0.0, 0.0],
        },
        C {
            head_null: false,
            ev_ts: 1100,
            now: 1000,
            win: 100,
            cur: [4.0, 6.0, 8.0],
            tgt: [1.0, 1.0, 1.0],
        },
        C {
            head_null: false,
            ev_ts: 1000,
            now: 1000,
            win: 100,
            cur: [4.0, 6.0, 8.0],
            tgt: [1.0, 1.0, 1.0],
        },
    ];
    let mut s = 0x10_e89001u32;
    for case in 0..900u32 {
        let now = (next(&mut s) % 100000) as i32;
        let delta = match case % 4 {
            0 => -((next(&mut s) % 200) as i32),
            _ => (next(&mut s) % 500) as i32,
        };
        cases.push(C {
            head_null: next(&mut s) & 0x700 == 0,
            ev_ts: now + delta,
            now,
            win: ((next(&mut s) % 500) + 1) as i32,
            cur: [fz(&mut s) * 8.0, fz(&mut s) * 8.0, fz(&mut s) * 8.0],
            tgt: [fz(&mut s) * 8.0, fz(&mut s) * 8.0, fz(&mut s) * 8.0],
        });
    }
    let mut tally = [0u32; 3];
    let mut checked = 0u32;
    for (ci, c) in cases.iter().enumerate() {
        emu.write_u32(cell_head, if c.head_null { 0 } else { node_va as u32 });
        emu.write_u32(node_va + 8, c.ev_ts as u32);
        wv3(&mut emu, node_va + 0x10, &c.tgt);
        emu.write_u32(this_va + 0x14c, c.win as u32);
        wv3(&mut emu, out_va, &c.cur);
        for k in 0..3u64 {
            let _ = k;
        }
        let eax = emu.call(0x6191c0, this_va as u32, 0, &[c.now as u32, out_va as u32]);
        let out_bin = rv3(&emu, out_va);
        let f14c_bin = emu.read_u32(this_va + 0x14c) as i32;

        let o = pos_lerp(c.head_null, c.ev_ts, c.now, c.win, c.cur, c.tgt, c.win);
        assert_eq!(eax, o.eax, "case {ci}: eax {eax:#x} vs {:#x}", o.eax);
        assert_eq!(
            f14c_bin, o.f14c,
            "case {ci}: +0x14c {f14c_bin:#x} vs {:#x}",
            o.f14c
        );
        for k in 0..3 {
            assert_eq!(
                out_bin[k].to_bits(),
                o.out[k].to_bits(),
                "case {ci}: out[{k}] {:#010x} vs {:#010x}",
                out_bin[k].to_bits(),
                o.out[k].to_bits()
            );
        }
        let _ = sent;
        if c.head_null {
            tally[0] += 1;
        } else if c.ev_ts.wrapping_sub(c.now) < 0 {
            tally[1] += 1;
        } else {
            tally[2] += 1;
        }
        checked += 1;
    }
    eprintln!(
        "pos_lerp: {checked} bit-exact; [null={} past={} lerp={}]",
        tally[0], tally[1], tally[2]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "pos_lerp leg {i} never hit");
    }
}
