//! Helpers shared by the object layer's tests: the LCG and the draws made from it, the float
//! compares that accept any NaN for any NaN, the special values every float input is probed
//! with, and the readers and builders the slices use.

#![allow(dead_code)]

use benilla_re_difftest::{Emu, RegisterX86};

/// A stack pointer inside the harness's stack, for slices that push.
pub const SLICE_ESP: u64 = 0x7008_0000;

/// One LCG step. Its low bits repeat quickly, so a choice uses the high bits.
pub fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *s
}

/// A random `f32` in `[lo, hi)` from the high bits.
pub fn frand(s: &mut u32, lo: f32, hi: f32) -> f32 {
    let m = f64::from(next(s) >> 8) / f64::from(1u32 << 24);
    (f64::from(lo) + m * (f64::from(hi) - f64::from(lo))) as f32
}

/// Any `f32`, from random bits: subnormals, infinities and NaN included.
pub fn frand_bits(s: &mut u32) -> f32 {
    f32::from_bits(next(s))
}

/// Asserts two floats have the same bits, or are both NaN.
pub fn assert_f32(binary: f32, ours: f32, ctx: &str) {
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

/// Asserts two doubles have the same bits, or are both NaN.
pub fn assert_f64(binary: f64, ours: f64, ctx: &str) {
    if binary.is_nan() && ours.is_nan() {
        return;
    }
    assert_eq!(
        binary.to_bits(),
        ours.to_bits(),
        "{ctx}: binary={:#018x} ours={:#018x}",
        binary.to_bits(),
        ours.to_bits()
    );
}

/// The `f32` one step above `v` in magnitude order of its bits.
pub fn next_up(v: f32) -> f32 {
    f32::from_bits(v.to_bits().wrapping_add(1))
}

/// The `f32` one step below `v` in bit order.
pub fn next_down(v: f32) -> f32 {
    f32::from_bits(v.to_bits().wrapping_sub(1))
}

/// The values every float input is probed with: both zeros, NaN, both infinities, the largest and
/// smallest finite values, a subnormal, and `±1`. A threshold `t` goes in `edges` and brings
/// itself, its negation and a step either side of each.
pub fn specials(edges: &[f32]) -> Vec<f32> {
    let mut v = vec![
        0.0,
        -0.0,
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        -f32::from_bits(1),
        1.0,
        -1.0,
    ];
    for &t in edges {
        for e in [t, -t] {
            v.push(e);
            v.push(next_up(e));
            v.push(next_down(e));
        }
    }
    v
}

/// [`specials`] without the infinities and without magnitudes past about 1e15, for inputs an
/// angle wrap sees. An infinity sends the C runtime's `fmod` to its error path, which needs state
/// the start-up sets and the emulator does not run, and past about 1e15 the emulator's partial
/// `fprem` rounds in `f64` where the CPU is exact.
pub fn specials_wrap(edges: &[f32]) -> Vec<f32> {
    let mut v = specials(edges);
    v.retain(|x| x.is_nan() || x.abs() < 1.0e15);
    v
}

/// A float of mixed magnitude, `m * 2^e` with `m` in `[-1, 1)` and `e` in `[-20, 20]`, so that the
/// three squares of a distance differ by many bits and the order of the sum shows.
pub fn mixed(s: &mut u32) -> f32 {
    let m = frand(s, -1.0, 1.0);
    let e = (next(s) >> 8) % 41;
    m * 2f32.powi(e as i32 - 20)
}

/// A point of random kind: full-range bits, a coordinate of world size, or mixed magnitude.
pub fn point(s: &mut u32, i: u32) -> [f32; 3] {
    match i % 3 {
        0 => [frand_bits(s), frand_bits(s), frand_bits(s)],
        1 => [
            frand(s, -100.0, 100.0),
            frand(s, -100.0, 100.0),
            frand(s, -100.0, 100.0),
        ],
        _ => [mixed(s), mixed(s), mixed(s)],
    }
}

/// Bit equality of two `f64`, any NaN equal to any NaN.
pub fn same_f64(a: f64, b: f64) -> bool {
    (a.is_nan() && b.is_nan()) || a.to_bits() == b.to_bits()
}

/// The `f64` at `va`.
pub fn read_f64(e: &Emu, va: u64) -> f64 {
    f64::from_le_bytes(e.read(va, 8).try_into().unwrap())
}

/// Replaces the three-byte instruction at `va` with `fstp qword [ebp-0x40]`, so a slice that ends
/// after it leaves the `f64` the stack held there.
pub fn patch_fstp_qword(e: &mut Emu, va: u64) {
    e.write(va, &[0xdd, 0x5d, 0xc0]);
}

/// The AH byte of EAX after `fnstsw ax`.
pub fn ah(e: &Emu) -> u8 {
    ((e.get_reg(RegisterX86::EAX) >> 8) & 0xff) as u8
}

/// The parity flag `test ah,mask` leaves: set when the masked byte has an even number of bits.
pub fn parity_even(masked: u8) -> bool {
    masked.count_ones().is_multiple_of(2)
}

/// Replaces the function at `fn_va` with `fld dword [slot]; ret n` and returns the slot, so a
/// slice's call leaves a chosen value in `st0`.
pub fn install_st0_stub(e: &mut Emu, fn_va: u64, n: u16) -> u64 {
    let slot = e.alloc(0x10);
    let mut bytes = vec![0xd9u8, 0x05];
    bytes.extend_from_slice(&(slot as u32).to_le_bytes());
    if n == 0 {
        bytes.push(0xc3);
    } else {
        bytes.push(0xc2);
        bytes.extend_from_slice(&n.to_le_bytes());
    }
    e.write(fn_va, &bytes);
    slot
}

/// A read-side `CDataStore` over `buf`: the vtable word, the data pointer, the base offset, the
/// chunk length, the total size and the read cursor. Returns the store and the buffer addresses.
pub fn make_read_store(e: &mut Emu, buf: &[u8]) -> (u64, u64) {
    let buf_va = e.alloc(buf.len().max(1) as u64);
    e.write(buf_va, buf);
    let store_va = e.alloc(0x18);
    let n = buf.len() as u32;
    for (i, v) in [0x1u32, buf_va as u32, 0, n, n, 0].iter().enumerate() {
        e.write_u32(store_va + 4 * i as u64, *v);
    }
    (store_va, buf_va)
}

/// A write-side `CDataStore` with `cap` bytes of room: the chunk length is `cap` and the write
/// cursor 0. Returns the store and the buffer addresses.
pub fn make_write_store(e: &mut Emu, cap: u32) -> (u64, u64) {
    let buf_va = e.alloc(u64::from(cap));
    let store_va = e.alloc(0x18);
    for (i, v) in [0x1u32, buf_va as u32, 0, cap, 0, 0].iter().enumerate() {
        e.write_u32(store_va + 4 * i as u64, *v);
    }
    (store_va, buf_va)
}
