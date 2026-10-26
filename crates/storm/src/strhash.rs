//! Storm's string hash for its hash tables (`SStrHashHT`, `0x64b3f0`): the name folded to upper
//! case with `/` read as `\`, then Bob Jenkins' `lookup2` hash (`bjhash`, `0x64b1d0`) with an
//! initial value of 0. And `SStrHash` (`0x64af90`), the seeded hash over a table of sixteen dwords
//! that several of the client's caches key names by.

/// The sixteen dwords `0x64af90` reads at `0x80e4e0`, indexed by a byte's high and low nibbles.
const STR_HASH_TABLE: [u32; 16] = [
    0x486e_26ee,
    0xdcaa_16b3,
    0xe191_8eef,
    0x202d_afdb,
    0x341c_7dc7,
    0x1c36_5303,
    0x40ef_2d37,
    0x65fd_5e49,
    0xd605_7177,
    0x904e_ce93,
    0x1c38_024f,
    0x98fd_323b,
    0xe306_1ae7,
    0xa39b_0fa1,
    0x9797_f25f,
    0xe444_4563,
];

/// `SStrHash(text, flags, seed)` as `0x64af90` computes it, `None` standing for a null `text`
/// (which sets error 0x57 and returns 0). `s1` starts at the seed, or 0x7fed7fed for a seed of 0,
/// and `s2` at 0xeeeeeeee. Flag bit 0 clear folds each byte first, `a` to `z` down by 0x20 and `/`
/// to `\`. Per byte `c` up to the first NUL: `s1 = (s1 + s2) ^ (T[c >> 4] - T[c & 15])`, then
/// `s2 = s2 * 33 + c + s1 + 3`, all modulo 2^32. A result of 0 is returned as 1.
pub fn str_hash(text: Option<&[u8]>, flags: u32, seed: u32) -> u32 {
    let Some(text) = text else {
        return 0;
    };
    let mut s1: u32 = if seed == 0 { 0x7fed_7fed } else { seed };
    let mut s2: u32 = 0xeeee_eeee;
    for &raw in text.iter().take_while(|&&c| c != 0) {
        let c = if flags & 1 != 0 {
            raw
        } else {
            match raw {
                b'a'..=b'z' => raw - 0x20,
                b'/' => b'\\',
                _ => raw,
            }
        };
        let mix =
            STR_HASH_TABLE[usize::from(c >> 4)].wrapping_sub(STR_HASH_TABLE[usize::from(c & 15)]);
        s1 = s1.wrapping_add(s2) ^ mix;
        s2 = s2
            .wrapping_mul(33)
            .wrapping_add(u32::from(c))
            .wrapping_add(s1)
            .wrapping_add(3);
    }
    if s1 == 0 {
        1
    } else {
        s1
    }
}

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
