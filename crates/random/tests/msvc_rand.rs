//! The C runtime's `rand` (`0x7400e5`) run as the slice `0x7400ed` to `0x740106`, from `imul` to
//! `and eax,0x7fff`: the seed in `ecx`, in place of the `_getptd` load before it, and the result
//! and new seed read from `eax` and `ecx`. The `imul` and `add` bytes are checked first.

use benilla_re_difftest::{Emu, RegisterX86, Wow};
use benilla_re_random::msvc_rand::rand_step;

// 0x7400ed: imul ecx,ecx,0x343fd | 0x7400f3: add ecx,0x269ec3
const LCG_BYTES: [u8; 12] = [
    0x69, 0xc9, 0xfd, 0x43, 0x03, 0x00, 0x81, 0xc1, 0xc3, 0x9e, 0x26, 0x00,
];

#[test]
fn msvc_rand_lcg_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    assert_eq!(emu.read(0x7400ed, 12), LCG_BYTES, "LCG bytes @ 0x7400ed");
    let scratch = emu.alloc(0x40); // [eax+0x14], where _holdrand is stored
    let stack = emu.alloc(0x400);

    // One step from `seed`: the new seed and the result.
    let step = |emu: &mut Emu, seed: u32| -> (u32, i32) {
        emu.set_reg(RegisterX86::ECX, u64::from(seed));
        emu.set_reg(RegisterX86::EAX, scratch);
        emu.set_reg(RegisterX86::ESP, stack + 0x300);
        emu.run(0x7400ed, 0x740106); // to the ret
        let new_seed = emu.get_reg(RegisterX86::ECX) as u32;
        let result = (emu.get_reg(RegisterX86::EAX) as u32) as i32;
        (new_seed, result)
    };

    let mut total = 0u32;

    // The sequence from the default seed 1, as an unseeded `math.random` draws it.
    let mut seed = 1u32;
    for i in 0..20_000 {
        let (bn, br) = step(&mut emu, seed);
        let (wn, wr) = rand_step(seed);
        assert_eq!((bn, br), (wn, wr), "seq#{i} seed={seed:#x}");
        seed = bn;
        total += 1;
    }

    // Edge seeds and random ones.
    let mut f = 0x1357_9bdfu32;
    let mut probes = vec![0u32, 1, 0xffff_ffff, 0x8000_0000, 0x7fff_ffff, 0x0001_0000];
    for _ in 0..20_000 {
        f = f.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        probes.push(f);
    }
    for s in probes {
        let (bn, br) = step(&mut emu, s);
        let (wn, wr) = rand_step(s);
        assert_eq!((bn, br), (wn, wr), "seed={s:#x}");
        assert!(
            (0..=0x7fff).contains(&br),
            "result in [0,RAND_MAX] seed={s:#x}"
        );
        total += 1;
    }

    eprintln!("msvc_rand: {total} cases bit-exact vs WoW.exe @ 0x7400e5 (math.random LCG)");
}
