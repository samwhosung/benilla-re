# spell: expiry

Nothing in the client fires when a cooldown ends.

The spell history (`cooldowns`) records start and length; a reader computes what remains from the clock when
it asks. The cooldown events are signalled when the history changes, never when time passes. This topic
records the census that shows it, so a client does not add a timer or an end-of-cooldown event.

Roots: `0x6e1910`

## Cooldown expiry is lazy

The four cooldown events have one signalling site each: `ACTIONBAR_UPDATE_COOLDOWN` (216) at `0x4e5c65`,
`SPELL_UPDATE_COOLDOWN` (262) at `0x4b31ba`, `BAG_UPDATE_COOLDOWN` (330) at `0x4f93d5` and
`PET_BAR_UPDATE_COOLDOWN` (354) at `0x4bce95`. Their callers all follow a change of a history list, except one
that follows a change of an item's charges field. The expiry sweep `0x6e1910` has two callers: the global
cooldown clear and the go's pruning, at most once in 120 seconds. None of these callers is a timer or a
per-frame update.

Addresses: `0x5d9540`, `0x5d9587`, `0x5d9360`, `0x5d937e`, `0x5d9398`, `0x5d9564`, `0x5d95a0`, `0x468070`, `0x4bce90`, `0x4f93d0`, `0x6e2e8e`, `0x6e2f8e`, `0x6e3080`, `0x6e3089`, `0x6e8505`, `0x6e8570`, `0x6e8601`, `0x6e89ad`, `0x6e95b9`, `0x6e965b`, `0x6e971c`, `0x6e9721`, `0x4e5c60`, `0x4e5c65`, `0x4b31b0`, `0x4b31ba`, `0x4f93d5`, `0x4bce95`, `0x6e1910`, `0x6e1678`, `0x6e862b`, `0x6e167d`, `0x6e2e77`, `0x6e2f89`, `0x6e3071`, `0x6e849d`, `0x6e8500`, `0x6e856b`, `0x6e85fc`, `0x6e89a8`, `0x6e95b0`, `0x6e9656`, `0x6e9712`
Confidence: verified
Proof: one
Test: none

`tables/event_sites.tsv` lists one signal site per event. `0x4e5c60` has one caller, the flush
`0x4b31b0`, whose twelve callers (calls only) are the global cooldown clear
and start, the item record recompute, the start or removal of a spell's records, the go's four own-cooldown
legs, the late item cooldown, the two cooldown packets and the list clear. `0x4bce90` has five callers (calls
`0x6e2e8e`, `0x6e3080`, `0x6e8601`, `0x6e95b9`; jump `0x6e971c`) and `0x4f93d0` eight (calls `0x5d9587`,
`0x6e2f8e`, `0x6e3089`, `0x6e8505`, `0x6e8570`, `0x6e89ad`, `0x6e965b`; jump `0x6e9721`); `0x5d9587` is in the
watcher `0x5d9540`, which `0x5d9360` registers at `0x5d937e` through `0x468070(1, 0x28, 0x14, ...)`: section 1
is the item block, which starts at the owner guid (`0x5d9564` compares it with the active player's), so offset
0x28 is dword 10 of the block, `ITEM_FIELD_SPELL_CHARGES` (dword 16 of the fields), five dwords long. The watcher
runs on a change of those bytes for the active player's item. `0x5d95a0` is registered at `0x5d9398` on offset
0x40, length 0x54: the 21 enchantment dwords.
`0x6e1910` has two callers (`0x6e1678`, `0x6e862b`). No dword of these addresses is stored. The
sweep in the go is guarded by a time stored in `0xcee8b8` (`cooldowns`, Expiry sweep and whole lists; `go`,
Pet cooldown and pruning).
