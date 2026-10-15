//! The static initializers of the network files (`systems/net/layout.md#static-initializers`).

use benilla_re_difftest::{Emu, Wow};
use benilla_re_net::{
    release, set_exit_flag, Init, EXIT_FLAG, INITIALIZERS, RELEASES, SHARED_COUNTER,
};

/// The empty functions the counters tail-call: `0x5b71c0` on the first count, `0x5b71d0` on the
/// last release. Each is a lone `ret`, so the stub that logs the call changes nothing.
const EMPTY_INIT: u64 = 0x5b71c0;
const EMPTY_TERM: u64 = 0x5b71d0;

/// `.data`, with its zero-filled tail: every global the initializers write is in it.
const DATA: (u64, usize) = (0x827000, 0xcfa154 - 0x827000);

/// The code of the three runs of network files (`systems/net/layout.md#object-files`).
const RUNS: &[(u32, u32)] = &[
    (0x536eb0, 0x5388a0),
    (0x5a9e00, 0x5be840),
    (0x5d1420, 0x5d3f20),
];

/// The initializer bodies of those files that compute nothing: copies, stored constants, the
/// command tables, critical sections and the random pool. They owe no test; they are listed so
/// that a computing body missing from [`INITIALIZERS`] fails [`initializer_constants`].
const NON_COMPUTING: &[u32] = &[
    0x536ec0, 0x5371f0, 0x5380d0, 0x538400, 0x5a9e10, 0x5aa150, 0x5aa280, 0x5ac5b0, 0x5ac890,
    0x5af820, 0x5b0270, 0x5b03a0, 0x5b1030, 0x5b03c0, 0x5b1160, 0x5b15c0, 0x5b16f0, 0x5b1a20,
    0x5b3530, 0x5b3860, 0x5b3a40, 0x5b48f0, 0x5b4bd0, 0x5b4cb0, 0x5b4cf0, 0x5b7860, 0x5b7b30,
    0x5b7bf0, 0x5b7f20, 0x5b8000, 0x5b87b0, 0x5b88a0, 0x5bc980, 0x5bd590, 0x5be200, 0x5d3b30,
];

/// The bodies the initializer table reaches in the network files, in table order: each slot that
/// points into a run holds a five-byte `jmp` or `call` to its body.
fn table_bodies(e: &Emu) -> Vec<u32> {
    let mut out = Vec::new();
    for slot in (0x827000u64..0x82e030).step_by(4) {
        let thunk = e.read_u32(slot);
        if !RUNS.iter().any(|&(lo, hi)| (lo..hi).contains(&thunk)) {
            continue;
        }
        let code = e.read(u64::from(thunk), 5);
        assert!(
            code[0] == 0xe9 || code[0] == 0xe8,
            "slot {slot:#x}: thunk {thunk:#x}"
        );
        let rel = u32::from_le_bytes(code[1..5].try_into().unwrap());
        out.push(thunk.wrapping_add(5).wrapping_add(rel));
    }
    out
}

fn same_f32(a: u32, b: u32) -> bool {
    a == b || (f32::from_bits(a).is_nan() && f32::from_bits(b).is_nan())
}

fn emu(wow: &Wow) -> Emu<'_> {
    let mut e = Emu::new(wow);
    e.stub(EMPTY_INIT, 0, 0);
    e.stub(EMPTY_TERM, 0, 0);
    e
}

/// Runs every computing initializer body in the order of the initializer table, on the image's
/// own constants, and compares each store with [`Init::apply`] over the same memory: the
/// constants each file holds, the quotients that read a doubling stored above them, and the shared
/// count as it climbs from its file value. Over the whole run, every byte of `.data` that changed
/// lies in a store the table names, and the table holds every body the initializer table reaches
/// in these files but the [`NON_COMPUTING`] ones.
#[test]
fn initializer_constants() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = emu(&wow);
    let bodies = table_bodies(&e);
    let computing: Vec<u32> = bodies
        .iter()
        .copied()
        .filter(|b| !NON_COMPUTING.contains(b))
        .collect();
    let ours: Vec<u32> = INITIALIZERS.iter().map(|&(b, _)| b).collect();
    assert_eq!(
        bodies.len(),
        276,
        "initializer slots into the network files"
    );
    assert_eq!(computing, ours, "the computing bodies, in table order");
    let before = e.read(DATA.0, DATA.1);
    let mut named = std::collections::HashSet::new();
    for &(body, init) in INITIALIZERS {
        let want = init.apply(|va| e.read_u32(u64::from(va)));
        e.clear_calls();
        e.call(u64::from(body), 0, 0, &[]);
        for &(va, v) in &want.stores {
            let got = e.read_u32(u64::from(va));
            assert!(
                same_f32(got, v),
                "{body:#x}: [{va:#x}] binary {got:#010x}, ours {v:#010x}"
            );
            for b in 0..4 {
                named.insert(u64::from(va) + b);
            }
        }
        let called = e.calls().iter().any(|c| c.addr == EMPTY_INIT);
        assert_eq!(called, want.calls_empty, "{body:#x}: empty-function call");
    }
    let after = e.read(DATA.0, DATA.1);
    for (i, (a, b)) in before.iter().zip(&after).enumerate() {
        let va = DATA.0 + i as u64;
        assert!(
            a == b || named.contains(&va),
            "byte {va:#x} changed ({a:#04x} to {b:#04x}) outside the table's stores"
        );
    }
}

/// Every float input regime: both zeros, NaN, the infinities, the extremes, the smallest normal
/// and subnormal, `±1`, and values whose square or quotient overflows or underflows `f32`.
fn specials() -> Vec<u32> {
    let mut v: Vec<f32> = vec![
        0.0,
        -0.0,
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(1),
        -f32::from_bits(1),
        1.0,
        -1.0,
        3.0,
        0.1,
        1.0e19,
        1.84467e19,
        1.0e-19,
        1.0e-23,
        std::f32::consts::PI,
        -std::f32::consts::TAU,
    ];
    let mut s: u32 = 0x5eed_1234;
    for _ in 0..24 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        v.push(f32::from_bits(s));
    }
    v.into_iter().map(f32::to_bits).collect()
}

/// Runs each float body over [`specials`] written into its operands (for a quotient, every pair),
/// and compares the stored result with [`Init::apply`]. The operands are put back after each body.
#[test]
fn initializer_formulas() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = emu(&wow);
    let xs = specials();
    for &(body, init) in INITIALIZERS {
        let (ins, dst): (Vec<u32>, u32) = match init {
            Init::Square { src, dst } | Init::Double { src, dst } => (vec![src], dst),
            Init::Quotient { num, den, dst } => (vec![num, den], dst),
            Init::CountUnit { .. } | Init::ListHead { .. } => continue,
        };
        let saved: Vec<u32> = ins.iter().map(|&a| e.read_u32(u64::from(a))).collect();
        let cases: Vec<Vec<u32>> = if ins.len() == 1 {
            xs.iter().map(|&x| vec![x]).collect()
        } else {
            xs.iter()
                .flat_map(|&a| xs.iter().map(move |&b| vec![a, b]))
                .collect()
        };
        for case in cases {
            for (&a, &v) in ins.iter().zip(&case) {
                e.write_u32(u64::from(a), v);
            }
            let want = init.apply(|va| e.read_u32(u64::from(va)));
            e.call(u64::from(body), 0, 0, &[]);
            let got = e.read_u32(u64::from(dst));
            assert!(
                same_f32(got, want.stores[0].1),
                "{body:#x} on {case:#010x?}: binary {got:#010x}, ours {:#010x}",
                want.stores[0].1
            );
        }
        for (&a, &v) in ins.iter().zip(&saved) {
            e.write_u32(u64::from(a), v);
        }
    }
}

/// The shared count: each [`Init::CountUnit`] body and each [`RELEASES`] function from the counts
/// 0, 1 and 2, the sign boundary and the wrap, against [`Init::apply`] and [`release`], with the
/// tail call to the empty function on the first count and the last release only.
#[test]
fn initializer_counters() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = emu(&wow);
    let counts = [
        0u32,
        1,
        2,
        3,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_fffe,
        0xffff_ffff,
    ];
    let c = u64::from(SHARED_COUNTER);
    for &(body, init) in INITIALIZERS {
        if !matches!(init, Init::CountUnit { .. }) {
            continue;
        }
        for &n in &counts {
            e.write_u32(c, n);
            let want = init.apply(|va| if va == SHARED_COUNTER { n } else { 0 });
            e.clear_calls();
            e.call(u64::from(body), 0, 0, &[]);
            assert_eq!(e.read_u32(c), want.stores[0].1, "{body:#x} from {n:#x}");
            let called = e.calls().iter().any(|k| k.addr == EMPTY_INIT);
            assert_eq!(called, want.calls_empty, "{body:#x} from {n:#x}: call");
        }
    }
    for &f in RELEASES {
        for &n in &counts {
            e.write_u32(c, n);
            let (left, last) = release(n);
            e.clear_calls();
            e.call(u64::from(f), 0, 0, &[]);
            assert_eq!(e.read_u32(c), left, "{f:#x} from {n:#x}");
            let called = e.calls().iter().any(|k| k.addr == EMPTY_TERM);
            assert_eq!(called, last, "{f:#x} from {n:#x}: call");
        }
    }
}

/// `0x5aa2b0` from every value of its flag byte: bit 0 set, the other bits kept, and the bytes
/// beside it untouched.
#[test]
fn exit_flag() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut e = emu(&wow);
    let at = u64::from(EXIT_FLAG);
    for v in 0..=255u8 {
        e.write(at - 1, &[0xa5, v, 0x5a]);
        e.call(0x5aa2b0, 0, 0, &[]);
        assert_eq!(
            e.read(at - 1, 3),
            vec![0xa5, set_exit_flag(v), 0x5a],
            "flag {v:#04x}"
        );
    }
}
