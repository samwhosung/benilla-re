//! The hit rectangle and resize bound scripts of a frame (`GetHitRectInsets`, `SetHitRectInsets`,
//! `GetMinResize`, `SetMinResize`, `GetMaxResize`, `SetMaxResize`) against the exe: each converts
//! its lengths through the layout scale with no widget scale, run as the slice of instructions that
//! computes one length, including the leaf helpers `0x41ad70`, `0x41ae40` and `0x41ae60`.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::ui_scale::{
    get_hit_rect_insets, get_resize, logical_to_device, point_to_logical, set_hit_rect_insets,
    set_resize, UI_SCALE,
};
use fpkit::{read_f64, rng};

struct Frame {
    this: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let this = emu.alloc(0x300);
    let scratch = emu.alloc(0x80);
    let ebp = scratch + 0x40;
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00;
    Frame { this, ebp, esp }
}
fn set_frame(emu: &mut Emu, f: &Frame) {
    emu.set_reg(RegisterX86::ESI, f.this);
    emu.set_reg(RegisterX86::ECX, f.this);
    emu.set_reg(RegisterX86::EDI, f.this);
    emu.set_reg(RegisterX86::EBP, f.ebp);
    emu.set_reg(RegisterX86::ESP, f.esp);
}
// A getter coordinate block: call 0x41ad70; fmul[1024]; push ecx; fmul[ebp-off]; fstp[esp];
// call 0x41ae40; sub esp,8; fstp qword[esp]. We fill [ebp-off] with `coord`, run, read f64 at ESP.
fn run_getter(emu: &mut Emu, f: &Frame, start: u64, end: u64, off: u64, coord: f32) -> f64 {
    set_frame(emu, f);
    emu.write_u32(f.ebp - off, coord.to_bits());
    emu.run(start, end);
    let esp = emu.get_reg(RegisterX86::ESP);
    read_f64(emu, esp)
}

// A setter coordinate block: call 0x41ad70; fmul[1024]; push ecx; fdivr[ebp-4]; fstp[esp];
// call 0x41ae60; fstp[ebp-res]. We pre-fill [ebp-4] with the f32 arg (skipping lua_tonumber),
// run, and read the f32 the binary stored at [ebp-res].
fn run_setter(emu: &mut Emu, f: &Frame, start: u64, end: u64, res_off: u64, arg: f32) -> f32 {
    set_frame(emu, f);
    emu.write_u32(f.ebp - 0x4, arg.to_bits());
    emu.run(start, end);
    emu.read_f32(f.ebp - res_off)
}

// Deterministic boundary probes for a length-coordinate getter / setter (no scale operand).
const GETTER_PROBES: [f32; 7] = [
    0.0,          // 1024·0 == 0.0
    1.0,          //
    1024.0,       //
    -512.0,       //
    1.0 / 1024.0, // 1024·coord == 1.0 exactly
    UI_SCALE,     // exercises the /0.8 reload at the scale's own value
    -2048.5,      //
];
const SETTER_PROBES: [f32; 8] = [
    0.0,       // 0/1024 == 0.0
    1.0,       //
    1024.0,    // 1024/1024 == 1.0 exactly
    2048.0,    //
    -512.0,    //
    0.8,       //
    1.0 / 0.8, //
    -4096.0,   //
];

/// GetHitRectInsets 0x775bf0 — four insets, each point_to_logical. Slice per inset block:
///   [ebp-4]:  [0x775caa, 0x775cc7)   [ebp-8]:  [0x775cce, 0x775ceb)
///   [ebp-0xc]:[0x775cf2, 0x775d0f)   [ebp-0x10]:[0x775d16, 0x775d33)
#[test]
fn luabind_775_get_hit_rect_insets() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (start, end, ebp-offset) per inset block, in push order.
    let blocks: [(u64, u64, u64, &str); 4] = [
        (0x775caa, 0x775cc7, 0x4, "s0"),
        (0x775cce, 0x775ceb, 0x8, "s1"),
        (0x775cf2, 0x775d0f, 0xc, "s2"),
        (0x775d16, 0x775d33, 0x10, "s3"),
    ];

    let mut total = 0u32;
    for &(start, end, off, label) in &blocks {
        for &coord in &GETTER_PROBES {
            let bin = run_getter(&mut emu, &f, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "GetHitRectInsets {label} coord={coord}"
            );
            total += 1;
        }
        let mut s = 0x775b_f000u32 ^ (start as u32);
        for _ in 0..1200 {
            let coord = rng(&mut s, -4096.0, 4096.0);
            let bin = run_getter(&mut emu, &f, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "GetHitRectInsets {label} fuzz"
            );
            total += 1;
        }
    }
    // The composed 4-tuple against the four blocks (independent path coverage).
    for (a, b, c, d) in [(0.0f32, 1.0, -2.0, 3.5), (10.0, -20.0, 512.0, 768.0)] {
        let b0 = run_getter(&mut emu, &f, 0x775caa, 0x775cc7, 0x4, a);
        let b1 = run_getter(&mut emu, &f, 0x775cce, 0x775ceb, 0x8, b);
        let b2 = run_getter(&mut emu, &f, 0x775cf2, 0x775d0f, 0xc, c);
        let b3 = run_getter(&mut emu, &f, 0x775d16, 0x775d33, 0x10, d);
        let (m0, m1, m2, m3) = get_hit_rect_insets(a, b, c, d);
        assert_eq!(m0.to_bits(), b0.to_bits(), "GetHitRectInsets tuple.0");
        assert_eq!(m1.to_bits(), b1.to_bits(), "GetHitRectInsets tuple.1");
        assert_eq!(m2.to_bits(), b2.to_bits(), "GetHitRectInsets tuple.2");
        assert_eq!(m3.to_bits(), b3.to_bits(), "GetHitRectInsets tuple.3");
        total += 4;
    }
    eprintln!("luabind_775_get_hit_rect_insets: {total} bit-exact (4 insets point_to_logical)");
}

/// SetHitRectInsets 0x775d50 — four args, each logical_to_device, stored f32. Slice per block
/// (start = call 0x41ad70 after lua_tonumber's fstp [ebp-4]; end = after the result fstp):
///   arg0 -> [ebp-0x10]: [0x775e3b, 0x775e55)   arg1 -> [ebp-0xc]: [0x775e64, 0x775e7e)
///   arg2 -> [ebp-8]:    [0x775e8d, 0x775ea7)   arg3 -> [ebp-4]:   [0x775eb6, 0x775ed6)
#[test]
fn luabind_775_set_hit_rect_insets() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    // (start, end, result-offset) per arg block, in arg order.
    let blocks: [(u64, u64, u64, &str); 4] = [
        (0x775e3b, 0x775e55, 0x10, "a0"),
        (0x775e64, 0x775e7e, 0xc, "a1"),
        (0x775e8d, 0x775ea7, 0x8, "a2"),
        (0x775eb6, 0x775ed6, 0x4, "a3"),
    ];

    let mut total = 0u32;
    for &(start, end, res, label) in &blocks {
        for &arg in &SETTER_PROBES {
            let bin = run_setter(&mut emu, &f, start, end, res, arg);
            assert_eq!(
                logical_to_device(arg).to_bits(),
                bin.to_bits(),
                "SetHitRectInsets {label} arg={arg}"
            );
            total += 1;
        }
        let mut s = 0x775d_5000u32 ^ (start as u32);
        for _ in 0..1200 {
            let arg = rng(&mut s, -4096.0, 4096.0);
            let bin = run_setter(&mut emu, &f, start, end, res, arg);
            assert_eq!(
                logical_to_device(arg).to_bits(),
                bin.to_bits(),
                "SetHitRectInsets {label} fuzz"
            );
            total += 1;
        }
    }
    // The composed 4-tuple against the four blocks (note: block result offsets differ from arg
    // index, so check each result slot independently).
    for (a, b, c, d) in [(0.0f32, 1024.0, -512.0, 2048.0), (5.0, -10.0, 512.0, 768.0)] {
        let r0 = run_setter(&mut emu, &f, 0x775e3b, 0x775e55, 0x10, a);
        let r1 = run_setter(&mut emu, &f, 0x775e64, 0x775e7e, 0xc, b);
        let r2 = run_setter(&mut emu, &f, 0x775e8d, 0x775ea7, 0x8, c);
        let r3 = run_setter(&mut emu, &f, 0x775eb6, 0x775ed6, 0x4, d);
        let (m0, m1, m2, m3) = set_hit_rect_insets(a, b, c, d);
        assert_eq!(m0.to_bits(), r0.to_bits(), "SetHitRectInsets tuple.0");
        assert_eq!(m1.to_bits(), r1.to_bits(), "SetHitRectInsets tuple.1");
        assert_eq!(m2.to_bits(), r2.to_bits(), "SetHitRectInsets tuple.2");
        assert_eq!(m3.to_bits(), r3.to_bits(), "SetHitRectInsets tuple.3");
        total += 4;
    }
    eprintln!("luabind_775_set_hit_rect_insets: {total} bit-exact (4 args logical_to_device)");
}

/// GetMinResize 0x775f20 / GetMaxResize 0x7761a0 — two coords each, point_to_logical.
///   GetMinResize: [ebp-0xc] [0x775fec, 0x776009);  [ebp-0x10] [wait]
/// Slices (call 0x41ad70 .. after fstp qword[esp]):
///   Min coord0 [ebp-0xc]:  [0x775fc8, 0x775fe5)    Min coord1 [ebp-0x10]: [0x775fec, 0x776009)
///   Max coord0 [ebp-4]:    [0x776248, 0x776265)    Max coord1 [ebp-8]:    [0x77626c, 0x776289)
#[test]
fn luabind_775_get_resize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let blocks: [(u64, u64, u64, &str); 4] = [
        (0x775fc8, 0x775fe5, 0xc, "Min.0"),
        (0x775fec, 0x776009, 0x10, "Min.1"),
        (0x776248, 0x776265, 0x4, "Max.0"),
        (0x77626c, 0x776289, 0x8, "Max.1"),
    ];

    let mut total = 0u32;
    for &(start, end, off, label) in &blocks {
        for &coord in &GETTER_PROBES {
            let bin = run_getter(&mut emu, &f, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "{label} coord={coord}"
            );
            total += 1;
        }
        let mut s = 0x775f_2000u32 ^ (start as u32);
        for _ in 0..1200 {
            let coord = rng(&mut s, -4096.0, 4096.0);
            let bin = run_getter(&mut emu, &f, start, end, off, coord);
            assert_eq!(
                point_to_logical(coord).to_bits(),
                bin.to_bits(),
                "{label} fuzz"
            );
            total += 1;
        }
    }
    // composed pairs
    for (w, h) in [(0.0f32, 0.0f32), (100.0, 200.0), (512.0, -768.0)] {
        let mw0 = run_getter(&mut emu, &f, 0x775fc8, 0x775fe5, 0xc, w);
        let mw1 = run_getter(&mut emu, &f, 0x775fec, 0x776009, 0x10, h);
        let (gw0, gw1) = get_resize(w, h);
        assert_eq!(gw0.to_bits(), mw0.to_bits(), "get_resize.0");
        assert_eq!(gw1.to_bits(), mw1.to_bits(), "get_resize.1");
        let mx0 = run_getter(&mut emu, &f, 0x776248, 0x776265, 0x4, w);
        let mx1 = run_getter(&mut emu, &f, 0x77626c, 0x776289, 0x8, h);
        let (gx0, gx1) = get_resize(w, h);
        assert_eq!(gx0.to_bits(), mx0.to_bits(), "get_resize.0");
        assert_eq!(gx1.to_bits(), mx1.to_bits(), "get_resize.1");
        total += 4;
    }
    eprintln!("luabind_775_get_resize: {total} bit-exact (Min/Max resize point_to_logical)");
}

/// SetMinResize 0x776020 / SetMaxResize 0x7762a0 — two args each, logical_to_device, stored f32.
/// Slices (call 0x41ad70 after lua_tonumber's fstp [ebp-4] .. after the result fstp). NB the
/// SECOND block of each fn interleaves the widget store-back integer moves before its result fstp,
/// so the slice extends past them to just after that fstp (straight-line, no branch join inside):
///   Min arg0 -> [ebp-0x10]: [0x7760fd, 0x776117)   Min arg1 -> [ebp-0x14]: [0x776126, 0x776146)
///   Max arg0 -> [ebp-8]:    [0x77637d, 0x776397)   Max arg1 -> [ebp-0xc]:  [0x7763a6, 0x7763cb)
#[test]
fn luabind_775_set_resize() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    let blocks: [(u64, u64, u64, &str); 4] = [
        (0x7760fd, 0x776117, 0x10, "Min.0"),
        (0x776126, 0x776146, 0x14, "Min.1"),
        (0x77637d, 0x776397, 0x8, "Max.0"),
        (0x7763a6, 0x7763cb, 0xc, "Max.1"),
    ];

    let mut total = 0u32;
    for &(start, end, res, label) in &blocks {
        for &arg in &SETTER_PROBES {
            let bin = run_setter(&mut emu, &f, start, end, res, arg);
            assert_eq!(
                logical_to_device(arg).to_bits(),
                bin.to_bits(),
                "{label} arg={arg}"
            );
            total += 1;
        }
        let mut s = 0x7760_2000u32 ^ (start as u32);
        for _ in 0..1200 {
            let arg = rng(&mut s, -4096.0, 4096.0);
            let bin = run_setter(&mut emu, &f, start, end, res, arg);
            assert_eq!(
                logical_to_device(arg).to_bits(),
                bin.to_bits(),
                "{label} fuzz"
            );
            total += 1;
        }
    }
    // composed pairs
    for (a, b) in [(0.0f32, 0.0f32), (1024.0, 2048.0), (-512.0, 768.0)] {
        let rn0 = run_setter(&mut emu, &f, 0x7760fd, 0x776117, 0x10, a);
        let rn1 = run_setter(&mut emu, &f, 0x776126, 0x776146, 0x14, b);
        let (sn0, sn1) = set_resize(a, b);
        assert_eq!(sn0.to_bits(), rn0.to_bits(), "set_resize.0");
        assert_eq!(sn1.to_bits(), rn1.to_bits(), "set_resize.1");
        let rx0 = run_setter(&mut emu, &f, 0x77637d, 0x776397, 0x8, a);
        let rx1 = run_setter(&mut emu, &f, 0x7763a6, 0x7763cb, 0xc, b);
        let (sx0, sx1) = set_resize(a, b);
        assert_eq!(sx0.to_bits(), rx0.to_bits(), "set_resize.0");
        assert_eq!(sx1.to_bits(), rx1.to_bits(), "set_resize.1");
        total += 4;
    }
    eprintln!("luabind_775_set_resize: {total} bit-exact (Min/Max resize logical_to_device)");
}
