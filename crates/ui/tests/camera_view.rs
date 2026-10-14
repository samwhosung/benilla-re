//! `CCamera::SetLookAt` (`0x50d5d0`) and the basis rows `CCamera::GetForward` (`0x511f40`),
//! `CCamera::GetLeft` (`0x511fe0`) and `CCamera::GetUp` (`0x512080`) run from the exe.
//!
//! `SetLookAt` runs with the basis builder `0x50a930` stubbed; the test reads the direction and the
//! roll's sine and cosine from the stub's arguments. The rows run with the rotation builder
//! `0x511e90` replaced by an island that returns a chosen matrix. The sine and cosine route
//! through the host's maths library in the emulator and in Rust alike. Every draw comes off the
//! high bits of an LCG, and each branch is tallied and required to be hit.
#![allow(
    clippy::float_cmp,
    clippy::needless_range_loop,
    clippy::neg_cmp_op_on_partial_ord
)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::camera_view::{basis_row, set_look_at};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// f32 in `[lo, hi)` from the LCG high bits.
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

/// Compare two `f32`s by bits; both-NaN passes (transcendental routing matched, payload unspeced).
fn eqf(bin: u32, mine: f32, ctx: &str) {
    if f32::from_bits(bin).is_nan() && mine.is_nan() {
        return;
    }
    assert_eq!(
        bin,
        mine.to_bits(),
        "{ctx}: binary {:#010x} ({}) vs rust {:#010x} ({mine})",
        bin,
        f32::from_bits(bin),
        mine.to_bits()
    );
}

/// Overwrite a function's prologue with raw island bytes (for boundary fns whose `st0`/pointer
/// return the `Emu::stub` mechanism cannot synthesize).
fn island(emu: &mut Emu, addr: u64, bytes: &[u8]) {
    emu.write(addr, bytes);
}
/// `mov eax, imm32; ret n` island bytes.
fn mov_eax_ret(val: u32, ret_n: u16) -> Vec<u8> {
    let mut b = vec![0xb8u8];
    b.extend_from_slice(&val.to_le_bytes());
    b.push(0xc2);
    b.extend_from_slice(&ret_n.to_le_bytes());
    b
}
// ===========================================================================================
fn run_3x3(emu: &mut Emu, fn_va: u64, this: u64, out: u64, src_off: u64, mbuf: u64) {
    fn body(emu: &mut Emu, fn_va: u64, this: u64, out: u64, src_off: u64, mbuf: u64) {
        let mut s = (fn_va as u32).wrapping_mul(2654435761);
        let mut tally_id = 0u32;
        let mut tally_mat = 0u32;
        for i in 0..3000usize {
            let identity = (next(&mut s) >> 8) & 1 == 0;
            let v = [
                rng(&mut s, -10.0, 10.0),
                rng(&mut s, -10.0, 10.0),
                rng(&mut s, -10.0, 10.0),
            ];
            let mut m = [0f32; 9];
            for mm in m.iter_mut() {
                *mm = rng(&mut s, -2.0, 2.0);
            }
            emu.write_u32(this + 0x98, if identity { 0 } else { 1 });
            emu.write_u32(this + 0x9c, 0);
            for k in 0..3 {
                emu.write_u32(this + src_off + 4 * k as u64, v[k].to_bits());
            }
            for k in 0..9 {
                emu.write_u32(mbuf + 4 * k as u64, m[k].to_bits());
            }
            for k in 0..3 {
                emu.write_u32(out + 4 * k, 0x7f80_0001 + k as u32); // poison
            }
            emu.call(fn_va, this as u32, 0, &[out as u32]);
            let mine = basis_row(v, identity, &m);
            for k in 0..3 {
                eqf(
                    emu.read_u32(out + 4 * k as u64),
                    mine[k],
                    &format!("{fn_va:#x} case {i} identity={identity} comp{k}"),
                );
            }
            if identity {
                tally_id += 1;
            } else {
                tally_mat += 1;
            }
        }
        eprintln!("{fn_va:#x}: 3000 bit-exact; identity={tally_id} matrix={tally_mat}");
        assert!(
            tally_id > 0 && tally_mat > 0,
            "{fn_va:#x} branch coverage gap"
        );
    }
    body(emu, fn_va, this, out, src_off, mbuf);
}

fn setup_3x3<'a>(wow: &'a Wow) -> (Emu<'a>, u64, u64, u64) {
    let mut emu = Emu::new(wow);
    let this = emu.alloc(0x40);
    let out = emu.alloc(12);
    let mbuf = emu.alloc(36);
    // 0x511e90 returns a pointer to the 3×3 matrix in eax — overwrite with `mov eax,mbuf; ret 4`.
    let isl = mov_eax_ret(mbuf as u32, 4);
    island(&mut emu, 0x511e90, &isl);
    (emu, this, out, mbuf)
}

#[test]
fn cam_511f40() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut emu, this, out, mbuf) = setup_3x3(&wow);
    run_3x3(&mut emu, 0x511f40, this, out, 0x14, mbuf);
}

#[test]
fn cam_511fe0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut emu, this, out, mbuf) = setup_3x3(&wow);
    run_3x3(&mut emu, 0x511fe0, this, out, 0x20, mbuf);
}

#[test]
fn cam_512080() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let (mut emu, this, out, mbuf) = setup_3x3(&wow);
    run_3x3(&mut emu, 0x512080, this, out, 0x2c, mbuf);
}
#[test]
fn cam_50d5d0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x40);
    let eye_p = emu.alloc(12);
    let tgt_p = emu.alloc(12);
    const BASIS: u64 = 0x50a930;
    emu.stub(BASIS, 0, 2);

    struct C {
        eye: [f32; 3],
        tgt: [f32; 3],
        roll: f32,
    }
    let mut cases: Vec<C> = vec![C {
        eye: [10.0, 20.0, 30.0],
        tgt: [10.0, 20.0, 30.0],
        roll: 0.0,
    }];
    let mut s = 0x050d_5d00u32;
    let mut below = 0u32;
    for _ in 0..4000 {
        cases.push(C {
            eye: [
                rng(&mut s, -500.0, 500.0),
                rng(&mut s, -500.0, 500.0),
                rng(&mut s, -200.0, 200.0),
            ],
            tgt: [
                rng(&mut s, -500.0, 500.0),
                rng(&mut s, -500.0, 500.0),
                rng(&mut s, -200.0, 200.0),
            ],
            roll: rng(&mut s, -6.5, 6.5),
        });
    }
    for (i, c) in cases.iter().enumerate() {
        for k in 0..3 {
            emu.write_u32(eye_p + 4 * k as u64, c.eye[k].to_bits());
            emu.write_u32(tgt_p + 4 * k as u64, c.tgt[k].to_bits());
        }
        emu.clear_calls();
        emu.call(
            0x50d5d0,
            this as u32,
            0,
            &[eye_p as u32, tgt_p as u32, c.roll.to_bits()],
        );
        let call = emu
            .calls()
            .iter()
            .find(|cc| cc.addr == BASIS)
            .copied()
            .unwrap_or_else(|| panic!("case {i}: basis 0x50a930 not called"));
        let dir_p = call.args[0] as u64; // &dir{dx,dy,dz}
        let roll_p = call.args[1] as u64; // &{flag, sin, cos}
        let mine = set_look_at(c.eye, c.tgt, c.roll);
        for k in 0..3 {
            eqf(
                emu.read_u32(dir_p + 4 * k as u64),
                mine.dir[k],
                &format!("50d5d0 case {i} dir[{k}]"),
            );
        }
        assert_eq!(
            emu.read_u32(roll_p),
            0,
            "50d5d0 case {i}: roll-struct flag must be 0"
        );
        eqf(
            emu.read_u32(roll_p + 4),
            mine.sin,
            &format!("50d5d0 case {i} sin"),
        );
        eqf(
            emu.read_u32(roll_p + 8),
            mine.cos,
            &format!("50d5d0 case {i} cos"),
        );
        if !(2f64.powi(-22) <= {
            let d: Vec<f64> = (0..3).map(|k| c.tgt[k] as f64 - c.eye[k] as f64).collect();
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt().abs()
        }) {
            below += 1;
        }
    }
    eprintln!(
        "cam_50d5d0: {} bit-exact (sin/cos structural; {below} below-eps)",
        cases.len()
    );
    assert!(below > 0, "50d5d0: eps below-threshold branch never hit");
}
