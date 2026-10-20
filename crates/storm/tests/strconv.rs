//! Storm's `SStrToFloat` (`0x64aaa0`), the power table it builds on first use (`0x64a680`) and
//! `SStrToInt` (`0x64ac60`), each run whole in the emulator against `str_to_float`, `pow10_table`
//! and `str_to_int`.
//!
//! `SStrToFloat` runs with its own callees: `0x64a670` and the table builder, `SStrToInt`, and the C
//! runtime's `pow` (`0x73f940`), whose `fyl2x` and `f2xm1` Unicorn computes. The reimplementation
//! takes its powers of ten from that same `pow`, called through a trampoline, so the comparison is
//! bit-exact for the conversion's own arithmetic and grammar, and `pow` is not under test. The
//! inputs keep every step a normal double: at most 60 digits on either side of the point and an
//! exponent within ±99. `SErrSetLastError` (`0x64e850`) is stubbed, since it calls `SetLastError`.

mod common;

use benilla_re_difftest::Emu;
use benilla_re_storm::{pow10_table, str_to_float, str_to_int};
use common::Wow;
use std::cell::Cell;
use std::collections::HashMap;

const POW_RANGE: i32 = 100;

/// `pow(10.0, n)` as the exe's `pow` answers it, for `n` within ±`POW_RANGE`, through
/// `fld qword [10.0]; fild dword [esp+4]; call 0x73f940; ret`.
fn pow10_oracle(emu: &mut Emu) -> HashMap<i32, f64> {
    let ten = emu.alloc(8);
    emu.write(ten, &10f64.to_le_bytes());
    let code = emu.alloc(0x20);
    let mut c = vec![0xdd, 0x05];
    c.extend_from_slice(&(ten as u32).to_le_bytes());
    c.extend_from_slice(&[0xdb, 0x44, 0x24, 0x04]);
    let call_at = code + c.len() as u64;
    let rel = (0x73f940i64 - (call_at as i64 + 5)) as i32;
    c.push(0xe8);
    c.extend_from_slice(&rel.to_le_bytes());
    c.push(0xc3);
    emu.write(code, &c);
    (-POW_RANGE..=POW_RANGE)
        .map(|n| (n, emu.call_f64(code, 0, 0, &[n as u32])))
        .collect()
}

/// The LCG the fuzz draws from; a choice uses the high bits.
fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s >> 8
}

fn digits(s: &mut u32, n: u32) -> Vec<u8> {
    (0..n).map(|_| b'0' + (next(s) % 10) as u8).collect()
}

/// A number-shaped string: sign, integer digits, a point, fraction digits, an exponent and a tail,
/// each present or absent, with leading zeros and runs long enough to fold several chunks.
fn shaped(s: &mut u32) -> Vec<u8> {
    let mut v = Vec::new();
    match next(s) % 4 {
        0 => v.push(b'-'),
        1 => v.push(b'+'),
        _ => {}
    }
    if next(s).is_multiple_of(5) {
        v.extend(std::iter::repeat_n(b'0', (next(s) % 12) as usize));
    }
    let int_len = [0, 1, 2, 9, 10, 11, 19, 20, 30][(next(s) % 9) as usize];
    v.extend(digits(s, int_len));
    if !next(s).is_multiple_of(3) {
        v.push(b'.');
        let frac_len = [0, 1, 5, 19, 20, 21, 25, 40][(next(s) % 8) as usize];
        v.extend(digits(s, frac_len));
    }
    if next(s).is_multiple_of(3) {
        v.push(b"eE"[(next(s) % 2) as usize]);
        match next(s) % 4 {
            0 => v.push(b'+'),
            1 => v.push(b'-'),
            2 => v.extend_from_slice(b"+-"),
            _ => {}
        }
        let e_len = next(s) % 3;
        v.extend(digits(s, e_len));
    }
    if next(s).is_multiple_of(4) {
        v.push(b"x .e+-9\x80\xff"[(next(s) % 9) as usize]);
    }
    v
}

/// A short string over the characters the grammar distinguishes.
fn scrambled(s: &mut u32) -> Vec<u8> {
    let alphabet = b"0123456789.-+eE x\x80";
    let len = next(s) % 9;
    (0..len)
        .map(|_| alphabet[(next(s) as usize) % alphabet.len()])
        .collect()
}

/// `0x64aaa0` whole over the grammar's cases, every chunk and fraction regime, and a fuzz; the
/// table it builds compared cell by cell with `pow10_table`; a null pointer.
#[test]
fn sstrtofloat_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x64e850, 0, 1);
    let pows = pow10_oracle(&mut emu);
    let missing = Cell::new(false);
    let pow10 = |n: i32| -> f64 {
        pows.get(&n).copied().unwrap_or_else(|| {
            missing.set(true);
            f64::NAN
        })
    };
    let buf = emu.alloc(0x200);
    let run = |emu: &mut Emu, text: &[u8]| -> f64 {
        let mut z = text.to_vec();
        z.push(0);
        emu.write(buf, &z);
        emu.call_f64(0x64aaa0, 0, 0, &[buf as u32])
    };

    // The table is built on the first call, and the guard set.
    assert_eq!(emu.read_u32(0xc52ba0), 0, "the table guard starts clear");
    run(&mut emu, b"1");
    assert_eq!(emu.read_u32(0xc52ba0), 1, "the first call sets the guard");
    let table = pow10_table(pow10);
    for (r, row) in table.iter().enumerate() {
        for (k, &cell) in row.iter().enumerate() {
            let va = 0xc52560 + 8 * (10 * r + k) as u64;
            let got = f64::from_bits(u64::from_le_bytes(emu.read(va, 8).try_into().unwrap()));
            assert_eq!(got.to_bits(), cell.to_bits(), "table row {r} column {k}");
        }
    }

    // The grammar, with the values it gives.
    let p = |n: i32| pows[&n];
    let cases: &[(&[u8], f64)] = &[
        (b"", 0.0),
        (b" 12", 0.0),
        (b"+12", 0.0),
        (b"12abc", 12.0),
        (b"-", -0.0),
        (b"--5", -0.0),
        (b"0x10", 0.0),
        (b".5", table[0][5]),
        (b"-.5", -table[0][5]),
        (b"1.", 1.0),
        (b"1.e2", p(2)),
        (b"1e", p(0)),
        (b"1e+-2", p(-2)),
        (b"1e-2", p(-2)),
        (b"1E2", p(2)),
        (b"e5", 0.0),
        (b"1 e5", 1.0),
        (b"1e 5", p(0)),
    ];
    for &(text, want) in cases {
        let got = run(&mut emu, text);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "{:?}: {got} vs {want}",
            String::from_utf8_lossy(text)
        );
    }

    let mut inputs: Vec<Vec<u8>> = cases.iter().map(|c| c.0.to_vec()).collect();
    for len in 1..=40usize {
        inputs.push(vec![b'9'; len]);
        inputs.push(b"0.".iter().copied().chain(vec![b'7'; len]).collect());
    }
    inputs.push(b"429496729".to_vec());
    inputs.push(b"429496728".to_vec());
    inputs.push(b"4294967295".to_vec());
    inputs.push(b"00000000000000000000000000000000001".to_vec());
    inputs.push(b"0000000000000000000000000000000000429496730".to_vec());
    let mut s = 0x64aa_a0f1u32;
    for _ in 0..4000 {
        inputs.push(shaped(&mut s));
        inputs.push(scrambled(&mut s));
    }
    let mut checked = 0;
    for text in &inputs {
        missing.set(false);
        let want = str_to_float(text, &table, pow10);
        if missing.get() {
            continue;
        }
        let got = run(&mut emu, text);
        assert_eq!(
            got.to_bits(),
            want.to_bits(),
            "{:?}: {got} vs {want}",
            String::from_utf8_lossy(text)
        );
        checked += 1;
    }

    // A null pointer: error 0x57 and 0.0.
    emu.clear_calls();
    let got = emu.call_f64(0x64aaa0, 0, 0, &[0]);
    assert_eq!(got.to_bits(), 0.0f64.to_bits(), "null answers 0.0");
    let calls = emu.calls();
    assert_eq!(calls.len(), 1, "null sets one error");
    assert_eq!((calls[0].addr, calls[0].args[0]), (0x64e850, 0x57));
    eprintln!("sstrtofloat_whole: {checked} strings bit-exact, 200 table cells");
}

/// `0x64ac60` whole over signs, every length to 12 digits, the 32-bit wrap, stops and a fuzz.
#[test]
fn sstrtoint_whole() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.stub(0x64e850, 0, 1);
    let buf = emu.alloc(0x100);
    let mut inputs: Vec<Vec<u8>> = [
        &b""[..],
        b"-",
        b"+5",
        b" 5",
        b"--5",
        b"-0",
        b"2147483647",
        b"2147483648",
        b"-2147483648",
        b"4294967295",
        b"4294967296",
        b"99999999999",
        b"12x3",
        b"\x80",
    ]
    .iter()
    .map(|t| t.to_vec())
    .collect();
    for len in 1..=12usize {
        inputs.push(b"1234567890987".iter().copied().take(len).collect());
        inputs.push(std::iter::once(b'-').chain(vec![b'9'; len]).collect());
    }
    let mut s = 0x64ac_6011u32;
    for _ in 0..2000 {
        inputs.push(scrambled(&mut s));
    }
    for text in &inputs {
        let mut z = text.clone();
        z.push(0);
        emu.write(buf, &z);
        let got = emu.call(0x64ac60, 0, 0, &[buf as u32]) as i32;
        assert_eq!(got, str_to_int(text), "{:?}", String::from_utf8_lossy(text));
    }
    emu.clear_calls();
    assert_eq!(emu.call(0x64ac60, 0, 0, &[0]), 0, "null answers 0");
    assert_eq!(
        (emu.calls()[0].addr, emu.calls()[0].args[0]),
        (0x64e850, 0x57)
    );
    eprintln!("sstrtoint_whole: {} strings bit-exact", inputs.len());
}
