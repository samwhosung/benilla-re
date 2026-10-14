//! `CLayoutFrame::ApplyDragDelta` (`0x768710`) run from the exe (`systems/ui/resizebounds.md#the-drag`).
//!
//! The function is a `__thiscall` that reads the frame's size through the vtable getters at `+0x1c`
//! and `+0x20`, runs a nine-way jump table on the grip, and hands the result to the resize
//! `0x768430` (`ret 0x14`). The test replaces the two getters with `fld` islands and the resize and
//! the child notification `0x7680e0` with stubs that log their arguments, and compares the logged
//! size and difference, or for grip 4 the child's words, with `apply_drag_delta`. A bound of 0 means
//! none. Every draw comes off the high bits of an LCG, and each case of the jump table is tallied
//! and required to be hit.
#![allow(clippy::float_cmp, clippy::too_many_arguments)]

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::resize_bounds::{apply_drag_delta as drag_resolve, DragResolve};

const APPLY: u64 = 0x768430; // FUN_00768430 (the apply; thiscall, ret 0x14)

const CHILD_NOTIFY: u64 = 0x7680e0; // FUN_007680e0 (case-4 follow-up; thiscall, ret 0x4)

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

/// f32 in `[lo, hi)` from the LCG high bits.
fn rng(s: &mut u32, lo: f32, hi: f32) -> f32 {
    lo + (next(s) >> 8) as f32 / (1u32 << 24) as f32 * (hi - lo)
}

/// A clamp bound: ~40% the `0.0` no-clamp sentinel, else in `[lo, hi)`.
fn bound(s: &mut u32, lo: f32, hi: f32) -> f32 {
    if (next(s) >> 8) % 5 < 2 {
        0.0
    } else {
        rng(s, lo, hi)
    }
}

/// `fld dword [cell]; ret` island — a per-case `st0` oracle for a niladic thiscall getter.
fn fld_stub(emu: &mut Emu, cell: u64) -> u64 {
    let addr = emu.alloc(8);
    let mut code = vec![0xd9u8, 0x05];
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(addr, &code);
    addr
}

/// `mov eax, ds:[cell]; ret` island — a per-case predicate oracle.
fn mov_eax_stub(emu: &mut Emu, cell: u64) -> u64 {
    let addr = emu.alloc(8);
    let mut code = vec![0xa1u8];
    code.extend_from_slice(&(cell as u32).to_le_bytes());
    code.push(0xc3);
    emu.write(addr, &code);
    addr
}

fn eqf(bin: u32, mine: f32, ctx: &str) {
    assert_eq!(
        bin,
        mine.to_bits(),
        "{ctx}: binary {:#010x} ({}) vs rust {:#010x} ({mine})",
        bin,
        f32::from_bits(bin),
        mine.to_bits()
    );
}

#[test]
fn geo_768710() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let cx_cell = emu.alloc(4);
    let cy_cell = emu.alloc(4);
    let cx_get = fld_stub(&mut emu, cx_cell);
    let cy_get = fld_stub(&mut emu, cy_cell);
    let vtable = emu.alloc(0x28);
    emu.write_u32(vtable + 0x1c, cx_get as u32);
    emu.write_u32(vtable + 0x20, cy_get as u32);
    let this = emu.alloc(0x80);
    emu.write_u32(this, vtable as u32);
    // case-4 child: vtable[+0xc] predicate + the +0x4/+0x8 accumulators.
    let pred_cell = emu.alloc(4);
    let pred = mov_eax_stub(&mut emu, pred_cell);
    let child_vt = emu.alloc(0x10);
    emu.write_u32(child_vt + 0xc, pred as u32);
    let child = emu.alloc(0x10);
    emu.write_u32(child, child_vt as u32);
    emu.stub(APPLY, 0, 5); // capture (arg0, cx, cy, dx, dy)
    emu.stub(CHILD_NOTIFY, 0, 1); // case-4 follow-up

    struct C {
        cx0: f32,
        cy0: f32,
        dx: f32,
        dy: f32,
        id: u32,
        scale: f32,
        min_w: f32,
        max_w: f32,
        min_h: f32,
        max_h: f32,
        child_present: bool,
        pred: bool,
        c4: f32,
        c8: f32,
    }
    let base = |id: u32| C {
        cx0: 150.0,
        cy0: 120.0,
        dx: 40.0,
        dy: -30.0,
        id,
        scale: 1.5,
        min_w: 0.0,
        max_w: 0.0,
        min_h: 0.0,
        max_h: 0.0,
        child_present: false,
        pred: false,
        c4: 0.0,
        c8: 0.0,
    };
    let mut cases: Vec<C> = Vec::new();
    // each anchor case 0..8, no clamp (sentinels).
    for id in 0..9u32 {
        cases.push(base(id));
    }
    // case-4 variants: present+pred (accumulate), present+!pred (fallthrough), absent (fallthrough).
    cases.push(C {
        child_present: true,
        pred: true,
        c4: 12.0,
        c8: -7.0,
        ..base(4)
    });
    cases.push(C {
        child_present: true,
        pred: false,
        c4: 5.0,
        c8: 9.0,
        ..base(4)
    });
    cases.push(C {
        child_present: false,
        ..base(4)
    });
    // clamp-active probes that force a pin on each clamp kind.
    cases.push(C {
        cx0: 100.0,
        dx: 300.0,
        min_w: 20.0,
        max_w: 0.0,
        ..base(0)
    }); // cx=100-300/1.5<20 -> rebate-min X; cy clamp via min_h
    cases.push(C {
        cy0: 100.0,
        dy: -300.0,
        min_h: 0.0,
        max_h: 200.0,
        ..base(1)
    }); // cy high -> rebate-max Y
    cases.push(C {
        cx0: 100.0,
        dx: -300.0,
        max_w: 180.0,
        ..base(6)
    }); // X rebate-max, Y pin
    cases.push(C {
        cx0: 100.0,
        cy0: 100.0,
        dx: 300.0,
        dy: 300.0,
        min_w: 60.0,
        max_w: 350.0,
        min_h: 60.0,
        max_h: 350.0,
        ..base(8)
    }); // pin both
    cases.push(C {
        cx0: 50.0,
        dx: -200.0,
        max_w: 120.0,
        ..base(5)
    }); // X pin high
    cases.push(C {
        cy0: 50.0,
        dy: 200.0,
        min_h: 90.0,
        ..base(7)
    }); // Y pin low
    cases.push(C { id: 9, ..base(9) }); // id>8 default

    let mut s = 0x0768_7100u32;
    for _ in 0..6000 {
        let id = (next(&mut s) >> 8) % 11; // 0..10
        cases.push(C {
            cx0: rng(&mut s, 0.0, 400.0),
            cy0: rng(&mut s, 0.0, 400.0),
            dx: rng(&mut s, -300.0, 300.0),
            dy: rng(&mut s, -300.0, 300.0),
            id,
            scale: rng(&mut s, 0.5, 2.5),
            min_w: bound(&mut s, 40.0, 200.0),
            max_w: bound(&mut s, 200.0, 380.0),
            min_h: bound(&mut s, 40.0, 200.0),
            max_h: bound(&mut s, 200.0, 380.0),
            child_present: next(&mut s) & 0x100 != 0,
            pred: next(&mut s) & 0x200 != 0,
            c4: rng(&mut s, -50.0, 50.0),
            c8: rng(&mut s, -50.0, 50.0),
        });
    }

    let mut tally = [0u32; 9];
    let mut default_hit = 0u32;
    let mut accum_hit = 0u32;
    let mut clamp_active = 0u32;
    let mut sentinel = 0u32;
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(cx_cell, c.cx0.to_bits());
        emu.write_u32(cy_cell, c.cy0.to_bits());
        emu.write_u32(this + 0x58, c.scale.to_bits());
        emu.write_u32(this + 0x5c, c.min_h.to_bits());
        emu.write_u32(this + 0x60, c.min_w.to_bits());
        emu.write_u32(this + 0x64, c.max_h.to_bits());
        emu.write_u32(this + 0x68, c.max_w.to_bits());
        emu.write_u32(this + 0x4, if c.child_present { child as u32 } else { 0 });
        emu.write_u32(pred_cell, c.pred as u32);
        emu.write_u32(child + 0x4, c.c4.to_bits());
        emu.write_u32(child + 0x8, c.c8.to_bits());
        emu.clear_calls();
        let eax = emu.call(
            0x768710,
            this as u32,
            0,
            &[0, c.dx.to_bits(), c.dy.to_bits(), c.id],
        );

        let child_accum = (c.id == 4 && c.child_present && c.pred).then_some((c.c4, c.c8));
        let mine = drag_resolve(
            c.cx0,
            c.cy0,
            c.dx,
            c.dy,
            c.id,
            c.scale,
            c.min_w,
            c.max_w,
            c.min_h,
            c.max_h,
            child_accum,
        );
        let applied = emu.calls().iter().any(|cc| cc.addr == APPLY);
        let notified = emu.calls().iter().any(|cc| cc.addr == CHILD_NOTIFY);
        match mine {
            DragResolve::Apply { cx, cy, dx, dy } => {
                assert!(applied, "710 case {i} id={}: apply not called", c.id);
                assert!(!notified, "710 case {i}: child-notify must not fire");
                let call = emu.calls().iter().find(|cc| cc.addr == APPLY).unwrap();
                eqf(call.args[1], cx, &format!("710 case {i} id={} cx", c.id));
                eqf(call.args[2], cy, &format!("710 case {i} id={} cy", c.id));
                eqf(call.args[3], dx, &format!("710 case {i} id={} dx", c.id));
                eqf(call.args[4], dy, &format!("710 case {i} id={} dy", c.id));
            }
            DragResolve::ChildAccum { c4, c8 } => {
                assert!(
                    notified,
                    "710 case {i}: child-notify FUN_007680e0 not called"
                );
                assert!(!applied, "710 case {i}: apply must not fire on accumulate");
                assert_eq!(eax, 1, "710 case {i}: case-4 accumulate must return 1");
                eqf(
                    emu.read_u32(child + 0x4),
                    c4,
                    &format!("710 case {i} child+4"),
                );
                eqf(
                    emu.read_u32(child + 0x8),
                    c8,
                    &format!("710 case {i} child+8"),
                );
                accum_hit += 1;
            }
        }

        if c.id <= 8 {
            tally[c.id as usize] += 1;
        } else {
            default_hit += 1;
        }
        if c.min_w == 0.0 && c.max_w == 0.0 && c.min_h == 0.0 && c.max_h == 0.0 {
            sentinel += 1;
        } else {
            clamp_active += 1;
        }
    }
    eprintln!(
        "geo_768710: {} bit-exact; per-id {:?} default={} accum={} (clamp_active={} sentinel={})",
        cases.len(),
        tally,
        default_hit,
        accum_hit,
        clamp_active,
        sentinel
    );
    for (id, h) in tally.iter().enumerate() {
        assert!(*h > 0, "geo_768710 anchor case {id} never hit");
    }
    assert!(default_hit > 0, "geo_768710 id>8 default never hit");
    assert!(
        accum_hit > 0,
        "geo_768710 case-4 child-accumulate never hit"
    );
    assert!(
        clamp_active > 0 && sentinel > 0,
        "geo_768710: need both clamp-active and sentinel coverage"
    );
}
