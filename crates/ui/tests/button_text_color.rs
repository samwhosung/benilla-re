//! The button text colour scripts (`SetTextColor`, `SetDisabledTextColor`, `SetHighlightTextColor`
//! and their getters) against the exe: each setter is run as the slice of instructions that packs the
//! colour, up to the call that stores it, and each getter as the slice that pushes the four
//! channels, with the Lua marshalling stubbed around it.

#![allow(clippy::float_cmp)]

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{normalize_channel, pack_color, unpack_color};

/// The colour field of each getter's button: `+0x394` text, `+0x48c` disabled, `+0x410` highlight.
mod get_field {
    pub const TEXT: u64 = 0x394;
    pub const DISABLED: u64 = 0x48c;
    pub const HIGHLIGHT: u64 = 0x410;
}
use fpkit::{next, rng};

fn dbits(c: &benilla_re_difftest::StubCall) -> f64 {
    f64::from_bits((u64::from(c.args[1]) << 32) | u64::from(c.args[0]))
}
fn pushes(emu: &Emu) -> Vec<f64> {
    emu.calls()
        .iter()
        .filter(|c| c.addr == 0x6f3810)
        .map(dbits)
        .collect()
}

/// A synthesized stack frame: `(ebp, esp)` with room below `ebp` for locals and above for pushes.
fn frame(emu: &mut Emu) -> (u64, u64) {
    let base = emu.alloc(0x200);
    let ebp = base + 0x100;
    let stack = emu.alloc(0x2000);
    let esp = stack + 0x1000;
    (ebp, esp)
}

// pack — Set*TextColor: 4 channels [0,1]-clamped -> u8 (c*255+0.5, __ftol) -> 0xAARRGGBB in ebx.
//   clamp reads LIVE 0x6f3620 -> patch it to `fld qword [edx*8 + TABLE]; ret`; 0x6f34d0 -> present.
//   The asymmetric narrowing (R/G/B keep f32, A keeps the LIVE f64) is inherited from pack_color.
// Slice starts at `mov edx,2` (the first fetch's arg index) so the patched 0x6f3620 reads TABLE[2].
// Slices: 0x780ee0 [0x780f6b,0x7810eb) · 0x781260 [0x7812eb,0x78146b) · 0x7815e0 [0x78166b,0x7817eb).
fn run_pack(name: &str, begin: u64, until: u64) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);

    // patch 0x6f3620 -> `fld qword [edx*8 + TABLE]; ret`  (dd 04 d5 <disp32> c3).
    let table = emu.alloc(0x40); // indexed by edx (2..=5) * 8
    let mut code = vec![0xddu8, 0x04, 0xd5];
    code.extend_from_slice(&(table as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(0x6f3620, &code);
    // 0x6f34d0 (is-arg-present, for the alpha arg5): stub -> present(1) so we always supply alpha.
    emu.stub(0x6f34d0, 1, 0);

    let run = |emu: &mut Emu, r: f64, g: f64, b: f64, a: f64| -> u32 {
        for (idx, v) in [(2u64, r), (3, g), (4, b), (5, a)] {
            emu.write(table + idx * 8, &v.to_bits().to_le_bytes());
        }
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EDI, ebp); // edi = self; the assembly writes ebx to [ebp-0x10]
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(begin, until); // halt at the C++ store; ebx is the assembled 0xAARRGGBB.
        emu.get_reg(RegisterX86::EBX) as u32
    };

    // deterministic: clamp edges (exactly 0.0/1.0), NaN, out-of-range, the *.5 quantize tie.
    for (r, g, b, a, _l) in [
        (0.0f64, 1.0, 0.5, 1.0, "edges"),
        (-0.1, 1.1, 0.0039215, 0.9, "out-of-range -> clamp"),
        (f64::NAN, 0.5, 1.0, 0.0, "nan -> 1.0"),
        (1.0, 1.0, 1.0, 1.0, "all 1"),
        (0.5, 0.25, 0.125, 0.75, "fracs"),
        (-0.0, 1.0, 0.0, 1.0, "neg-zero kept (quantizes 0)"),
        (
            0.5 / 255.0,
            1.5 / 255.0,
            2.5 / 255.0,
            3.5 / 255.0,
            "half-ties",
        ),
    ] {
        let bin = run(&mut emu, r, g, b, a);
        let mine = pack_color(r, g, b, a);
        assert_eq!(
            bin, mine,
            "{name} det r={r} g={g} b={b} a={a}: bin={bin:#010x} mine={mine:#010x}"
        );
    }

    let mut s = (begin as u32) ^ 0x780f_72a5;
    let mut clamp_lo = 0u32;
    let mut clamp_hi = 0u32;
    let mut in_range = 0u32;
    for _ in 0..6000 {
        // mix in-range [0,1] with out-of-range to exercise both clamp branches on every channel.
        let mk = |s: &mut u32| -> f64 {
            match (next(s) >> 8) % 3 {
                0 => f64::from(rng(s, -0.5, 0.0)), // below -> clamp 0
                1 => f64::from(rng(s, 1.0, 1.5)),  // >=1 -> clamp 1
                _ => f64::from(rng(s, 0.0, 0.9999)),
            }
        };
        let (r, g, b, a) = (mk(&mut s), mk(&mut s), mk(&mut s), mk(&mut s));
        for v in [r, g, b, a] {
            if v < 0.0 {
                clamp_lo += 1;
            } else if v >= 1.0 {
                clamp_hi += 1;
            } else {
                in_range += 1;
            }
        }
        let bin = run(&mut emu, r, g, b, a);
        let mine = pack_color(r, g, b, a);
        assert_eq!(
            bin, mine,
            "{name} r={r} g={g} b={b} a={a}: bin={bin:#010x} mine={mine:#010x}"
        );
    }
    eprintln!(
        "{name}: 6007 bit-exact (clamp_lo={clamp_lo} clamp_hi={clamp_hi} in_range={in_range})"
    );
    assert!(
        clamp_lo > 0 && clamp_hi > 0 && in_range > 0,
        "{name}: need all three clamp dispositions"
    );
}

#[test]
fn pack_settextcolor_780ee0() {
    run_pack("pack_settextcolor_780ee0", 0x780f6b, 0x7810eb);
}
#[test]
fn pack_setdisabledtextcolor_781260() {
    run_pack("pack_setdisabledtextcolor_781260", 0x7812eb, 0x78146b);
}
#[test]
fn pack_sethighlighttextcolor_7815e0() {
    run_pack("pack_sethighlighttextcolor_7815e0", 0x78166b, 0x7817eb);
}

// unpack — Get*TextColor: u32 0xAARRGGBB -> 4x (byte/255), pushed (R,G,B,A) as f64.
//   colour u32 read from [obj+OFF]; 0x6f3810 stubbed/captured; 0x81c6b4 written to INV255.
// Slices: 0x781100 [0x7811a8,0x78124e) +0x394 · 0x781480 [0x781528,0x7815ce) +0x48c ·
//         0x781800 [0x7818a8,0x78194e) +0x410.
fn run_unpack(name: &str, begin: u64, until: u64, field: u64) {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    let widget = emu.alloc(0x600); // room for the largest field offset (0x48c)
    let run = |emu: &mut Emu, color: u32| -> Vec<f64> {
        emu.write_u32(widget + field, color);
        emu.set_reg(RegisterX86::EAX, widget);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(begin, until);
        pushes(emu)
    };
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
    let mut s = (begin as u32) ^ 0x7811_a8be;
    for _ in 0..3000 {
        det.push(next(&mut s));
    }
    for color in det {
        let p = run(&mut emu, color);
        assert_eq!(p.len(), 4, "{name}: expected 4 pushes, got {}", p.len());
        let m = unpack_color(color); // (R,G,B,A), all f64.
        for (i, (g, e)) in p.iter().zip(m.iter()).enumerate() {
            assert_eq!(
                g.to_bits(),
                e.to_bits(),
                "{name} color={color:#010x} idx={i}"
            );
        }
    }
    eprintln!("{name}: 3008 bit-exact");
}

#[test]
fn unpack_gettextcolor_781100() {
    run_unpack(
        "unpack_gettextcolor_781100",
        0x7811a8,
        0x78124e,
        get_field::TEXT,
    );
}
#[test]
fn unpack_getdisabledtextcolor_781480() {
    run_unpack(
        "unpack_getdisabledtextcolor_781480",
        0x781528,
        0x7815ce,
        get_field::DISABLED,
    );
}
#[test]
fn unpack_gethighlighttextcolor_781800() {
    run_unpack(
        "unpack_gethighlighttextcolor_781800",
        0x7818a8,
        0x78194e,
        get_field::HIGHLIGHT,
    );
}

/// Exercise the per-channel normalize directly across the full byte domain via 0x781100's B (low)
/// channel (color = 0x000000BB -> push idx 2) — covers every fild/fmul leaf bit-exact.
#[test]
fn normalize_channel_full_domain_781100() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let (ebp, esp) = frame(&mut emu);
    emu.stub(0x6f3810, 0, 2);
    let widget = emu.alloc(0x600);
    let run = |emu: &mut Emu, byte: u8| -> f64 {
        emu.write_u32(widget + get_field::TEXT, u32::from(byte));
        emu.set_reg(RegisterX86::EAX, widget);
        emu.set_reg(RegisterX86::ESI, 0x1234);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, esp);
        emu.clear_calls();
        emu.run(0x7811a8, 0x78124e);
        pushes(emu)[2] // B channel = color & 0xff
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
    eprintln!("normalize_channel_full_domain_781100: 256 bit-exact");
}
