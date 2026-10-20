//! `luaO_int2fb` (`0x6f5810`) and `computesizes` (`0x6fab00`), each run whole: neither calls
//! anything. `computesizes` reads `nums` through `ecx`, `ntotal` in `edx` and `narray` through
//! its first stack argument, and writes `nhash` and then `narray` through the two stack pointers.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_lua::{computesizes, luao_int2fb};

const INT2FB: u64 = 0x6f5810;
const COMPUTESIZES: u64 = 0x6fab00;
/// `nums` slots the harness gives `computesizes`: the 25 that `numuse` fills, zeros after, so a
/// walk past the last slice reads what the reimplementation reads.
const SLOTS: usize = 32;

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

#[test]
fn computesizes_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let nums_va = emu.alloc(4 * SLOTS as u64);
    let narray_va = emu.alloc(4);
    let nhash_va = emu.alloc(4);

    let mut checked = 0u32;
    let mut case = |emu: &mut Emu, nums: &[i32; SLOTS], ntotal: i32, narray: i32, ctx: &str| {
        for (i, v) in nums.iter().enumerate() {
            emu.write_u32(nums_va + 4 * i as u64, *v as u32);
        }
        emu.write_u32(narray_va, narray as u32);
        emu.write_u32(nhash_va, 0x5a5a_5a5a);
        emu.call(
            COMPUTESIZES,
            nums_va as u32,
            ntotal as u32,
            &[narray_va as u32, nhash_va as u32],
        );
        let got = (
            emu.read_u32(narray_va) as i32,
            emu.read_u32(nhash_va) as i32,
        );
        let want = computesizes(nums, ntotal, narray);
        assert_eq!(
            got, want,
            "{ctx}: nums={nums:?} ntotal={ntotal} narray={narray}"
        );
        checked += 1;
    };

    // Each slice alone, full (exactly half of its power of two: kept) and one key short (not).
    for i in 1..=24usize {
        let full = 1i32 << (i - 1);
        for (count, label) in [(full, "half"), (full - 1, "under half")] {
            let mut nums = [0i32; SLOTS];
            nums[i] = count;
            case(
                &mut emu,
                &nums,
                count + 3,
                count,
                &format!("slice {i} {label}"),
            );
            // With key 1 present the slice is one key further from its bound.
            nums[0] = 1;
            case(
                &mut emu,
                &nums,
                count + 4,
                count + 1,
                &format!("slice {i} {label} and key 1"),
            );
        }
    }
    // Small tables, every combination of the first five slices.
    let caps = [1i32, 1, 2, 4, 8];
    let mut counts = [0i32; 5];
    loop {
        let mut nums = [0i32; SLOTS];
        nums[..5].copy_from_slice(&counts);
        let narray: i32 = counts.iter().sum();
        case(
            &mut emu,
            &nums,
            narray + 2,
            narray,
            &format!("small {counts:?}"),
        );
        let mut k = 0;
        while k < 5 {
            counts[k] += 1;
            if counts[k] <= caps[k] {
                break;
            }
            counts[k] = 0;
            k += 1;
        }
        if k == 5 {
            break;
        }
    }
    // Random tables in numuse's domain: each slice at most its key count, narray their sum.
    let mut s = 0x6fab_0000u32;
    for n in 0..4000 {
        let mut nums = [0i32; SLOTS];
        let top = 1 + (lcg(&mut s) % 24) as usize;
        for (i, slot) in nums.iter_mut().enumerate().take(top + 1) {
            let cap = if i == 0 { 1 } else { 1u32 << (i - 1) };
            *slot = match lcg(&mut s) % 4 {
                0 => 0,
                1 => cap as i32,
                _ => (lcg(&mut s) % (cap + 1)) as i32,
            };
        }
        let narray: i32 = nums.iter().sum();
        let ntotal = narray + (lcg(&mut s) % 64) as i32;
        case(&mut emu, &nums, ntotal, narray, &format!("fuzz {n}"));
    }
    // Outside numuse's domain: negative counts, which `jle` skips, and narray above the sum.
    for n in 0..500 {
        let mut nums = [0i32; SLOTS];
        for slot in nums.iter_mut().take(25) {
            *slot = (lcg(&mut s) % 9) as i32 - 3;
        }
        let narray = (lcg(&mut s) % (1 << 20)) as i32;
        let ntotal = (lcg(&mut s) >> 2) as i32 - (1 << 28);
        case(&mut emu, &nums, ntotal, narray, &format!("domain {n}"));
    }
    eprintln!("computesizes: {checked} cases bit-exact vs WoW.exe @ 0x6fab00");
}
