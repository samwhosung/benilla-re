//! The backdrop scripts (`GetBackdrop`, `SetBackdrop`, `GetBackdropColor`, `SetBackdropColor`, and
//! the border colour pair) against the exe: each is run as the slice of instructions that converts
//! the edge sizes or packs or unpacks the colour, with the Lua marshalling stubbed around it.
//!
//! The sizes take no widget scale, as a length does not; the colour getters push all four channels
//! as f64 and the setters pack as `SetVertexColor` does.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{normalize_channel, pack_color, unpack_color};
use benilla_re_ui::ui_scale::{get_backdrop, logical_to_device, point_to_logical, set_backdrop};
use fpkit::{next, read_f64, rng};

fn dbits(c: &benilla_re_difftest::StubCall) -> f64 {
    f64::from_bits((u64::from(c.args[1]) << 32) | u64::from(c.args[0]))
}

/// A synthesized stack frame: `(this, ebp, esp, edi)`. `this` and `edi` both point at a 0x300
/// widget block (the binding uses esi/edi=this and edi as the widget pointer); ebp has room below
/// for locals and esp room above for the x87 pushes (`push ecx`/`sub esp,8`/`fstp qword [esp]`).
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

/// GetBackdrop 0x777370 — six scale-free device->logical pushes (edge insets + tile sizes).
///   each block: call 0x41ad70; fmul [1024]; fmul [ebp-N]=coord; fstp dword [esp];
///   call 0x41ae40 (/0.8); fstp qword [esp]. The coord slots: blocks 1,2,3 = [ebp-8],
///   block 4 = [ebp-4], block 5 = [ebp-0xc], block 6 = [ebp-0x10]. Result read at ESP (f64).
#[test]
fn luabind_777_get_backdrop() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (start, end, coord_off): the six byte-identical point_to_logical blocks, each diffed.
    let blocks: [(u64, u64, u64, &str); 6] = [
        (0x7774da, 0x7774f7, 0x8, "tile1"),
        (0x77751c, 0x777539, 0x8, "tile2"),
        (0x7775da, 0x7775f7, 0x8, "insetA"),
        (0x777616, 0x777633, 0x4, "insetB"),
        (0x777652, 0x77766f, 0xc, "insetC"),
        (0x77768e, 0x7776ab, 0x10, "insetD"),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, off: u64, coord: f32| -> f64 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - off, coord.to_bits());
        emu.run(start, end);
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };

    let mut total = 0u32;
    for &(start, end, off, label) in &blocks {
        // deterministic: coord==0 (=> 0.0), exact powers (1024*coord on an f32 boundary), negative.
        for coord in [
            0.0f32,
            1.0,
            -1.0,
            512.0,
            1.0 / 1024.0, // 1024*coord == 1.0 exactly
            -2048.5,
            255.0,
        ] {
            let bin = run(&mut emu, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "GetBackdrop {label} coord={coord}"
            );
            total += 1;
        }
        let mut s = 0x7773_7000u32 ^ (start as u32);
        for _ in 0..1500 {
            let coord = rng(&mut s, -4096.0, 4096.0);
            let bin = run(&mut emu, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "GetBackdrop {label} fuzz coord={coord}"
            );
            total += 1;
        }
    }

    // get_backdrop composes the six coords (in push order) — check the tuple against the blocks.
    let coords = [10.0f32, -20.0, 0.0, 512.0, 1.0, 768.0];
    let mine = get_backdrop(
        coords[0], coords[1], coords[2], coords[3], coords[4], coords[5],
    );
    for (i, &(start, end, off, _)) in blocks.iter().enumerate() {
        let bin = run(&mut emu, start, end, off, coords[i]);
        assert_eq!(mine[i].to_bits(), bin.to_bits(), "get_backdrop[{i}]");
        total += 1;
    }
    eprintln!(
        "luabind_777_get_backdrop: {total} bit-exact (6 coords point_to_logical, scale-free)"
    );
}

/// SetBackdrop 0x7776e0 — six scale-free logical->device pushes.
///   each block: call 0x41ad70; fmul [1024]; fdivr [ebp-N]=arg; fstp dword [esp];
///   call 0x41ae60 (*0.8); fstp dword [result]. Blocks 1,2 store to [edi+0x4c]/[edi+0x48];
///   blocks 3..6 store to [ebp-8]/[ebp-0xc]/[ebp-0x10]/[ebp-0x14] (== the arg slot). Read the f32.
#[test]
fn luabind_777_set_backdrop() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (start, end, arg_off, result kind): result_edi_off>=0 => read [edi+off]; else [ebp-arg_off].
    struct B {
        start: u64,
        end: u64,
        arg_off: u64,
        edi_off: Option<u64>,
        label: &'static str,
    }
    let blocks = [
        B {
            start: 0x777922,
            end: 0x77793c,
            arg_off: 0x14,
            edi_off: Some(0x4c),
            label: "tile1",
        },
        B {
            start: 0x77797b,
            end: 0x777995,
            arg_off: 0x14,
            edi_off: Some(0x48),
            label: "tile2",
        },
        B {
            start: 0x777a1b,
            end: 0x777a35,
            arg_off: 0x8,
            edi_off: None,
            label: "insetA",
        },
        B {
            start: 0x777a74,
            end: 0x777a8e,
            arg_off: 0xc,
            edi_off: None,
            label: "insetB",
        },
        B {
            start: 0x777acd,
            end: 0x777ae7,
            arg_off: 0x10,
            edi_off: None,
            label: "insetC",
        },
        B {
            start: 0x777b26,
            end: 0x777b40,
            arg_off: 0x14,
            edi_off: None,
            label: "insetD",
        },
    ];
    let run = |emu: &mut Emu, b: &B, arg: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - b.arg_off, arg.to_bits());
        emu.run(b.start, b.end);
        match b.edi_off {
            Some(o) => emu.read_f32(f.this + o),
            None => emu.read_f32(f.ebp - b.arg_off),
        }
    };

    let mut total = 0u32;
    for b in &blocks {
        for arg in [
            0.0f32,
            1.0,
            1024.0, // 1024/1024 == 1.0 exactly
            2048.0,
            -512.0,
            0.8,
            1.0 / 0.8,
        ] {
            let bin = run(&mut emu, b, arg);
            assert_eq!(
                logical_to_device(arg).to_bits(),
                bin.to_bits(),
                "SetBackdrop {} arg={arg}",
                b.label
            );
            total += 1;
        }
        let mut s = 0x7776_e000u32 ^ (b.start as u32);
        for _ in 0..1500 {
            let arg = rng(&mut s, -4096.0, 4096.0);
            let bin = run(&mut emu, b, arg);
            assert_eq!(
                logical_to_device(arg).to_bits(),
                bin.to_bits(),
                "SetBackdrop {} fuzz arg={arg}",
                b.label
            );
            total += 1;
        }
    }
    // set_backdrop composes the six args — check the tuple against the blocks.
    let args = [10.0f32, -20.0, 0.0, 512.0, 1.0, 768.0];
    let mine = set_backdrop(args[0], args[1], args[2], args[3], args[4], args[5]);
    for (i, b) in blocks.iter().enumerate() {
        let bin = run(&mut emu, b, args[i]);
        assert_eq!(mine[i].to_bits(), bin.to_bits(), "set_backdrop[{i}]");
        total += 1;
    }
    eprintln!("luabind_777_set_backdrop: {total} bit-exact (6 args logical_to_device, scale-free)");
}

/// Get{,Border}BackdropColor 0x777bc0 / 0x777f60 — unpack 0xAARRGGBB to 4 channels, ALL pushed
///   as f64 (symmetric). The packed dword is read from [eax+0x60] (color) / [eax+0x64] (border);
///   we point eax at the widget block and write the color. The 4 f64 pushes go through stubbed
///   0x6f3810 (captured). The two rows are diffed at their OWN bytes (clone byte-verified).
#[test]
fn luabind_777_get_backdrop_color() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    emu.stub(0x6f3810, 0, 0); // lua_pushnumber: capture, pop nothing (caller cleans the qword)

    fn pushes(emu: &Emu) -> Vec<f64> {
        emu.calls()
            .iter()
            .filter(|c| c.addr == 0x6f3810)
            .map(dbits)
            .collect()
    }

    // (start, end, field_off, label) for the two byte-identical unpack rows.
    // eax holds the struct pointer; [eax+field] holds the packed 0xAARRGGBB.
    let rows: [(u64, u64, u64, &str); 2] = [
        (0x777c78, 0x777d1b, 0x60, "GetBackdropColor"),
        (0x778018, 0x7780bb, 0x64, "GetBackdropBorderColor"),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, field: u64, color: u32| -> Vec<f64> {
        set_frame(emu, &f);
        // eax = the struct; the slice does `mov edi,[eax+field]` at its head.
        emu.set_reg(RegisterX86::EAX, f.this);
        emu.write_u32(f.this + field, color);
        emu.clear_calls();
        emu.run(start, end);
        pushes(emu)
    };

    let mut total = 0u32;
    for &(start, end, field, label) in &rows {
        let mut det: Vec<u32> = vec![
            0x0000_0000,
            0xffff_ffff,
            0xff00_0000,
            0x00ff_0000,
            0x0000_ff00,
            0x0000_00ff,
            0x8040_2010,
            0x0103_07ff,
        ];
        let mut s = 0x777b_c000u32 ^ (start as u32);
        for _ in 0..2000 {
            det.push(next(&mut s));
        }
        for color in det {
            let p = run(&mut emu, start, end, field, color);
            assert_eq!(p.len(), 4, "{label}: expected 4 pushes, got {}", p.len());
            let m = unpack_color(color);
            for (i, (g, e)) in p.iter().zip(m.iter()).enumerate() {
                assert_eq!(
                    g.to_bits(),
                    e.to_bits(),
                    "{label} color={color:#010x} idx={i}: bin={g} mine={e}"
                );
            }
            total += 1;
        }
    }
    eprintln!(
        "luabind_777_get_backdrop_color: {total} colors bit-exact (4 channels f64, both rows)"
    );
}

/// Exercise the per-channel f64-unpack normalize directly across the full byte domain.
#[test]
fn luabind_777_normalize_full_domain() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    emu.stub(0x6f3810, 0, 0);
    // every byte appears in the R channel (color>>16) of GetBackdropColor -> push idx 0 (f64).
    let run = |emu: &mut Emu, byte: u8| -> f64 {
        set_frame(emu, &f);
        emu.set_reg(RegisterX86::EAX, f.this);
        emu.write_u32(f.this + 0x60, u32::from(byte) << 16);
        emu.clear_calls();
        emu.run(0x777c78, 0x777d1b);
        emu.calls()
            .iter()
            .filter(|c| c.addr == 0x6f3810)
            .map(dbits)
            .next()
            .unwrap()
    };
    for b in 0u8..=255 {
        let bin = run(&mut emu, b);
        assert_eq!(
            bin.to_bits(),
            normalize_channel(b).to_bits(),
            "normalize byte={b} bin={bin} mine={}",
            normalize_channel(b)
        );
    }
    eprintln!("luabind_777_normalize_full_domain: 256 bit-exact (byte * 1/255 f64)");
}

/// Set{,Border}BackdropColor 0x777d30 / 0x7780d0 — colour pack (R/G/B f32-clamp, A live-f64),
///   assembled 0xAARRGGBB. arg2=R, arg3=G, arg4=B, arg5=A (defaults 1.0). The clamp reads the
///   LIVE 0x6f3620 value, so 0x6f3620 is patched to `fld qword [edx*8+TABLE]; ret`; per-arg-index
///   f64s are written, and the assembled u32 (in ebx, stored to [ebp-0x10]) is read. The two rows
///   are diffed at their own bytes (clone byte-verified). 0x6f34d0 (alpha-present) -> present(1).
#[test]
fn luabind_777_set_backdrop_color() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);
    // patch 0x6f3620 (arg-N fetcher, LIVE f64 in st0) to `fld qword [edx*8 + TABLE]; ret`.
    let table = emu.alloc(0x80);
    let mut code = vec![0xddu8, 0x04, 0xd5]; // fld qword [edx*8 + disp32]
    code.extend_from_slice(&(table as u32).to_le_bytes());
    code.push(0xc3); // ret
    emu.write(0x6f3620, &code);
    emu.stub(0x6f34d0, 1, 0); // alpha (arg5) present

    // (start, end, store_off, label) for the two byte-identical pack rows.
    //   colour assembled into ebx, stored to [ebp-0x10]; we read ebx directly.
    let rows: [(u64, u64, &str); 2] = [
        (0x777dbb, 0x777f35, "SetBackdropColor"),
        (0x77815b, 0x7782d5, "SetBackdropBorderColor"),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, r: f64, g: f64, b: f64, a: f64| -> u32 {
        for (idx, v) in [(2u64, r), (3, g), (4, b), (5, a)] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        set_frame(emu, &f);
        emu.set_reg(RegisterX86::EBX, 0);
        emu.set_reg(RegisterX86::EDX, 2);
        emu.run(start, end);
        emu.get_reg(RegisterX86::EBX) as u32
    };

    let mut total = 0u32;
    for &(start, end, label) in &rows {
        // f32-vs-f64 keep divergence: 0x3fe020202ad49d06 quantizes to 128 via the f32 keep
        // (R/G/B) but 129 via the un-narrowed f64 keep (alpha) — probes the alpha asymmetry.
        let div = f64::from_bits(0x3fe0_2020_2ad4_9d06); // 0.50392158856712466
        for (r, g, b, a) in [
            (0.0f64, 1.0, 0.5, 1.0),
            (-0.1, 1.1, 0.0039215, 0.9),
            (f64::NAN, 0.5, 1.0, 0.0),
            (1.0, 1.0, 1.0, 1.0),
            (0.5, 0.25, 0.125, 0.75),
            (0.5 / 255.0, 1.5 / 255.0, 2.5 / 255.0, 3.5 / 255.0),
            (-0.0, 1.0, 0.0, f64::NAN), // alpha NaN -> 1.0 (live path); -0.0 not < 0 -> kept
            (div, div, div, div),       // f32-keep (R/G/B) -> 128, f64-keep (A) -> 129
        ] {
            let bin = run(&mut emu, start, end, r, g, b, a);
            let mine = pack_color(r, g, b, a);
            assert_eq!(
                bin, mine,
                "{label} det r={r} g={g} b={b} a={a}: bin={bin:#010x} mine={mine:#010x}"
            );
            total += 1;
        }
        let mut s = 0x777d_3000u32 ^ (start as u32);
        let (mut lo, mut hi, mut mid) = (0u32, 0u32, 0u32);
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
            let bin = run(&mut emu, start, end, r, g, b, a);
            assert_eq!(
                bin,
                pack_color(r, g, b, a),
                "{label} fuzz r={r} g={g} b={b} a={a}"
            );
            total += 1;
        }
        assert!(
            lo > 0 && hi > 0 && mid > 0,
            "{label}: need all clamp dispositions (lo={lo} hi={hi} mid={mid})"
        );
        eprintln!("{label}: bit-exact (clamp_lo={lo} clamp_hi={hi} in_range={mid})");
    }
    eprintln!("luabind_777_set_backdrop_color: {total} bit-exact (RGB f32-clamp, A live-f64 pack)");
}
