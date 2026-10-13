//! The text the client's Lua makes of a number: `luaV_tostring` (`0x6f7c80`) formats with CRT
//! `sprintf` (`0x73fec4`) and `"%.14g"` (`0x871960`), MSVC's `%g`, which differs from C99's in four
//! ways, each pinned below by running `sprintf` itself:
//!
//! 1. the exponent always has three digits (`1e+020`): it is copied from the template `"e+000"`
//!    (`0x8001e0`) at `0x40b612` and the digits are added onto it (`0x40b645`, `0x40b65c`,
//!    `0x40b665`);
//! 2. `-0.0` prints `"0"`: `$I10_OUTPUT` clears the sign at `0x411778`;
//! 3. infinities and NaNs print `1.#INF`, `-1.#INF`, `1.#QNAN` and `-1.#IND` (`0x8003dc` to
//!    `0x8003f4`, chosen at `0x4117aa` to `0x4117f2`);
//! 4. it is not correctly rounded: `_fltout2` (`0x40ff90`) makes 17 digits half-up (`0x411930`)
//!    and `_fptostr` (`0x40fef9`) rounds that string to 14, half-up (`0x40ff3d`).
//!
//! `sprintf` is `__cdecl (char *buf, const char *fmt, double)`. `_cfltcvt_init` (`0x409a29`) must
//! run first: it fills `_cfltcvt_tab` (`0x82f3f8`), whose static entries all point at
//! `_amsg_exit(2)`, "floating point support not loaded". With it run, nothing is stubbed.

use benilla_re_difftest::{Emu, Wow};

/// `_cfltcvt_init`: fills `_cfltcvt_tab` at `0x82f3f8`.
const CFLTCVT_INIT: u64 = 0x409a29;
/// MSVC CRT `sprintf`, writing through a `FILE` on its stack (`_flag = 0x42`, `_cnt = 0x7fffffff`).
const SPRINTF: u64 = 0x73fec4;
/// `"%.14g"`, Lua 5.0's `lua_number2str` format.
const FMT_14G: u32 = 0x871960;

fn format_g14(wow: &Wow, v: f64) -> String {
    let mut emu = Emu::new(wow);
    emu.call(CFLTCVT_INIT, 0, 0, &[]);
    let buf = emu.alloc(64);
    emu.write(buf, &[0u8; 64]);
    let bits = v.to_bits();
    emu.call(
        SPRINTF,
        0,
        0,
        &[
            buf as u32,
            FMT_14G,
            (bits & 0xffff_ffff) as u32,
            (bits >> 32) as u32,
        ],
    );
    let raw = emu.read(buf, 64);
    let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).into_owned()
}

#[test]
fn msvc_g14_number_to_text() {
    let Some(wow) = Wow::load() else {
        return;
    };

    // Control: a plain integer comes back as its digits, so an empty result (the `_amsg_exit`
    // failure) cannot pass.
    assert_eq!(format_g14(&wow, 3.0), "3", "control: plain integer");

    let cases: &[(f64, &str)] = &[
        (0.0, "0"),
        (-0.0, "0"), // the sign is re-cleared at 0x411778
        (3.0, "3"),
        (0.8, "0.8"), // a non-integer keeps its decimals
        (-0.5, "-0.5"),
        (0.1, "0.1"),
        (0.1 + 0.2, "0.3"),
        (1.0 / 3.0, "0.33333333333333"),
        (1e13, "10000000000000"), // f-style: exp 13 < precision 14
        (1e14, "1e+014"),         // e-style: exp 14 >= precision 14 (0x40b7be/0x40b7c0)
        (1e15, "1e+015"),
        (1e17, "1e+017"),
        (1e20, "1e+020"), // THREE exponent digits, not "1e+20"
        (1e-4, "0.0001"), // f-style: exp -4 is not < -4 (0x40b7b9/0x40b7bc)
        (1e-5, "1e-005"),
        (1.5e-5, "1.5e-005"),
        (2147483648.0, "2147483648"),
        (4294967296.0, "4294967296"),
        (123456789012345678.0, "1.2345678901235e+017"),
        (1e308, "1e+308"),
        (1e-308, "1e-308"),
        (5e-324, "4.9406564584125e-324"),
        (f64::INFINITY, "1.#INF"),
        (f64::NEG_INFINITY, "-1.#INF"),
    ];
    for (v, want) in cases {
        assert_eq!(&format_g14(&wow, *v), want, "%.14g of {v:e}");
    }

    // NaNs, by bit pattern (Rust's `f64::NAN` is one particular quiet NaN).
    assert_eq!(
        format_g14(&wow, f64::from_bits(0x7ff8_0000_0000_0000)),
        "1.#QNAN"
    );
    assert_eq!(
        format_g14(&wow, f64::from_bits(0xfff8_0000_0000_0000)),
        "-1.#IND"
    );

    // Double rounding: a correctly rounded formatter gives "-0.11594045739606" and
    // "1.2345678901234e+14" for these two.
    assert_eq!(
        format_g14(&wow, f64::from_bits(0xbfbd_ae46_18cc_a433)),
        "-0.11594045739607",
        "double rounding: 17 digits half-up, then 14 half-up"
    );
    assert_eq!(
        format_g14(&wow, 123456789012345.0),
        "1.2345678901235e+014",
        "exact 14-digit tie rounds AWAY from zero here, not half-even"
    );

    // Longest output is 21 chars + NUL, so luaV_tostring's `char s[32]` cannot overflow.
    for (v, _) in cases {
        assert!(
            format_g14(&wow, *v).len() <= 21,
            "{v:e} fits the 32-byte buffer"
        );
    }
}
