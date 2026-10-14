//! The scroll frame offset scripts (`SetHorizontalScroll`, `SetVerticalScroll`,
//! `GetHorizontalScroll`, `GetVerticalScroll`, `GetHorizontalScrollRange`,
//! `GetVerticalScrollRange`) against the exe: each converts one length through the layout scale with
//! no widget scale, run as the slice of instructions from the `call 0x41ad70` to the store or push,
//! including the leaf helpers. The scroll field is loaded before the slice.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::ui_scale::{
    get_horizontal_scroll, get_horizontal_scroll_range, get_vertical_scroll,
    get_vertical_scroll_range, logical_to_device, point_to_logical, set_horizontal_scroll,
    set_vertical_scroll,
};
use fpkit::{read_f64, rng};

struct Frame {
    this: u64,
    ebp: u64,
    esp: u64,
}
fn frame(emu: &mut Emu) -> Frame {
    let this = emu.alloc(0x400);
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
// Getters: device_to_logical slice — call 0x41ad70; fmul [1024]; push ecx; fmul [ebp-4]=field;
// fstp [esp]; call 0x41ae40; sub esp,8; fstp qword [esp]. Reads the f64 at the final ESP.
// The struct field [eax+0x32X] is copied into [ebp-4] just BEFORE the slice, so we put `field`
// at [ebp-4] directly. All four getters are byte-identical in this slice (modulo masked call rel32).
fn run_getter(emu: &mut Emu, f: &Frame, start: u64, end: u64, field: f32) -> f64 {
    set_frame(emu, f);
    emu.write_u32(f.ebp - 0x4, field.to_bits());
    emu.run(start, end);
    let esp = emu.get_reg(RegisterX86::ESP);
    read_f64(emu, esp)
}

#[test]
fn luabind_791_getters() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    struct G {
        name: &'static str,
        start: u64,
        end: u64,
        ours: fn(f32) -> f64,
    }
    let getters = [
        G {
            name: "GetHorizontalScroll",
            start: 0x7915dc,
            end: 0x7915f9,
            ours: get_horizontal_scroll,
        },
        G {
            name: "GetVerticalScroll",
            start: 0x7916bc,
            end: 0x7916d9,
            ours: get_vertical_scroll,
        },
        G {
            name: "GetHorizontalScrollRange",
            start: 0x79179c,
            end: 0x7917b9,
            ours: get_horizontal_scroll_range,
        },
        G {
            name: "GetVerticalScrollRange",
            start: 0x79187c,
            end: 0x791899,
            ours: get_vertical_scroll_range,
        },
    ];

    let mut total = 0u32;
    for g in &getters {
        // deterministic boundaries: field==0 (=> 0.0); field==1.0/1024 (1024·field == 1.0 exactly);
        // field==1024 (1024·1024); a negative; and a value with no exact f32 product.
        for field in [0.0f32, 1.0 / 1024.0, 1024.0, -1.0, 0.5, 123.456] {
            let bin = run_getter(&mut emu, &f, g.start, g.end, field);
            let mine = (g.ours)(field);
            assert_eq!(mine.to_bits(), bin.to_bits(), "{} field={field}", g.name);
            // independent: the shared kernel matches the named row.
            assert_eq!(point_to_logical(field).to_bits(), bin.to_bits());
            total += 1;
        }
    }
    let mut s = 0x7915_dc00u32;
    for g in &getters {
        for _ in 0..1500 {
            let field = rng(&mut s, -4096.0, 4096.0);
            let bin = run_getter(&mut emu, &f, g.start, g.end, field);
            let mine = (g.ours)(field);
            assert_eq!(mine.to_bits(), bin.to_bits(), "{} fuzz", g.name);
            assert_eq!(point_to_logical(field).to_bits(), bin.to_bits());
            total += 1;
        }
    }
    eprintln!(
        "luabind_791_getters: {total} bit-exact (4 ScrollFrame getters device_to_logical, 1024·field /0.8)"
    );
}

// Setters: SetHorizontalScroll 0x7912f0 / SetVerticalScroll 0x791400 — logical_to_device(luaArg) =
// (luaArg/1024) f32 ·0.8 stored f32. Slice: call 0x41ad70; fmul [1024]; push ecx; fdivr [ebp-4]=arg;
// fstp [esp]; call 0x41ae60; fstp [ebp-4]. luaArg is at [ebp-4] before the slice; result f32 at
// [ebp-4]. Byte-identical between the two rows (modulo masked call rel32 + the later delegate target).
fn run_setter(emu: &mut Emu, f: &Frame, start: u64, end: u64, arg: f32) -> f32 {
    set_frame(emu, f);
    emu.write_u32(f.ebp - 0x4, arg.to_bits());
    emu.run(start, end);
    emu.read_f32(f.ebp - 0x4)
}

#[test]
fn luabind_791_setters() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let f = frame(&mut emu);

    struct S {
        name: &'static str,
        start: u64,
        end: u64,
        ours: fn(f32) -> f32,
    }
    let setters = [
        S {
            name: "SetHorizontalScroll",
            start: 0x791399,
            end: 0x7913b3,
            ours: set_horizontal_scroll,
        },
        S {
            name: "SetVerticalScroll",
            start: 0x7914a9,
            end: 0x7914c3,
            ours: set_vertical_scroll,
        },
    ];

    let mut total = 0u32;
    for s in &setters {
        // deterministic boundaries: arg==0; arg==1024 (1024/1024 == 1.0 exactly); arg==0.8; negative.
        for arg in [0.0f32, 1024.0, 2048.0, -512.0, 0.8, 1.0 / 0.8] {
            let bin = run_setter(&mut emu, &f, s.start, s.end, arg);
            let mine = (s.ours)(arg);
            assert_eq!(mine.to_bits(), bin.to_bits(), "{} arg={arg}", s.name);
            // independent: the shared kernel matches the named row.
            assert_eq!(logical_to_device(arg).to_bits(), bin.to_bits());
            total += 1;
        }
    }
    let mut rs = 0x7912_f000u32;
    for s in &setters {
        for _ in 0..2000 {
            let arg = rng(&mut rs, -4096.0, 4096.0);
            let bin = run_setter(&mut emu, &f, s.start, s.end, arg);
            let mine = (s.ours)(arg);
            assert_eq!(mine.to_bits(), bin.to_bits(), "{} fuzz", s.name);
            assert_eq!(logical_to_device(arg).to_bits(), bin.to_bits());
            total += 1;
        }
    }
    eprintln!(
        "luabind_791_setters: {total} bit-exact (2 ScrollFrame setters logical_to_device, arg/1024 ·0.8)"
    );
}
