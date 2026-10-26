//! `0x788ac0`, the scrolling message frame's current scroll
//! (`systems/ui/messageframe.md#the-current-scroll`): how many lines the frame is scrolled up
//! from the newest.

/// `(position − scroll + count) mod count` in 32-bit signed arithmetic with a truncated
/// remainder, as `cdq; idiv` gives it; 0 for a count of 0 or a position of −1. `idiv` faults on
/// a count of −1 with the dividend `i32::MIN`; the test keeps off that input.
pub fn current_scroll(count: i32, position: i32, scroll: i32) -> i32 {
    if count == 0 || position == -1 {
        return 0;
    }
    position.wrapping_sub(scroll).wrapping_add(count) % count
}
