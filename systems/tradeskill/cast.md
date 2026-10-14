# tradeskill: cast

The profession window casts through the ordinary player cast: one `CMSG_CAST_SPELL` for the recipe spell, no packet of its own. A repeat count makes the client cast again after each completion. How a spell with effect 47 opens a window is `spell/cast`; how a failed or cancelled cast ends a repeat is
`spell/failures` and `spell/cancel`; the completion that drives the next cast is `spell/go`.

Roots: none, the function rows of this file belong to `ui`

## DoTradeSkill

`DoTradeSkill` (`0x500280`) takes an index and an optional repeat count, default 1. It casts the spell of the row with the active player as caster, no item and no target (`0x6e5a90`). Before the cast, a count above 1 is latched. The count is first cut to the row's available count, as an unsigned compare, so a negative count becomes the available count and a count of 0 stays 0.
An index out of range or a missing row does nothing. A header row casts the spell id -1. A row with an available count of 0 still casts once. The function returns no value.

Addresses: `0x500280`, `0x500291`, `0x5002c0`, `0x5002ce`, `0x5002df`, `0x5002e1`, `0x5002f5`, `0x5002f8`, `0x5002fe`, `0x500307`, `0x50030e`, `0x500310`, `0x500312`, `0x500314`, `0x6e5a90`, `0x4fd770`
Confidence: verified
Proof: one
Test: none

`0x5002c0` defaults the count to 1 and `0x5002ce` to `0x5002df` replace it with the second argument when that is a number. `0x5002e1` bounds the index against the full count. `0x5002f5` loads the available count and `0x5002f8` takes the lesser of the two as unsigned (`jb`). `0x5002fe` compares the count with 1 (`jbe`) and `0x500307` calls the latch with the spell id and the count.
`0x50030e` and `0x500310` push the zero target, `0x500312` zeroes the item and `0x500314` calls `0x6e5a90`.

## Repeat latch

The latch is the repeat spell `0xbde05c` and the repeat count `0xbde060`. `0x4fd770` stores both and raises `UPDATE_TRADESKILL_RECAST`. `0x4fd790` zeroes the spell and raises the event, only when a spell is set; the count stays. When the player's cast of a spell completes, `0x4fd7b0` takes its id: with a latched spell, a nonzero count and the same id, it
decrements the count; a count that is still above 0 casts the spell again with `0x6e5a90` and raises the event, and a count that reaches 0 returns with nothing else. Any other case clears the latch.

Addresses: `0x4fd770`, `0x4fd776`, `0x4fd781`, `0x4fd790`, `0x4fd799`, `0x4fd7a8`, `0x4fd7b0`, `0x4fd7b7`, `0x4fd7bf`, `0x4fd7c3`, `0x4fd7ca`, `0x4fd7cf`, `0x4fd7d7`, `0x4fd7e1`, `0x4fd7e6`, `0x4fd7eb`, `0x500230`, `0x50023b`, `0x50023f`, `0x50024c`, `0x6e81e7`, `0x6e1a55`, `0x6e4ac2`, `0x4fc87a`
Confidence: verified
Proof: one
Test: none

`0x4fd776` stores the count and `0x4fd781` signals event 502. `0x4fd799` clears the spell. `0x4fd7b7` and `0x4fd7bf` test the spell and the count, `0x4fd7c3` compares the id, `0x4fd7ca` stores the decremented count and `0x4fd7cf` returns when it is 0 (`je 0x4fd7eb`). `0x4fd7d7` calls the cast and `0x4fd7e1` signals. Every failed test jumps to `0x4fd7e6`, which jumps to the clear, so the last completion leaves the spell latched with a count of 0 until the next completion or a close.
The completion handler calls `0x4fd7b0` at `0x6e81e7`; the clear is called by the cast-failure handler (`0x6e1a55`), the cast cancel (`0x6e4ac2`) and the window close (`0x4fc87a`). `GetTradeskillRepeatCount` (`0x500230`) returns the count when the latched spell is the selected spell and the count is above 1, else 1: `0x50023b` loads the spell, `0x50023f` compares it with the selection and `0x50024c` compares the count with 1.
