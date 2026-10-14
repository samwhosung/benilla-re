//! The rectangle clamps `0x509bf0` and `0x509e20` run from the exe.
//!
//! Both read the screen extents through `0x41ae60` (x) and `0x41ae70` (y); the test replaces
//! each with an island `fld dword [cell]; ret 4` so a case chooses the width and the height.
//! `0x509bf0` (`__fastcall`: `ecx` the out rectangle, `edx` the mode, four floats on the stack,
//! `ret 0x10`) returns the outcode in `eax` and, in mode 0, writes the moved rectangle through
//! `ecx`. `0x509e20` (`__thiscall`: `ecx` the rectangle, no stack argument) changes the rectangle
//! in place. Floats are read back and compared by their bits, the outcode as an integer. The cases
//! straddle every outcode bit and every move on both axes, in mode 0 and in other modes, and each
//! branch is tallied and required to be hit. Every draw comes off the high bits of an LCG.
#![allow(clippy::float_cmp)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::screen_rect::{
    clamp_rect_into_screen as clamp_rect, outcode_and_clamp_rect as clip_outcode,
};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// Overwrite a callee with `fld dword ptr [cell]; ret N` (a per-case `st0` f32 oracle; `ret_n` =
/// bytes the callee cleans, 0 ⇒ bare `ret`).
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

/// Assert a transcribed `f32` matches the binary's stored `f32` bit-for-bit.
fn assert_f32(mine: f32, bin: f32, ctx: &str) {
    assert_eq!(
        mine.to_bits(),
        bin.to_bits(),
        "{ctx}: rust {:#010x} ({mine}) vs binary {:#010x} ({bin})",
        mine.to_bits(),
        bin.to_bits()
    );
}

#[test]
fn geo_509bf0() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // the two viewport-extent fns (each pushes 1.0f, then `ret 4`) become per-case f32 oracles.
    let cex = emu.alloc(4); // X extent (0x41ae60)
    let cey = emu.alloc(4); // Y extent (0x41ae70)
    patch_fld(&mut emu, 0x41ae60, cex, 4);
    patch_fld(&mut emu, 0x41ae70, cey, 4);
    let out = emu.alloc(0x10);

    // rect = [r0, r1, r2, r3]; r0/r2 = Y pair, r1/r3 = X pair. With W=H=1.0:
    //   x_lo=0, x_hi=W=1.0, y_lo=0.01875, y_hi=H-0.0375=0.9625.
    struct C {
        rect: [f32; 4],
        mode: u32,
        w: f32,
        h: f32,
    }
    let mut cases: Vec<C> = vec![
        // fully inside, mode 0 -> outcode 0, verbatim copy.
        C {
            rect: [0.8, 0.1, 0.1, 0.9],
            mode: 0,
            w: 1.0,
            h: 1.0,
        },
        // bit2 (r0 > y_hi): Y-high shove.
        C {
            rect: [1.2, 0.1, 0.1, 0.9],
            mode: 0,
            w: 1.0,
            h: 1.0,
        },
        // bit3 (r2 < y_lo): Y-low shove.
        C {
            rect: [0.5, 0.1, -0.1, 0.9],
            mode: 0,
            w: 1.0,
            h: 1.0,
        },
        // bit0 (r1 < x_lo): X-low shove.
        C {
            rect: [0.5, -0.2, 0.1, 0.9],
            mode: 0,
            w: 1.0,
            h: 1.0,
        },
        // bit1 (r3 > x_hi): X-high shove.
        C {
            rect: [0.5, 0.1, 0.1, 1.5],
            mode: 0,
            w: 1.0,
            h: 1.0,
        },
        // combo bit0 | bit2 (both axes shoved).
        C {
            rect: [1.2, -0.2, 0.1, 0.9],
            mode: 0,
            w: 1.0,
            h: 1.0,
        },
        // combo bit1 | bit3.
        C {
            rect: [0.5, 0.1, -0.1, 1.5],
            mode: 0,
            w: 1.0,
            h: 1.0,
        },
        // mode != 0, inside -> outcode 0, no rect write.
        C {
            rect: [0.8, 0.1, 0.1, 0.9],
            mode: 1,
            w: 1.0,
            h: 1.0,
        },
        // mode != 0, several bits -> outcode only.
        C {
            rect: [1.2, -0.2, -0.1, 1.5],
            mode: 2,
            w: 1.0,
            h: 1.0,
        },
    ];
    let mut s = 0x509b_00f0u32;
    for _ in 0..6000 {
        let coord = |s: &mut u32| (next(s) >> 8) as f32 / (1u32 << 24) as f32 * 1.7 - 0.3;
        let w = 0.6 + (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 0.6;
        let h = 0.6 + (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 0.6;
        cases.push(C {
            rect: [coord(&mut s), coord(&mut s), coord(&mut s), coord(&mut s)],
            mode: (next(&mut s) >> 8) % 4, // 0 ~25% of the time (the clamp+write path)
            w,
            h,
        });
    }

    // tally: [outcode bit0, bit1, bit2, bit3, inside(mode0,oc=0), mode!=0]
    let mut bits = [0u32; 4];
    let mut inside = 0u32;
    let mut mode_nz = 0u32;
    let mut checked = 0u32;
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(cex, c.w.to_bits());
        emu.write_u32(cey, c.h.to_bits());
        let args = [
            c.rect[0].to_bits(),
            c.rect[1].to_bits(),
            c.rect[2].to_bits(),
            c.rect[3].to_bits(),
        ];
        // poison the out-rect so a missing write (mode != 0) would be caught if we read it.
        for k in 0..4 {
            emu.write_u32(out + (k as u64) * 4, 0xdead_beef);
        }
        let bin_oc = emu.call(0x509bf0, out as u32, c.mode, &args);
        let (mine_oc, mine_rect) = clip_outcode(c.rect, c.mode, c.w, c.h);
        assert_eq!(
            bin_oc, mine_oc,
            "geo_509bf0 case {i}: outcode {bin_oc:#x} vs {mine_oc:#x} (rect {:?} mode {})",
            c.rect, c.mode
        );
        // rect is only written in mode 0.
        if c.mode == 0 {
            let mr = mine_rect.expect("mode 0 must yield a rect");
            for (k, &mk) in mr.iter().enumerate() {
                assert_f32(
                    mk,
                    emu.read_f32(out + (k as u64) * 4),
                    &format!("geo_509bf0 case {i} rect[{k}] (oc={bin_oc:#x})"),
                );
            }
            if bin_oc == 0 {
                inside += 1;
            }
        } else {
            assert!(mine_rect.is_none(), "mode != 0 must yield None");
            mode_nz += 1;
        }
        for (b, h) in bits.iter_mut().enumerate() {
            if bin_oc & (1 << b) != 0 {
                *h += 1;
            }
        }
        checked += 1;
    }
    eprintln!(
        "geo_509bf0: {checked} bit-exact; outcode bits [b0(x_lo)={} b1(x_hi)={} b2(y_hi)={} \
         b3(y_lo)={}] inside(mode0,oc=0)={inside} mode!=0={mode_nz}",
        bits[0], bits[1], bits[2], bits[3]
    );
    for (b, h) in bits.iter().enumerate() {
        assert!(*h > 0, "geo_509bf0 outcode bit {b} never set");
    }
    assert!(inside > 0, "geo_509bf0 inside (oc=0) never hit");
    assert!(mode_nz > 0, "geo_509bf0 mode != 0 never hit");
}

/// Independent (coverage-grader) replica of the four sequential shove conditions of `geo_509e20`,
/// returning which steps fired — kept separate from the lib so the tally is not the code under test.
fn fired_steps(rect: [f32; 4], cap_w: f32, cap_h: f32) -> [bool; 4] {
    let mut r = rect;
    let w = (f64::from(r[0]) - f64::from(r[2])) as f32;
    let h = (f64::from(r[3]) - f64::from(r[1])) as f32;
    let mut fired = [false; 4];
    if cap_h <= r[0] {
        fired[0] = true;
        r[0] = cap_h;
        r[2] = (f64::from(cap_h) - f64::from(w)) as f32;
    }
    if r[2] < 0.0 {
        fired[1] = true;
        r[2] = 0.0;
        r[0] = w;
    }
    if r[1] < 0.0 {
        fired[2] = true;
        r[1] = 0.0;
        r[3] = h;
    }
    if r[3] > cap_w {
        fired[3] = true;
    }
    fired
}

#[test]
fn geo_509e20() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cex = emu.alloc(4); // X extent (0x41ae60)
    let cey = emu.alloc(4); // Y extent (0x41ae70)
    patch_fld(&mut emu, 0x41ae60, cex, 4);
    patch_fld(&mut emu, 0x41ae70, cey, 4);
    let rect_va = emu.alloc(0x10);

    // rect = [r0, r1, r2, r3]; r0/r2 = Y pair -> [0,H], r1/r3 = X pair -> [0,W].
    struct C {
        rect: [f32; 4],
        w: f32,
        h: f32,
    }
    let mut cases: Vec<C> = vec![
        // fully inside -> unchanged.
        C {
            rect: [0.8, 0.1, 0.1, 0.9],
            w: 1.0,
            h: 1.0,
        },
        // step1 only (Y-high): r0 >= H, and H - w >= 0 so step2 doesn't re-fire.
        C {
            rect: [1.2, 0.1, 0.3, 0.9],
            w: 1.0,
            h: 1.0,
        },
        // step1 + step2: r0 >= H shoves r2 negative, then r2<0 re-shove.
        C {
            rect: [1.2, 0.1, 0.1, 0.9],
            w: 1.0,
            h: 1.0,
        },
        // step2 only (Y-low): r2 < 0, r0 < H.
        C {
            rect: [0.5, 0.1, -0.1, 0.9],
            w: 1.0,
            h: 1.0,
        },
        // step3 only (X-low): r1 < 0 (sets r3 = h = 1.0, not > W).
        C {
            rect: [0.5, -0.1, 0.1, 0.9],
            w: 1.0,
            h: 1.0,
        },
        // step4 only (X-high): r3 > W.
        C {
            rect: [0.5, 0.1, 0.1, 1.5],
            w: 1.0,
            h: 1.0,
        },
    ];
    let mut s = 0x509e_0020u32;
    for _ in 0..6000 {
        let coord = |s: &mut u32| (next(s) >> 8) as f32 / (1u32 << 24) as f32 * 1.7 - 0.3;
        let w = 0.6 + (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 0.6;
        let h = 0.6 + (next(&mut s) >> 8) as f32 / (1u32 << 24) as f32 * 0.6;
        cases.push(C {
            rect: [coord(&mut s), coord(&mut s), coord(&mut s), coord(&mut s)],
            w,
            h,
        });
    }

    let mut steps = [0u32; 4]; // [Y-high, Y-low, X-low, X-high]
    let mut unchanged = 0u32;
    let mut checked = 0u32;
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(cex, c.w.to_bits());
        emu.write_u32(cey, c.h.to_bits());
        for k in 0..4 {
            emu.write_u32(rect_va + (k as u64) * 4, c.rect[k].to_bits());
        }
        emu.call(0x509e20, rect_va as u32, 0, &[]);
        let bin: [f32; 4] = core::array::from_fn(|k| emu.read_f32(rect_va + (k as u64) * 4));
        let mine = clamp_rect(c.rect, c.w, c.h);
        for k in 0..4 {
            assert_f32(
                mine[k],
                bin[k],
                &format!(
                    "geo_509e20 case {i} rect[{k}] (in {:?} W={} H={})",
                    c.rect, c.w, c.h
                ),
            );
        }
        let fired = fired_steps(c.rect, c.w, c.h);
        if fired.iter().any(|&f| f) {
            for (k, f) in fired.iter().enumerate() {
                if *f {
                    steps[k] += 1;
                }
            }
        } else {
            unchanged += 1;
        }
        checked += 1;
    }
    eprintln!(
        "geo_509e20: {checked} bit-exact; shoves [Y-high={} Y-low={} X-low={} X-high={}] \
         unchanged={unchanged}",
        steps[0], steps[1], steps[2], steps[3]
    );
    for (k, h) in steps.iter().enumerate() {
        assert!(*h > 0, "geo_509e20 shove step {k} never fired");
    }
    assert!(unchanged > 0, "geo_509e20 unchanged (no shove) never hit");
}
