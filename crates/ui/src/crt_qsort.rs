//! The C runtime's `qsort` (`0x73f727`) with its two helpers, `shortsort` (`0x73f87b`) and `swap`
//! (`0x73f8c9`), over a slice of records. The routine is not stable, so the order that equal
//! elements end in comes from these steps alone, and the comparator sees its arguments in the
//! order the steps give them.
//!
//! The exe works on byte pointers `lo`, `hi`, `loguy` and `higuy`, all `width` bytes apart. This
//! model indexes elements instead. One comparison keeps its byte arithmetic, because it mixes a
//! one-byte offset with element spans: `higuy - 1 - lo >= hi - loguy`, a signed test.
//!
//! Entry: `systems/ui/auctionlists.md#the-c-runtime-qsort`.

/// Ranges of this many elements or fewer go to `shortsort`.
const CUTOFF: usize = 8;

/// `qsort(base, items.len(), width, comp)`. `comp` returns the comparator's `int`; only its sign
/// is read. It returns at once, calling nothing, for fewer than two elements or a zero width.
pub fn qsort<T, F>(items: &mut [T], width: usize, mut comp: F)
where
    F: FnMut(&T, &T) -> i32,
{
    if items.len() < 2 || width == 0 {
        return;
    }
    let width = width as i64;
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut lo = 0usize;
    let mut hi = items.len() - 1;
    loop {
        let size = hi - lo + 1;
        if size <= CUTOFF {
            shortsort(items, lo, hi, &mut comp);
        } else {
            // The middle element becomes the pivot, kept at `lo`.
            items.swap(lo + size / 2, lo);
            let mut loguy = lo;
            let mut higuy = hi + 1;
            loop {
                loop {
                    loguy += 1;
                    if !(loguy <= hi && comp(&items[loguy], &items[lo]) <= 0) {
                        break;
                    }
                }
                loop {
                    higuy -= 1;
                    if !(higuy > lo && comp(&items[higuy], &items[lo]) >= 0) {
                        break;
                    }
                }
                if higuy < loguy {
                    break;
                }
                items.swap(loguy, higuy);
            }
            items.swap(lo, higuy);
            let left = (higuy - lo) as i64 * width - 1;
            let right = (hi as i64 - loguy as i64) * width;
            if left >= right {
                if lo + 1 < higuy {
                    stack.push((lo, higuy - 1));
                }
                if loguy < hi {
                    lo = loguy;
                    continue;
                }
            } else {
                if loguy < hi {
                    stack.push((loguy, hi));
                }
                if lo + 1 < higuy {
                    hi = higuy - 1;
                    continue;
                }
            }
        }
        match stack.pop() {
            Some((l, h)) => {
                lo = l;
                hi = h;
            }
            None => return,
        }
    }
}

/// `shortsort(lo, hi, width, comp)`: a selection sort. Each pass takes the first element of the
/// range that no later one beats (`comp(p, max) > 0`), swaps it to `hi` and shortens the range.
fn shortsort<T, F>(items: &mut [T], lo: usize, mut hi: usize, comp: &mut F)
where
    F: FnMut(&T, &T) -> i32,
{
    while hi > lo {
        let mut max = lo;
        for p in lo + 1..=hi {
            if comp(&items[p], &items[max]) > 0 {
                max = p;
            }
        }
        items.swap(max, hi);
        hi -= 1;
    }
}
