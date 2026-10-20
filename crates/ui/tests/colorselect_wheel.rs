//! The thumb and the drag of `ColorSelect` run from the exe: `0x78bc20` (`systems/ui/colorselect.md#the-thumbs`)
//! and `0x78bd80` (`#dragging`).
//!
//! `fsincos`, `fpatan` and `fsqrt` route through the host's maths library in the emulator and in
//! Rust alike, so they compare bit for bit. The thumb runs with the frame move `0x767c70` stubbed
//! to log the offset it is given and the width getter replaced by a `fld` island; the drag runs with
//! the rectangle getter `0x768320` stubbed to copy one chosen rectangle for both frames, and the
//! calls after the stores (`0x78bae0`, `0x76b320`) stubbed. Each branch is tallied and required to
//! be hit; two NaNs compare equal. Every draw comes off the high bits of an LCG.
//!
//! Host double (`docs/METHOD.md#the-harness`): `geo_78bc20` runs `fsincos` at `0x78bc77`.
//! `geo_78bd80` runs `fpatan` at `0x78be40`. The emulator computes each in host `double`, so where
//! its result reaches a compared value the host's math stands in for the x87's, and the test shows
//! the code around it.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::colorselect_wheel::{drag_colour as cart2polar, thumb_offset as polar2cart};

const FUN_78BC20: u64 = 0x78bc20;

const FUN_78BD80: u64 = 0x78bd80;

const FUN_767C70: u64 = 0x767c70; // polar→cart emit (apply, boundary)

const FUN_768320: u64 = 0x768320; // rect write-through getter (boundary)

const FUN_78BAE0: u64 = 0x78bae0; // cart→polar trailing apply (boundary)

const FUN_76B320: u64 = 0x76b320; // cart→polar trailing notify (boundary)

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// f32 in `[-range, range)` from the LCG high bits.
fn ff(s: &mut u32, range: f32) -> f32 {
    let m = (next(s) >> 8) as f32 / (1u32 << 24) as f32;
    m * 2.0 * range - range
}

/// f32 in `[lo, hi)` from the LCG high bits.
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

/// Compare two `f32`s by bits; both-NaN passes.
fn eqf(mine: f32, bin: f32, ctx: &str) {
    if mine.is_nan() && bin.is_nan() {
        return;
    }
    assert_eq!(
        mine.to_bits(),
        bin.to_bits(),
        "{ctx}: rust {:#010x} ({mine}) vs binary {:#010x} ({bin})",
        mine.to_bits(),
        bin.to_bits()
    );
}

/// `fld dword [cell]; ret` island — a per-case `st0` oracle for a niladic thiscall float getter.
fn fld_stub(emu: &mut Emu, cell: u64) -> u64 {
    let addr = emu.alloc(8);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(addr, &code);
    addr
}

#[test]
fn geo_78bc20() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    // `this` state struct; the frame object + its vtable + the radius getter island.
    let this = emu.alloc(0x400);
    let obj = emu.alloc(0x40);
    let vt = emu.alloc(0x40);
    let cb_cell = emu.alloc(4);
    let getter = fld_stub(&mut emu, cb_cell);
    emu.write_u32(vt + 0x1c, getter as u32);
    emu.write_u32(obj + 0x24, vt as u32);
    emu.write_u32(this + 0x318, obj as u32); // knob frame (non-null guard)
    emu.write_u32(this + 0x31c, obj as u32); // second frame (non-null guard)
    emu.stub(FUN_767C70, 0, 6); // emit: ret 0x18; args[3] = -mx, args[4] = -my

    struct C {
        deg: f32,
        rf: f32,
        cb: f32,
    }
    let mut cases: Vec<C> = vec![
        C {
            deg: 0.0,
            rf: 1.0,
            cb: 100.0,
        },
        C {
            deg: 90.0,
            rf: 1.0,
            cb: 100.0,
        },
        C {
            deg: 45.0,
            rf: 0.5,
            cb: 64.0,
        },
        C {
            deg: 180.0,
            rf: 0.25,
            cb: 33.0,
        },
        C {
            deg: 270.0,
            rf: 1.0,
            cb: 50.0,
        },
        C {
            deg: 359.9,
            rf: 0.75,
            cb: 17.3,
        },
        C {
            deg: -120.0,
            rf: 0.9,
            cb: 12.0,
        },
    ];
    let mut s = 0x78bc_0020u32;
    for _ in 0..5000 {
        cases.push(C {
            deg: ff(&mut s, 720.0),
            rf: rng(&mut s, 0.0, 1.0),
            cb: rng(&mut s, 0.5, 400.0),
        });
    }

    // quadrant coverage of θ (sign of cos/sin).
    let mut quad = [0u32; 4];
    let mut checked = 0u32;
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(this + 0x328, c.deg.to_bits());
        emu.write_u32(this + 0x32c, c.rf.to_bits());
        emu.write_u32(cb_cell, c.cb.to_bits());
        emu.clear_calls();
        emu.call(FUN_78BC20, this as u32, 0, &[]);
        let call = emu
            .calls()
            .iter()
            .find(|cc| cc.addr == FUN_767C70)
            .copied()
            .unwrap_or_else(|| panic!("case {i}: emit FUN_00767c70 not called"));
        let bin = [f32::from_bits(call.args[3]), f32::from_bits(call.args[4])];
        let mine = polar2cart(c.deg, c.rf, c.cb);
        eqf(
            mine[0],
            bin[0],
            &format!("geo_78bc20 case {i} deg={} -mx", c.deg),
        );
        eqf(
            mine[1],
            bin[1],
            &format!("geo_78bc20 case {i} deg={} -my", c.deg),
        );

        let theta = (f64::from(c.deg) * f64::from(f32::from_bits(0x3c8e_fa35))) as f32;
        let (sin, cos) = f64::from(theta).sin_cos();
        let q = match (cos >= 0.0, sin >= 0.0) {
            (true, true) => 0,
            (false, true) => 1,
            (false, false) => 2,
            (true, false) => 3,
        };
        quad[q] += 1;
        checked += 1;
    }
    eprintln!("geo_78bc20: {checked} bit-exact; θ-quadrant coverage {quad:?}");
    for (q, h) in quad.iter().enumerate() {
        assert!(*h > 0, "geo_78bc20 θ-quadrant {q} never hit");
    }
}

#[test]
fn geo_78bd80() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let this = emu.alloc(0x400);
    // both frames non-null (their pointer value is otherwise irrelevant — FUN_00768320 is stubbed).
    emu.write_u32(this + 0x318, (this + 0x100) as u32);
    emu.write_u32(this + 0x320, (this + 0x180) as u32);
    // cursor struct (the [ebp+8] stack arg): px @ +0x24, py @ +0x28.
    let cur = emu.alloc(0x30);
    // synthesized rect [R0,R1,R2,R3] delivered to BOTH FUN_00768320 calls via write-through.
    let rectbuf = emu.alloc(0x10);
    emu.stub_write_through(FUN_768320, 0, rectbuf, 0x10, 1, 1);
    emu.stub(FUN_78BAE0, 0, 0); // trailing apply (thiscall, no stack args)
    emu.stub(FUN_76B320, 0, 1); // trailing notify (thiscall, one stack arg)

    // sentinels so an un-written output field is visibly stale (never silently a match).
    const SENT_A: u32 = 0x7f80_0000; // +Inf bits as a sentinel
    const SENT_R: u32 = 0xdead_beef;
    const SENT_T: u32 = 0xfeed_face;

    struct C {
        px: f32,
        py: f32,
        rect: [f32; 4],
        knob: bool,
        slider: bool,
    }
    let mut cases: Vec<C> = Vec::new();
    // knob-only probes: spread the cursor across the disc (radius<1, ==/>1) and the quadrants.
    let rect_c = [0.0, 0.0, 200.0, 200.0]; // R0=0,R1=0,R2=200,R3=200 -> center (100,100), half (100,100)
    for &(px, py) in &[
        (100.0, 100.0), // center -> mag 0
        (150.0, 100.0), // +x
        (100.0, 150.0), // +y
        (50.0, 50.0),   // -x,-y
        (180.0, 180.0), // mag > 1 (clamp radiusFrac)
        (200.0, 100.0), // mag exactly 1 on x
        (300.0, 300.0), // mag >> 1
    ] {
        cases.push(C {
            px,
            py,
            rect: rect_c,
            knob: true,
            slider: false,
        });
    }
    // slider-only probes: t below 0, inside, above 1, and the ÷0 (R2==R0) degenerate.
    for &(py, r0, r2) in &[
        (50.0f32, 0.0f32, 200.0f32), // t=0.25
        (-30.0, 0.0, 200.0),         // t<0 -> 0
        (260.0, 0.0, 200.0),         // t>1 -> 1
        (200.0, 0.0, 200.0),         // t==1 -> 1
        (0.0, 0.0, 200.0),           // t==0 -> 0 boundary
        (100.0, 50.0, 50.0),         // R2==R0 -> ÷0 (py>R0 -> +Inf -> 1.0)
        (40.0, 50.0, 50.0),          // ÷0, py<R0 -> -Inf -> 0.0
    ] {
        cases.push(C {
            px: 0.0,
            py,
            rect: [r0, 0.0, r2, 0.0],
            knob: false,
            slider: true,
        });
    }
    // both-mode probes.
    cases.push(C {
        px: 130.0,
        py: 70.0,
        rect: [10.0, 20.0, 190.0, 180.0],
        knob: true,
        slider: true,
    });
    cases.push(C {
        px: 60.0,
        py: 240.0,
        rect: [0.0, 0.0, 220.0, 220.0],
        knob: true,
        slider: true,
    });

    let mut s = 0x78bd_0080u32;
    for _ in 0..6000 {
        let r0 = ff(&mut s, 50.0);
        let r1 = ff(&mut s, 50.0);
        // keep spans usually non-degenerate, occasionally near-zero to probe big magnitudes.
        let r2 = r0 + rng(&mut s, 5.0, 400.0);
        let r3 = r1 + rng(&mut s, 5.0, 400.0);
        cases.push(C {
            px: rng(&mut s, -100.0, 500.0),
            py: rng(&mut s, -100.0, 500.0),
            rect: [r0, r1, r2, r3],
            knob: next(&mut s) & 0x100 != 0,
            slider: next(&mut s) & 0x200 != 0,
        });
    }

    // coverage: [knob, slider, mag<1, mag>=1, t<0, t in[0,1), t>=1]
    let mut tally = [0u32; 7];
    let mut checked = 0u32;
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(cur + 0x24, c.px.to_bits());
        emu.write_u32(cur + 0x28, c.py.to_bits());
        for (k, v) in c.rect.iter().enumerate() {
            emu.write_u32(rectbuf + 4 * k as u64, v.to_bits());
        }
        emu.write_u32(this + 0x314, c.knob as u32); // byte flag (low byte read)
        emu.write_u32(this + 0x315, c.slider as u32);
        emu.write_u32(this + 0x328, SENT_A);
        emu.write_u32(this + 0x32c, SENT_R);
        emu.write_u32(this + 0x330, SENT_T);
        emu.clear_calls();
        emu.call(FUN_78BD80, this as u32, 0, &[cur as u32]);

        let mine = cart2polar(c.px, c.py, c.rect);
        if c.knob {
            let ba = emu.read_f32(this + 0x328);
            let br = emu.read_f32(this + 0x32c);
            eqf(mine.angle_deg, ba, &format!("geo_78bd80 case {i} angle"));
            eqf(
                mine.radius_frac,
                br,
                &format!("geo_78bd80 case {i} radiusFrac"),
            );
            tally[0] += 1;
            if mine.radius_frac < 1.0 {
                tally[2] += 1;
            } else {
                tally[3] += 1;
            }
        } else {
            // un-written -> still the sentinel (proves the knob store is gated by the flag).
            assert_eq!(
                emu.read_u32(this + 0x328),
                SENT_A,
                "case {i}: angle written despite knob flag clear"
            );
        }
        if c.slider {
            let bt = emu.read_f32(this + 0x330);
            eqf(mine.slider_t, bt, &format!("geo_78bd80 case {i} slider_t"));
            tally[1] += 1;
            let tv = (f64::from(c.py) - f64::from(c.rect[0]))
                / (f64::from(c.rect[2]) - f64::from(c.rect[0]));
            if tv < 0.0 {
                tally[4] += 1;
            } else if tv < 1.0 {
                tally[5] += 1;
            } else {
                tally[6] += 1;
            }
        } else {
            assert_eq!(
                emu.read_u32(this + 0x330),
                SENT_T,
                "case {i}: slider_t written despite slider flag clear"
            );
        }
        checked += 1;
    }
    eprintln!(
        "geo_78bd80: {checked} cases bit-exact; knob={} slider={} [mag<1={} mag>=1={}] \
         [t<0={} t∈[0,1)={} t>=1={}]",
        tally[0], tally[1], tally[2], tally[3], tally[4], tally[5], tally[6]
    );
    let labels = [
        "knob",
        "slider",
        "mag<1",
        "mag>=1",
        "t<0",
        "t in [0,1)",
        "t>=1",
    ];
    for (k, h) in tally.iter().enumerate() {
        assert!(
            *h > 0,
            "geo_78bd80 coverage bucket '{}' never hit",
            labels[k]
        );
    }
}
