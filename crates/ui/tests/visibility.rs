//! `CSimpleFrame::UpdateScale` (`0x76ac90`), `SimpleFrameShowNow` (`0x76ae10`) and
//! `CSimpleFrame::Hide` (`0x76ad50`) (`systems/ui/visibility.md`) run from the exe.
//!
//! Each runs on a synthesized frame with empty child lists and with the calls it makes after its
//! own computation stubbed: the layout scale setter `0x768360`, the strata join `0x764780` and
//! leave `0x764920`, and the `OnShow` and `OnHide` slots. The test compares the value stored and the
//! return with `effective_scale`, `effective_visible_show` and `effective_visible_hide`. Every draw
//! comes off the high bits of an LCG.

use benilla_re_difftest::{Emu, Wow};
use benilla_re_ui::visibility::{
    effective_scale as eff_scale, effective_visible_hide as eff_hide,
    effective_visible_show as eff_show,
};

fn next(s: &mut u32) -> u32 {
    *s = s.wrapping_mul(1664525).wrapping_add(1013904223);
    *s
}

#[test]
fn effective_scale() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    // geometry SetScale (0x768360) writes layoutScale +0x7c; here it's a recorder stub (thiscall,
    // 2 stack args = ret 8): args[0] = the f32 effectiveScale handed down (the owned product).
    let setscale = 0x768360u64;
    emu.stub(setscale, 1, 2);
    let frame = emu.alloc(0x320);
    // empty child lists -> no recursion (region +0x1b8, frame +0x300).
    emu.write_u32(frame + 0x1b8, 0);
    emu.write_u32(frame + 0x300, 0);

    struct C {
        parent: f32,
        own: f32,
        cur: f32,
        force: bool,
    }
    let mut cases: Vec<C> = vec![
        // changed past ε, no force -> apply.
        C {
            parent: 1.0,
            own: 1.0,
            cur: 0.5,
            force: false,
        },
        // unchanged (new == cur), no force -> skip.
        C {
            parent: 1.0,
            own: 1.0,
            cur: 1.0,
            force: false,
        },
        // sub-ε change, no force -> skip.
        C {
            parent: 1.0,
            own: 1.0,
            cur: 1.0 + 1e-7,
            force: false,
        },
        // sub-ε change, FORCED -> apply anyway.
        C {
            parent: 1.0,
            own: 1.0,
            cur: 1.0 + 1e-7,
            force: true,
        },
        // unchanged but forced -> apply.
        C {
            parent: 1.25,
            own: 0.8,
            cur: 1.0,
            force: true,
        },
        // product needs f32 rounding (0.1*0.1).
        C {
            parent: 0.1,
            own: 0.1,
            cur: 0.0,
            force: false,
        },
        // exactly ε apart -> apply (>= ε).
        C {
            parent: 1.0,
            own: 1.0,
            cur: 1.0 - f32::from_bits(0x3480_0000),
            force: false,
        },
    ];
    let mut s = 0x76ac_0090u32;
    for _ in 0..3000 {
        let f = |s: &mut u32| (next(s) >> 8) as f32 / (1u32 << 20) as f32 * 4.0;
        let parent = f(&mut s);
        let own = f(&mut s);
        let new = (f64::from(parent) * f64::from(own)) as f32;
        // cur: often near `new` (exercise the ε boundary), sometimes far.
        let cur = match (next(&mut s) >> 8) % 4 {
            0 => new,                                                           // exact
            1 => new + (next(&mut s) >> 8) as f32 / (1u32 << 26) as f32 * 1e-6, // ~ε scale
            _ => f(&mut s),                                                     // far
        };
        cases.push(C {
            parent,
            own,
            cur,
            force: next(&mut s) & 0x100 != 0,
        });
    }

    let mut tally = [0u32; 3]; // [skip, apply_force, apply_eps]
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(frame + 0xbc, c.parent.to_bits());
        emu.write_u32(frame + 0xb8, c.own.to_bits());
        emu.write_u32(frame + 0x7c, c.cur.to_bits());
        emu.clear_calls();
        // the function returns its bool in `al` (`mov al,1` / `xor al,al`); EAX's upper bytes are
        // stale, so mask to the low byte.
        let al = emu.call(0x76ac90, frame as u32, 0, &[c.force as u32]) & 0xff;
        let setscale_call = emu.calls().iter().find(|cc| cc.addr == setscale).copied();
        let bin_apply = setscale_call.is_some();

        let o = eff_scale(c.parent, c.own, c.cur, c.force);
        assert_eq!(
            al != 0,
            o.apply,
            "effective_scale case {i}: al-apply {} vs {} (p={} o={} cur={} force={})",
            al != 0,
            o.apply,
            c.parent,
            c.own,
            c.cur,
            c.force
        );
        assert_eq!(
            bin_apply, o.apply,
            "effective_scale case {i}: SetScale-fired {bin_apply} vs apply {}",
            o.apply
        );
        if let Some(call) = setscale_call {
            assert_eq!(
                call.args[0],
                o.new_scale.to_bits(),
                "effective_scale case {i}: new_scale {:#010x} vs {:#010x} (p={} o={})",
                call.args[0],
                o.new_scale.to_bits(),
                c.parent,
                c.own
            );
        }
        if !o.apply {
            tally[0] += 1;
        } else if c.force {
            tally[1] += 1;
        } else {
            tally[2] += 1;
        }
    }
    eprintln!(
        "effective_scale: {} bit-exact; [skip={} apply(force)={} apply(ε)={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "effective_scale leg {i} never hit");
    }
}

/// Build a frame + scratch vtable wired so a visibility kernel runs to its `ret` with the
/// orchestration stubbed: bucket add/remove + OnShow/OnHide recorded, child lists empty, the
/// deferred-layout resolve skipped (`+0x114 != 0`), the autoraise bit (`+0xb4`) clear.
struct VisRig {
    frame: u64,
    parent: u64, // a second frame used as parent when present
}

fn vis_rig(emu: &mut Emu) -> VisRig {
    // bucket add (show, 0x764780) / remove (hide, 0x764920): thiscall, 2 stack args (ret 8) — the
    // leading `push ebx` at the call site is a register save, not an argument.
    emu.stub(0x764780, 1, 2);
    emu.stub(0x764920, 1, 2);
    // OnShow (primary vtable +0x30) / OnHide (+0x34): thiscall, no stack args.
    let on_show = emu.alloc(8);
    let on_hide = emu.alloc(8);
    emu.stub(on_show, 0, 0);
    emu.stub(on_hide, 0, 0);
    let vtable = emu.alloc(0x80);
    emu.write_u32(vtable + 0x30, on_show as u32);
    emu.write_u32(vtable + 0x34, on_hide as u32);

    let mk = |emu: &mut Emu, vt: u64| -> u64 {
        let f = emu.alloc(0x320);
        emu.write_u32(f, vt as u32); // primary vtable @ +0
        emu.write_u32(f + 0xa0, f as u32); // screenRoot (bucket call's ecx; stubbed)
        emu.write_u32(f + 0x114, 1); // pending != 0 -> skip the 0x768060 resolve
        emu.write_u32(f + 0xb4, 0); // autoraise bit clear -> skip 0x76a5b0
        emu.write_u32(f + 0x1b8, 0); // empty region-child list
        emu.write_u32(f + 0x300, 0); // empty frame-child list
        f
    };
    let frame = mk(emu, vtable);
    let parent = mk(emu, vtable);
    emu.write_u32(frame + 0x9c, 0); // default: no parent (set per case)
    VisRig { frame, parent }
}

#[test]
fn effective_visible_show() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rig = vis_rig(&mut emu);

    struct C {
        shown: bool,
        parent: Option<bool>, // None = no parent (+0x9c==0); Some(pv) = parent.effectiveVisible
        cur: u32,             // current effectiveVisible (+0xd4)
    }
    let mut cases: Vec<C> = vec![
        // no parent, shown, currently hidden -> transition to visible.
        C {
            shown: true,
            parent: None,
            cur: 0,
        },
        // no parent, shown, already visible -> no transition, eax 1.
        C {
            shown: true,
            parent: None,
            cur: 1,
        },
        // not shown -> eax 0, effectiveVisible untouched, no transition.
        C {
            shown: false,
            parent: None,
            cur: 0,
        },
        // not shown, but currently visible -> eax 0, +0xd4 left as-is (1), no transition.
        C {
            shown: false,
            parent: None,
            cur: 1,
        },
        // parent visible, shown, hidden -> transition.
        C {
            shown: true,
            parent: Some(true),
            cur: 0,
        },
        // parent visible, shown, already visible -> no transition.
        C {
            shown: true,
            parent: Some(true),
            cur: 1,
        },
        // parent hidden -> eax 0, no transition (even though shown).
        C {
            shown: true,
            parent: Some(false),
            cur: 0,
        },
    ];
    let mut s = 0x76ae_0010u32;
    for _ in 0..2000 {
        let parent = match (next(&mut s) >> 8) % 3 {
            0 => None,
            1 => Some(true),
            _ => Some(false),
        };
        cases.push(C {
            shown: next(&mut s) & 0x100 != 0,
            parent,
            cur: next(&mut s) & 0x200, // 0 or nonzero
        });
    }

    let mut tally = [0u32; 4]; // [transition, no-trans(already-vis), not-shown, parent-hidden]
    for (i, c) in cases.iter().enumerate() {
        emu.write_u32(rig.frame + 0xd0, c.shown as u32);
        emu.write_u32(rig.frame + 0xd4, c.cur);
        match c.parent {
            None => emu.write_u32(rig.frame + 0x9c, 0),
            Some(pv) => {
                emu.write_u32(rig.frame + 0x9c, rig.parent as u32);
                emu.write_u32(rig.parent + 0xd4, pv as u32);
            }
        }
        emu.clear_calls();
        let eax = emu.call(0x76ae10, rig.frame as u32, 0, &[]);
        let bin_eff = emu.read_u32(rig.frame + 0xd4);
        let bin_trans = emu.calls().iter().any(|cc| cc.addr == 0x764780);

        let o = eff_show(c.shown, c.parent, c.cur);
        assert_eq!(eax, o.eax, "show case {i}: eax {eax} vs {}", o.eax);
        assert_eq!(
            bin_eff, o.effective,
            "show case {i}: effectiveVisible {bin_eff} vs {}",
            o.effective
        );
        assert_eq!(
            bin_trans, o.transition,
            "show case {i}: transition {bin_trans} vs {}",
            o.transition
        );
        if o.transition {
            tally[0] += 1;
        } else if !c.shown {
            tally[2] += 1;
        } else if c.parent == Some(false) {
            tally[3] += 1;
        } else {
            tally[1] += 1;
        }
    }
    eprintln!(
        "effective_visible_show: {} bit-exact; [transition={} already-vis={} not-shown={} parent-hidden={}]",
        cases.len(),
        tally[0],
        tally[1],
        tally[2],
        tally[3]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "show leg {i} never hit");
    }
}

#[test]
fn effective_visible_hide() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let rig = vis_rig(&mut emu);

    // hide ignores shown/parent; the only input that matters is the current effectiveVisible (+0xd4).
    let mut cases: Vec<u32> = vec![1, 0, 1, 0];
    let mut s = 0x76ad_0050u32;
    for _ in 0..1500 {
        cases.push(next(&mut s) & 0x400); // 0 or nonzero
    }

    let mut tally = [0u32; 2]; // [no-transition (already hidden), transition (was visible)]
    for (i, &cur) in cases.iter().enumerate() {
        emu.write_u32(rig.frame + 0xd4, cur);
        emu.clear_calls();
        let eax = emu.call(0x76ad50, rig.frame as u32, 0, &[]);
        let bin_eff = emu.read_u32(rig.frame + 0xd4);
        let bin_trans = emu.calls().iter().any(|cc| cc.addr == 0x764920);

        let o = eff_hide(cur);
        assert_eq!(eax, o.eax, "hide case {i}: eax {eax} vs {}", o.eax);
        assert_eq!(
            bin_eff, o.effective,
            "hide case {i}: effectiveVisible {bin_eff} vs {}",
            o.effective
        );
        assert_eq!(
            bin_trans, o.transition,
            "hide case {i}: transition {bin_trans} vs {} (cur={cur})",
            o.transition
        );
        tally[o.transition as usize] += 1;
    }
    eprintln!(
        "effective_visible_hide: {} bit-exact; [already-hidden={} transition={}]",
        cases.len(),
        tally[0],
        tally[1]
    );
    for (i, h) in tally.iter().enumerate() {
        assert!(*h > 0, "hide leg {i} never hit");
    }
}
