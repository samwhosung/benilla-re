//! The facing and screen conversions of the script functions
//! (`systems/glue/select.md#facing-in-degrees`, `systems/glue/script.md#cursor-and-screen-size`).

mod common;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_glue::lua_scale::{lua_deg2rad, lua_model_scale, lua_rad2deg};
use common::*;

/// The facings the getters read: the create facing at `0xb420f4` and the select facing at
/// `0xb4217c`.
const MODEL_FACING: u64 = 0x00b4_20f4;
const MODEL_FACING2: u64 = 0x00b4_217c;

/// Reads a little-endian `f64` at `va`.
fn read_f64(e: &Emu, va: u64) -> f64 {
    let b = e.read(va, 8);
    f64::from_bits(u64::from_le_bytes(b.try_into().unwrap()))
}

/// Runs the getters `GetCharacterCreateFacing` (`0x471910`, to `0x471922`) and
/// `GetCharacterSelectFacing` (`0x473650`, to `0x473662`) up to their `lua_pushnumber`, and reads the `f64`
/// they leave at `[esp]`. The facings run from -2π to 2π and to large magnitudes.
#[test]
fn lua_rad2deg_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let stack = e.alloc(0x40) + 0x20; // esp, with 8 bytes below for `sub esp,8`
                                      // (begin, end, global): the two getters have the same bytes but for the global they read.
    let cases: [(u64, u64, u64); 2] = [
        (0x471910, 0x471922, MODEL_FACING),
        (0x473650, 0x473662, MODEL_FACING2),
    ];
    let mut s: u32 = 0x9a1c_0001;
    for (begin, end, global) in cases {
        for i in 0..4000u32 {
            // Facings from -2π to 2π, and large magnitudes.
            let facing = match (next(&mut s) >> 8) % 3 {
                0 => fuzz_unit(&mut s, 12.6) - 6.3, // about -2π to 2π
                1 => fuzz_unit(&mut s, 2.0) - 1.0,
                _ => fuzz_unit(&mut s, 2.0e3) - 1.0e3,
            };
            e.write_u32(global, facing.to_bits());
            e.set_reg(RegisterX86::ESP, stack);
            e.run(begin, end);
            let bin = read_f64(&e, e.get_reg(RegisterX86::ESP));
            assert_f64(
                bin,
                lua_rad2deg(facing),
                &format!("lua_rad2deg[{begin:#x}][{i}] facing={facing}"),
            );
        }
    }
}

/// Runs the setters `SetCharacterCreateFacing` (`0x471930`, from `0x47195a` to `0x471969`) and
/// `SetCharacterSelectFacing` (`0x473670`, from `0x47369a` to `0x4736a9`) from their call of `lua_tonumber`,
/// which is overwritten by `fld qword [eax]` (and three `nop`s) to inject a chosen `double`, and reads the
/// `f32` they leave at `[esp]`.
#[test]
fn lua_deg2rad_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let input = e.alloc(8);
    let stack = e.alloc(0x40) + 0x20;
    // (call, end): the two setters have the same layout.
    let cases: [(u64, u64); 2] = [(0x47195a, 0x471969), (0x47369a, 0x4736a9)];
    let mut s: u32 = 0x33b7_0001;
    for (call_site, end) in cases {
        e.write(call_site, &[0xdd, 0x00, 0x90, 0x90, 0x90]); // fld qword [eax]; nop; nop; nop
        for i in 0..4000u32 {
            // Degrees as a Lua `double`: angles, and large magnitudes.
            let deg = match (next(&mut s) >> 8) % 3 {
                0 => f64::from(fuzz_unit(&mut s, 360.0)),
                1 => f64::from(fuzz_unit(&mut s, 720.0)) - 360.0,
                _ => f64::from(fuzz_unit(&mut s, 1.0e6)) - 5.0e5,
            };
            e.write(input, &deg.to_bits().to_le_bytes());
            e.set_reg(RegisterX86::EAX, input);
            e.set_reg(RegisterX86::ESP, stack);
            e.run(call_site, end);
            let bin = e.read_f32(e.get_reg(RegisterX86::ESP));
            assert_f32(
                bin,
                lua_deg2rad(deg),
                &format!("lua_deg2rad[{call_site:#x}][{i}] deg={deg}"),
            );
        }
    }
}

/// Runs the arithmetic of `GetCursorPosition` (`0x46dad0`, from `0x46db05`), `GetScreenWidth` (`0x46dd30`,
/// from `0x46dd44`) and `GetScreenHeight` (`0x46dd80`, from `0x46dd94`) from their call of `0x41ad70`,
/// which is overwritten by `fld dword [eax]` (and three `nop`s) to inject the screen scale, and reads
/// the `f32` they leave at `[esp]` for `0x41ae40`.
#[test]
fn lua_model_scale_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let scale_in = e.alloc(4);
    let frame = e.alloc(0x40) + 0x20; // ebp: the coordinate at [ebp-offset]
    let stack = e.alloc(0x40) + 0x20; // esp, with room for `push ecx`
                                      // (call, end, offset of the coordinate) for the three getters.
    let cases: [(u64, u64, u64); 3] = [
        (0x46db05, 0x46db17, 0x8), // GetCursorPosition, the first axis
        (0x46dd44, 0x46dd56, 0x4), // GetScreenWidth
        (0x46dd94, 0x46dda6, 0x4), // GetScreenHeight
    ];
    let mut s: u32 = 0x5e02_0001;
    for (call_site, end, coord_off) in cases {
        e.write(call_site, &[0xd9, 0x00, 0x90, 0x90, 0x90]); // fld dword [eax]; nop; nop; nop
        for i in 0..4000u32 {
            let scale = fuzz_unit(&mut s, 4.0) - 2.0; // the scale `0x41ad70` returns in st0
            let coord = fuzz_unit(&mut s, 4.0) - 2.0; // the scaled coordinate
            e.write_u32(scale_in, scale.to_bits());
            e.write_u32(frame - coord_off, coord.to_bits());
            e.set_reg(RegisterX86::EAX, scale_in);
            e.set_reg(RegisterX86::EBP, frame);
            e.set_reg(RegisterX86::ESP, stack);
            e.run(call_site, end);
            let bin = e.read_f32(e.get_reg(RegisterX86::ESP));
            assert_f32(
                bin,
                lua_model_scale(scale, coord),
                &format!("lua_model_scale[{call_site:#x}][{i}] scale={scale} coord={coord}"),
            );
        }
    }
}
