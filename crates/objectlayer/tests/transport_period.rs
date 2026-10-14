//! The period of a moving transport, from the client's own path builder. The test runs `0x5f4cc0`
//! on a `TaxiPathNode.dbc` and halts at `0x5f4f47`, after the finalising loop, where the handler's
//! period (`+0x3c`, ms) is complete and the registration tail has not run
//! (`systems/objectlayer/transports.md#the-ship-path`). The Catmull-Rom segment lengths, the
//! cumulative knot sums, the constant-acceleration time of each span with its `__ftol`, the leg
//! sums and the stop delays all run as the client's bytes; nothing is transcribed.
//!
//! Each of nine ships' periods equals the one a live server reported:
//! - a segment's length is the sum of 20 sub-chords (`0x453760`, `t = 0.05 k`) in `f32`, on the
//!   position basis `0xb05e10`; the cumulative parameter is the `f64` sum of the `f32` lengths
//!   (`0x453300`), the segment mode `[spline+0x28]` being 1;
//! - a span is a run between two stops, a leg's ends included: its time is the constant-acceleration
//!   solve over the parameter delta, times 1000, rounded half away from zero and truncated by
//!   `__ftol` to whole milliseconds, per span;
//! - a leg lasts the sum of its spans, and the period is the sum of the legs plus the stop delays
//!   times 1000, each stop counted once;
//! - the basis tables are filled at start-up by `0x453fa0` (position) and `0x454080` (tangent), which
//!   the test runs first.
//!
//! The file is a fixture: `BENILLA_RE_TAXIPATHNODE_DBC` names an extracted `TaxiPathNode.dbc`, and
//! the test skips without it. It is read as 36-byte records and published through the storage
//! globals `[0xc0d6a4]` (base) and `[0xc0d6a8]` (count). The game object template's `data0` to
//! `data2` (path, speed, acceleration) are planted at `GO+0x214` for `0x621b00` and `0x5f8150`, with
//! the type 15 at `fields+0x3c`. The allocator (`0x6462e0`, `0x646320`, `0x646430`) is the harness's
//! arena, and the handler's registration gate `0x5f4970` returns at once.

use benilla_re_difftest::{fixture, Emu, RegisterX86, Wow};

const BUILDER: u64 = 0x5f4cc0;
const HALT: u64 = 0x5f4f47;
const DBC_BASE_G: u64 = 0x00c0_d6a4;
const DBC_COUNT_G: u64 = 0x00c0_d6a8;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SMEM_FREE: u64 = 0x646430;
const MO_TRANSPORT_VTABLE: u32 = 0x80b798;
const STACK_TOP: u64 = 0x700f_f000;

/// The path id, the speed (`data1`), the acceleration (`data2`) and the period a server reported.
const CASES: &[(u32, u32, u32, i64)] = &[
    (241, 30, 1, 350818),
    (285, 30, 1, 303463),
    (292, 30, 1, 329313),
    (293, 30, 1, 316251),
    (295, 30, 1, 295579),
    (301, 30, 1, 333044),
    (302, 30, 1, 356284),
    (303, 30, 1, 317040),
    (436, 1, 1, 1208014),
];

fn compute_period(wow: &Wow, recs: &[u8], rc: u32, path: u32, speed: u32, accel: u32) -> i64 {
    let mut e = Emu::new(wow);

    let dbc_va = e.alloc(recs.len() as u64);
    e.write(dbc_va, recs);
    e.write_u32(DBC_BASE_G, dbc_va as u32);
    e.write_u32(DBC_COUNT_G, rc);

    let go = e.alloc(0x300);
    let fields = e.alloc(0x100);
    e.write_u32(go + 0x110, fields as u32);
    e.write_u32(fields + 0x3c, 15);
    let datarec = e.alloc(0x100);
    e.write_u32(go + 0x214, datarec as u32);
    e.write_u32(datarec + 0x1c, path);
    e.write_u32(datarec + 0x20, speed);
    e.write_u32(datarec + 0x24, accel);

    let handler = e.alloc(0x100);
    e.write_u32(handler, MO_TRANSPORT_VTABLE);
    e.write_u32(handler + 4, go as u32);

    let arena = e.alloc(0x80_0000);
    e.stub_alloc(SMEM_ALLOC, arena, 0x2000, 4);
    e.stub_realloc(SMEM_REALLOC, 5);
    e.stub(SMEM_FREE, 0, 4);
    e.stub(0x5f4970, 0, 0);

    e.call(0x453fa0, 0, 0, &[]);
    e.call(0x454080, 0, 0, &[]);

    e.set_reg(RegisterX86::ECX, handler);
    e.set_reg(RegisterX86::ESP, STACK_TOP);
    e.run(BUILDER, HALT);
    i64::from(e.read_u32(handler + 0x3c))
}

#[test]
fn transport_period_9_paths() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let Some(dbc) = fixture("BENILLA_RE_TAXIPATHNODE_DBC") else {
        return;
    };
    assert_eq!(&dbc[0..4], b"WDBC", "not a WDBC file");
    let rc = u32::from_le_bytes(dbc[4..8].try_into().unwrap());
    let rs = u32::from_le_bytes(dbc[12..16].try_into().unwrap());
    assert_eq!(rs, 36, "TaxiPathNode record size");
    let recs = &dbc[20..20 + (rc as usize) * 36];

    let mut all_ok = true;
    for &(p, sp, ac, expected) in CASES {
        let got = compute_period(&wow, recs, rc, p, sp, ac);
        let delta = got - expected;
        eprintln!("path {p:>4}: computed={got:>8}  expected={expected:>8}  delta={delta:+}");
        if delta != 0 {
            all_ok = false;
        }
    }
    assert!(
        all_ok,
        "some paths differ from the reported periods (see the deltas above)"
    );
}
