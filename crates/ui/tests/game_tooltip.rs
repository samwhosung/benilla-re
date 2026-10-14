//! The `GameTooltip` scripts that carry colour or length arithmetic (`SetPadding`, `SetOwner`,
//! `AddLine`, `AddDoubleLine`, `SetText`) against the exe, each run as the slice of instructions
//! that computes the value.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{pack_color, pack_gradient_color};
use benilla_re_ui::tooltip::{add_double_line, add_line, set_text};
use benilla_re_ui::ui_scale::{logical_to_device, set_owner, set_padding, UI_SCALE};
use fpkit::{next, rng};

/// A synthesized stack frame: `(ebp, esp)` with room below `ebp` for locals and above for the
/// `push`-reserved result slots.
fn frame(emu: &mut Emu) -> (u64, u64) {
    let base = emu.alloc(0x200);
    let ebp = base + 0x100;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    (ebp, esp)
}

/// Patch `0x6f3620` (the lua arg-N fetcher, returns the arg as a LIVE f64 in st0) to
/// `fld qword [edx*8 + TABLE]; ret` so the per-arg-index f64 we wrote into `TABLE` lands LIVE in st0.
/// Returns the TABLE base; index it by the lua arg index (2..) `* 8`. (The `luabind_79a` idiom.)
fn patch_arg_fetch(emu: &mut Emu) -> u64 {
    let table = emu.alloc(0x80); // indexed by edx (2..=9) * 8
    let mut code = vec![0xddu8, 0x04, 0xd5]; // fld qword [edx*8 + disp32]
    code.extend_from_slice(&(table as u32).to_le_bytes());
    code.push(0xc3); // ret
    emu.write(0x6f3620, &code);
    table
}

/// Setters: SetPadding 0x530f10 / SetOwner 0x5310d0 — logical_to_device(arg_f32).
///   SetPadding slice [0x530fa8, 0x530fc3): arg f32 at [ebp-4]; result fstp [esp] (f32) — read ESP.
///   SetOwner X slice [0x531367, 0x531381): arg [ebp-0xc]; result fstp [ebp-0xc].
///   SetOwner Y slice [0x5313a0, 0x5313ba): arg [ebp-8];  result fstp [ebp-8].
///   Each slice is `call 0x41ad70; fmul[1024]; fdivr[arg]; fstp[esp]; call 0x41ae60; fstp result`.
#[test]
fn luabind_530_setters() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);

    // SetPadding: arg at [ebp-4], result fstp [esp] -> read at the post-push ESP.
    let run_padding = |emu: &mut Emu, arg: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write_u32(ebp - 0x4, arg.to_bits());
        emu.run(0x530fa8, 0x530fc3);
        emu.read_f32(emu.get_reg(RegisterX86::ESP))
    };
    // SetOwner block: arg at [ebp-slot], result fstp [ebp-slot].
    let run_owner = |emu: &mut Emu, start: u64, end: u64, slot: u64, arg: f32| -> f32 {
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write_u32(ebp - slot, arg.to_bits());
        emu.run(start, end);
        emu.read_f32(ebp - slot)
    };

    // deterministic: 0, exact 1024 (1024/1024 == 1.0), 2048, negatives, exact 0.8 / 1/0.8.
    for arg in [
        0.0f32,
        1.0,
        1024.0,
        2048.0,
        -512.0,
        UI_SCALE,
        1.0 / UI_SCALE,
        1.0 / 1024.0, // 1024·result == 0.8 path; tiny input
    ] {
        let bp = run_padding(&mut emu, arg);
        let bx = run_owner(&mut emu, 0x531367, 0x531381, 0xc, arg);
        let by = run_owner(&mut emu, 0x5313a0, 0x5313ba, 0x8, arg);
        assert_eq!(
            set_padding(arg).to_bits(),
            bp.to_bits(),
            "SetPadding arg={arg}"
        );
        assert_eq!(
            set_owner(arg, arg).0.to_bits(),
            bx.to_bits(),
            "SetOwner.x arg={arg}"
        );
        assert_eq!(
            set_owner(arg, arg).1.to_bits(),
            by.to_bits(),
            "SetOwner.y arg={arg}"
        );
        // independent: the shared kernel equals each named row's binary slice.
        assert_eq!(
            logical_to_device(arg).to_bits(),
            bp.to_bits(),
            "kernel arg={arg}"
        );
    }

    let mut s = 0x530f_1000u32;
    let mut total = 8u32;
    for _ in 0..3000 {
        let arg = rng(&mut s, -4096.0, 4096.0);
        let bp = run_padding(&mut emu, arg);
        let bx = run_owner(&mut emu, 0x531367, 0x531381, 0xc, arg);
        let by = run_owner(&mut emu, 0x5313a0, 0x5313ba, 0x8, arg);
        assert_eq!(
            set_padding(arg).to_bits(),
            bp.to_bits(),
            "SetPadding fuzz arg={arg}"
        );
        assert_eq!(
            set_owner(arg, arg).0.to_bits(),
            bx.to_bits(),
            "SetOwner.x fuzz"
        );
        assert_eq!(
            set_owner(arg, arg).1.to_bits(),
            by.to_bits(),
            "SetOwner.y fuzz"
        );
        total += 1;
    }
    eprintln!(
        "luabind_530_setters: {total} bit-exact (SetPadding + SetOwner.x/.y logical_to_device)"
    );
}

/// AddLine 0x531630 — ONE colour pack (R=arg3, G=arg4 f32-clamp; B=arg5 live-f64), alpha forced
///   0xff. slice [0x531709, 0x531811): `mov edx,3` .. `mov [ebp-0x10],edi`; edi = 0xFFRRGGBB.
#[test]
fn luabind_530_add_line() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let table = patch_arg_fetch(&mut emu);

    let run = |emu: &mut Emu, r: f64, g: f64, b: f64| -> u32 {
        for (idx, v) in [(3u64, r), (4, g), (5, b)] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.run(0x531709, 0x531811);
        emu.read_u32(ebp - 0x10) // 0xFFRRGGBB
    };

    // div = 0.50392158856712466: f32-keep (R/G) -> 128, un-narrowed f64-keep (B, 3rd channel) -> 129.
    let div = f64::from_bits(0x3fe0_2020_2ad4_9d06);
    let (mut lo, mut hi, mut mid) = (0u32, 0u32, 0u32);
    for (r, g, b) in [
        (0.0f64, 1.0, 0.5),
        (-0.1, 1.1, 0.0039215),
        (f64::NAN, 0.5, 1.0),
        (1.0, 1.0, 1.0),
        (0.5, 0.25, 0.125),
        (0.5 / 255.0, 1.5 / 255.0, 2.5 / 255.0), // *.5 quantize ties
        (-0.0, 1.0, f64::NAN),                   // B NaN -> 1.0 (live path); -0.0 not<0 -> kept
        (div, div, div),                         // f32-keep (R/G)=128 vs f64-keep (B)=129
    ] {
        for v in [r, g, b] {
            if v < 0.0 {
                lo += 1;
            } else if v >= 1.0 {
                hi += 1;
            } else {
                mid += 1;
            }
        }
        let bin = run(&mut emu, r, g, b);
        assert_eq!(
            add_line(r, g, b),
            bin,
            "AddLine det r={r} g={g} b={b}: bin={bin:#010x} mine={:#010x}",
            add_line(r, g, b)
        );
        // independent: the shared kernel equals the binary slice.
        assert_eq!(pack_gradient_color(r, g, b), bin);
    }

    let mut s = 0x5316_3000u32;
    for _ in 0..5000 {
        let mk = |s: &mut u32| -> f64 {
            match (next(s) >> 8) % 3 {
                0 => f64::from(rng(s, -0.5, 0.0)),
                1 => f64::from(rng(s, 1.0, 1.5)),
                _ => f64::from(rng(s, 0.0, 0.9999)),
            }
        };
        let (r, g, b) = (mk(&mut s), mk(&mut s), mk(&mut s));
        for v in [r, g, b] {
            if v < 0.0 {
                lo += 1;
            } else if v >= 1.0 {
                hi += 1;
            } else {
                mid += 1;
            }
        }
        let bin = run(&mut emu, r, g, b);
        assert_eq!(add_line(r, g, b), bin, "AddLine fuzz r={r} g={g} b={b}");
    }
    assert!(
        lo > 0 && hi > 0 && mid > 0,
        "AddLine: need all clamp dispositions"
    );
    eprintln!(
        "luabind_530_add_line: 5008 bit-exact (R/G f32-clamp, B live-f64, A=0xff; lo={lo} hi={hi} mid={mid})"
    );
}

/// AddDoubleLine 0x531840 — TWO colour packs (each R/G f32-clamp, B live-f64), alpha forced 0xff.
///   colour0 args 4,5,6 -> [ebp-0x14]; colour1 args 7,8,9 -> [ebp-0x10]. slice [0x531927, 0x531b60):
///   `mov edx,4` (colour0 guard+pack) .. `mov [ebp-0x10],edi` (colour1 store). 0x6f34d0 -> present.
#[test]
fn luabind_530_add_double_line() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let table = patch_arg_fetch(&mut emu);
    emu.stub(0x6f34d0, 1, 0); // both colours present

    let run = |emu: &mut Emu, c0: [f64; 3], c1: [f64; 3]| -> (u32, u32) {
        for (idx, v) in [
            (4u64, c0[0]),
            (5, c0[1]),
            (6, c0[2]),
            (7, c1[0]),
            (8, c1[1]),
            (9, c1[2]),
        ] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EDI, 0);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x531927, 0x531b60);
        (emu.read_u32(ebp - 0x14), emu.read_u32(ebp - 0x10))
    };

    let div = f64::from_bits(0x3fe0_2020_2ad4_9d06);
    for (c0, c1, _l) in [
        ([0.0f64, 1.0, 0.5], [0.5, 0.25, 0.125], "edges/fracs"),
        (
            [-0.1, 1.1, 0.0039215],
            [1.0, 1.0, 1.0],
            "out-of-range / all 1",
        ),
        ([f64::NAN, 0.5, 1.0], [0.0, f64::NAN, 0.999], "nan"),
        (
            [0.5 / 255.0, 1.5 / 255.0, 2.5 / 255.0],
            [3.5 / 255.0, 0.5, 0.5],
            "half-ties",
        ),
        (
            [div, div, div],
            [div, div, div],
            "f32-vs-f64 keep divergence (B is f64)",
        ),
    ] {
        let (b0, b1) = run(&mut emu, c0, c1);
        let (m0, m1) = add_double_line(c0[0], c0[1], c0[2], c1[0], c1[1], c1[2]);
        assert_eq!(
            b0, m0,
            "AddDoubleLine c0 {c0:?}: bin={b0:#010x} mine={m0:#010x}"
        );
        assert_eq!(
            b1, m1,
            "AddDoubleLine c1 {c1:?}: bin={b1:#010x} mine={m1:#010x}"
        );
    }

    let mut s = 0x5318_4000u32;
    for _ in 0..4000 {
        let mk = |s: &mut u32| -> f64 {
            match (next(s) >> 8) % 3 {
                0 => f64::from(rng(s, -0.5, 0.0)),
                1 => f64::from(rng(s, 1.0, 1.5)),
                _ => f64::from(rng(s, 0.0, 0.9999)),
            }
        };
        let c0 = [mk(&mut s), mk(&mut s), mk(&mut s)];
        let c1 = [mk(&mut s), mk(&mut s), mk(&mut s)];
        let (b0, b1) = run(&mut emu, c0, c1);
        let (m0, m1) = add_double_line(c0[0], c0[1], c0[2], c1[0], c1[1], c1[2]);
        assert_eq!(b0, m0, "AddDoubleLine c0 fuzz {c0:?}");
        assert_eq!(b1, m1, "AddDoubleLine c1 fuzz {c1:?}");
    }
    eprintln!(
        "luabind_530_add_double_line: 4005 bit-exact (two 0xFFRRGGBB, R/G f32-clamp, B live-f64)"
    );
}

/// SetText 0x531b90 — ONE 4-channel colour pack (R/G/B f32-clamp args 3,4,5; A live-f64 arg6,
///   default 1.0). slice [0x531c5e, 0x531dd3): `mov edx,3` .. `mov [ebp-0x14],ebx`; ebx = 0xAARRGGBB.
///   The assembly order is A,R,G,B but the value is the standard 0xAARRGGBB == pack_color.
///   0x6f34d0 (alpha-present) -> present(1).
#[test]
fn luabind_530_set_text() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    let table = patch_arg_fetch(&mut emu);
    emu.stub(0x6f34d0, 1, 0); // alpha present so arg6 is supplied

    let run = |emu: &mut Emu, r: f64, g: f64, b: f64, a: f64| -> u32 {
        for (idx, v) in [(3u64, r), (4, g), (5, b), (6, a)] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBX, 0);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x531c5e, 0x531dd3);
        emu.read_u32(ebp - 0x14) // 0xAARRGGBB
    };

    let div = f64::from_bits(0x3fe0_2020_2ad4_9d06); // f32-keep (R/G/B)=128 vs f64-keep (A)=129
    let (mut lo, mut hi, mut mid) = (0u32, 0u32, 0u32);
    for (r, g, b, a) in [
        (0.0f64, 1.0, 0.5, 1.0),
        (-0.1, 1.1, 0.0039215, 0.9),
        (f64::NAN, 0.5, 1.0, 0.0),
        (1.0, 1.0, 1.0, 1.0),
        (0.5, 0.25, 0.125, 0.75),
        (0.5 / 255.0, 1.5 / 255.0, 2.5 / 255.0, 3.5 / 255.0),
        (-0.0, 1.0, 0.0, f64::NAN), // alpha NaN -> 1.0 (live path)
        (div, div, div, div),       // R/G/B (f32) -> 128, A (f64) -> 129
    ] {
        for v in [r, g, b, a] {
            if v < 0.0 {
                lo += 1;
            } else if v >= 1.0 {
                hi += 1;
            } else {
                mid += 1;
            }
        }
        let bin = run(&mut emu, r, g, b, a);
        assert_eq!(
            set_text(r, g, b, a),
            bin,
            "SetText det r={r} g={g} b={b} a={a}: bin={bin:#010x} mine={:#010x}",
            set_text(r, g, b, a)
        );
        assert_eq!(pack_color(r, g, b, a), bin); // independent kernel
    }

    let mut s = 0x531b_9000u32;
    for _ in 0..5000 {
        let mk = |s: &mut u32| -> f64 {
            match (next(s) >> 8) % 3 {
                0 => f64::from(rng(s, -0.5, 0.0)),
                1 => f64::from(rng(s, 1.0, 1.5)),
                _ => f64::from(rng(s, 0.0, 0.9999)),
            }
        };
        let (r, g, b, a) = (mk(&mut s), mk(&mut s), mk(&mut s), mk(&mut s));
        for v in [r, g, b, a] {
            if v < 0.0 {
                lo += 1;
            } else if v >= 1.0 {
                hi += 1;
            } else {
                mid += 1;
            }
        }
        let bin = run(&mut emu, r, g, b, a);
        assert_eq!(
            set_text(r, g, b, a),
            bin,
            "SetText fuzz r={r} g={g} b={b} a={a}"
        );
    }
    assert!(
        lo > 0 && hi > 0 && mid > 0,
        "SetText: need all clamp dispositions"
    );
    eprintln!(
        "luabind_530_set_text: 5008 bit-exact (0xAARRGGBB, R/G/B f32-clamp, A live-f64; lo={lo} hi={hi} mid={mid})"
    );
}
