//! `SetChatWindowColor` (`0x4a14f0`) and `SetChatWindowAlpha` (`0x4a15d0`) against the exe: each
//! component is the Lua number times the f64 255.0 (`0x806498`), truncated by `__ftol`, with no
//! clamp. The first test runs the multiply, the second the truncation too.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::color::{scale_255, scaled_byte};
use fpkit::next;

/// The canonical chat `fmul qword [0x806498]` source site (all four R/G/B/alpha sites are identical).
const FMUL_SRC: u64 = 0x4a1569;
/// `fmul qword ptr [0x806498]` = `DC 0D 98 64 80 00` (the verbatim owned-arith bytes).
const FMUL_BYTES: [u8; 6] = [0xdc, 0x0d, 0x98, 0x64, 0x80, 0x00];
const __FTOL: u64 = 0x40a2b0;

/// An f64 in `[lo, hi)` from 48 high bits (two draws; the low bits of the LCG repeat quickly).
fn rngd(s: &mut u32, lo: f64, hi: f64) -> f64 {
    let hi48 = (u64::from(next(s) >> 8) << 24) | u64::from(next(s) >> 8);
    lo + (hi48 as f64) / ((1u64 << 48) as f64) * (hi - lo)
}

/// product island: fld qword [in]; <source fmul qword [0x806498]>; fstp qword [out] — the LIVE f64.
#[test]
fn chat_scale_product() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // Byte-verify the `fmul qword [0x806498]` inside 0x4a15d0, which is the same operation as the
    // slice that is run.
    assert_eq!(
        emu.read(0x4a1617, 6),
        vec![0xdc, 0x0d, 0x98, 0x64, 0x80, 0x00],
        "fmul[0x806498] @0x4a1617 (0x4a15d0)"
    );

    let in_cell = emu.alloc(8);
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x40);

    // fld qword [in_cell]  =  DD 05 <disp32>
    let mut code = vec![0xddu8, 0x05];
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    // copied source fmul qword [0x806498]=255.0
    let fmul_off = code.len() as u64;
    code.extend_from_slice(&FMUL_BYTES);
    // fstp qword [out_cell]  =  DD 1D <disp32>
    code.extend_from_slice(&[0xdd, 0x1d]);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);

    // byte-verify the island's fmul == the source site (else the island proves nothing).
    assert_eq!(
        emu.read(isl + fmul_off, 6),
        emu.read(FMUL_SRC, 6),
        "chat island fmul != source @ {FMUL_SRC:#x}"
    );

    let run = |emu: &mut Emu, x: f64| -> f64 {
        emu.write(in_cell, &x.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        f64::from_bits(
            u64::from(emu.read_u32(out_cell)) | (u64::from(emu.read_u32(out_cell + 4)) << 32),
        )
    };

    let chk = |emu: &mut Emu, x: f64, total: &mut u32| {
        let bin = run(emu, x);
        assert_eq!(
            bin.to_bits(),
            scale_255(x).to_bits(),
            "scale_255 x={x} ({:#018x}): binary {bin} vs rust {}",
            x.to_bits(),
            scale_255(x)
        );
        *total += 1;
    };

    let mut total = 0u32;
    // deterministic probes: the runtime [0,1] band edges, ±0, a single alpha step, and exact pivots.
    let eps = f64::from_bits(1);
    let probes: [f64; 12] = [
        0.0,
        -0.0,
        1.0,
        0.5,
        1.0 / 255.0,
        254.0 / 255.0,
        f64::from_bits(1.0f64.to_bits() - 1), // 1 - 1 ulp
        f64::from_bits(1.0f64.to_bits() + 1), // 1 + 1 ulp
        eps,
        -eps,
        2.0,    // no clamp: > 1 still scales
        -123.5, // negative still scales (no clamp)
    ];
    for x in probes {
        chk(&mut emu, x, &mut total);
    }
    // spanning fuzz: the dense runtime [0,1] band, plus a wide band (no clamp) incl. negatives.
    let mut s = 0x4a14_f000u32;
    for _ in 0..6000 {
        chk(&mut emu, rngd(&mut s, 0.0, 1.0), &mut total);
    }
    for _ in 0..3000 {
        chk(&mut emu, rngd(&mut s, -1e6, 1e6), &mut total);
    }
    eprintln!(
        "chat_scale_product: {total} bit-exact (island: fmul qword [0x806498] == {FMUL_SRC:#x})"
    );
}

/// byte island: fld qword [in]; <source fmul>; call __ftol; read EAX — the stored color/alpha byte.
#[test]
fn chat_scale_byte() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let in_cell = emu.alloc(8);
    let isl = emu.alloc(0x40);
    let stack = emu.alloc(0x1000);
    let esp = stack + 0xf00; // __ftol uses a stack frame (push ebp; sub esp); give it room

    // fld qword [in_cell]  =  DD 05 <disp32>
    let mut code = vec![0xddu8, 0x05];
    code.extend_from_slice(&(in_cell as u32).to_le_bytes());
    // copied source fmul qword [0x806498]=255.0
    let fmul_off = code.len() as u64;
    code.extend_from_slice(&FMUL_BYTES);
    // call __ftol  =  E8 <rel32>, rel32 = target - (next_instr_va)
    let call_va = isl + code.len() as u64;
    let rel = (__FTOL as i64 - (call_va as i64 + 5)) as i32;
    code.push(0xe8);
    code.extend_from_slice(&rel.to_le_bytes());
    let isl_end = isl + code.len() as u64; // __ftol's `ret` lands here
    emu.write(isl, &code);

    assert_eq!(
        emu.read(isl + fmul_off, 6),
        emu.read(FMUL_SRC, 6),
        "chat byte-island fmul != source @ {FMUL_SRC:#x}"
    );

    let run = |emu: &mut Emu, x: f64| -> u8 {
        emu.set_reg(RegisterX86::ESP, esp);
        emu.write(in_cell, &x.to_bits().to_le_bytes());
        emu.run(isl, isl_end);
        (emu.get_reg(RegisterX86::EAX) as i32) as u8 // __ftol result, low byte (the stored color byte)
    };

    let chk = |emu: &mut Emu, x: f64, total: &mut u32| {
        assert_eq!(
            run(emu, x),
            scaled_byte(x),
            "scaled_byte x={x} ({:#018x})",
            x.to_bits()
        );
        *total += 1;
    };

    let mut total = 0u32;
    // every byte target x = k/255 and a touch above/below (the __ftol truncate pivots at integers).
    for k in 0u32..=255 {
        for d in [-1e-9, 0.0, 1e-9] {
            chk(&mut emu, f64::from(k) / 255.0 + d, &mut total);
        }
    }
    // exact-integer-product pivots: x·255 just below / at an integer flips the truncated byte.
    let pivots = [
        0.0f64,
        1.0,
        1.0 / 255.0,
        1.0 / 255.0 - 1e-12,
        2.0 / 255.0,
        2.0 / 255.0 - 1e-12,
        254.0 / 255.0,
        128.0 / 255.0,
        0.999,
        0.001,
        0.99999,
    ];
    for x in pivots {
        chk(&mut emu, x, &mut total);
    }
    // dense runtime-domain fuzz.
    let mut s = 0x4a15_d000u32;
    for _ in 0..4000 {
        chk(&mut emu, rngd(&mut s, 0.0, 1.0), &mut total);
    }
    eprintln!("chat_scale_byte: {total} bit-exact (island: fmul;__ftol == {FMUL_SRC:#x})");
}
