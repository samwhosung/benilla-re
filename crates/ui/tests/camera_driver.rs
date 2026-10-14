//! `CCamera::FilterFacing` (`0x50d810`), `CCamera::PlaceEye` (`0x50de00`) and
//! `CCamera::IntegrateAxes` (`0x5112d0`) run from the exe.
//!
//! The camera object, the CVar records and the clock are built in emulator memory; the forward-row
//! getter in the vtable is replaced by an island that returns a chosen vector. `FilterFacing`
//! returns in `st0` and is compared as an `f64`. `IntegrateAxes` runs with every queued end 0, so
//! `CCamera::StopAxis` (`0x50fd90`) is never called. Every draw comes off the high bits of an LCG,
//! and each branch is tallied and required to be hit.
#![allow(
    clippy::float_cmp,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::neg_cmp_op_on_partial_ord
)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::camera_driver::{filter_facing, integrate_axes, place_eye, KbState};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}
/// f32 in `[lo, hi)` from the LCG high bits.
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}
fn eqf(bin: u32, mine: f32, ctx: &str) {
    assert_eq!(
        bin,
        mine.to_bits(),
        "{ctx}: binary {:#010x} ({}) vs rust {:#010x} ({mine})",
        bin,
        f32::from_bits(bin),
        mine.to_bits()
    );
}
fn eqf64(bin: u64, mine: f64, ctx: &str) {
    assert_eq!(
        bin,
        mine.to_bits(),
        "{ctx}: binary {:#018x} ({}) vs rust {:#018x} ({mine})",
        bin,
        f64::from_bits(bin),
        mine.to_bits()
    );
}

/// `mov eax, imm32; ret 4` island — returns a fixed pointer, cleans one stack arg (thiscall).
fn mov_eax_ret4_stub(emu: &mut Emu, val: u32) -> u64 {
    let addr = emu.alloc(8);
    let mut code = vec![0xb8u8];
    code.extend_from_slice(&val.to_le_bytes());
    code.extend_from_slice(&[0xc2, 0x04, 0x00]);
    emu.write(addr, &code);
    addr
}

// ============================ 50d810 ============================

#[test]
fn cam_50d810() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200);
    // runtime gate: ds:0xbe1074 -> obj; obj+0x28 != 0 enables.
    let gate = emu.alloc(0x40);
    emu.write_u32(0xbe1074, gate as u32);

    struct C {
        field: f32,
        target: f32,
        gate_on: bool,
        p3: bool,
    }
    let mut cases: Vec<C> = vec![
        // disabled via gate / via p3.
        C {
            field: 0.5,
            target: 1.0,
            gate_on: false,
            p3: true,
        },
        C {
            field: 0.5,
            target: 1.0,
            gate_on: true,
            p3: false,
        },
        // deadzone (|d|<=0.09).
        C {
            field: 1.0,
            target: 1.05,
            gate_on: true,
            p3: true,
        },
        // small step (deadzone<|d|<=15°).
        C {
            field: 1.0,
            target: 1.15,
            gate_on: true,
            p3: true,
        },
        // max step (|d|>15°).
        C {
            field: 0.0,
            target: 2.0,
            gate_on: true,
            p3: true,
        },
        // wrap: target far below.
        C {
            field: 3.0,
            target: -3.0,
            gate_on: true,
            p3: true,
        },
    ];
    let mut s = 0x050d_8100u32;
    let (mut dz, mut small, mut big, mut dis, mut wrapped) = (0u32, 0u32, 0u32, 0u32, 0u32);
    for _ in 0..6000 {
        cases.push(C {
            field: rng(&mut s, -6.5, 6.5),
            target: rng(&mut s, -6.5, 6.5),
            gate_on: !(next(&mut s) >> 8).is_multiple_of(5),
            p3: !(next(&mut s) >> 8).is_multiple_of(5),
        });
    }
    let pi = std::f64::consts::PI;
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(this + 0x184, c.field.to_bits());
        emu.write_u32(gate + 0x28, u32::from(c.gate_on));
        let got = emu.call_f64(
            0x50d810,
            this as u32,
            0,
            &[c.target.to_bits(), u32::from(c.p3)],
        );
        let enabled = c.gate_on && c.p3;
        let mine = filter_facing(c.field, c.target, enabled);
        eqf64(
            got.to_bits(),
            mine,
            &format!(
                "50d810 case {i} (field={} tgt={} en={enabled})",
                c.field, c.target
            ),
        );
        if !enabled {
            dis += 1;
        } else {
            // classify by the f32-rounded unwrapped delta
            let mut d = c.target as f64 - c.field as f64;
            if d <= f32::from_bits(0x4049_0fdb) as f64 {
                if d < -(f32::from_bits(0x4049_0fdb) as f64) {
                    d += f32::from_bits(0x40c9_0fdb) as f64;
                }
            } else {
                d -= f32::from_bits(0x40c9_0fdb) as f64;
            }
            let adf = (d as f32 as f64).abs();
            let _ = pi;
            if adf <= 0.09000001 {
                dz += 1;
            } else if adf <= 0.26179942 {
                small += 1;
            } else {
                big += 1;
            }
            if !(0.0..=6.2831855).contains(&mine) {
                wrapped += 1;
            }
        }
    }
    eprintln!("cam_50d810: {} bit-exact (deadzone={dz} small={small} big={big} disabled={dis} rewrap-out={wrapped})", cases.len());
    assert!(
        dz > 0 && small > 0 && big > 0 && dis > 0,
        "50d810 coverage gap"
    );
}

// ============================ 50de00 ============================

#[test]
fn cam_50de00() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // dir vector + vtable getter island returning &dir; `this` with vtable at +0.
    let dir_p = emu.alloc(12);
    let getter = mov_eax_ret4_stub(&mut emu, dir_p as u32);
    let vtable = emu.alloc(0x10);
    emu.write_u32(vtable + 4, getter as u32);
    let this = emu.alloc(0x10);
    emu.write_u32(this, vtable as u32);
    let out_p = emu.alloc(12);
    let p_p = emu.alloc(12);
    let off_p = emu.alloc(12);

    struct C {
        p: [f32; 3],
        dir: [f32; 3],
        t: f32,
        off: [f32; 3],
    }
    let mut cases: Vec<C> = Vec::new();
    let mut s = 0x050d_e000u32;
    for _ in 0..5000 {
        cases.push(C {
            p: [
                rng(&mut s, -500.0, 500.0),
                rng(&mut s, -500.0, 500.0),
                rng(&mut s, -200.0, 200.0),
            ],
            dir: [
                rng(&mut s, -1.0, 1.0),
                rng(&mut s, -1.0, 1.0),
                rng(&mut s, -1.0, 1.0),
            ],
            t: rng(&mut s, 0.0, 30.0),
            off: [
                rng(&mut s, -50.0, 50.0),
                rng(&mut s, -50.0, 50.0),
                rng(&mut s, -50.0, 50.0),
            ],
        });
    }
    for (i, c) in cases.iter().enumerate() {
        for k in 0..3 {
            emu.write_u32(dir_p + 4 * k as u64, c.dir[k].to_bits());
            emu.write_u32(p_p + 4 * k as u64, c.p[k].to_bits());
            emu.write_u32(off_p + 4 * k as u64, c.off[k].to_bits());
        }
        emu.call(
            0x50de00,
            this as u32,
            0,
            &[out_p as u32, p_p as u32, c.t.to_bits(), off_p as u32],
        );
        let mine = place_eye(c.p, c.dir, c.t, c.off);
        for k in 0..3 {
            eqf(
                emu.read_u32(out_p + 4 * k as u64),
                mine[k],
                &format!("50de00 case {i} out[{k}]"),
            );
        }
    }
    eprintln!(
        "cam_50de00: {} bit-exact (z-term f32 asymmetry)",
        cases.len()
    );
}

// ============================ 5112d0 ============================

#[test]
fn cam_5112d0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x300);
    // five cvar objects, each {+0x24 = float}, pointed to by the DAT globals.
    let cvar_addrs = [
        (0xbe0fe0u64, emu.alloc(0x30)), // zoom A
        (0xbe0e6cu64, emu.alloc(0x30)), // zoom B
        (0xbe0fbcu64, emu.alloc(0x30)), // zoom rate
        (0xbe1094u64, emu.alloc(0x30)), // yaw rate
        (0xbe0f78u64, emu.alloc(0x30)), // pitch rate
    ];
    for (g, obj) in cvar_addrs {
        emu.write_u32(g, obj as u32);
    }

    // field offsets
    const F90: u64 = 0x90;
    const F128: u64 = 0x128;
    const LAST: u64 = 0x12c;
    const RELEASE: u64 = 0x144;
    const HANDLE: u64 = 0x15c;
    const ZOOM: u64 = 0x198;
    const ZMIR: u64 = 0xec;
    const YAW: u64 = 0x210;
    const YMIR: u64 = 0xf0;
    const PITCH: u64 = 0x1e0;
    const PMIR: u64 = 0xf4;

    struct C {
        st: KbState,
        za: f32,
        zb: f32,
        zr: f32,
        yr: f32,
        pr: f32,
        now: i32,
    }
    fn base() -> KbState {
        KbState {
            mode: 0,
            axes: 0,
            start_ms: [0; 6],
            end_ms: [0; 6],
            queued_end: [0; 6],
            distance: 5.0,
            live_distance: 0.0,
            yaw: 1.0,
            live_yaw: 0.0,
            pitch: 0.0,
            live_pitch: 0.0,
        }
    }
    let mut cases: Vec<C> = Vec::new();
    // one case per axis (relative mode active), plus release-mode + suppress variants.
    for axis in 0..6u32 {
        let mut st = base();
        st.axes = 1 << (axis * 2); // active, relative
        cases.push(C {
            st,
            za: 2.0,
            zb: 1.5,
            zr: 12.0,
            yr: 90.0,
            pr: 90.0,
            now: 100,
        });
    }
    // release mode (bit 2i+1 set): exercises the clear-bits + release-tick path.
    for axis in 0..6u32 {
        let mut st = base();
        st.axes = (1 << (axis * 2)) | (1 << (axis * 2 + 1));
        st.end_ms[axis as usize] = 80;
        cases.push(C {
            st,
            za: 2.0,
            zb: 1.5,
            zr: 12.0,
            yr: 90.0,
            pr: 90.0,
            now: 100,
        });
    }
    // mirror-suppress variants.
    for (axis, bit) in [(1u32, 0x4000000u32), (2, 0x1000000), (4, 0x2000000)] {
        let mut st = base();
        st.axes = 1 << (axis * 2);
        st.mode = bit;
        cases.push(C {
            st,
            za: 2.0,
            zb: 1.5,
            zr: 12.0,
            yr: 90.0,
            pr: 90.0,
            now: 100,
        });
    }
    // zoom upper-clamp (large product) and zoom lower-clamp (drive zoom negative).
    {
        let mut st = base();
        st.axes = 1 << 2; // zoom-in axis
        st.distance = 49.5;
        cases.push(C {
            st,
            za: 60.0,
            zb: 1.0,
            zr: 5000.0,
            yr: 0.0,
            pr: 0.0,
            now: 100,
        });
    }
    {
        let mut st = base();
        st.axes = 1; // zoom-out axis
        st.distance = 0.2;
        cases.push(C {
            st,
            za: 1.0,
            zb: 1.0,
            zr: 5000.0,
            yr: 0.0,
            pr: 0.0,
            now: 100,
        });
    }
    // negative elapsed -> axis skipped.
    {
        let mut st = base();
        st.axes = 1 << 4;
        st.start_ms[2] = 200;
        cases.push(C {
            st,
            za: 2.0,
            zb: 1.5,
            zr: 12.0,
            yr: 90.0,
            pr: 90.0,
            now: 100,
        });
    }

    let mut s = 0x0511_2d00u32;
    for _ in 0..3000 {
        let mut st = base();
        // random active mask over the 6 axes; random relative/release per active axis.
        let mut mask = 0u32;
        for axis in 0..6u32 {
            if (next(&mut s) >> 8).is_multiple_of(2) {
                mask |= 1 << (axis * 2);
                if (next(&mut s) >> 8).is_multiple_of(3) {
                    mask |= 1 << (axis * 2 + 1);
                    st.end_ms[axis as usize] = (next(&mut s) >> 8) as i32 % 150;
                }
            }
        }
        st.axes = mask;
        st.mode = (next(&mut s) >> 8) & 0x7000000;
        st.distance = rng(&mut s, -2.0, 55.0);
        st.yaw = rng(&mut s, -1.0, 7.0);
        st.pitch = rng(&mut s, -2.0, 2.0);
        for a in 0..6 {
            st.start_ms[a] = (next(&mut s) >> 8) as i32 % 200;
        }
        cases.push(C {
            st,
            za: rng(&mut s, 0.0, 30.0),
            zb: rng(&mut s, 0.0, 4.0),
            zr: rng(&mut s, 0.0, 4000.0),
            yr: rng(&mut s, 0.0, 200.0),
            pr: rng(&mut s, 0.0, 200.0),
            now: 50 + (next(&mut s) >> 8) as i32 % 200,
        });
    }

    let mut zoom_clamp_hi = 0u32;
    let mut zoom_clamp_lo = 0u32;
    let mut yaw_wrap = 0u32;
    let mut pitch_clamp = 0u32;
    let mut release_hits = 0u32;
    for (i, c) in cases.iter().enumerate() {
        // write cvar values
        emu.write_u32(cvar_addrs[0].1 + 0x24, c.za.to_bits());
        emu.write_u32(cvar_addrs[1].1 + 0x24, c.zb.to_bits());
        emu.write_u32(cvar_addrs[2].1 + 0x24, c.zr.to_bits());
        emu.write_u32(cvar_addrs[3].1 + 0x24, c.yr.to_bits());
        emu.write_u32(cvar_addrs[4].1 + 0x24, c.pr.to_bits());
        // write this fields
        emu.write_u32(this + F90, c.st.mode);
        emu.write_u32(this + F128, c.st.axes);
        for a in 0..6u64 {
            emu.write_u32(this + LAST + a * 4, c.st.start_ms[a as usize] as u32);
            emu.write_u32(this + RELEASE + a * 4, c.st.end_ms[a as usize] as u32);
            emu.write_u32(this + HANDLE + a * 4, c.st.queued_end[a as usize] as u32);
        }
        emu.write_u32(this + ZOOM, c.st.distance.to_bits());
        emu.write_u32(this + ZMIR, c.st.live_distance.to_bits());
        emu.write_u32(this + YAW, c.st.yaw.to_bits());
        emu.write_u32(this + YMIR, c.st.live_yaw.to_bits());
        emu.write_u32(this + PITCH, c.st.pitch.to_bits());
        emu.write_u32(this + PMIR, c.st.live_pitch.to_bits());

        emu.call(0x5112d0, this as u32, 0, &[c.now as u32]);

        let mut mine = c.st;
        integrate_axes(&mut mine, c.za, c.zb, c.zr, c.yr, c.pr, c.now);

        eqf(
            emu.read_u32(this + ZOOM),
            mine.distance,
            &format!("5112d0 case {i} zoom"),
        );
        eqf(
            emu.read_u32(this + ZMIR),
            mine.live_distance,
            &format!("5112d0 case {i} live_distance"),
        );
        eqf(
            emu.read_u32(this + YAW),
            mine.yaw,
            &format!("5112d0 case {i} yaw"),
        );
        eqf(
            emu.read_u32(this + YMIR),
            mine.live_yaw,
            &format!("5112d0 case {i} live_yaw"),
        );
        eqf(
            emu.read_u32(this + PITCH),
            mine.pitch,
            &format!("5112d0 case {i} pitch"),
        );
        eqf(
            emu.read_u32(this + PMIR),
            mine.live_pitch,
            &format!("5112d0 case {i} live_pitch"),
        );
        assert_eq!(emu.read_u32(this + F128), mine.axes, "5112d0 case {i} axes");
        assert_eq!(emu.read_u32(this + F90), mine.mode, "5112d0 case {i} mode");
        for a in 0..6u64 {
            assert_eq!(
                emu.read_u32(this + LAST + a * 4),
                mine.start_ms[a as usize] as u32,
                "5112d0 case {i} start_ms[{a}]"
            );
        }

        if mine.distance == 50.0 {
            zoom_clamp_hi += 1;
        }
        if mine.distance == 0.0 {
            zoom_clamp_lo += 1;
        }
        if mine.yaw < c.st.yaw - 0.001 && (c.st.axes & (1 << 4)) != 0 {
            yaw_wrap += 1;
        }
        if mine.pitch == f32::from_bits(0x3fc6_d3f2) || mine.pitch == f32::from_bits(0xbfc6_d3f2) {
            pitch_clamp += 1;
        }
        if (c.st.axes & 0xaaa) != 0 {
            release_hits += 1;
        }
    }
    eprintln!("cam_5112d0: {} bit-exact (zoom_hi={zoom_clamp_hi} zoom_lo={zoom_clamp_lo} yaw_wrap={yaw_wrap} pitch_clamp={pitch_clamp} release={release_hits})", cases.len());
    assert!(
        zoom_clamp_hi > 0 && zoom_clamp_lo > 0 && pitch_clamp > 0 && release_hits > 0,
        "5112d0 coverage gap"
    );
}
