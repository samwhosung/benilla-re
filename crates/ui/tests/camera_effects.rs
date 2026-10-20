//! `EvaluateShake` (`0x511610`), `CCamera::ComputeBob` (`0x511920`) and the slope probe
//! `CCamera::ProbeSlope` (`0x50d900`) run from the exe.
//!
//! The shake and the bob route `fsin`, `fcos`, `fsincos` and `f2xm1` through the host's maths
//! library in the emulator and in Rust alike, so they compare bit for bit. The bob is run past
//! its two gates, with the unit lookup and the speed routine replaced by islands that return chosen
//! values. The probe runs with the world ray `0x672170` replaced: once as a miss, and once as a
//! counter island that writes chosen hit points into the segment it is given. Every draw comes off
//! the high bits of an LCG, and each branch is tallied and required to be hit.
//!
//! Host double (`docs/METHOD.md#the-harness`): `cam_511610` runs `fsin` at `0x511625`; `f2xm1` at
//! `0x511643`; `fsincos` at `0x511692` and `0x5116af`. `cam_50d900` runs `fsincos` at `0x50d9b4`.
//! `cam_511920` runs `fsin` at `0x511a1c` and `0x511a61` and `fsincos` at `0x511a39`. The emulator
//! computes each in host `double`, so where its result reaches a compared value the host's math
//! stands in for the x87's, and the test shows the code around it.
#![allow(
    clippy::float_cmp,
    clippy::too_many_arguments,
    clippy::needless_range_loop,
    clippy::vec_init_then_push
)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::camera_effects::{
    compute_bob as bob, evaluate_shake as shake, probe_slope_hit, probe_slope_no_hit, ShakeAxis,
};

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
/// `fld dword [cell]; ret n` island bytes.
fn fld_ret(cell: u32, ret_n: u16) -> Vec<u8> {
    let mut b = vec![0xd9u8, 0x05];
    b.extend_from_slice(&cell.to_le_bytes());
    b.push(0xc2);
    b.extend_from_slice(&ret_n.to_le_bytes());
    b
}
/// `fld dword [cell]; ret`: a getter that returns the float in `cell` in `st0`.
fn fld_stub(emu: &mut Emu, cell: u64) -> u64 {
    let addr = emu.alloc(8);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(addr, &code);
    addr
}

// ===========================================================================================
// 0x511610 — damped sinusoidal shake (cdecl)
// ===========================================================================================
#[test]
fn cam_511610() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let out = emu.alloc(12);

    struct C {
        o: [f32; 3],
        phase: f32,
        amp: f32,
        freq: f32,
        t: f32,
        decay: bool,
        decay_t: f32,
        axis: u32,
    }
    let mut cases: Vec<C> = Vec::new();
    // each axis mode {0,1,2,3(no-op)} with and without decay.
    for axis in 0..4u32 {
        for decay in [false, true] {
            cases.push(C {
                o: [1.0, 2.0, 3.0],
                phase: 0.7,
                amp: 4.0,
                freq: 2.5,
                t: 0.3,
                decay,
                decay_t: 0.5,
                axis,
            });
        }
    }
    let mut s = 0x0511_6100u32;
    for _ in 0..4000 {
        cases.push(C {
            o: [
                rng(&mut s, -5.0, 5.0),
                rng(&mut s, -5.0, 5.0),
                rng(&mut s, -5.0, 5.0),
            ],
            phase: rng(&mut s, -6.5, 6.5),
            amp: rng(&mut s, -10.0, 10.0),
            freq: rng(&mut s, 0.0, 8.0),
            t: rng(&mut s, 0.0, 4.0),
            decay: (next(&mut s) >> 8) & 1 == 0,
            decay_t: rng(&mut s, 0.0, 3.0),
            axis: (next(&mut s) >> 8) % 4,
        });
    }

    let mut tally = [0u32; 4];
    let mut decay_hit = 0u32;
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(out, c.o[0].to_bits());
        emu.write_u32(out + 4, c.o[1].to_bits());
        emu.write_u32(out + 8, c.o[2].to_bits());
        emu.call(
            0x511610,
            0,
            0,
            &[
                out as u32,
                c.phase.to_bits(),
                c.amp.to_bits(),
                c.freq.to_bits(),
                c.t.to_bits(),
                c.decay as u32,
                c.decay_t.to_bits(),
                c.axis,
            ],
        );
        let axis = match c.axis {
            0 => ShakeAxis::Cos,
            1 => ShakeAxis::CosQuarter,
            2 => ShakeAxis::Z,
            _ => ShakeAxis::None,
        };
        let mine = shake(c.o, c.phase, c.amp, c.freq, c.t, c.decay, c.decay_t, axis);
        for k in 0..3 {
            eqf(
                emu.read_u32(out + 4 * k as u64),
                mine[k],
                &format!("511610 case {i} axis={} comp{k}", c.axis),
            );
        }
        tally[c.axis as usize] += 1;
        if c.decay {
            decay_hit += 1;
        }
    }
    eprintln!(
        "cam_511610: {} bit-exact; per-axis {:?} decay={}",
        cases.len(),
        tally,
        decay_hit
    );
    for (a, h) in tally.iter().enumerate() {
        assert!(*h > 0, "cam_511610 axis {a} never hit");
    }
    assert!(decay_hit > 0, "cam_511610 decay path never hit");
}

// ===========================================================================================
// 0x511920 — weapon/head-bob (thiscall)
// ===========================================================================================
const CV_H: u64 = 0x00be_1064; // cvar ptr (amp_h)
const CV_F: u64 = 0x00be_0cf8; // cvar ptr (freq)
const CV_V: u64 = 0x00be_10c8; // cvar ptr (amp_v)

#[test]
fn cam_511920() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let this = emu.alloc(0x240);
    let out = emu.alloc(12);
    let obj = emu.alloc(0xd00); // the 0x468460 result object (obj+0xc98 = phase)
                                // cvar buffers; the .data globals hold POINTERS to them, value at +0x24.
    let cvh = emu.alloc(0x28);
    let cvf = emu.alloc(0x28);
    let cvv = emu.alloc(0x28);
    emu.write_u32(CV_H, cvh as u32);
    emu.write_u32(CV_F, cvf as u32);
    emu.write_u32(CV_V, cvv as u32);
    // 0x7c4c90 returns the speed in st0 — overwrite with an `fld [cell]; ret 4` island.
    let speed_cell = emu.alloc(4);
    let isl = fld_ret(speed_cell as u32, 4);
    island(&mut emu, 0x7c4c90, &isl);
    // gates: 0x468460 -> obj (nonzero); 0x5105e0 -> 1.
    emu.stub(0x468460, obj as u32, 3);
    emu.stub(0x5105e0, 1, 1);
    // tick 0x42c010 (a jmp-thunk to the GetTickCount path) -> chosen tick via `mov eax,[cell]; ret`.
    let tick_cell = emu.alloc(4);
    {
        let mut isl = vec![0xa1u8];
        isl.extend_from_slice(&(tick_cell as u32).to_le_bytes());
        isl.push(0xc3);
        island(&mut emu, 0x42c010, &isl);
    }

    struct C {
        phase: f32,
        amp_h: f32,
        freq: f32,
        amp_v: f32,
        speed_raw: f32,
        tick: u32,
        last: u32,
    }
    let mut cases: Vec<C> = Vec::new();
    // speed clamp: below 0.5, inside, above 1.5 (×7.2 to invert the /7.2).
    for sr in [0.5f32 * 7.2, 1.0 * 7.2, 1.5 * 7.2, 3.0 * 7.2, 0.1 * 7.2] {
        cases.push(C {
            phase: 0.6,
            amp_h: 2.0,
            freq: 1.3,
            amp_v: 1.5,
            speed_raw: sr,
            tick: 5000,
            last: 1000,
        });
    }
    let mut s = 0x0511_9200u32;
    for _ in 0..4000 {
        cases.push(C {
            phase: rng(&mut s, -6.5, 6.5),
            amp_h: rng(&mut s, -4.0, 4.0),
            freq: rng(&mut s, 0.0, 5.0),
            amp_v: rng(&mut s, -4.0, 4.0),
            speed_raw: rng(&mut s, -2.0, 20.0),
            tick: next(&mut s),
            last: next(&mut s),
        });
    }

    let (mut lo, mut mid, mut hi) = (0u32, 0u32, 0u32);
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(obj + 0xc98, c.phase.to_bits());
        emu.write_u32(cvh + 0x24, c.amp_h.to_bits());
        emu.write_u32(cvf + 0x24, c.freq.to_bits());
        emu.write_u32(cvv + 0x24, c.amp_v.to_bits());
        emu.write_u32(speed_cell, c.speed_raw.to_bits());
        emu.write_u32(tick_cell, c.tick);
        emu.write_u32(this + 0x238, c.last);
        for k in 0..3 {
            emu.write_u32(out + 4 * k, 0xdead_0000 + k as u32); // poison
        }
        emu.call(0x511920, this as u32, 0, &[out as u32]);
        let mine = bob(
            c.phase,
            c.amp_h,
            c.freq,
            c.amp_v,
            c.speed_raw,
            c.tick,
            c.last,
        );
        for k in 0..3 {
            eqf(
                emu.read_u32(out + 4 * k as u64),
                mine[k],
                &format!("511920 case {i} comp{k}"),
            );
        }
        let sp = c.speed_raw / 7.2;
        if sp < 0.5 {
            lo += 1;
        } else if sp > 1.5 {
            hi += 1;
        } else {
            mid += 1;
        }
    }
    eprintln!(
        "cam_511920: {} bit-exact; speed-clamp lo={lo} mid={mid} hi={hi}",
        cases.len()
    );
    assert!(
        lo > 0 && mid > 0 && hi > 0,
        "cam_511920 speed-clamp coverage gap"
    );
}

#[test]
fn cam_50d900() {
    // Two parts: the probe with both rays missing, then with chosen hit points.
    let Some(wow) = Wow::load() else {
        return;
    };

    // ---- both rays miss ----
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200);
    // param2 object: vtable with +0x14 = GetPosition (writes eye out-param), +0x18 = heading getter.
    let eye_payload = emu.alloc(12);
    let head_cell = emu.alloc(4);
    let head_get = fld_stub(&mut emu, head_cell);
    // GetPosition (vtable+0x14): an executable island the slot points at; hooked to write the eye
    // out-param (arg0) and return (thiscall, ret 4).
    let getpos_island = emu.alloc(8);
    emu.stub_write_through(getpos_island, 0, eye_payload, 12, 0, 1);
    let vtable = emu.alloc(0x20);
    emu.write_u32(vtable + 0x14, getpos_island as u32);
    emu.write_u32(vtable + 0x18, head_get as u32);
    let obj = emu.alloc(0x10);
    emu.write_u32(obj, vtable as u32);
    // validity 0x5105a0 -> nonzero; raycast 0x672170 -> no-op no-hit (ret 0x10, 4 stack args).
    emu.stub(0x5105a0, 1, 1);
    emu.stub(0x672170, 0, 4);

    struct C {
        eye: [f32; 3],
        heading: f32,
    }
    let mut cases: Vec<C> = Vec::new();
    let mut s = 0x050d_9000u32;
    for _ in 0..3000 {
        cases.push(C {
            eye: [
                rng(&mut s, -2000.0, 2000.0),
                rng(&mut s, -2000.0, 2000.0),
                rng(&mut s, -100.0, 500.0),
            ],
            heading: rng(&mut s, -6.5, 6.5),
        });
    }
    let mut clamp_hi = 0u32;
    for (i, c) in cases.iter().enumerate() {
        for k in 0..3 {
            emu.write_u32(eye_payload + 4 * k as u64, c.eye[k].to_bits());
        }
        emu.write_u32(head_cell, c.heading.to_bits());
        emu.write_u32(this + 0x188, 0); // last update time = 0
        emu.write_u32(this + 0xa0, 0x7f7fffff); // sentinel to detect a write
                                                // param3 = current time (elapsed 1000 >= 100 -> proceeds)
        emu.call(0x50d900, this as u32, 0, &[obj as u32, 1000]);
        let got = emu.read_f32(this + 0xa0);
        let mine = probe_slope_no_hit(c.eye, c.heading);
        eqf(
            got.to_bits(),
            mine,
            &format!("50d900 case {i} eye={:?} h={}", c.eye, c.heading),
        );
        if mine == f32::from_bits(0x3eb2_b8c2) {
            clamp_hi += 1;
        }
    }
    eprintln!(
        "cam_50d900: {} bit-exact (no-hit path; +20° clamp={clamp_hi}).",
        cases.len()
    );
    assert!(clamp_hi > 0, "50d900 no-hit path expected the +20° clamp");

    // ---- the first ray hits ----
    // 0x672170 writes the clipped hit point into its 2nd stack arg (the probe segment struct, disasm
    // 50da38/50daa7) and 0x50d900 reads it directly (it never tests the return). Within ONE 0x50d900
    // call there are exactly two 0x672170 calls (the first ray, then the second; the 100 ms throttle always
    // passes here, since 1000 ms have elapsed). We replace 0x672170 with a counter island that, on call N∈{0,1}, copies
    // points[N] (3 floats) into [esp+8] (the 2nd stack arg) and toggles the counter — delivering a
    // chosen hit point per probe so the slope sweeps the 10-entry table (0x808a40) and both signs.
    let mut emu = Emu::new(&wow);
    let this = emu.alloc(0x200);
    let eye_payload = emu.alloc(12);
    let head_cell = emu.alloc(4);
    let head_get = fld_stub(&mut emu, head_cell);
    let getpos_island = emu.alloc(8);
    emu.stub_write_through(getpos_island, 0, eye_payload, 12, 0, 1);
    let vtable = emu.alloc(0x20);
    emu.write_u32(vtable + 0x14, getpos_island as u32);
    emu.write_u32(vtable + 0x18, head_get as u32);
    let obj = emu.alloc(0x10);
    emu.write_u32(obj, vtable as u32);
    emu.stub(0x5105a0, 1, 1);
    // counter + 2-point table (3 floats each); island overwrites 0x672170's bytes.
    let ctr = emu.alloc(4);
    let points = emu.alloc(24);
    {
        let c32 = (ctr as u32).to_le_bytes();
        let p32 = (points as u32).to_le_bytes();
        let mut isl: Vec<u8> = Vec::new();
        isl.extend_from_slice(&[0x8b, 0x15]); // mov edx, ds:[ctr]
        isl.extend_from_slice(&c32);
        isl.extend_from_slice(&[0x6b, 0xd2, 0x0c]); // imul edx, edx, 12
        isl.extend_from_slice(&[0x81, 0xc2]); // add edx, points
        isl.extend_from_slice(&p32);
        isl.extend_from_slice(&[0x8b, 0x4c, 0x24, 0x08]); // mov ecx, [esp+8]  (2nd stack arg)
        isl.extend_from_slice(&[0x8b, 0x02, 0x89, 0x01]); // mov eax,[edx]; mov [ecx],eax
        isl.extend_from_slice(&[0x8b, 0x42, 0x04, 0x89, 0x41, 0x04]); // [edx+4]->[ecx+4]
        isl.extend_from_slice(&[0x8b, 0x42, 0x08, 0x89, 0x41, 0x08]); // [edx+8]->[ecx+8]
        isl.extend_from_slice(&[0xa1]); // mov eax, ds:[ctr]
        isl.extend_from_slice(&c32);
        isl.extend_from_slice(&[0x83, 0xf0, 0x01]); // xor eax, 1
        isl.extend_from_slice(&[0xa3]); // mov ds:[ctr], eax
        isl.extend_from_slice(&c32);
        isl.extend_from_slice(&[0x33, 0xc0]); // xor eax, eax
        isl.extend_from_slice(&[0xc2, 0x10, 0x00]); // ret 0x10
        emu.write(0x672170, &isl);
    }

    // host-side mirror of the constants the slope reads.
    const BACKOFF: f64 = f32::from_bits(0x3e8e_38e4) as f64; // 0.2778
    const EYE_Z: f64 = f32::from_bits(0x3fd5_5555) as f64; // 5/3
    const TBL: [u32; 10] = [
        0x0000_0000,
        0x3db8_51ec,
        0x3e38_51ec,
        0x3e8a_3d71,
        0x3eb8_51ec,
        0x3ef0_a3d7,
        0x3f14_7ae1,
        0x3f33_3333,
        0x3f56_8f5c,
        0x3f80_0000,
    ];
    // recompute slope exactly as the transcription (to classify table-segment / sign / clamp).
    let slope_of = |eye: [f32; 3], heading: f32, p1: [f32; 3], p2z: f32| -> f32 {
        let cosf = (heading as f64).cos() as f32;
        let sinf = (heading as f64).sin() as f32;
        let bx = ((p1[0] as f64) - (cosf as f64) * BACKOFF) as f32;
        let by = ((p1[1] as f64) - (sinf as f64) * BACKOFF) as f32;
        let dx = (bx as f64) - eye[0] as f64;
        let dy = (by as f64) - eye[1] as f64;
        let den = (dy * dy + dx * dx).sqrt();
        let num = ((p2z as f64) - (p1[2] as f64)) + EYE_Z;
        (num / den) as f32
    };
    let seg_of = |slope: f32| -> usize {
        let mag = (if 0.0 <= slope { slope } else { -slope }) as f64;
        for i in (0..10usize).rev() {
            if (f32::from_bits(TBL[i]) as f64) <= mag {
                return i;
            }
        }
        0
    };

    struct H {
        eye: [f32; 3],
        heading: f32,
        p1: [f32; 3],
        p2z: f32,
    }
    let mut cases: Vec<H> = Vec::new();
    // targeted: heading 0 (cos=1, sin=0), eye 0, p1=(1+BACKOFF, 0, 0) ⇒ bx=1, by=0 ⇒ den=1 ⇒
    // slope = p2z + 5/3. Sweep p2z = sv − 5/3 to place slope at sv (both signs, every table segment,
    // beyond ±1 → ±20° clamp).
    let p1x0 = 1.0f32 + f32::from_bits(0x3e8e_38e4); // bx = p1x0 - cos·BACKOFF = 1.0 at heading 0
    for sv in [
        0.05f32, 0.13, 0.22, 0.31, 0.40, 0.52, 0.63, 0.75, 0.90, 1.5, -0.05, -0.13, -0.22, -0.31,
        -0.40, -0.52, -0.63, -0.75, -0.90, -1.5, 0.0,
    ] {
        cases.push(H {
            eye: [0.0, 0.0, 0.0],
            heading: 0.0,
            p1: [p1x0, 0.0, 0.0],
            p2z: sv - (5.0 / 3.0),
        });
    }
    // fuzz: varied heading (exercises the sin/cos backoff pairing), eye, and both hit points.
    let mut s = 0x050d_9001u32;
    for _ in 0..5000 {
        let eye = [
            rng(&mut s, -50.0, 50.0),
            rng(&mut s, -50.0, 50.0),
            rng(&mut s, -50.0, 50.0),
        ];
        // p1 hit ahead of the eye (den bounded away from 0); p2z spans a wide Δz both signs.
        let p1 = [
            eye[0] + rng(&mut s, 1.0, 40.0),
            eye[1] + rng(&mut s, -20.0, 20.0),
            rng(&mut s, -30.0, 30.0),
        ];
        cases.push(H {
            eye,
            heading: rng(&mut s, -6.5, 6.5),
            p1,
            p2z: rng(&mut s, -60.0, 60.0),
        });
    }

    let mut seg_hits = [0u32; 10];
    let mut sign_neg = 0u32;
    let mut sign_pos = 0u32;
    let mut chi = 0u32;
    let mut clo = 0u32;
    for (i, h) in cases.iter().enumerate() {
        emu.write_u32(ctr, 0); // probe-1 = points[0], probe-2 = points[1]
        for k in 0..3 {
            emu.write_u32(points + 4 * k as u64, h.p1[k].to_bits());
            emu.write_u32(eye_payload + 4 * k as u64, h.eye[k].to_bits());
        }
        // points[1] = probe-2 hit; only its z reaches the slope, but supply a full point.
        emu.write_u32(points + 12, (h.eye[0] + 2.0).to_bits());
        emu.write_u32(points + 16, (h.eye[1] + 2.0).to_bits());
        emu.write_u32(points + 20, h.p2z.to_bits());
        emu.write_u32(head_cell, h.heading.to_bits());
        emu.write_u32(this + 0x188, 0);
        emu.write_u32(this + 0xa0, 0x7f7f_ffff);
        emu.call(0x50d900, this as u32, 0, &[obj as u32, 1000]);
        let got = emu.read_f32(this + 0xa0);
        let mine = probe_slope_hit(h.eye, h.heading, h.p1, h.p2z);
        eqf(
            got.to_bits(),
            mine,
            &format!(
                "50d900-hit case {i} eye={:?} h={} p1={:?} p2z={}",
                h.eye, h.heading, h.p1, h.p2z
            ),
        );
        let slope = slope_of(h.eye, h.heading, h.p1, h.p2z);
        if slope.is_finite() {
            seg_hits[seg_of(slope)] += 1;
            if 0.0 <= slope {
                sign_neg += 1;
            } else {
                sign_pos += 1;
            }
        }
        if mine == f32::from_bits(0x3eb2_b8c2) {
            chi += 1;
        }
        if mine == f32::from_bits(0xbeb2_b8c2) {
            clo += 1;
        }
    }
    eprintln!(
        "cam_50d900-hit: {} bit-exact (segments {:?}; sign -/+={sign_neg}/{sign_pos}; \
         clamp +20°={chi} -20°={clo})",
        cases.len(),
        seg_hits
    );
    for (i, h) in seg_hits.iter().enumerate().skip(1) {
        assert!(*h > 0, "50d900-hit table segment {i} never hit");
    }
    assert!(
        sign_neg > 0 && sign_pos > 0 && chi > 0 && clo > 0,
        "50d900-hit sign/clamp coverage gap (-/+={sign_neg}/{sign_pos} +20={chi} -20={clo})"
    );
}
