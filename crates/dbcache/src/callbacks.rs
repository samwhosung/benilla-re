//! The callback lists of the `DBCache` nodes (`systems/dbcache/callbacks.md#callback-list-links` and
//! `#callback-order`).
//!
//! A node keeps the callbacks of the callers waiting for its record in a doubly linked list whose
//! links sit at offset 0 of each `DBCACHECALLBACK`. A link is two dwords: the previous link's
//! address, then the next node, which at the end of the list is the terminator's address with
//! bit 0 set. The terminator is the list object's own link, at `+4`. [`link`] is `0x56d0e0`, whole,
//! for a list with link offset 0; [`keep`] and [`run_order`] are what `0x55ba30` keeps and what a
//! response runs.

/// A little-endian dword image of the memory a list lives in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Image {
    base: u32,
    words: Vec<u32>,
}

impl Image {
    /// `len` zeroed dwords from `base`.
    pub fn new(base: u32, len: usize) -> Image {
        Image {
            base,
            words: vec![0; len],
        }
    }

    pub fn get(&self, addr: u32) -> u32 {
        self.words[((addr - self.base) / 4) as usize]
    }

    pub fn set(&mut self, addr: u32, v: u32) {
        self.words[((addr - self.base) / 4) as usize] = v;
    }

    /// The bytes the image holds, as the emulator's memory reads them.
    pub fn bytes(&self) -> Vec<u8> {
        self.words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}

/// An empty list at `list`, as the node constructor `0x55b250` leaves it: link offset 0 at `+0`,
/// the terminator at `+4` pointing back at itself and forward at itself tagged with bit 0
/// (`0x55b2aa` to `0x55b2bb`).
pub fn init_list(img: &mut Image, list: u32) {
    img.set(list, 0);
    img.set(list + 4, list + 4);
    img.set(list + 8, (list + 4) | 1);
}

/// The link of the node at `link` that follows it: the node `next` names, or the terminator when
/// `next` is tagged (`0x56d0fb` to `0x56d107`, `0x56d15a` to `0x56d167`). Offset 0.
fn next_link(next: u32) -> u32 {
    if next & 1 != 0 || next == 0 {
        next & !1
    } else {
        next
    }
}

/// `0x56d0e0(node, mode, reference)` on the list at `list`, whole. It unlinks `node` if it is
/// linked, then links it directly after `reference` (mode 1) or directly before it (any other
/// mode, 2 in the exe). A `reference` of 0 stands for the terminator, so mode 1 makes the node
/// the head and mode 2 the tail. `node` must not be 0 and a `reference` must be linked in the
/// list.
pub fn link(img: &mut Image, list: u32, node: u32, mode: u32, reference: u32) {
    let l = node;
    // Unlink: the next link's previous becomes this link's previous, and the previous link's
    // next becomes this link's next (`0x56d0f2` to `0x56d120`).
    let prev = img.get(l);
    if prev != 0 {
        let next = img.get(l + 4);
        img.set(next_link(next), prev);
        img.set(prev + 4, next);
        img.set(l, 0);
        img.set(l + 4, 0);
    }
    let r = if reference != 0 { reference } else { list + 4 };
    if mode == 1 {
        // After `r` (`0x56d14d` to `0x56d188`).
        img.set(l, r);
        let after = img.get(r + 4);
        img.set(l + 4, after);
        img.set(next_link(after), l);
        img.set(r + 4, node);
    } else {
        // Before `r` (`0x56d137` to `0x56d14a`).
        let before = img.get(r);
        img.set(l, before);
        let before_next = img.get(before + 4);
        img.set(l + 4, before_next);
        img.set(before + 4, node);
        img.set(r, l);
    }
}

/// The nodes of the list at `list` from the head along the next links, as the response walk
/// reads them (`0x55bf12` to `0x55bf3e`): `[list + 8]`, then `[node + 4]`, until a tagged or
/// zero next.
pub fn nodes(img: &Image, list: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut at = img.get(list + 8);
    while at & 1 == 0 && at != 0 {
        out.push(at);
        at = img.get(at + 4);
    }
    out
}

/// A kept callback: the function, its data, and the id the caller passed (the two dwords the
/// function is handed a pointer to).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Callback {
    pub func: u32,
    pub data: u32,
    pub id: [u32; 2],
}

impl Callback {
    /// Whether two callbacks are the same to the dedupe scan and to a cancel: the function and
    /// the data, not the id (`0x55bbd8`, `0x55bbe0`, `0x55c15c`, `0x55c167`).
    pub fn same(&self, other: &Callback) -> bool {
        self.func == other.func && self.data == other.data
    }
}

/// What `0x55ba30` does with a callback for a key whose node already exists and has no record
/// yet: it keeps the callback at the end of the list, unless `dedupe` is set and a callback with
/// the same function and data is there (`0x55bbb3` to `0x55bc16`). A null function keeps nothing
/// (`0x55bbab`).
pub fn keep(list: &mut Vec<Callback>, cb: Callback, dedupe: bool) {
    if cb.func == 0 {
        return;
    }
    if dedupe && list.iter().any(|c| c.same(&cb)) {
        return;
    }
    list.push(cb);
}

/// What `0x55c0c0` does to a pending key's list: removes every callback with the function and
/// data, not the first only (`0x55c150` to `0x55c1aa`).
pub fn cancel(list: &mut Vec<Callback>, func: u32, data: u32) {
    list.retain(|c| !(c.func == func && c.data == data));
}

/// The order a response runs a key's callbacks: the order they were kept.
pub fn run_order(list: &[Callback]) -> Vec<Callback> {
    list.to_vec()
}
