# ui: the gossip window

What an NPC's gossip message delivers to the interface: the option and quest records it fills, the
greeting it picks, the Lua functions that read them, what selecting an option sends, and the
innkeeper's bind confirmation. The quest lists of a quest giver that is not a gossip NPC are `questgiver`.

The code is `GossipInfo.cpp`, `0x4e1f00` to `0x4e2b70`: the message handler `0x4e26e0`, the greeting
builder `0x4e2010` and the eight Lua functions registered by the loop at `0x4e2b60`; the bind flow is in
`GameUI.cpp` and `Player_C.cpp`.

Roots: `0x4e26e0`, `0x4e2010`, `0x4e22d0`, `0x4e1fa0`, `0x4e2840`, `0x4e2320`

## The gossip message

Opcode 0x17d, handled by `0x4e26e0`: `u64 guid`, `u32 textId`, `u32 optionCount`, the options, `u32
questCount` and the quests. The guid is latched as the gossip target (a zero guid closes the window).
An option is `u32 index`, `u8 icon`, `u8 coded` and a C string of up to 0x800 bytes. A quest is `u32
questId`, `u32 icon`, `u32 level` and a C string of up to 0x200 bytes. The option count above 15 is
read as 15; the quest count has no upper limit though 32 rows exist.

Addresses: `0x4e26e0`, `0x4e26f2`, `0x4e26fa`, `0x4e26ff`, `0x4e270a`, `0x4e2715`, `0x4e271d`, `0x4e2720`, `0x4e273b`, `0x4e2746`, `0x4e2751`, `0x4e2768`, `0x4e276e`, `0x4e2770`, `0x4e2789`, `0x4e27a6`, `0x4e27ae`, `0x4e27b9`, `0x4e27c9`, `0x4e27df`, `0x4e26a0`, `0x4e1fa0`, `0x4e1f12`, `0x4e1f1e`, `0xbbbe90`, `0xbb74c0`, `0xbc3f58`
Confidence: verified
Proof: one
Test: none

The handler is registered by the block at `0x4e1f12` for opcode 0x17d. The count clamp is `cmp eax,0x10; jb`,
unsigned, so any count of 16 or more, a huge one too, becomes 15 (`0x4e2722`). The option store is 16 slots of
0x80c bytes at `0xbbbe90`: text at `+0`, the index at `+0x800`, the coded byte at `+0x804` and the icon dword at `+0x808`.
`0x4e26a0` resets the 16 index words to -1 and zeroes the 32 quest ids (`0x4e26c0`), so a stale option slot keeps its text. The quest store is 32
rows of 0x20c bytes at `0xbb74c0`: id at `+0`, level at `+4`, icon at `+8`, title at `+0xc`. The quest loop tests
only `jle` on its count (`0x4e2795`), so more than 32 rows write past the array.

## The icon names

`GetGossipOptions()` returns two strings for each used option, in slot order: the text and the type name
`gossip`, `vendor`, `taxi`, `trainer`, `healer`, `binder`, `banker`, `petition`, `tabard`, `battlemaster` or
`auctioneer`, picked by the icon byte (0 to 10; 11, 12 and 13 are also `gossip`; 14 gives nil). Slots with
an index below 0 are skipped, so holes are compacted, and an index with its top bit set hides the option. The count
returned is twice the option count. The icon is never checked: a byte of 15 or more reads past the name
table into the function registration table.

Addresses: `0x4e28d0`, `0x4e28ec`, `0x4e28f5`, `0x4e28fe`, `0x4e2903`, `0x4e2909`, `0x4e2912`, `0x4e2928`, `0x84b7ac`, `0x4e2b60`, `0x4e2b7d`, `0x4e28c0`, `0x6f3890`
Confidence: verified
Proof: one
Test: none

The type is `mov edx,[eax*4+0x84b7ac]` with `eax` the zero-extended icon (`0x4e2903`, `0x4e2909`). The table
holds pointers to the eleven names, `gossip` three more times and a null at index 14. `jl` at `0x4e28ec`
is a signed test. `0x6f3890` pushes nil for a null pointer. `0x4e2b60` registers eight functions,
`GetGossipText` `0x4e28c0` to `CloseGossip` `0x4e2b20`.

## Selecting an option

`SelectGossipOption(n [, code])` is refused with no event or packet while the greeting is pending
(`0xbbb670`), for an index outside 0 to 15 after decrementing, or for an unused slot. An option whose coded
byte is set and no code given fires `GOSSIP_ENTER_CODE` (id 298) with the 1-based index and sends nothing.
Otherwise it sends opcode 0x17c: the target GUID, the slot's stored index (not the argument) and, only for a
coded option, the code string. Then the slots are reset.

Addresses: `0x4e2a30`, `0x4e2320`, `0x4e2326`, `0x4e26a0`, `0xbbb670`, `0xbc3f58`
Confidence: verified
Proof: one
Test: none

The binding calls `0x4e2320(index-1, code)`. The packet is `u32 0x17c`, `u64 [0xbc3f58]`, `u32 [slot+0x800]` and
`CString` when `[slot+0x804] & 1`.

## The greeting

After the message the handler asks the NPC text cache (query opcode 0x17f) for the text id. A text id of
0 or a cache miss makes the greeting pending (`0xbbb670` = 1) and shows nothing. With the record it takes a
column from the NPC's gender and draws one of the eight entries at random, weighted by their
probabilities among those with text in that column. The text is expanded for `$` macros (`questtext`),
run through the language rewrite (`chatlanguage`) and stored, and `GOSSIP_SHOW` (id 297) fires. With no entry
having text it logs `Missing gossip text!` and closes the window (`GOSSIP_CLOSED`, id 299).

Addresses: `0x4e2010`, `0x4e2068`, `0x4e2089`, `0x4e20b1`, `0x4e20c1`, `0x4e20fb`, `0x4e2110`, `0x4e2126`, `0x4e2151`, `0x4e2159`, `0x4e215c`, `0x4e216e`, `0x4e217d`, `0x4e225f`, `0x4e2274`, `0x4e229a`, `0x4e22b0`, `0x4e22d0`, `0x55d530`, `0x55d53d`, `0x4531e0`, `0x882664`, `0xc0e264`, `0xbbb678`, `0x7ffd74`, `0x801628`
Confidence: verified
Proof: one
Test: none

The column is the second for gender 1 and otherwise the first; for an object it is the second only when
entry 0's first column is empty. The NPC text record holds eight probabilities at `+0`, eight text pairs of two pointers from `+0x20` and eight
languages from `+0x60`. The gender is the second byte of `UNIT_FIELD_BYTES_0` at `[+0x110]+0x7a`; `sete` fires on 1 only
(`0x4e20c1` to `0x4e20fb`). The first loop sums probabilities of entries with text in the column and rounds the sum to
f32 once (`0x4e2126`). The draw takes the next random word (`0x4531e0`), makes a float in [1,2) from its low 23
bits, and the threshold is `(2 - r) * sum`; the second loop returns the first entry whose running sum is at
least the threshold (`fcomp`, `test ah,0x41`, `jne`: equality selects). With every probability 0 the threshold is
0 and the first entry with text is chosen. `0x55d530` returns null for id 0 before it sends anything. The cache callback
`0x4e22d0` calls `0x4e2010` again on success and closes the window on failure. The text goes to `0xbbb678`
(`0x4e229a`, its only writer) right before the event; a bail leaves the previous text there.

## Quest greetings

The quest giver panel's texts (greeting, quest text, progress, reward) go through `0x500bd0`, which expands
`$` macros and stores an empty result as a single space, never as empty, into its buffer. The objective text
is the exception: an empty source stays empty (`0x500dc0`).

Addresses: `0x500bd0`, `0x500cc9`, `0x500c88`, `0x500dc0`, `0x500e27`, `0x506f70`, `0x501a30`, `0x501a40`, `0x501a60`, `0x501a70`, `0xbde068`, `0xbde288`, `0xbdf0f8`, `0xbdf500`, `0xbdf900`, `0xbdf958`
Confidence: verified
Proof: one
Test: none

The setter `0x500bd0(kind, src)` calls the expander `0x506f70` with the active player (`0x500cc9`), replaces an empty
buffer with `" "` and copies it by kind through the jump table of the setter: kind 0 the greeting (0xbde068, 0x200 bytes), 1
the quest text (0xbde288), 2 the progress text (0xbdf0f8), 3 the reward text. The Lua getters return the
buffers: `GetTitleText` 0xbdf900, `GetGreetingText` 0xbde068, `GetQuestText` 0xbde288, `GetObjectiveText`
0xbdf500, `GetProgressText` 0xbdf0f8 and `GetRewardText` 0xbdf958.

## The gossip point of interest

Opcode 0x224, `0x4e2840`: `u32 flags`, `f32 x`, `f32 y`, `u32 icon`, `u32 data`, a C string of up to 0x40 bytes. It
stores them in the second static blip with the setter `0x6dac10` (`minimapblips#setting-a-static-blip`):
the flags, position, icon and `data` as the importance, and the name copied (63 characters and a terminator). It
fires no event itself; the setter's chain ends in `WORLD_MAP_UPDATE`.

Addresses: `0x4e2840`, `0x4e2847`, `0x4e2850`, `0x4e285b`, `0x4e2866`, `0x4e2871`, `0x4e287c`, `0x4e2889`, `0x4e288e`, `0x4e28aa`, `0x4e28b8`, `0x6dac10`, `0x4e1f1e`, `0x418e30`, `0x419130`, `0x4191b0`
Confidence: verified
Proof: one
Test: none

The reads are `0x418e30` (flags), `0x419130` twice (x, y), `0x418e30` twice (icon, then `data` into the argument
slot that held the packet pointer, which `esi` still holds) and `0x4191b0(buf, 0x40)`. The call is
`0x6dac10(1, flags, x, y, icon, data, name)` (`0x4e28a5` loads slot 1).

## The innkeeper bind

`SMSG_BINDER_CONFIRM` (opcode 0x2eb) is a bare `u64` binder GUID, handled by an arm of `0x5e38c0` that calls the
player's `0x5dfdc0`. If the binder is within interaction range, the GUID is latched in `0xc4d7c0` and the
event `CONFIRM_BINDER` (id 513) fires with the player's current area name: the sub-area's, else the zone's,
else the global `HOME_INN`. Out of range, nothing happens. `ConfirmBinder()` calls the same function with a
zero GUID, which re-checks the range against the latched GUID and sends opcode 0x1b5 (`CMSG_BINDER_ACTIVATE`:
the latched GUID); with nothing latched or out of range it sends nothing. Nothing clears the latch.

Addresses: `0x5dfdc0`, `0x5dfdd7`, `0x5dfdf5`, `0x5dfdfe`, `0x5dfe3c`, `0x5dfe64`, `0x5dfe70`, `0x5dfe5e`, `0x5dfec3`, `0x5dfef8`, `0x5dff0d`, `0x5dff18`, `0x5dff3a`, `0x5e4aa2`, `0x5e4aa9`, `0x5e4ad3`, `0x5e3732`, `0x48dd00`, `0x48dd46`, `0x48d210`, `0x5dff50`, `0xc4d7c0`, `0xc4c28c`, `0xb4e314`, `0xb4e318`, `0x703f50`
Confidence: verified
Proof: one
Test: none

The range test is `fcomp [0xc4c28c]; fnstsw; test ah,0x41; je` (`0x5dfe3c`), taken out of range when the squared
distance exceeds the threshold, so equality passes and a NaN passes too; `0xc4c28c` is `(50/9)^2 =
30.864195` computed by the static initializer `0x5f1350`. The sibling `0x5dff50` (behind `CheckBinderDist`) tests
with `jp` and sends a NaN out of range. The area is the leaf area at `0xb4e318`, else the zone at `0xb4e314`, by
`[rows + id*4]` with the name at `+0x2c + 4*locale`. The packet is `push 0x1b5`, `0x418190`, the two latch dwords by
`0x418370`, `0x5ab630`; the binding pushes no Lua value, and no handler is registered for opcode 0x1b5. Two sibling
latches serve the talent master (`0xc4d7a0`) and the pet untrainer (`0xc4d7b0`).
