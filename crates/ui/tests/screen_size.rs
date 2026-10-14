//! The screen size scripts (`GetScreenWidth`, `GetScreenHeight`, `GetCursorPosition`) and the video
//! mode list's aspect filter against the exe: each is run as the slice of instructions that
//! computes its value, including the leaf helpers of the layout scale.
//!
//! Probes: the device scale 1.0 (the UI parent absent) and 0.8 where a stray narrowing or a
//! missing division shows, equal and zero coordinates, and for the filter the aspect ratios 4:3,
//! 16:9 and 5:4 and the pairs either side of 1.248.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::screen_size::{
    aspect_ratio, aspect_ratio_kept, get_screen_height, get_screen_width, screen_to_logical,
    UI_SCALE_Y,
};
use benilla_re_ui::ui_scale::{get_point, point_to_logical, UI_SCALE};
use fpkit::{next, read_f64, rng};

// the global widget pointer 0x494590 dereferences (null => device scale = 1.0 fallback 0x7ff9d8).
const GLOBAL_WIDGET_PTR: u64 = 0xb4b44c;

struct Frame {
    this: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let this = emu.alloc(0x300);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    Frame { this, ebp, esp }
}
fn set_frame(emu: &mut Emu, f: &Frame) {
    emu.set_reg(RegisterX86::ESI, f.this);
    emu.set_reg(RegisterX86::ECX, f.this);
    emu.set_reg(RegisterX86::EDI, f.this);
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESP, f.esp);
}
/// GetScreenWidth 0x48b480 / GetScreenHeight 0x48b4d0 — device->logical of the screen extent.
///   slice: push 1.0; call 0x41ae60/70 (scale_axis); fstp [ebp-4]; call 0x494590 (device scale S);
///   fdivr [ebp-4]; fstp [ebp-4]; call 0x41ad70; fmul [1024]; push ecx; fmul [ebp-4]; fstp [esp];
///   call 0x41ae40 (/0.8); sub esp,8; mov ecx,esi; fstp qword [esp]. Result read at final ESP (f64).
///
/// The device scale S comes from 0x494590: it reads the global widget [0xb4b44c]; if null it pushes
/// 1.0 (0x7ff9d8), else fld [widget+0x7c]. We diff BOTH paths: null (S=1.0) and a synth widget whose
/// [+0x7c] holds a chosen scale.
#[test]
fn luabind_48b_screen_getters() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    // a synth global widget for the [+0x7c] device-scale path.
    let widget = emu.alloc(0x100);

    // run a screen-getter slice; `s` is the device scale 0x494590 should report. `use_widget`
    // selects the [+0x7c] path (else the null/1.0 fallback).
    let run = |emu: &mut Emu, start: u64, end: u64, s: f32, use_widget: bool| -> f64 {
        set_frame(emu, &f);
        if use_widget {
            emu.write_u32(GLOBAL_WIDGET_PTR, widget as u32);
            emu.write_u32(widget + 0x7c, s.to_bits());
        } else {
            emu.write_u32(GLOBAL_WIDGET_PTR, 0); // null => 0x494590 pushes 1.0
        }
        emu.run(start, end);
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };

    struct Row {
        name: &'static str,
        start: u64,
        end: u64,
        ours: fn(f32) -> f64,
        axis: f64,
    }
    let rows = [
        Row {
            name: "GetScreenWidth",
            start: 0x48b485,
            end: 0x48b4be,
            ours: get_screen_width,
            axis: f64::from(UI_SCALE), // 0.8
        },
        Row {
            name: "GetScreenHeight",
            start: 0x48b4d5,
            end: 0x48b50e,
            ours: get_screen_height,
            axis: f64::from(UI_SCALE_Y), // 0.6
        },
    ];

    let mut total = 0u32;
    for row in &rows {
        // null path: S == 1.0 (fallback). deterministic probes.
        for s in [1.0f32, 0.8, 1.25, 2.0, 0.5] {
            // null fallback always reports 1.0 regardless of s, so only s==1.0 is meaningful there;
            // the chosen-s values are exercised via the widget path below.
            if s == 1.0 {
                let bin = run(&mut emu, row.start, row.end, 1.0, false);
                assert_eq!(
                    (row.ours)(1.0).to_bits(),
                    bin.to_bits(),
                    "{} null-path S=1.0",
                    row.name
                );
                // kernel cross-check with the explicit per-axis scale.
                assert_eq!(
                    screen_to_logical(row.axis, 1.0).to_bits(),
                    bin.to_bits(),
                    "{} kernel S=1.0",
                    row.name
                );
                total += 1;
            }
            // widget path: 0x494590 reports the chosen device scale s.
            let bin = run(&mut emu, row.start, row.end, s, true);
            assert_eq!(
                (row.ours)(s).to_bits(),
                bin.to_bits(),
                "{} widget S={s}",
                row.name
            );
            assert_eq!(
                screen_to_logical(row.axis, s).to_bits(),
                bin.to_bits(),
                "{} kernel S={s}",
                row.name
            );
            total += 1;
        }
        let mut st = 0x48b4_8000u32 ^ (row.start as u32);
        for _ in 0..3000 {
            let s = rng(&mut st, 0.1, 4.0);
            let bin = run(&mut emu, row.start, row.end, s, true);
            assert_eq!(
                (row.ours)(s).to_bits(),
                bin.to_bits(),
                "{} fuzz S={s}",
                row.name
            );
            total += 1;
        }
    }
    eprintln!(
        "luabind_48b_screen_getters: {total} bit-exact (Width 0.8 / Height 0.6, /S /0.8 f64)"
    );
}

/// GetCursorPosition 0x48b820 — two coords, each scale-free point_to_logical push.
///   X slice [0x48b855, 0x48b872): call 0x41ad70; fmul [1024]; push ecx; fmul [ebp-8]=x;
///     fstp [esp]; call 0x41ae40; sub esp,8; fstp qword [esp]. Read f64 at final ESP.
///   Y slice [0x48b879, 0x48b896): identical, coord = [ebp-4]=y.
#[test]
fn luabind_48b_cursor_position() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (start, end, coord_off): the two byte-identical point_to_logical blocks.
    let blocks: [(u64, u64, u64, &str); 2] = [
        (0x48b855, 0x48b872, 0x8, "cursor.x"),
        (0x48b879, 0x48b896, 0x4, "cursor.y"),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, off: u64, coord: f32| -> f64 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - off, coord.to_bits());
        emu.run(start, end);
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };

    let mut total = 0u32;
    for &(start, end, off, label) in &blocks {
        for coord in [0.0f32, 1.0, -1.0, 512.0, 1.0 / 1024.0, -2048.5, 800.0] {
            let bin = run(&mut emu, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "GetCursorPosition {label} coord={coord}"
            );
            total += 1;
        }
        let mut s = 0x48b8_2000u32 ^ (start as u32);
        for _ in 0..2000 {
            let coord = rng(&mut s, -4096.0, 4096.0);
            let bin = run(&mut emu, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "GetCursorPosition {label} fuzz coord={coord}"
            );
            total += 1;
        }
    }
    // get_point composes the two coords; check against the X/Y blocks.
    for (x, y) in [(0.0f32, 0.0f32), (640.0, 480.0), (-10.0, 20.0)] {
        let bx = run(&mut emu, 0x48b855, 0x48b872, 0x8, x);
        let by = run(&mut emu, 0x48b879, 0x48b896, 0x4, y);
        let (mx, my) = get_point(x, y);
        assert_eq!(mx.to_bits(), bx.to_bits(), "get_point.x");
        assert_eq!(my.to_bits(), by.to_bits(), "get_point.y");
        total += 2;
    }
    eprintln!(
        "luabind_48b_cursor_position: {total} bit-exact (2 coords point_to_logical, scale-free)"
    );
}

/// 0x48bc90 — video-mode aspect filter. slice [0x48bcfa, 0x48bd0b): fild [ebp-0x10]=w;
///   fidiv [ebp-0x8]=h; fcomp [0x804570]=1.248; fnstsw ax. Read AH (= FPU status hi byte): C0 (bit
///   0 of AH) == 1 iff ratio < 1.248; the binary's `test ah,1; jne <skip>` => mode KEPT iff C0==0.
///   We diff the keep-predicate (C0 bit) AND cross-check the ratio value at a slot store.
#[test]
fn luabind_48b_aspect_filter() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // run the fild;fidiv;fcomp;fnstsw slice; return the binary's "kept" boolean (C0 == 0).
    // NB: the slice's `mov [ebp-0x8],eax` (0x48bcfd) writes the height into [ebp-0x8] FROM eax —
    // so we put h in EAX (the real code has eax = the height field there), and fidiv reads it.
    // The fnstsw clobbers AX, so the height must be supplied via EAX, not a pre-write of [ebp-8].
    let run_kept = |emu: &mut Emu, w: i32, h: i32| -> bool {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0x10, w as u32);
        emu.set_reg(RegisterX86::EAX, h as u32 as u64);
        emu.run(0x48bcfa, 0x48bd0b);
        let ah = (emu.get_reg(RegisterX86::EAX) >> 8) & 0xff;
        (ah & 1) == 0 // C0 == 0 => ratio >= 1.248 => kept
    };

    // To independently diff the RATIO value bit-exact, run an island that stores st0 to a slot:
    // `fild [ebp-0x10]; fidiv [ebp-0x8]; fstp qword [esp]` assembled from the binary's own bytes
    // (0x48bcfa..0x48bd03) + an appended `fstp qword [esp]` capture; byte-verify the prefix.
    let ratio_bytes = emu.read(0x48bcfa, 9); // fild [ebp-0x10] (3) + fidiv [ebp-0x8] (3) + ...
                                             // 0x48bcfa: db 45 f0 (fild) ; 0x48bcfd: 89 45 f8 (mov [ebp-8],eax — NOT part of compute) ...
                                             // The mov at 0x48bcfd writes h into [ebp-8] from eax; in our island we pre-write [ebp-8], so we
                                             // assemble fild[ebp-0x10]; fidiv[ebp-0x8] directly (skip the mov) + fstp qword [esp].
    assert_eq!(
        &ratio_bytes[0..3],
        &[0xdb, 0x45, 0xf0],
        "fild [ebp-0x10] prefix"
    );
    // fidiv dword [ebp-0x8] = da 75 f8 (verify against the binary at 0x48bd00).
    let fidiv = emu.read(0x48bd00, 3);
    assert_eq!(&fidiv[..], &[0xda, 0x75, 0xf8], "fidiv [ebp-0x8] bytes");
    // island region (rwx); assemble fild; fidiv; fstp qword [esp]; hlt-equivalent via run end.
    let island = emu.alloc(0x40);
    let mut code = Vec::new();
    code.extend_from_slice(&[0xdb, 0x45, 0xf0]); // fild dword [ebp-0x10]
    code.extend_from_slice(&[0xda, 0x75, 0xf8]); // fidiv dword [ebp-0x8]
    code.extend_from_slice(&[0xdd, 0x1c, 0x24]); // fstp qword [esp]
    let isl_end = island + code.len() as u64;
    emu.write(island, &code);
    let run_ratio = |emu: &mut Emu, w: i32, h: i32| -> f64 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0x10, w as u32);
        emu.write_u32(f.ebp - 0x8, h as u32);
        emu.run(island, isl_end);
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };

    let mut total = 0u32;
    // deterministic probes incl. the exact 1.248 boundary and the common aspect ratios.
    // 1.248 = 1.2480000257492065 (f32). The exact boundary: pick w,h with w/h == that f64? The
    // comparison is ratio(f64) vs f64(1.248_f32); we probe near-boundary integer pairs.
    let probes: &[(i32, i32)] = &[
        (800, 600),   // 1.333.. (4:3)  >= 1.248 => kept
        (1024, 768),  // 4:3 kept
        (1280, 1024), // 1.25  >= 1.248 => kept (just above)
        (1280, 720),  // 16:9 kept
        (1024, 821),  // 1.2473.. < 1.248 => dropped
        (1024, 820),  // 1.2487.. >= 1.248 => kept
        (1, 1),       // 1.0 dropped
        (3, 2),       // 1.5 kept
        (5, 4),       // 1.25 kept
        (-800, 600),  // negative ratio dropped
        (640, 480),   // 4:3 kept
    ];
    for &(w, h) in probes {
        let bin_kept = run_kept(&mut emu, w, h);
        assert_eq!(
            aspect_ratio_kept(w, h),
            bin_kept,
            "aspect kept w={w} h={h} ratio={}",
            aspect_ratio(w, h)
        );
        let bin_ratio = run_ratio(&mut emu, w, h);
        assert_eq!(
            aspect_ratio(w, h).to_bits(),
            bin_ratio.to_bits(),
            "aspect ratio w={w} h={h}"
        );
        total += 1;
    }
    // fuzz integer w,h pairs spanning the threshold.
    let mut s = 0x48bc_9000u32;
    let (mut kept, mut dropped) = (0u32, 0u32);
    for _ in 0..4000 {
        let w = (256 + ((next(&mut s) >> 8) % 4096)) as i32;
        let h = (256 + ((next(&mut s) >> 8) % 4096)) as i32;
        let bin_kept = run_kept(&mut emu, w, h);
        assert_eq!(
            aspect_ratio_kept(w, h),
            bin_kept,
            "aspect kept fuzz w={w} h={h}"
        );
        let bin_ratio = run_ratio(&mut emu, w, h);
        assert_eq!(
            aspect_ratio(w, h).to_bits(),
            bin_ratio.to_bits(),
            "aspect ratio fuzz w={w} h={h}"
        );
        if bin_kept {
            kept += 1;
        } else {
            dropped += 1;
        }
        total += 1;
    }
    assert!(
        kept > 0 && dropped > 0,
        "aspect filter: need both dispositions (kept={kept} dropped={dropped})"
    );

    // The exact `== threshold` boundary.
    // The integer fild;fidiv ratio can never land EXACTLY on 1.2480000257492065, so the strict-vs-
    // inclusive (`<` vs `<=`) reading of the binary's `fcomp; test ah,1; jne` is settled by an
    // island that feeds st0 the threshold's EXACT f64 bits (and ±1 ulp) into the binary's OWN
    // `fcomp [0x804570]; fnstsw ax`. Bytes byte-verified against the binary at 0x48bd03/0x48bd09.
    let fcomp = emu.read(0x48bd03, 6); // fcomp dword [0x804570]
    assert_eq!(
        &fcomp[..],
        &[0xd8, 0x1d, 0x70, 0x45, 0x80, 0x00],
        "fcomp [0x804570] bytes"
    );
    let fnstsw = emu.read(0x48bd09, 2); // fnstsw ax
    assert_eq!(&fnstsw[..], &[0xdf, 0xe0], "fnstsw ax bytes");
    // island: fld qword [esp] (st0 = chosen ratio) ; fcomp dword [0x804570] ; fnstsw ax.
    let bisland = emu.alloc(0x40);
    let mut bcode = Vec::new();
    bcode.extend_from_slice(&[0xdd, 0x04, 0x24]); // fld qword [esp]
    bcode.extend_from_slice(&fcomp); // fcomp dword [0x804570] (binary's own bytes)
    bcode.extend_from_slice(&fnstsw); // fnstsw ax (binary's own bytes)
    let bisl_end = bisland + bcode.len() as u64;
    emu.write(bisland, &bcode);
    let kept_for = |emu: &mut Emu, ratio: f64| -> bool {
        set_frame(emu, &f);
        // place the chosen ratio as a qword at [esp] for the `fld qword [esp]`.
        emu.write(f.esp, &ratio.to_bits().to_le_bytes());
        emu.set_reg(RegisterX86::EAX, 0);
        emu.run(bisland, bisl_end);
        let ah = (emu.get_reg(RegisterX86::EAX) >> 8) & 0xff;
        (ah & 1) == 0 // C0==0 => ratio >= threshold => kept
    };
    let thr = f64::from(f32::from_bits(0x3f9f_be77)); // 1.2480000257492065
    let above = f64::from_bits(thr.to_bits() + 1); // thr + 1 ulp
    let below = f64::from_bits(thr.to_bits() - 1); // thr - 1 ulp
    for (ratio, label) in [(thr, "==thr"), (above, "thr+ulp"), (below, "thr-ulp")] {
        let bin = kept_for(&mut emu, ratio);
        // our predicate is `ratio >= thr`; feed it the same chosen ratio directly.
        let mine = ratio >= thr;
        assert_eq!(
            bin, mine,
            "boundary {label}: ratio={ratio} bin_kept={bin} mine={mine}"
        );
    }
    // confirm the exact-boundary disposition: at ratio == thr the binary KEEPS (inclusive `>=`).
    assert!(
        kept_for(&mut emu, thr),
        "ratio == 1.248 must be KEPT (>=, not strict >)"
    );
    assert!(
        !kept_for(&mut emu, below),
        "ratio == 1.248 - 1ulp must be DROPPED"
    );

    eprintln!(
        "luabind_48b_aspect_filter: {total} bit-exact + 3 ==thr boundary probes (fild;fidiv ratio + 1.248 keep; kept={kept} dropped={dropped})"
    );
}
