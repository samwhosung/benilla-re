//! The quest cache's cancel `0x563280` against the exe (`systems/dbcache/callbacks.md#quest-cache-cancel`).
//!
//! `quest_cancel_difftest` builds the quest cache with the exe's own initializer `0x554d70`, keeps
//! callbacks through the lookup `0x562a40`, cancels through `0x563280` and compares the list the
//! exe holds, as function and data in order, with `callbacks::keep` and `callbacks::cancel`. A
//! cancel while the key's list is running (the node's byte `+0x1909`) removes nothing and prints
//! one console line; a key with no node does nothing. Only the allocator, the network send and the
//! console are stubbed, and each callback is a stub.

mod common;

use benilla_re_dbcache::callbacks::{cancel, keep, Callback};
use benilla_re_difftest::{Emu, Wow};
use common::next;
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

/// The initializer that builds the quest cache at `0xc0e1b0`.
const BUILD_CACHE: u64 = 0x554d70;
const CACHE: u32 = 0xc0e1b0;
/// `__thiscall(ecx = cache, key, &id, callback, data, dedupe)` `ret 0x14`.
const GET_RECORD: u64 = 0x562a40;
/// `__thiscall(ecx = cache, key, function, data)` `ret 0xc`.
const CANCEL: u64 = 0x563280;
const CONSOLE_PRINTF: u64 = 0x63cd50;
const SMEM_ALLOC: u64 = 0x6462e0;
const SMEM_REALLOC: u64 = 0x646320;
const SMEM_FREE: u64 = 0x646430;
const SEND: u64 = 0x5ab630;

struct Rig<'a> {
    e: Emu<'a>,
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
        e.stub(SEND, 0, 1);
        let prints = Rc::new(Cell::new(0u32));
        let p2 = prints.clone();
        e.stub_fn(CONSOLE_PRINTF, 0, move |_| {
            p2.set(p2.get() + 1);
            0
        });
        let funcs = (0..callbacks)
            .map(|_| {
                let addr = e.alloc(0x10);
                e.stub(addr, 0, 2);
                addr as u32
            })
            .collect();
        e.call(BUILD_CACHE, 0, 0, &[]);
        Rig { e, prints, funcs }
    }

    /// `0x562a40` keeping callback `func` with `data` for `key`.
    fn get(&mut self, key: u32, func: usize, data: u32, dedupe: bool) {
        let id = self.e.alloc(8);
        let cb = self.funcs[func];
        let ret = self.e.call(
            GET_RECORD,
            CACHE,
            0,
            &[key, id as u32, cb, data, u32::from(dedupe)],
        );
        assert_eq!(ret, 0, "a pending key has no record");
    }

    fn cancel(&mut self, key: u32, func: usize, data: u32) {
        let f = self.funcs[func];
        self.e.call(CANCEL, CACHE, 0, &[key, f, data]);
    }

    /// The node of `key`, along the cache's list of all nodes (the first at `+0xc`, each next at
    /// `+0x10`).
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

    /// The callbacks the exe holds for `key`, from the tail of its list (the tail at `+0x1904`,
    /// each previous at `+4`): the function at `+8` and the data at `+0x18`.
    fn list_of(&self, key: u32) -> Vec<(u32, u32)> {
        let Some(node) = self.node_of(key) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        let mut at = self.e.read_u32(u64::from(node) + 0x1904);
        while at & 1 == 0 && at != 0 {
            let a = u64::from(at);
            out.push((self.e.read_u32(a + 8), self.e.read_u32(a + 0x18)));
            at = self.e.read_u32(a + 4);
        }
        out
    }
}

fn pairs(list: &[Callback]) -> Vec<(u32, u32)> {
    list.iter().map(|c| (c.func, c.data)).collect()
}

#[test]
fn quest_cancel_difftest() {
    let Some(wow) = Wow::load() else {
        return;
    };
    let mut s = 0x9e57_c4ceu32;
    let (mut cancelled, mut multi, mut emptied, mut ignored) = (0, 0, 0, 0);
    for case in 0..48u32 {
        let mut rig = Rig::new(&wow, 3);
        let keys = [
            0x1000 + next(&mut s) % 0x100,
            0x2000,
            0x0300_0007,
            0x8000_0005,
        ];
        let mut kept: HashMap<u32, Vec<Callback>> = HashMap::new();
        for n in 0..2 + (next(&mut s) >> 20) % 12 {
            let key = keys[(next(&mut s) >> 16) as usize % keys.len()];
            let func = (next(&mut s) >> 16) as usize % 3;
            let data = 0xd000 + (next(&mut s) >> 16) % 3;
            let dedupe = (next(&mut s) >> 16).is_multiple_of(3);
            let new_node = !kept.contains_key(&key);
            let cb = Callback {
                func: rig.funcs[func],
                data,
                id: [0, 0],
            };
            keep(kept.entry(key).or_default(), cb, dedupe && !new_node);
            rig.get(key, func, data, dedupe);
            assert_eq!(rig.list_of(key), pairs(&kept[&key]), "case {case} ask {n}");
            if (next(&mut s) >> 16).is_multiple_of(3) {
                let list = kept.get_mut(&key).unwrap();
                let held = list.len();
                cancel(list, cb.func, cb.data);
                rig.cancel(key, func, data);
                cancelled += 1;
                multi += u32::from(held - list.len() >= 2);
                emptied += u32::from(list.is_empty());
                assert_eq!(rig.list_of(key), pairs(list), "case {case} cancel {n}");
            }
        }
        // A key with no node, in an empty bucket or not, changes nothing and prints nothing.
        for probe in [5u32, 6, 0x0100_0001, 0x0200_0002, 0x0300_0003, 0xffff_fff0] {
            if !kept.contains_key(&probe) {
                rig.cancel(probe, 0, 0);
            }
        }
        assert_eq!(rig.prints.get(), 0, "case {case}");
        // While the key's list is being run the cancel removes nothing and prints a line.
        let live: Vec<u32> = kept.keys().copied().collect();
        let victim = live[(next(&mut s) >> 16) as usize % live.len()];
        if let Some(first) = kept[&victim].first().copied() {
            let node = rig.node_of(victim).expect("the key has a node");
            rig.e.write(u64::from(node) + 0x1909, &[1]);
            let func = rig.funcs.iter().position(|&f| f == first.func).unwrap();
            rig.cancel(victim, func, first.data);
            rig.e.write(u64::from(node) + 0x1909, &[0]);
            assert_eq!(rig.prints.get(), 1, "case {case}: ignored with a line");
            assert_eq!(rig.list_of(victim), pairs(&kept[&victim]), "case {case}");
            ignored += 1;
        }
        for (key, list) in &kept {
            assert_eq!(rig.list_of(*key), pairs(list), "case {case} key {key:#x}");
        }
    }
    assert!(cancelled > 20 && multi > 3 && emptied > 3 && ignored > 20);
}
