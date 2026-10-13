//! The core computations against the exe, each over fixed edge cases and random inputs.

use benilla_re_difftest::{Emu, RegisterX86, Wow};

/// `HeapNodeIsNotLater` (`0x422970`), `__thiscall(ecx = &a, [esp+4] = &b)`, run whole: the keys
/// are at `+0xc`.
#[test]
fn heap_key_le_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    let node_a = emu.alloc(0x40);
    let node_b = emu.alloc(0x40);

    let mut checked = 0u64;
    for (ka, kb) in synth_pairs() {
        emu.write_u32(node_a + 0xc, ka as u32);
        emu.write_u32(node_b + 0xc, kb as u32);
        // ret 4
        let binary = emu.call(0x422970, node_a as u32, 0, &[node_b as u32]) & 0xff;
        let ours = u32::from(benilla_re_core::heap_key_le(ka, kb));
        assert_eq!(
            binary, ours,
            "heap_key_le({ka:#010x}, {kb:#010x}): binary={binary} ours={ours}"
        );
        checked += 1;
    }
    eprintln!("heap_key_le: {checked} cases bit-exact vs WoW.exe @ 0x422970");
}

/// The frame time of `ContextUpdateFrameTime` (`0x420f70`): the slice `0x420faa` to `0x420fca`,
/// `fild [ebp-4]` to `fstp [ebp-0xc]`.
#[test]
fn dt_seconds_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let ctx = emu.alloc(0x80); // esi: the slice also writes [esi+0x40] and reads [esi+0x44]
    let stack = emu.alloc(0x200);
    let ebp = stack + 0x100;

    let mut checked = 0u64;
    for delta in synth_deltas() {
        emu.set_reg(RegisterX86::ESI, ctx);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, ebp - 0x40);
        emu.write_u32(ebp - 4, delta as u32); // the difference in ms
        emu.write_u32(ctx + 0x44, 0); // flags clear; bit 2's branch does not change the result
        emu.run(0x420faa, 0x420fca);
        let binary = emu.read_f32(ebp - 0xc).to_bits();
        let ours = benilla_re_core::dt_seconds(delta).to_bits();
        assert_eq!(
            binary, ours,
            "dt_seconds({delta}): binary={binary:#010x} ours={ours:#010x}"
        );
        checked += 1;
    }
    eprintln!("dt_seconds: {checked} cases bit-exact vs WoW.exe @ 0x420f70 (x87 53-bit)");
}

/// `UpdateSyncState` (`0x424710`) run whole for categories 0 to 0x1f and three large ones, with
/// the three functions it routes to stubbed so the call log shows which one runs.
#[test]
fn demux_route_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    use benilla_re_core::InputDemux;
    let mut emu = Emu::new(&wow);
    emu.stub(0x424790, 0, 0); // ResetSyncState, ret
    emu.stub(0x424810, 0, 1); // UpdateSyncKeyState, ret 4
    emu.stub(0x424920, 0, 1); // UpdateSyncMouseState, ret 4
    let ctx = emu.alloc(0x40); // ecx, not null, which would take the SetLastError path
    let payload = emu.alloc(0x40); // [ebp+8], read for categories 8, 9, 0xb and 0xe
    let catp = emu.alloc(0x10); // edx, the category

    let cats: Vec<u32> = (0u32..0x20)
        .chain([0x100, 0x7fff_ffff, 0xffff_ffff])
        .collect();
    for category in cats {
        emu.write_u32(catp, category);
        emu.clear_calls();
        emu.call(0x424710, ctx as u32, catp as u32, &[payload as u32]);
        let got: Vec<u64> = emu.calls().iter().map(|c| c.addr).collect();
        let want: Vec<u64> = match benilla_re_core::demux_route(category) {
            InputDemux::FlushKeyQueue => vec![0x424790],
            InputDemux::KeyEvent => vec![0x424810],
            InputDemux::ButtonMask => vec![0x424920],
            InputDemux::None => vec![],
        };
        assert_eq!(
            got, want,
            "demux category {category:#x}: binary {got:?} ours {want:?}"
        );
    }
    eprintln!("demux_route: categories routed bit-exact vs WoW.exe @ 0x424710");
}

/// `PopDueContext` (`0x421430`) run whole on a slot's heap, with the comparator reached through the
/// node vtable `0x801368` and the slot lock stubbed: the element taken, the array after, and each
/// survivor's index. Random arrays, not always heaps, with keys of every size and many ties, reach
/// every choice of child and every stop.
#[test]
fn heap_pop_min_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    const STRIDE: u64 = 0x40; // bytes per element
    const MAXN: u32 = 24;
    let mut emu = Emu::new(&wow);
    emu.stub(0x6579b0, 0, 0); // the lock's enter (ecx, ret)
    emu.stub(0x6579c0, 0, 0); // SCritSect::Leave (ecx, ret)
    let slot = emu.alloc(0x80);
    let array = emu.alloc(4 * u64::from(MAXN + 1));
    let elems = emu.alloc(STRIDE * u64::from(MAXN + 1));
    let table = emu.alloc(0x20);
    emu.write_u32(table, slot as u32); // slot 0
    emu.write_u32(0x0088_3f18, table as u32); // s_slots
    emu.write_u32(slot + 0x2c, array as u32); // the heap's array
    emu.write_u32(slot + 0x34, 0x30); // the node's offset in an element
    let owner = (slot + 0x24) as u32; // the node's owner, at node+4

    let mut lcg = 0x1234_5678u32;
    let mut rng = || {
        lcg = lcg.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        lcg
    };
    let mut trials = 0u64;
    for t in 0..4000u32 {
        let n = (rng() % MAXN + 1) as usize;
        // An element's id is its first position; keys are full range or from a few values.
        let tie_mod = [0i32, 3, 5, 1 << 20][(t & 3) as usize];
        let mut keys = vec![0i32; n];
        for (i, k) in keys.iter_mut().enumerate() {
            *k = if tie_mod == 0 {
                rng() as i32
            } else {
                (rng() as i32).rem_euclid(tie_mod.max(1))
            };
            let ei = elems + i as u64 * STRIDE;
            emu.write_u32(ei + 0x30, 0x0080_1368); // the node vtable, whose slot is 0x422970
            emu.write_u32(ei + 0x34, owner); // node+4
            emu.write_u32(ei + 0x38, i as u32); // node+8, the index
            emu.write_u32(ei + 0x3c, *k as u32); // node+0xc, the key
            emu.write_u32(array + i as u64 * 4, ei as u32);
        }
        emu.write_u32(slot + 0x28, n as u32); // the count
        let popped = u64::from(emu.call(0x421430, 0, 0, &[])); // ecx = slot 0

        let mut heap: Vec<u32> = (0..n as u32).collect();
        let want = benilla_re_core::heap_pop_min(&mut heap, |id| keys[id as usize]);
        let pop_id = ((popped - elems) / STRIDE) as u32;
        assert_eq!(Some(pop_id), want, "n={n} keys={keys:?} popped id");
        for (pos, &want_id) in heap.iter().enumerate() {
            let ptr = u64::from(emu.read_u32(array + pos as u64 * 4));
            let got_id = ((ptr - elems) / STRIDE) as u32;
            assert_eq!(got_id, want_id, "n={n} pos={pos} keys={keys:?} array order");
            let idx = emu.read_u32(elems + u64::from(got_id) * STRIDE + 0x38);
            assert_eq!(idx, pos as u32, "n={n} pos={pos} index back-ptr");
        }
        trials += 1;
    }
    eprintln!("heap_pop_min: {trials} random heaps re-heapified bit-exact vs WoW.exe @ 0x421430");
}

/// `ResiftHeapNode` (`0x421860`) run whole with `ecx` = an element's node, on the same heap as
/// above. The capacity is large enough that its grow (`0x422b50`) does nothing; it takes no lock.
#[test]
fn heap_resift_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    const STRIDE: u64 = 0x40;
    const MAXN: u32 = 24;
    let mut emu = Emu::new(&wow);
    let slot = emu.alloc(0x80);
    let array = emu.alloc(4 * u64::from(MAXN + 4));
    let elems = emu.alloc(STRIDE * u64::from(MAXN + 1));
    let owner = (slot + 0x24) as u32; // the node's owner, at node+4
    emu.write_u32(slot + 0x24, MAXN + 4); // the capacity
    emu.write_u32(slot + 0x2c, array as u32); // the array
    emu.write_u32(slot + 0x30, 0); // the growth step, unused without a grow
    emu.write_u32(slot + 0x34, 0x30); // the node's offset in an element

    let mut lcg = 0x2468_0ace_u32;
    let mut rng = || {
        lcg = lcg.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        lcg
    };
    let mut trials = 0u64;
    for t in 0..4000u32 {
        let n = (rng() % MAXN + 1) as usize;
        let tie_mod = [0i32, 3, 5, 1 << 20][(t & 3) as usize];
        let mut keys = vec![0i32; n];
        for (id, k) in keys.iter_mut().enumerate() {
            *k = if tie_mod == 0 {
                rng() as i32
            } else {
                (rng() as i32).rem_euclid(tie_mod)
            };
            let ei = elems + id as u64 * STRIDE;
            emu.write_u32(ei + 0x30, 0x0080_1368); // the node vtable
            emu.write_u32(ei + 0x34, owner);
            emu.write_u32(ei + 0x38, id as u32);
            emu.write_u32(ei + 0x3c, *k as u32);
            emu.write_u32(array + id as u64 * 4, ei as u32);
        }
        emu.write_u32(slot + 0x28, n as u32); // the count
        let ti = (rng() as usize) % n; // the element to move
        let node = elems + ti as u64 * STRIDE + 0x30;
        emu.call(0x421860, node as u32, 0, &[]);

        let mut heap: Vec<u32> = (0..n as u32).collect();
        benilla_re_core::heap_resift(&mut heap, ti, |id| keys[id as usize]);
        for (pos, &want_id) in heap.iter().enumerate() {
            let ptr = u64::from(emu.read_u32(array + pos as u64 * 4));
            let got_id = ((ptr - elems) / STRIDE) as u32;
            assert_eq!(
                got_id, want_id,
                "n={n} ti={ti} pos={pos} keys={keys:?} array order"
            );
            let idx = emu.read_u32(elems + u64::from(got_id) * STRIDE + 0x38);
            assert_eq!(idx, pos as u32, "n={n} ti={ti} pos={pos} index back-ptr");
        }
        trials += 1;
    }
    eprintln!("heap_resift: {trials} random heaps re-sifted bit-exact vs WoW.exe @ 0x421860");
}

/// `IEvtQueueRegister` (`0x424f00`) and `IEvtQueueDispatch` (`0x4245b0`) run whole: a list made by
/// `0x422d50`, handlers registered with random priorities and many ties (`SMemAlloc` stubbed as a
/// bump allocator), then a dispatch with `UpdateSyncState` and the error check stubbed and each
/// handler a stub, whose call log is the order they ran in.
#[test]
fn event_dispatch_order_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    const MAXK: u64 = 8;
    let mut emu = Emu::new(&wow);
    let ctx = emu.alloc(0x220);
    let head = ctx + 0x5c; // category 0's list
    let nodes = emu.alloc(0x40 * MAXK); // the handler records
    let fnbase = emu.alloc(0x10 * MAXK); // one address per handler function
    let payload = emu.alloc(0x20);
    emu.stub(0x64df60, 0, 0); // SErrIsDisplayingError: no fatal error
    emu.stub(0x424710, 0, 1); // UpdateSyncState, ret 4
    emu.stub_alloc(0x6462e0, nodes, 0x40, 4); // SMemAlloc, ret 0x10
    for i in 0..MAXK {
        // Each returns 1, so the walk goes on to every handler.
        emu.stub(fnbase + i * 0x10, 1, 0);
    }

    let mut lcg = 0x0bad_f00d_u32;
    let mut rng = || {
        lcg = lcg.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        lcg
    };
    let mut trials = 0u64;
    for t in 0..400u32 {
        let k = (rng() % (MAXK as u32 - 1) + 2) as usize; // 2 to MAXK handlers
        let spread = [1u32, 3, 8, 1000][(t & 3) as usize]; // a small spread gives many ties
        emu.reset_bump(nodes);
        emu.call(0x422d50, head as u32, 0, &[]); // an empty list
        emu.write_u32(ctx + 0x44, 0); // flags, read by 0x4224f0

        let mut prios = vec![0u32; k];
        for (i, p) in prios.iter_mut().enumerate() {
            let prio = (rng() % spread) as f32; // whole numbers, exact
            *p = prio.to_bits();
            let handler = fnbase + i as u64 * 0x10;
            emu.call(0x424f00, ctx as u32, 0, &[handler as u32, i as u32, *p]);
        }
        emu.clear_calls();
        emu.call(0x4245b0, ctx as u32, 0, &[payload as u32]); // category 0

        let got: Vec<usize> = emu
            .calls()
            .iter()
            .filter(|c| (fnbase..fnbase + MAXK * 0x10).contains(&c.addr))
            .map(|c| ((c.addr - fnbase) / 0x10) as usize)
            .collect();
        let want = benilla_re_core::event_dispatch_order(&prios);
        assert_eq!(got, want, "k={k} spread={spread} prios(bits)={prios:?}");
        trials += 1;
    }
    eprintln!("event_dispatch_order: {trials} register+dispatch runs order-exact vs WoW.exe");
}

/// The arithmetic of `ClientPointToFractions` (`0x423e20`): the slice `0x423f7f` to `0x423fb1`,
/// with the point and the rectangle in its frame. The clamp and `GetClientRect` before it do not
/// run.
#[test]
fn cursor_normalize_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    let frame = emu.alloc(0x200);
    let ebp = frame + 0x100;
    let u_ptr = emu.alloc(4);
    let v_ptr = emu.alloc(4);
    emu.set_reg(RegisterX86::EBP, ebp);
    emu.set_reg(RegisterX86::ESP, ebp - 0x40); // unused by the slice, but mapped

    let mut checked = 0u64;
    for (x, y, rect) in synth_cursor() {
        // [ebp-0xc] = x, [ebp-0x10] = y, the rectangle from [ebp-0x20] (left, top, right,
        // bottom), and the result pointers [ebp+8] = u, [ebp+0xc] = v.
        emu.write_u32(ebp - 0xc, x as u32);
        emu.write_u32(ebp - 0x10, y as u32);
        emu.write_u32(ebp - 0x20, rect.left as u32);
        emu.write_u32(ebp - 0x1c, rect.top as u32);
        emu.write_u32(ebp - 0x18, rect.right as u32);
        emu.write_u32(ebp - 0x14, rect.bottom as u32);
        emu.write_u32(ebp + 8, u_ptr as u32);
        emu.write_u32(ebp + 0xc, v_ptr as u32);
        emu.run(0x423f7f, 0x423fb1); // to the epilogue
        let bu = emu.read_f32(u_ptr).to_bits();
        let bv = emu.read_f32(v_ptr).to_bits();
        let [ou, ov] = benilla_re_core::cursor_normalize(x, y, rect);
        let r = (rect.left, rect.top, rect.right, rect.bottom);
        assert_eq!(
            bu,
            ou.to_bits(),
            "u x={x} rect={r:?}: binary={bu:#010x} ours={:#010x}",
            ou.to_bits()
        );
        assert_eq!(
            bv,
            ov.to_bits(),
            "v y={y} rect={r:?}: binary={bv:#010x} ours={:#010x}",
            ov.to_bits()
        );
        checked += 1;
    }
    eprintln!("cursor_normalize: {checked} cases bit-exact vs WoW.exe @ 0x423e20 (x87 53-bit)");
}

/// (x, y, rect) cases: common resolutions with points on, in and outside the edges (the slice does
/// not clamp), then random points over `[-w, 2w)`.
fn synth_cursor() -> Vec<(i32, i32, benilla_re_core::ClientRect)> {
    use benilla_re_core::ClientRect;
    let rects = [
        ClientRect {
            left: 0,
            top: 0,
            right: 1024,
            bottom: 768,
        },
        ClientRect {
            left: 0,
            top: 0,
            right: 1280,
            bottom: 1024,
        },
        ClientRect {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        },
        ClientRect {
            left: 0,
            top: 0,
            right: 800,
            bottom: 600,
        },
        ClientRect {
            left: 0,
            top: 0,
            right: 1,
            bottom: 1,
        },
        ClientRect {
            left: 10,
            top: 20,
            right: 810,
            bottom: 620,
        }, // an offset origin
        ClientRect {
            left: -50,
            top: -30,
            right: 1234,
            bottom: 567,
        },
    ];
    let mut v = Vec::new();
    for r in rects {
        let w = r.right - r.left;
        let h = r.bottom - r.top;
        let xs = [0, 1, -1, w / 2, w - 1, w, w + 1, 2 * w, -w];
        let ys = [0, 1, -1, h / 2, h - 1, h, h + 1, 2 * h, -h];
        for x in xs {
            for y in ys {
                v.push((x, y, r));
            }
        }
    }
    let mut s: u32 = 0x51ed_270b;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..8000 {
        let r = rects[(next() % 2) as usize];
        let w = (r.right - r.left) as u32;
        let h = (r.bottom - r.top) as u32;
        let x = (next() % (3 * w)) as i32 - w as i32;
        let y = (next() % (3 * h)) as i32 - h as i32;
        v.push((x, y, r));
    }
    v
}

fn synth_deltas() -> Vec<i32> {
    let mut v = vec![
        0,
        1,
        -1,
        i32::MIN,
        i32::MAX,
        1 << 23,
        (1 << 23) + 1,
        1 << 24, // past 24 bits, where `delta as f32` would round
        (1 << 24) + 1,
        0x4000_0000,
        -0x4000_0000,
    ];
    // Every frame time from 0 to 2000 ms.
    v.extend(0..2000);
    // Random values over the full range.
    let mut s: u32 = 0x9e37_79b9;
    for _ in 0..20_000 {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        v.push(s as i32);
    }
    v
}

/// Edge values crossed with each other, the pairs whose difference wraps (where `a <= b` differs),
/// and random pairs.
fn synth_pairs() -> Vec<(i32, i32)> {
    let edges = [
        0i32,
        1,
        -1,
        100,
        -100,
        i32::MIN,
        i32::MAX,
        0x4000_0000,
        -0x4000_0000,
        0x3fff_ffff,
    ];
    let mut v = Vec::new();
    for &x in &edges {
        for &y in &edges {
            v.push((x, y));
        }
    }
    // The difference overflows i32.
    v.extend([
        (i32::MIN, 1),
        (i32::MAX, -1),
        (1, i32::MIN),
        (-1, i32::MAX),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
    ]);
    let mut s: u32 = 0x1234_5678;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s as i32
    };
    for _ in 0..20_000 {
        v.push((next(), next()));
    }
    v
}

/// The delay of `IEvtTimerCreate` (`0x428b30`): the slice `0x428c4e` to `0x428c6e`, `fild qword
/// [ebp-0x10]; fmul [0x801360]; fstp dword [esi+0x14]`, among stores that do not touch the result.
#[test]
fn evttimer_ticks_to_seconds_matches_binary() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut emu = Emu::new(&wow);
    emu.set_reg(RegisterX86::FPCW, 0x027F);
    emu.write_u32(0x801360, 0x3a83_126f); // 0.001f, as in the image
    let ctx = emu.alloc(0x40); // esi: the slice writes +0, +0x14, +0x18, +0x1c and +0x20
    let frame = emu.alloc(0x80);
    let ebp = frame + 0x40; // [ebp-0x10] to [ebp+0x1c]
    let stack = emu.alloc(0x200);

    let mut checked = 0u64;
    let case = |emu: &mut Emu, tick: i64, ctxn: &str| {
        emu.write(ebp - 0x10, &tick.to_le_bytes());
        emu.write(ctx + 0x14, &[0u8; 4]);
        emu.set_reg(RegisterX86::ESI, ctx);
        emu.set_reg(RegisterX86::EBP, ebp);
        emu.set_reg(RegisterX86::ESP, stack + 0x100);
        emu.run(0x428c4e, 0x428c6e);
        let binary = emu.read_f32(ctx + 0x14);
        let ours = benilla_re_core::evttimer_ticks_to_seconds(tick);
        if binary.is_nan() && ours.is_nan() {
            return;
        }
        assert_eq!(
            binary.to_bits(),
            ours.to_bits(),
            "{ctxn}: tick={tick} binary={:#010x} ours={:#010x}",
            binary.to_bits(),
            ours.to_bits()
        );
    };
    for t in [
        0i64,
        1,
        1000,
        -1,
        1_000_000,
        i64::from(i32::MAX),
        0x7fff_ffff_ffff,
    ] {
        case(&mut emu, t, "boundary");
    }
    let mut s: u32 = 0x428b_3001;
    let mut next = || {
        s = s.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        s
    };
    for _ in 0..5000 {
        let tick = ((u64::from(next()) << 32) | u64::from(next())) as i64;
        case(&mut emu, tick, "fuzz");
        checked += 1;
    }
    eprintln!(
        "evttimer_ticks_to_seconds: {} cases bit-exact vs WoW.exe @ 0x428b30",
        checked + 7
    );
}
