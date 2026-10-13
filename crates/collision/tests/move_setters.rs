//! The six speed setters, `CMovement::SetTurnRate` (`0x7c6ff0`) to `CMovement::SetSwimBackSpeed`
//! (`0x7c7170`), run whole with `CMovement::UpdateAnchors` (`0x7c5cd0`) and
//! `CMovement::UpdateCurrentSpeed` (`0x7c5c20`) stubbed; each stores before those calls, so the
//! stubs do not change the field or `eax`. `CMovement::GetCurrentSpeed` (`0x7c4c90`) runs whole.

use benilla_re_collision::move_setters::{eps_should_store, get_speed, GetSpeedState};
use benilla_re_difftest::{Emu, Wow};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

fn fz(s: &mut u32) -> f32 {
    ((next(s) >> 9) as f32 / (1 << 23) as f32) * 8.0 - 4.0
}
fn assert_bits(b: f32, o: f32, what: &str) {
    assert_eq!(
        b.to_bits(),
        o.to_bits(),
        "{what}: binary={:#010x} ours={:#010x}",
        b.to_bits(),
        o.to_bits()
    );
}

const P2_23: f32 = f32::from_bits(0x3400_0000);
const P2_22: f32 = f32::from_bits(0x3480_0000);
const P15_22: f32 = f32::from_bits(0x34c0_0000);
const QNAN: f32 = f32::from_bits(0x7fc0_0000);
const SNAN: f32 = f32::from_bits(0x7f80_0001);

fn setter_pairs() -> Vec<(f32, f32)> {
    let mut v: Vec<(f32, f32)> = vec![
        (0.0, 0.0),
        (0.0, P2_23),
        (0.0, -P2_23),
        (0.0, P2_22),
        (0.0, -P2_22),
        (0.0, P15_22),
        (1.0, 1.0),
        (1.0, f32::from_bits(0x3f80_0001)),
        (1.0, f32::from_bits(0x3f80_0002)),
        (1.5, f32::from_bits(0x3fc0_0001)),
        (1.5, f32::from_bits(0x3fc0_0002)),
        (2.0, f32::from_bits(0x4000_0001)),
        (5.0, 5.0),
        (5.0, -5.0),
        (-3.0, 2.0),
        (-3.0, f32::from_bits(0xc040_0001)),
        (QNAN, 1.0),
        (1.0, QNAN),
        (QNAN, QNAN),
        (1.0, SNAN),
        (123.456, 123.456),
        (123.456, 123.4561),
    ];
    let mut s = 0x1234_abcdu32;
    for _ in 0..400 {
        let field = fz(&mut s);

        let arg = match (next(&mut s) >> 8) % 6 {
            0 => field,
            1 => field + P2_23,
            2 => field - P2_22,
            3 => field + fz(&mut s) * 1.0e-6,
            4 => field + fz(&mut s) * 1.0e-3,
            _ => fz(&mut s),
        };
        v.push((field, arg));
    }
    v
}

#[test]
fn move_setters_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);

    emu.stub(0x7c5cd0, 0, 1);
    emu.stub(0x7c5c20, 0, 1);

    let this_va = emu.alloc(0x100);
    let tp_va = emu.alloc(0x40);
    let mut checked = 0u32;

    let setters: [(u64, u64); 6] = [
        (0x7c6ff0, 0x9c),
        (0x7c7030, 0x8c),
        (0x7c7080, 0x90),
        (0x7c70d0, 0x88),
        (0x7c7120, 0x94),
        (0x7c7170, 0x98),
    ];
    let pairs = setter_pairs();
    for &(addr, off) in &setters {
        for &(field, arg) in &pairs {
            emu.write_u32(this_va + off, field.to_bits());
            let eax = emu.call(addr, this_va as u32, 0, &[arg.to_bits()]);
            let stored = emu.read_u32(this_va + off);

            let store = eps_should_store(field, arg);
            let want_eax = u32::from(store);
            let want_stored = if store {
                arg.to_bits()
            } else {
                field.to_bits()
            };
            assert_eq!(
                eax, want_eax,
                "setter {addr:#x} off {off:#x} field={:#010x} arg={:#010x}: eax binary={eax} ours={want_eax}",
                field.to_bits(),
                arg.to_bits()
            );
            assert_eq!(
                stored, want_stored,
                "setter {addr:#x} off {off:#x} field={:#010x} arg={:#010x}: stored binary={stored:#010x} ours={want_stored:#010x}",
                field.to_bits(),
                arg.to_bits()
            );
            checked += 1;
        }
    }

    type Case = (u32, u32, Option<(u8, i32, f32)>, f32, f32, f32, f32, f32);
    let mut cases: Vec<Case> = vec![
        (0x0, 0, None, 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x10, 0, None, 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x0020_0000, 0, None, 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x100, 1, None, 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x1, 0, Some((0, 4, 8.0)), 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x1, 0, Some((0, 3, 1.0)), 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x1, 0, Some((1, -2, 5.0)), 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x1, 0, Some((2, 7, 9.5)), 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x1, 0, Some((0, 0, 5.0)), 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x1, 0, Some((0, 0, -5.0)), 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x1, 0, Some((4, 7, 9.0)), 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x3, 0, Some((0x4, 7, 9.0)), 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x0020_0001, 0, None, 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x0020_0003, 0, None, 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x0020_0003, 0, None, 1.0, 2.0, 3.0, 9.0, 4.0),
        (0x0020_0003, 0, None, 1.0, 2.0, 3.0, 4.0, 4.0),
        (0x1, 0, None, 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x3, 0, None, 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x3, 0, None, 1.0, 5.0, 3.0, 4.0, 5.0),
        (0x3, 0, None, 1.0, 3.0, 3.0, 4.0, 5.0),
        (0x101, 0, None, 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x101, 0, None, 6.0, 2.0, 3.0, 4.0, 5.0),
        (0x101, 0, None, 2.0, 2.0, 3.0, 4.0, 5.0),
        (0x1, 1, None, 1.0, 2.0, 3.0, 4.0, 5.0),
        (0x3, 0, None, 1.0, QNAN, 3.0, 4.0, 5.0),
        (0x3, 0, None, 1.0, 2.0, QNAN, 4.0, 5.0),
        (0x0020_0003, 0, None, 1.0, 2.0, 3.0, QNAN, 5.0),
        (0x101, 0, None, QNAN, 2.0, 3.0, 4.0, 5.0),
    ];
    let mut s = 0x0c0f_fee5u32;
    for _ in 0..400 {
        let mut flags = next(&mut s) & 0xf;
        if next(&mut s) & 0x10000 != 0 {
            flags |= 0x2;
        }
        if next(&mut s) & 0x20000 != 0 {
            flags |= 0x100;
        }
        if next(&mut s) & 0x40000 != 0 {
            flags |= 0x0020_0000;
        }
        let param = if next(&mut s) & 0x80000 != 0 { 1 } else { 0 };
        let transport = if next(&mut s) & 0x100000 != 0 {
            let tf = (next(&mut s) & 0x7) as u8;
            let i24 = (next(&mut s) as i32) % 97 + 1;
            Some((tf, i24, fz(&mut s) * 100.0))
        } else {
            None
        };
        cases.push((
            flags,
            param,
            transport,
            fz(&mut s),
            fz(&mut s),
            fz(&mut s),
            fz(&mut s),
            fz(&mut s),
        ));
    }

    for &(flags, param, transport, walk, run, runback, swim, swimback) in &cases {
        let st = GetSpeedState {
            flags,
            walk,
            run,
            runback,
            swim,
            swimback,
            transport,
        };
        emu.write_u32(this_va + 0x40, st.flags);
        emu.write_u32(this_va + 0x88, st.walk.to_bits());
        emu.write_u32(this_va + 0x8c, st.run.to_bits());
        emu.write_u32(this_va + 0x90, st.runback.to_bits());
        emu.write_u32(this_va + 0x94, st.swim.to_bits());
        emu.write_u32(this_va + 0x98, st.swimback.to_bits());
        match st.transport {
            Some((tf, i24, f30)) => {
                emu.write_u32(this_va + 0xa4, tp_va as u32);
                emu.write_u32(tp_va + 0x18, u32::from(tf));
                emu.write_u32(tp_va + 0x24, i24 as u32);
                emu.write_u32(tp_va + 0x30, f30.to_bits());
            }
            None => emu.write_u32(this_va + 0xa4, 0),
        }
        let got = emu.call_f32(0x7c4c90, this_va as u32, 0, &[param]);
        let want = get_speed(&st, param);
        assert_bits(
            got,
            want,
            &format!("0x7c4c90 get_speed flags={flags:#x} param={param}"),
        );
        checked += 1;
    }

    eprintln!("move_setters: {checked} rounds bit-exact vs WoW.exe (9 addresses)");
}
