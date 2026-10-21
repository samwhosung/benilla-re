//! The callback lists of the item cache against the exe (`systems/dbcache/callbacks.md#callback-list-links`
//! and `#callback-order`).
//!
//! `callback_list_link_difftest` runs the link `0x56d0e0` whole on lists of up to six nodes and
//! compares every dword of the list and its nodes with `callbacks::link` after each operation.
//!
//! `callback_order_difftest` runs the exe's own `0x554c30` (which builds the cache), the lookup
//! `0x55ba30`, the cancel `0x55c0c0` and the response handler's receive `0x55bdb0` on one cache.
//! Only the allocator, the network send, the console and the record decoder `0x7c9640` (its own
//! test is `dbcache_itemstats`) are stubbed, and each callback is a stub that logs what it was
//! called with. It compares the calls a response makes, in order, with `callbacks::keep`,
//! `callbacks::cancel` and `callbacks::run_order`, for lookups that keep callbacks with and
//! without the dedupe flag, cancels, and responses that accept and refuse, one record at a time
//! and in a batch.

mod common;

use benilla_re_dbcache::callbacks::{
    cancel, init_list, keep, link, nodes, run_order, Callback, Image,
};
use benilla_re_difftest::{Emu, RegisterX86, Wow};
use common::{make_datastore, next};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// `__thiscall(ecx = list, [esp+4] = node, mode, reference)` `ret 0xc`.
const LINK: u64 = 0x56d0e0;
/// The static initializer that builds the item cache at `0xc0e2a0`.
const BUILD_CACHE: u64 = 0x554c30;
const CACHE: u32 = 0xc0e2a0;
/// `__thiscall(ecx = cache, key, &id, callback, data, dedupe)` `ret 0x14`.
const GET_RECORD: u64 = 0x55ba30;
/// `__thiscall(ecx = cache, store, single)` `ret 8`.
const RECEIVE: u64 = 0x55bdb0;
/// `__thiscall(ecx = cache, key, function, data)` `ret 0xc`.
const CANCEL: u64 = 0x55c0c0;
/// `ConsolePrintf(format, ...)`, `__cdecl`.
const CONSOLE_PRINTF: u64 = 0x63cd50;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SMEM_FREE: u64 = 0x646430;
const SEND: u64 = 0x5ab630;
const ITEM_STATS_READ: u64 = 0x7c9640;

#[test]
fn callback_list_link_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    const NODES: usize = 6;
    const WORDS: usize = 0x100;
    let mut e = Emu::new(&wow);
    let block = e.alloc(4 * WORDS as u64) as u32;
    let list = block;
    let node_at = |i: usize| block + 0x40 + 0x20 * i as u32;
    let mut s = 0x5eed_0001u32;
    // Which (mode, reference kind, node already linked) combinations ran: the reference is the
    // terminator (0), the head (1), a middle node (2) or the tail (3) of the list as it was.
    let mut seen = [[[0u32; 2]; 4]; 2];
    let mut ops = 0u32;
    for run in 0..60u32 {
        let mut img = Image::new(block, WORDS);
        init_list(&mut img, list);
        e.write(u64::from(block), &img.bytes());
        for step in 0..40u32 {
            let node = node_at((next(&mut s) >> 16) as usize % NODES);
            let linked = img.get(node) != 0;
            let in_list = nodes(&img, list);
            let mode = 1 + ((next(&mut s) >> 16) & 1);
            // The reference: the terminator, or a linked node other than `node`.
            let others: Vec<u32> = in_list.iter().copied().filter(|&n| n != node).collect();
            let pick = (next(&mut s) >> 16) as usize % (others.len() + 1);
            let reference = if pick == 0 { 0 } else { others[pick - 1] };
            let kind = match in_list.iter().position(|&n| n == reference) {
                _ if reference == 0 => 0,
                Some(0) => 1,
                Some(at) if at == in_list.len() - 1 => 3,
                _ => 2,
            };
            let before = e.read(u64::from(block), 4 * WORDS);
            e.call(LINK, list, 0, &[node, mode, reference]);
            link(&mut img, list, node, mode, reference);
            let got = e.read(u64::from(block), 4 * WORDS);
            assert_eq!(
                got,
                img.bytes(),
                "run {run} step {step}: mode {mode} node {node:#x} reference {reference:#x} \
                 (list was {in_list:x?}; memory before {before:02x?})"
            );
            seen[mode as usize - 1][kind][usize::from(linked)] += 1;
            ops += 1;
        }
    }
    eprintln!("callback_list_link: {ops} operations");
    // Both modes ran on every kind of reference, linking a fresh node and moving a linked one.
    for (mode, kinds) in seen.iter().enumerate() {
        for (kind, states) in kinds.iter().enumerate() {
            for (linked, &n) in states.iter().enumerate() {
                assert!(
                    n > 0,
                    "mode {} kind {kind} linked {linked} never ran",
                    mode + 1
                );
            }
        }
    }
}

/// One call a callback stub saw.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Seen {
    func: u32,
    key: u32,
    id: [u32; 2],
    data: u32,
    status: u32,
}

/// A request the test makes: the key, which callback function, its data and the dedupe flag.
#[derive(Clone, Copy, Debug)]
struct Ask {
    key: u32,
    func: usize,
    data: u32,
    dedupe: bool,
    /// The two dwords of the id the caller passes.
    id: [u32; 2],
}

struct Rig<'a> {
    e: Emu<'a>,
    log: Rc<RefCell<Vec<Seen>>>,
    sends: Rc<Cell<u32>>,
    prints: Rc<Cell<u32>>,
    funcs: Vec<u32>,
}

impl<'a> Rig<'a> {
    fn new(wow: &'a Wow, callbacks: usize) -> Rig<'a> {
        let mut e = Emu::new(wow);
        let arena = e.alloc(0x40_0000);
        e.stub_alloc_sized(SMEM_ALLOC, arena, arena + 0x40_0000, 0, 4);
        e.stub_realloc_sized(SMEM_REALLOC, 5);
        e.stub(SMEM_FREE, 1, 4);
        e.stub(ITEM_STATS_READ, 0, 1);
        let sends = Rc::new(Cell::new(0u32));
        let s2 = sends.clone();
        e.stub_fn(SEND, 0, move |_| {
            s2.set(s2.get() + 1);
            0
        });
        let prints = Rc::new(Cell::new(0u32));
        let p2 = prints.clone();
        e.stub_fn(CONSOLE_PRINTF, 0, move |_| {
            p2.set(p2.get() + 1);
            0
        });
        let log: Rc<RefCell<Vec<Seen>>> = Rc::default();
        let mut funcs = Vec::new();
        for _ in 0..callbacks {
            let addr = e.alloc(0x10);
            funcs.push(addr as u32);
            let l = log.clone();
            // `function(key = ecx, &id = edx, data, status)` `ret 8`.
            e.stub_fn(addr, 2, move |io| {
                let key = io.reg(RegisterX86::ECX) as u32;
                let id_at = io.reg(RegisterX86::EDX);
                l.borrow_mut().push(Seen {
                    func: addr as u32,
                    key,
                    id: [io.read_u32(id_at), io.read_u32(id_at + 4)],
                    data: io.arg(0),
                    status: io.arg(1),
                });
                0
            });
        }
        e.call(BUILD_CACHE, 0, 0, &[]);
        Rig {
            e,
            log,
            sends,
            prints,
            funcs,
        }
    }

    /// `0x55ba30` for `ask`; the callback is the function with index `ask.func`, or none.
    fn get(&mut self, ask: &Ask, with_callback: bool) -> u32 {
        let id = self.e.alloc(8);
        self.e.write_u32(id, ask.id[0]);
        self.e.write_u32(id + 4, ask.id[1]);
        let cb = if with_callback {
            self.funcs[ask.func]
        } else {
            0
        };
        self.e.call(
            GET_RECORD,
            CACHE,
            0,
            &[ask.key, id as u32, cb, ask.data, u32::from(ask.dedupe)],
        )
    }

    /// `0x55c0c0` for the function with index `func`.
    fn cancel(&mut self, key: u32, func: usize, data: u32) {
        let f = self.funcs[func];
        self.e.call(CANCEL, CACHE, 0, &[key, f, data]);
    }

    /// The node of `key`, found along the cache's list of all nodes: the first at `+0xc`, each
    /// next at `+0x10` of its predecessor (link offset 0xc, the next dword of the link), a
    /// tagged next ending the list.
    fn node_of(&self, key: u32) -> Option<u32> {
        let mut at = self.e.read_u32(u64::from(CACHE) + 0xc);
        while at & 1 == 0 && at != 0 {
            if self.e.read_u32(u64::from(at)) == key {
                return Some(at);
            }
            at = self.e.read_u32(u64::from(at) + 0x10);
        }
        None
    }

    /// The callbacks the exe holds for `key`, from the head of its node's list at `+0x1f4`
    /// (the first at `+0x1fc`, each next at `+4`), with the function, id and data each holds.
    fn list_of(&self, key: u32) -> Vec<Callback> {
        let Some(node) = self.node_of(key) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut at = self.e.read_u32(u64::from(node) + 0x1fc);
        while at & 1 == 0 && at != 0 {
            let a = u64::from(at);
            out.push(Callback {
                func: self.e.read_u32(a + 8),
                data: self.e.read_u32(a + 0x18),
                id: [self.e.read_u32(a + 0x10), self.e.read_u32(a + 0x14)],
            });
            at = self.e.read_u32(a + 4);
        }
        out
    }

    /// `0x55bdb0` over `keys` (a key with its top bit set is a refusal): one record with `single`
    /// set, else a count byte first.
    fn receive(&mut self, keys: &[u32], single: bool) {
        let mut buf = Vec::new();
        if !single {
            buf.push(keys.len() as u8);
        }
        for k in keys {
            buf.extend_from_slice(&k.to_le_bytes());
        }
        let ds = make_datastore(&mut self.e, &buf);
        self.e
            .call(RECEIVE, CACHE, 0, &[ds as u32, u32::from(single)]);
    }
}

#[test]
fn callback_order_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0xca11_bac4u32;
    let mut accepted_calls = 0u32;
    let mut refused_calls = 0u32;
    let mut deduped = 0u32;
    let mut multi = 0u32;
    let mut cancelled = 0u32;
    let mut emptied = 0u32;
    let mut ignored = 0u32;
    let mut multi_removed = 0u32;
    for case in 0..48u32 {
        let mut rig = Rig::new(&wow, 3);
        let keys = [0x1000 + next(&mut s) % 0x100, 0x2000, 0x0300_0007, 0x4abc];
        // The callbacks each key keeps, in the model; a key is there once it has been asked.
        let mut kept: HashMap<u32, Vec<Callback>> = HashMap::new();
        let mut want_sends = 0u32;
        let asks = 2 + (next(&mut s) >> 20) % 10;
        for n in 0..asks {
            let key = keys[(next(&mut s) >> 16) as usize % keys.len()];
            let ask = Ask {
                key,
                func: (next(&mut s) >> 16) as usize % 3,
                data: 0xd000 + (next(&mut s) >> 16) % 3,
                dedupe: (next(&mut s) >> 16).is_multiple_of(3),
                id: [0x1111_0000 + n, 0xaaaa_0000 + case],
            };
            let new_node = !kept.contains_key(&key);
            let list = kept.entry(key).or_default();
            let before = list.len();
            let cb = Callback {
                func: rig.funcs[ask.func],
                data: ask.data,
                id: ask.id,
            };
            // A new node never meets the flag; an existing pending node does.
            keep(list, cb, ask.dedupe && !new_node);
            deduped += u32::from(list.len() == before);
            want_sends += u32::from(new_node);
            let ret = rig.get(&ask, true);
            assert_eq!(ret, 0, "case {case} ask {n}: a pending key has no record");
            assert_eq!(
                rig.list_of(key),
                kept[&key],
                "case {case} ask {n}: the list the exe holds"
            );
            // Sometimes cancel what was just asked, and ask again with the flag set: an
            // emptied list takes the callback.
            if (next(&mut s) >> 16).is_multiple_of(4) {
                rig.cancel(key, ask.func, ask.data);
                let list = kept.get_mut(&key).unwrap();
                let held = list.len();
                cancel(list, cb.func, cb.data);
                cancelled += 1;
                // Every callback with the function and data goes, not the first only.
                multi_removed += u32::from(held - list.len() >= 2);
                assert_eq!(
                    rig.list_of(key),
                    *list,
                    "case {case} ask {n}: the list after the cancel"
                );
                if list.is_empty() {
                    emptied += 1;
                    // A cancel on the emptied list removes nothing.
                    rig.cancel(key, ask.func, ask.data);
                    assert!(rig.list_of(key).is_empty());
                    // Half the time the pending key is left with no callback at all.
                    if (next(&mut s) >> 16).is_multiple_of(2) {
                        continue;
                    }
                    let again = Ask {
                        dedupe: true,
                        id: [0x2222_0000 + n, 0xbbbb_0000 + case],
                        ..ask
                    };
                    let cb2 = Callback { id: again.id, ..cb };
                    keep(list, cb2, true);
                    assert_eq!(rig.get(&again, true), 0);
                    assert_eq!(rig.list_of(key), *kept.get(&key).unwrap());
                }
            }
        }
        // A cancel for a key with no node does nothing, whether its bucket is empty or not.
        for probe in [5u32, 6, 0x0100_0001, 0x0200_0002, 0x0300_0003] {
            if !kept.contains_key(&probe) {
                rig.cancel(probe, 0, 0);
            }
        }
        assert_eq!(
            rig.sends.get(),
            want_sends,
            "case {case}: one query per new key"
        );
        assert!(
            rig.log.borrow().is_empty(),
            "case {case}: asking runs nothing"
        );
        assert_eq!(rig.prints.get(), 0, "case {case}: nothing ignored yet");

        // A cancel while a key's list is being run is ignored: the node's byte `+0x201` stands for
        // the run, and the exe prints the console line instead of removing.
        let pending: Vec<u32> = kept.keys().copied().collect();
        let victim = pending[(next(&mut s) >> 16) as usize % pending.len()];
        if let Some(first) = kept[&victim].first().copied() {
            let node = rig.node_of(victim).expect("the key has a node");
            rig.e.write(u64::from(node) + 0x201, &[1]);
            let func = rig.funcs.iter().position(|&f| f == first.func).unwrap();
            rig.cancel(victim, func, first.data);
            rig.e.write(u64::from(node) + 0x201, &[0]);
            assert_eq!(
                rig.prints.get(),
                1,
                "case {case}: the cancel is ignored with a line"
            );
            ignored += 1;
        }

        // Answer the keys: some accepted, some refused, as single records and as a batch, plus
        // keys nobody asked for.
        let mut order: Vec<u32> = kept.keys().copied().collect();
        order.sort_unstable();
        // A deterministic shuffle.
        for i in (1..order.len()).rev() {
            let j = (next(&mut s) >> 16) as usize % (i + 1);
            order.swap(i, j);
        }
        let mut stream: Vec<(u32, bool)> = order
            .iter()
            .map(|&k| (k, !(next(&mut s) >> 16).is_multiple_of(3)))
            .collect();
        stream.insert(stream.len() / 2, (0x0777_7777, true));
        stream.push((0x0888_8888, false));
        let batch = case % 2 == 0;
        let mut want: Vec<Seen> = Vec::new();
        for &(key, accept) in &stream {
            if let Some(list) = kept.remove(&key) {
                for cb in run_order(&list) {
                    want.push(Seen {
                        func: cb.func,
                        key,
                        id: cb.id,
                        data: cb.data,
                        status: u32::from(accept),
                    });
                }
            }
        }
        if batch && stream.len() > 1 {
            let wire: Vec<u32> = stream
                .iter()
                .map(|&(k, ok)| if ok { k } else { k | 0x8000_0000 })
                .collect();
            rig.receive(&wire, false);
            multi += 1;
        } else {
            for &(k, ok) in &stream {
                rig.receive(&[if ok { k } else { k | 0x8000_0000 }], true);
            }
        }
        let got = rig.log.borrow().clone();
        assert_eq!(got, want, "case {case}: callbacks of the answers, in order");
        accepted_calls += want.iter().filter(|w| w.status == 1).count() as u32;
        refused_calls += want.iter().filter(|w| w.status == 0).count() as u32;

        // An accepted key now holds its record: asking returns it, keeps nothing and sends
        // nothing; a refused key is gone, so asking again is a new node and sends a query.
        let sends = rig.sends.get();
        let unasked = |k: u32| k == 0x0777_7777 || k == 0x0888_8888;
        for &(key, accept) in stream.iter().filter(|&&(k, _)| !unasked(k)) {
            let ask = Ask {
                key,
                func: 0,
                data: 1,
                dedupe: false,
                id: [1, 2],
            };
            let ret = rig.get(&ask, true);
            if accept {
                assert_ne!(ret, 0, "case {case}: key {key:#x} was accepted");
            } else {
                assert_eq!(ret, 0, "case {case}: key {key:#x} was refused");
            }
        }
        let refused = stream.iter().filter(|&&(k, ok)| !ok && !unasked(k)).count() as u32;
        assert_eq!(
            rig.sends.get() - sends,
            refused,
            "case {case}: refused keys ask again"
        );
        assert_eq!(
            rig.log.borrow().len(),
            want.len(),
            "case {case}: nothing ran again"
        );

        // No key and no callback: nothing is kept, sent or returned for a key without a node.
        let sends = rig.sends.get();
        assert_eq!(
            rig.get(
                &Ask {
                    key: 0,
                    func: 0,
                    data: 0,
                    dedupe: false,
                    id: [0, 0]
                },
                true
            ),
            0
        );
        let fresh = Ask {
            key: 0x00ab_cdef,
            func: 0,
            data: 0,
            dedupe: false,
            id: [0, 0],
        };
        assert_eq!(rig.get(&fresh, false), 0);
        assert_eq!(rig.sends.get(), sends, "case {case}: no callback, no query");
        assert!(rig.node_of(fresh.key).is_none());
    }
    eprintln!(
        "callback_order: {accepted_calls} calls with status 1, {refused_calls} with 0, \
         {deduped} asks not kept, {cancelled} cancels ({multi_removed} removed two or more, {emptied} \
         emptied a list, {ignored} ignored while running), {multi} batches"
    );
    assert!(accepted_calls > 0 && refused_calls > 0 && deduped > 0 && multi > 0);
    assert!(cancelled > 0 && emptied > 0 && ignored > 0 && multi_removed > 0);
}
