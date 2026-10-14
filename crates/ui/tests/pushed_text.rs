//! The button scripts that convert a text offset or extent (`SetPushedTextOffset`,
//! `GetPushedTextOffset`, `GetTextWidth`, `GetTextHeight`) against the exe: each converts through
//! the layout scale with no widget scale, run as the slice of instructions that computes one length,
//! including the leaf helpers `0x41ad70`, `0x41ae40` and `0x41ae60`.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::ui_scale::{
    get_pushed_text_offset, get_text_height, get_text_width, logical_to_device, point_to_logical,
    set_pushed_text_offset,
};
use fpkit::{read_f64, rng};

struct Frame {
    this: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let this = emu.alloc(0x600);
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

/// GetPushedTextOffset 0x782180 / GetTextWidth 0x782290 / GetTextHeight 0x782390 —
///   point_to_logical of one device coord, pushed as f64. Each slice: call 0x41ad70; fmul [1024];
///   fmul [ebp-N]=coord; fstp dword [esp]; call 0x41ae40 (/0.8); sub esp,8; fstp qword [esp].
///   Result f64 read at ESP after the slice.
#[test]
fn luabind_782_getters() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (start, end, coord_off, label) — each a byte-identical point_to_logical block.
    //   GetPushedTextOffset x [ebp-8], y [ebp-4]; GetTextWidth/Height [ebp-4].
    let blocks: [(u64, u64, u64, &str); 4] = [
        (0x782237, 0x782254, 0x8, "GetPushedTextOffset.x"),
        (0x78225b, 0x782278, 0x4, "GetPushedTextOffset.y"),
        (0x782354, 0x782371, 0x4, "GetTextWidth"),
        (0x782454, 0x782471, 0x4, "GetTextHeight"),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, off: u64, coord: f32| -> f64 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - off, coord.to_bits());
        emu.run(start, end);
        read_f64(emu, emu.get_reg(RegisterX86::ESP))
    };

    let mut total = 0u32;
    for &(start, end, off, label) in &blocks {
        // deterministic: coord==0 (=> 0.0), 1024·coord==1.0 exactly (f32 tie at fstp dword [esp]),
        // exact powers + negatives + the literal text-extent zero a no-font-string yields.
        for coord in [
            0.0f32,
            1.0,
            -1.0,
            512.0,
            1.0 / 1024.0, // 1024·coord == 1.0 exactly
            -2048.5,
            255.0,
            1024.0,
        ] {
            let bin = run(&mut emu, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "{label} coord={coord}"
            );
            total += 1;
        }
        let mut s = 0x7820_0000u32 ^ (start as u32);
        for _ in 0..2000 {
            let coord = rng(&mut s, -4096.0, 4096.0);
            let bin = run(&mut emu, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "{label} fuzz coord={coord}"
            );
            total += 1;
        }
    }

    // get_pushed_text_offset composes the two coords against the x/y blocks; the width/height
    // single-coord rows compose trivially (checked above against point_to_logical).
    for (x, y) in [(0.0f32, 0.0f32), (10.0, -20.0), (512.0, 768.0)] {
        let bx = run(&mut emu, 0x782237, 0x782254, 0x8, x);
        let by = run(&mut emu, 0x78225b, 0x782278, 0x4, y);
        let (mx, my) = get_pushed_text_offset(x, y);
        assert_eq!(mx.to_bits(), bx.to_bits(), "get_pushed_text_offset.x");
        assert_eq!(my.to_bits(), by.to_bits(), "get_pushed_text_offset.y");
        // GetTextWidth/Height are the same point_to_logical of a single coord.
        let bw = run(&mut emu, 0x782354, 0x782371, 0x4, x);
        let bh = run(&mut emu, 0x782454, 0x782471, 0x4, y);
        assert_eq!(get_text_width(x).to_bits(), bw.to_bits(), "get_text_width");
        assert_eq!(
            get_text_height(y).to_bits(),
            bh.to_bits(),
            "get_text_height"
        );
        total += 4;
    }
    eprintln!(
        "luabind_782_getters: {total} bit-exact (PushedTextOffset/TextWidth/TextHeight point_to_logical)"
    );
}

/// SetPushedTextOffset 0x782030 — two logical_to_device pushes (x then y offset arg).
///   each block: call 0x41ad70; fmul [1024]; fdivr [ebp-4]=arg; fstp dword [esp]; call 0x41ae60
///   (·0.8); fstp dword [result]. x-result [ebp-0xc], y-result [ebp-8]. Read the f32.
#[test]
fn luabind_782_set_pushed_text_offset() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (start, end, result_off): arg always at [ebp-4]; the f32 result stored at [ebp-result_off].
    let blocks: [(u64, u64, u64, &str); 2] = [
        (0x7820eb, 0x782105, 0xc, "x"),
        (0x782114, 0x782131, 0x8, "y"),
    ];
    let run = |emu: &mut Emu, start: u64, end: u64, res_off: u64, arg: f32| -> f32 {
        set_frame(emu, &f);
        emu.write_u32(f.ebp - 0x4, arg.to_bits());
        emu.run(start, end);
        emu.read_f32(f.ebp - res_off)
    };

    let mut total = 0u32;
    for &(start, end, res_off, label) in &blocks {
        // deterministic: arg==0 (=> 0.0), 1024/1024==1.0 exactly, the exact 0.8/1.25 UI scales.
        for arg in [
            0.0f32,
            1.0,
            1024.0, // arg/1024 == 1.0 exactly
            2048.0,
            -512.0,
            0.8,
            1.0 / 0.8,
            -2048.5,
        ] {
            let bin = run(&mut emu, start, end, res_off, arg);
            assert_eq!(
                logical_to_device(arg).to_bits(),
                bin.to_bits(),
                "SetPushedTextOffset {label} arg={arg}"
            );
            total += 1;
        }
        let mut s = 0x7820_3000u32 ^ (start as u32);
        for _ in 0..2500 {
            let arg = rng(&mut s, -4096.0, 4096.0);
            let bin = run(&mut emu, start, end, res_off, arg);
            assert_eq!(
                logical_to_device(arg).to_bits(),
                bin.to_bits(),
                "SetPushedTextOffset {label} fuzz arg={arg}"
            );
            total += 1;
        }
    }

    // set_pushed_text_offset composes the two args — check the pair against the x/y blocks.
    for (x, y) in [(0.0f32, 0.0f32), (10.0, -20.0), (512.0, 768.0)] {
        let bx = run(&mut emu, 0x7820eb, 0x782105, 0xc, x);
        let by = run(&mut emu, 0x782114, 0x782131, 0x8, y);
        let (mx, my) = set_pushed_text_offset(x, y);
        assert_eq!(mx.to_bits(), bx.to_bits(), "set_pushed_text_offset.x");
        assert_eq!(my.to_bits(), by.to_bits(), "set_pushed_text_offset.y");
        total += 2;
    }
    eprintln!(
        "luabind_782_set_pushed_text_offset: {total} bit-exact (2 args logical_to_device, scale-free)"
    );
}
