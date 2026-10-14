//! The computations of the main loop, the event queue and the window input
//! (`systems/core/`): the scheduler heap, the frame time, the order handlers run in, the input
//! routing, the cursor fractions and the timer delay.
//!
//! The x87 runs at 53-bit precision: `WinMain` (`0x404100`) sets the control word `0x023F`
//! (`METHOD.md#the-harness`), so each operation rounds to `f64` and an `fstp dword` to `f32`.

#![forbid(unsafe_code)]

/// `HeapNodeIsNotLater` (`0x422970`), the scheduler heap's comparator
/// (`systems/core/loop.md#scheduler-heap`): true when key `a` is not later than `b`, as
/// `(i32)(a − b) <= 0` on a wrapping subtraction (`sub eax,[b+0xc]; setle`), so due times may wrap.
/// It differs from `a <= b` when the difference overflows `i32`.
#[must_use]
pub fn heap_key_le(a: i32, b: i32) -> bool {
    a.wrapping_sub(b) <= 0
}

/// The frame time of `ContextUpdateFrameTime` (`0x420f70`, `systems/core/loop.md#frame-time`):
/// the signed millisecond difference times `0.001f` (`[0x801360]`, `0x3a83126f`), `fild; fmul;
/// fstp dword`. The 53-bit product is rounded once to `f32`, which differs from
/// `delta_ms as f32 * 0.001` for a difference above 2^24.
#[must_use]
pub fn dt_seconds(delta_ms: i32) -> f32 {
    const MS_TO_S: f32 = f32::from_bits(0x3a83_126f); // [0x801360]
    (f64::from(delta_ms) * f64::from(MS_TO_S)) as f32
}

/// `PopDueContext` (`0x421430`) takes the heap's root (`systems/core/loop.md#scheduler-heap`): the
/// last element moves into the hole and sifts down, into the right child when it is not later than
/// the left (`0x4214ec`, `0x4214f2`), stopping when the element is not later than that child
/// (`0x421506`). `heap[i]` is an element id and `key(id)` its key; on return `heap` holds the
/// survivors in their new positions. `None` for an empty heap.
#[must_use]
pub fn heap_pop_min(heap: &mut Vec<u32>, key: impl Fn(u32) -> i32) -> Option<u32> {
    let count = heap.len();
    if count == 0 {
        return None;
    }
    let root = heap[0];
    let n = count - 1;
    if n == 0 {
        heap.clear();
        return Some(root);
    }
    let last = heap[n]; // the element sifted down from the root
    let last_index = n - 1; // [ebp-8]
    let last_parent = n.wrapping_sub(2) >> 1; // [ebp-0xc]
    let mut hole = 0usize;
    if last_index >= 1 {
        // `test; jbe` skips the loop for n <= 1
        loop {
            let left = 2 * hole + 1;
            let child = if left >= last_index {
                left // no right child
            } else if heap_key_le(key(heap[left + 1]), key(heap[left])) {
                left + 1 // the right child is not later
            } else {
                left
            };
            if heap_key_le(key(last), key(heap[child])) {
                break;
            }
            heap[hole] = heap[child];
            hole = child;
            if child > last_parent {
                break;
            }
        }
    }
    heap[hole] = last;
    heap.truncate(n);
    Some(root)
}

/// `ResiftHeapNode` (`0x421860`) moves the element at `i` after its key changed
/// (`systems/core/loop.md#scheduler-heap`): it removes it, sifting the hole down as
/// [`heap_pop_min`] does and filling it with the last element, then inserts it again from the last
/// slot, sifting up while its parent is later. The count is unchanged.
pub fn heap_resift(heap: &mut [u32], i: usize, key: impl Fn(u32) -> i32) {
    let count = heap.len();
    if count == 0 {
        return;
    }
    let saved = heap[i];
    let n = count - 1; // the count without `saved`
    let last = heap[count - 1];

    // Remove at `i`: sift the hole down over [0, n) and put `last` in it.
    let mut hole = i;
    if n != i {
        let last_index = n - 1;
        let last_parent = n.wrapping_sub(2) >> 1;
        if i < last_index && i <= last_parent {
            loop {
                let left = 2 * hole + 1;
                let child = if left >= last_index {
                    left
                } else if heap_key_le(key(heap[left + 1]), key(heap[left])) {
                    left + 1
                } else {
                    left
                };
                if heap_key_le(key(last), key(heap[child])) {
                    break;
                }
                heap[hole] = heap[child];
                hole = child;
                if child > last_parent {
                    break;
                }
            }
        }
        heap[hole] = last;
    }

    // Insert `saved` from slot n, sifting up.
    let mut hole2 = n;
    while hole2 != 0 {
        let parent = (hole2 - 1) >> 1;
        if heap_key_le(key(heap[parent]), key(saved)) {
            break;
        }
        heap[hole2] = heap[parent];
        hole2 = parent;
    }
    heap[hole2] = saved;
}

/// The order the handlers of a category run in, from `IEvtQueueRegister` (`0x424f00`) and
/// `IEvtQueueDispatch` (`0x4245b0`): a handler is linked before the first record whose priority is
/// not greater than its own (`systems/core/events.md#handler-order`), and the dispatch walks from
/// the head (`systems/core/events.md#dispatch`). So the highest priority runs first and, among
/// equal priorities, the newest. Takes the priorities' bits in registration order and returns the
/// registration indices in the order they run, when no handler stops the walk.
#[must_use]
pub fn event_dispatch_order(prios: &[u32]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..prios.len()).collect();
    // A stable ascending sort, reversed.
    order.sort_by(|&a, &b| {
        f32::from_bits(prios[a])
            .partial_cmp(&f32::from_bits(prios[b]))
            .expect("handler priorities are finite (non-NaN)")
    });
    order.reverse();
    order
}

/// The function `UpdateSyncState` (`0x424710`) sends an event category to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum InputDemux {
    /// `ResetSyncState` (`0x424790`): empties the held keys (category 2).
    FlushKeyQueue,
    /// `UpdateSyncKeyState` (`0x424810`): a key down (8) or up (9).
    KeyEvent,
    /// `UpdateSyncMouseState` (`0x424920`): a button down (0xb) or up (0xe).
    ButtonMask,
    /// Nothing, for every other category.
    None,
}

/// `UpdateSyncState` (`0x424710`, `systems/core/events.md#held-keys-and-repeats`): `category − 2`,
/// unsigned, against 0xc (`ja`), so only 2 to 0xe route; it indexes the byte map `0x424780` into the
/// four-entry table `0x424770`, whose slot 3 does nothing.
#[must_use]
pub fn demux_route(category: u32) -> InputDemux {
    if !(2..=0xe).contains(&category) {
        return InputDemux::None;
    }
    // The byte map at 0x424780, by category − 2.
    const MAP: [u8; 13] = [0, 3, 3, 3, 3, 3, 1, 1, 3, 2, 3, 3, 2];
    match MAP[(category - 2) as usize] {
        0 => InputDemux::FlushKeyQueue,
        1 => InputDemux::KeyEvent,
        2 => InputDemux::ButtonMask,
        _ => InputDemux::None,
    }
}

/// The client rectangle in pixels, the `RECT` from `GetClientRect` (`0x42cea0`), read as `i32`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

/// The cursor as fractions of the client area, from `ClientPointToFractions` (`0x423e20`,
/// `systems/core/input.md#cursor-coordinates`), its arithmetic at `0x423f7f` to `0x423faf`:
/// `u = x / w` (`fild; fidiv; fstp dword`) and `v = 1 − y / h` (`fild; fidiv; fsubr 1.0; fstp
/// dword`), so y runs up from the bottom. Each operation rounds to `f64`; `w` and `h` are wrapping
/// `i32` differences, and `x` is not offset by `left`.
#[must_use]
pub fn cursor_normalize(x: i32, y: i32, rect: ClientRect) -> [f32; 2] {
    let w = rect.right.wrapping_sub(rect.left);
    let h = rect.bottom.wrapping_sub(rect.top);
    let u = (f64::from(x) / f64::from(w)) as f32;
    let v = (1.0_f64 - f64::from(y) / f64::from(h)) as f32;
    [u, v]
}

/// A timer's delay in seconds, as `IEvtTimerCreate` (`0x428b30`, `systems/core/events.md#timers`)
/// stores it at `+0x14`: `fild qword; fmul [0x801360]; fstp dword` (`0x428c4e` to `0x428c6b`), the
/// 64-bit count times `0.001f` at 53 bits, rounded once to `f32`.
#[must_use]
pub fn evttimer_ticks_to_seconds(tick: i64) -> f32 {
    ((tick as f64) * f64::from(0.001_f32)) as f32
}
