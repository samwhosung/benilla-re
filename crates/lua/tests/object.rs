//! `luaO_int2fb` (`0x6f5810`), run whole: it calls nothing.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lua::luao_int2fb;

const INT2FB: u64 = 0x6f5810;

fn lcg(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

#[test]
fn luao_int2fb_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let mut inputs: Vec<u32> = (0..=0x1_0000).collect();
    for bit in 0..32 {
        let p = 1u32 << bit;
        inputs.extend([p.wrapping_sub(1), p, p.wrapping_add(1)]);
    }
    inputs.extend([
        0xffff_fffd,
        0xffff_fffe,
        0xffff_ffff,
        0x7fff_ffff,
        0x8000_0001,
    ]);
    let mut s = 0x6f58_1000u32;
    for _ in 0..20_000 {
        let x = lcg(&mut s);
        inputs.push(x >> (lcg(&mut s) % 32));
    }
    for &x in &inputs {
        // `edx` holds junk: the body never reads it.
        let got = emu.call(INT2FB, x, 0xdead_beef, &[]);
        let want = luao_int2fb(x);
        assert_eq!(got, want, "int2fb({x:#x}): binary={got:#x} ours={want:#x}");
    }
    // The decode `OP_NEWTABLE` applies: 9 items size an array of 10.
    let b = luao_int2fb(9);
    assert_eq!((b & 7) << (b >> 3), 10);
    eprintln!(
        "luao_int2fb: {} inputs bit-exact vs WoW.exe @ 0x6f5810",
        inputs.len()
    );
}
