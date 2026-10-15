//! Storm's string hash for its hash tables (`SStrHashHT`, `0x64b3f0`): the name folded to upper
//! case with `/` read as `\`, then Bob Jenkins' `lookup2` hash (`bjhash`, `0x64b1d0`) with an
//! initial value of 0.

/// The hash of a name as `0x64b3f0` computes it: at most its first 1023 bytes, `a` to `z` raised
/// by 0x20 and `/` replaced by `\`, every other byte kept, then [`lookup2`] over them with
/// initial value 0.
pub fn str_hash_ht(name: &[u8]) -> u32 {
    let folded: Vec<u8> = name
        .iter()
        .take_while(|&&c| c != 0)
        .take(0x3ff)
        .map(|&c| match c {
            b'a'..=b'z' => c - 0x20,
            b'/' => b'\\',
            _ => c,
        })
        .collect();
    lookup2(&folded, 0)
}

/// Bob Jenkins' `lookup2` hash of `key` with `initval`, as `0x64b1d0` runs it: `a` and `b` start
/// at 0x9e3779b9 and `c` at `initval`; each 12-byte block adds three little-endian words and
/// mixes; the length is added to `c`, the last 0 to 11 bytes are added (`c` takes bytes 8 to 10
/// from its second byte up), and one more mix returns `c`.
pub fn lookup2(key: &[u8], initval: u32) -> u32 {
    let mut a: u32 = 0x9e37_79b9;
    let mut b: u32 = 0x9e37_79b9;
    let mut c: u32 = initval;
    let word = |k: &[u8]| u32::from_le_bytes([k[0], k[1], k[2], k[3]]);
    let mut k = key;
    while k.len() >= 12 {
        a = a.wrapping_add(word(&k[0..4]));
        b = b.wrapping_add(word(&k[4..8]));
        c = c.wrapping_add(word(&k[8..12]));
        mix(&mut a, &mut b, &mut c);
        k = &k[12..];
    }
    c = c.wrapping_add(key.len() as u32);
    let byte = |i: usize, shift: u32| u32::from(k[i]) << shift;
    let n = k.len();
    if n >= 11 {
        c = c.wrapping_add(byte(10, 24));
    }
    if n >= 10 {
        c = c.wrapping_add(byte(9, 16));
    }
    if n >= 9 {
        c = c.wrapping_add(byte(8, 8));
    }
    if n >= 8 {
        b = b.wrapping_add(byte(7, 24));
    }
    if n >= 7 {
        b = b.wrapping_add(byte(6, 16));
    }
    if n >= 6 {
        b = b.wrapping_add(byte(5, 8));
    }
    if n >= 5 {
        b = b.wrapping_add(byte(4, 0));
    }
    if n >= 4 {
        a = a.wrapping_add(byte(3, 24));
    }
    if n >= 3 {
        a = a.wrapping_add(byte(2, 16));
    }
    if n >= 2 {
        a = a.wrapping_add(byte(1, 8));
    }
    if n >= 1 {
        a = a.wrapping_add(byte(0, 0));
    }
    mix(&mut a, &mut b, &mut c);
    c
}

fn mix(a: &mut u32, b: &mut u32, c: &mut u32) {
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 13);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 8);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 13);
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 12);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 16);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 5);
    *a = a.wrapping_sub(*b).wrapping_sub(*c) ^ (*c >> 3);
    *b = b.wrapping_sub(*c).wrapping_sub(*a) ^ (*a << 10);
    *c = c.wrapping_sub(*a).wrapping_sub(*b) ^ (*b >> 15);
}
