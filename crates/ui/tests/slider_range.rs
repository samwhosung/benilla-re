//! `Slider::GetMinMaxValues` (`0x7906e0`) against the exe: the maximum is rebuilt as the stored range
//! plus the stored minimum (`fadd dword [edi+0x318]`) and pushed as an f64.

mod fpkit;

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_ui::frame_methods::max_value;
use fpkit::rng;

/// The owned `fld dword [edi+0x31c]; fadd dword [edi+0x318]` source site.
const ADD_SRC: u64 = 0x790778;
/// `fld dword [edi+0x31c]` (`D9 87 1C 03 00 00`) + `fadd dword [edi+0x318]` (`D8 87 18 03 00 00`).
const ADD_BYTES: [u8; 12] = [
    0xd9, 0x87, 0x1c, 0x03, 0x00, 0x00, // fld  dword [edi+0x31c]  (range)
    0xd8, 0x87, 0x18, 0x03, 0x00, 0x00, // fadd dword [edi+0x318]  (min)
];

#[test]
fn minmax_max_value() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    let this = emu.alloc(0x400); // edi base; fields at [edi+0x318]=min, [edi+0x31c]=range
    let out_cell = emu.alloc(8);
    let isl = emu.alloc(0x40);

    // copied source fld dword [edi+0x31c]; fadd dword [edi+0x318]
    let add_off = 0u64;
    let mut code = ADD_BYTES.to_vec();
    // fstp qword [out_cell]  =  DD 1D <disp32>
    code.extend_from_slice(&[0xdd, 0x1d]);
    code.extend_from_slice(&(out_cell as u32).to_le_bytes());
    let isl_end = isl + code.len() as u64;
    emu.write(isl, &code);

    // byte-verify the island's fld;fadd == the source site (else the island proves nothing).
    assert_eq!(
        emu.read(isl + add_off, 12),
        emu.read(ADD_SRC, 12),
        "minmax island fld;fadd != source @ {ADD_SRC:#x}"
    );

    let run = |emu: &mut Emu, min: f32, range: f32| -> f64 {
        emu.set_reg(RegisterX86::EDI, this);
        emu.write_u32(this + 0x318, min.to_bits());
        emu.write_u32(this + 0x31c, range.to_bits());
        emu.run(isl, isl_end);
        f64::from_bits(
            u64::from(emu.read_u32(out_cell)) | (u64::from(emu.read_u32(out_cell + 4)) << 32),
        )
    };

    let chk = |emu: &mut Emu, min: f32, range: f32, total: &mut u32| {
        let bin = run(emu, min, range);
        assert_eq!(
            bin.to_bits(),
            max_value(min, range).to_bits(),
            "max_value min={min} range={range} ({:#010x},{:#010x}): binary {bin} vs rust {}",
            min.to_bits(),
            range.to_bits(),
            max_value(min, range)
        );
        *total += 1;
    };

    let mut total = 0u32;
    // deterministic boundary pairs.
    let eps = f32::from_bits(1);
    let probes: [(f32, f32); 14] = [
        (0.0, 0.0),
        (0.0, 1.0),
        (-0.0, 0.0),
        (1.0, 0.0),     // range 0 -> max == min
        (0.0, 100.0),   // slider 0..100
        (-50.0, 100.0), // min<0
        (1e6, 1e6),     // large, exact
        (-1e6, 2e6),    // large cancellation toward 0
        (1.0, -1.0),    // negative range -> max < min (binary computes it literally)
        (f32::MIN, f32::MAX),
        (eps, eps), // smallest subnormal step
        (1.0, eps), // 1 + 1ulp(min subnormal) -> f32->f64 widen exact, f64 add exact
        (0.5, 0.25),
        (123.456, 7.89),
    ];
    for (min, range) in probes {
        chk(&mut emu, min, range, &mut total);
    }
    // fuzz: realistic slider ranges and a wide band incl. negatives.
    let mut s = 0x7906_e000u32;
    for _ in 0..6000 {
        chk(
            &mut emu,
            rng(&mut s, 0.0, 1000.0),
            rng(&mut s, 0.0, 1000.0),
            &mut total,
        );
    }
    for _ in 0..3000 {
        chk(
            &mut emu,
            rng(&mut s, -1e6, 1e6),
            rng(&mut s, -1e6, 1e6),
            &mut total,
        );
    }
    eprintln!("minmax_max_value: {total} bit-exact (island: fld;fadd == {ADD_SRC:#x})");
}
