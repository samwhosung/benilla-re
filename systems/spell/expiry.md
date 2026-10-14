# spell: expiry

Nothing in the client fires when a cooldown ends.

The spell history (`cooldowns`) records start and length; a reader computes what remains from the clock when
it asks. The cooldown events are signalled when the history changes, never when time passes. This topic
records the census that shows it, so a client does not add a timer or an end-of-cooldown event.

Roots: `0x6e1910`

## Cooldown expiry is lazy

The four cooldown events have one signalling site each: `ACTIONBAR_UPDATE_COOLDOWN` (216) at `0x4e5c65`,
`SPELL_UPDATE_COOLDOWN` (262) at `0x4b31ba`, `BAG_UPDATE_COOLDOWN` (330) at `0x4f93d5` and
`PET_BAR_UPDATE_COOLDOWN` (354) at `0x4bce95`. `0x4e5c60` has one caller, the flush `0x4b31b0`, whose twelve
callers all follow a change of a history list: the global cooldown clear and start, the item record
recompute, the start or removal of a spell's records, the four own-cooldown legs of the spell go, the late item
cooldown, the cooldown and item cooldown packets and the list clear. The expiry sweep `0x6e1910` has two
callers: the global cooldown clear and the go's pruning, at most once in 120 seconds. None of these callers is a timer or a per-frame
update.

Addresses: `0x4e5c60`, `0x4e5c65`, `0x4b31b0`, `0x4b31ba`, `0x4f93d5`, `0x4bce95`, `0x6e1910`, `0x6e1678`, `0x6e862b`, `0x6e167d`, `0x6e2e77`, `0x6e2f89`, `0x6e3071`, `0x6e849d`, `0x6e8500`, `0x6e856b`, `0x6e85fc`, `0x6e89a8`, `0x6e95b0`, `0x6e9656`, `0x6e9712`
Confidence: verified
Proof: one
Test: none

`tables/event_sites.tsv` lists one signal site for each of the four events. The scan of the listing for
`call 0x4b31b0` finds 12 sites (`0x6e167d`, `0x6e2e77`, `0x6e2f89`, `0x6e3071`, `0x6e849d`, `0x6e8500`, `0x6e856b`,
`0x6e85fc`, `0x6e89a8`, `0x6e95b0`, `0x6e9656`, `0x6e9712`) and no `jmp`; for `call 0x6e1910` two (`0x6e1678`,
`0x6e862b`); the image holds no dword of either address. `0x4b31b0` itself calls `0x4e5c60` and signals 262. The
sweep is guarded in the go by a time stored in `0xcee8b8` (`cooldowns`, Expiry sweep and whole lists; `go`,
Pet cooldown and pruning).
