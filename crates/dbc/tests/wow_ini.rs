//! The `Wow.ini` integer read against the exe (`systems/dbc.md`, Wow.ini values): `ProfileParseInt`
//! (`0x445470`) runs whole, `SStrToInt` (`0x64ac60`) as the exe has it. The values cover the quoted
//! form with every length to five and a closing quote or a NUL at each place, bytes above 0x7f,
//! signs, digits running past 32 bits, and leading spaces and plus signs, which `SStrToInt` does not
//! read.

use benilla_re_dbc::profile::parse_int;
use benilla_re_difftest::{Emu, Wow};

const PARSE_INT: u64 = 0x445470;

struct Lcg(u32);
impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        self.0 >> 8
    }
}

/// A value of up to seven bytes drawn from a set that reaches every branch.
fn value(g: &mut Lcg) -> Vec<u8> {
    const BYTES: &[u8] = b"''''0123456789-+ aZ\x7f\x80\xff";
    let n = (g.next() % 8) as usize;
    (0..n)
        .map(|_| BYTES[(g.next() as usize) % BYTES.len()])
        .collect()
}

#[test]
fn wow_ini_value_int_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = Emu::new(&wow);
    let buf = e.alloc(0x40);
    let mut cases: Vec<Vec<u8>> = [
        &b"3"[..],
        b"-2",
        b"0",
        b"",
        b"'",
        b"''",
        b"'a",
        b"'ab'",
        b"'abcd'",
        b"'abcde",
        b"'\xff\xfe\xfd\xfc",
        b"4294967296",
        b"-2147483648",
        b"99999999999",
        b" 5",
        b"+5",
        b"12abc",
        b"--1",
    ]
    .iter()
    .map(|v| v.to_vec())
    .collect();
    let mut g = Lcg(0x4454_7000);
    for _ in 0..20_000 {
        cases.push(value(&mut g));
    }
    for v in &cases {
        let mut bytes = v.clone();
        bytes.push(0);
        e.write(buf, &bytes);
        let bin = e.call(PARSE_INT, buf as u32, 0, &[]) as i32;
        assert_eq!(bin, parse_int(v), "{:?}", String::from_utf8_lossy(v));
    }
    eprintln!(
        "wow_ini_value_int: {} cases bit-exact vs WoW.exe @ 0x445470",
        cases.len()
    );
}
