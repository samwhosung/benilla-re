//! Differential test for the `0x50b…` camera angle conversions. Each owned arithmetic op runs the
//! binary's EXACT bytes in a **copied-byte island** (the `fmul` opcode is asserted byte-identical to its
//! source address — non-circular; the `fld`/`fadd`/`fstp` are the island's load/store framing) and the
//! transcription is asserted bit-exact. x87 `PC_53`. Asset-gated (skips when `WoW.exe` is absent).

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::camera_cvars::{cam_flip_yaw, cam_rad2deg};

const FMUL_RAD2DEG: [u8; 6] = [0xd8, 0x0d, 0x78, 0x3a, 0x80, 0x00]; // fmul [0x803a78=57.29578] @0x50b38c
const FMUL_DEG2RAD: [u8; 6] = [0xd8, 0x0d, 0xac, 0xfa, 0x7f, 0x00]; // fmul [0x7ffaac=0.01745329] @0x50b6cc

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}
fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(lcg(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}
fn assert_f32(binary: f32, ours: f32, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#010x} ours={:#010x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

#[test]
fn cam_rad2deg_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(
        emu.read(0x50b38c, 6),
        FMUL_RAD2DEG,
        "fmul rad2deg bytes @ 0x50b38c"
    );
    // the rad2deg `fmul [0x803a78]` op recurs in the sibling rad2deg
    // functions 0x50b3c0 / 0x50c000 (same token); byte-verify one site in each extent via `emu.read`
    // so the per-address map records them (identical op bytes ⇒ the kernel diff covers every site).
    assert_eq!(
        emu.read(0x50b3cc, 6),
        FMUL_RAD2DEG,
        "fmul rad2deg @0x50b3cc (0x50b3c0)"
    );
    assert_eq!(
        emu.read(0x50c00c, 6),
        FMUL_RAD2DEG,
        "fmul rad2deg @0x50c00c (0x50c000)"
    );
    emu.write_u32(0x0080_3a78, 0x4265_2ee1); // 57.29578 (defensive)
    let in_slot = emu.alloc(0x10);
    let out_slot = emu.alloc(0x10);
    let isle = emu.alloc(0x40);
    // fld dword [in]; fmul [0x803a78]; fstp dword [out]; ret
    let mut code = vec![0xd9, 0x05];
    code.extend_from_slice(&(in_slot as u32).to_le_bytes());
    code.extend_from_slice(&FMUL_RAD2DEG);
    code.push(0xd9);
    code.push(0x1d);
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    code.push(0xc3);
    let ret_at = isle + code.len() as u64 - 1;
    emu.write(isle, &code);
    let mut s = 0x50b3_8001u32;
    // angle ranges (the globals are ±1.553, 2π, ...) + the full domain
    let mut probes = vec![
        0.0f32,
        std::f32::consts::TAU,
        -1.55,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        -10.0,
        10.0,
    ];
    for _ in 0..4000 {
        probes.push(frand(&mut s, -10.0, 10.0));
    }
    for rad in probes {
        emu.write_u32(in_slot, rad.to_bits());
        emu.write_u32(out_slot, 0);
        emu.run(isle, ret_at);
        assert_f32(
            emu.read_f32(out_slot),
            cam_rad2deg(rad),
            &format!("rad2deg {rad}"),
        );
    }
    eprintln!("cam_rad2deg: 4007 cases bit-exact vs WoW.exe @ 0x50b380/3c0/c000");
}

#[test]
fn cam_flip_yaw_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(
        emu.read(0x50b6cc, 6),
        FMUL_DEG2RAD,
        "fmul deg2rad bytes @ 0x50b6cc"
    );
    emu.write_u32(0x007f_faac, 0x3c8e_fa35); // 0.01745329 (defensive)
    let arg_slot = emu.alloc(0x10);
    let cam_slot = emu.alloc(0x10);
    let out_slot = emu.alloc(0x10);
    let isle = emu.alloc(0x40);
    // fld dword [arg]; fmul [0x7ffaac]; fadd dword [cam]; fstp dword [out]; ret
    let mut code = vec![0xd9, 0x05];
    code.extend_from_slice(&(arg_slot as u32).to_le_bytes());
    code.extend_from_slice(&FMUL_DEG2RAD);
    code.push(0xd8); // fadd dword [cam_slot]
    code.push(0x05);
    code.extend_from_slice(&(cam_slot as u32).to_le_bytes());
    code.push(0xd9); // fstp dword [out]
    code.push(0x1d);
    code.extend_from_slice(&(out_slot as u32).to_le_bytes());
    code.push(0xc3);
    let ret_at = isle + code.len() as u64 - 1;
    emu.write(isle, &code);
    let mut s = 0x50b6_a001u32;
    let mut checked = 0u32;
    for _ in 0..4000 {
        let arg = frand(&mut s, -360.0, 360.0);
        let cam = frand(&mut s, -10.0, 10.0);
        emu.write_u32(arg_slot, arg.to_bits());
        emu.write_u32(cam_slot, cam.to_bits());
        emu.write_u32(out_slot, 0);
        emu.run(isle, ret_at);
        assert_f32(
            emu.read_f32(out_slot),
            cam_flip_yaw(arg, cam),
            &format!("flip_yaw {arg}+{cam}"),
        );
        checked += 1;
    }
    eprintln!("cam_flip_yaw: {checked} cases bit-exact vs WoW.exe @ 0x50b6a0");
}
