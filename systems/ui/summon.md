# ui: the summon confirmation

A summon request from another player latches the summoner, the zone and a deadline, signals `CONFIRM_SUMMON`, and three
Lua functions read the latch for the dialog; `ConfirmSummon` answers it. There is no function to decline.

Roots: `0x4963a0`, `0x48b660`, `0x48b6a0`, `0x48b720`, `0x48b770`

## The request

Opcode 0x2ab, `0x5e6140`: `u64 summonerGuid`, `i32 zoneId`, `u32 delayMs`, 16 bytes. The handler drops the request, with no
state and no event, when the active player object exists and is dead or a ghost, and keeps it when the object is missing. Otherwise
`0x4963a0(zone, delay, guid)` stores the GUID (`0xb4e358`, `0xb4e35c`), the zone (`0xb4e354`) and the deadline `now + delay`
(`0xb4e350`) and signals `CONFIRM_SUMMON` (event 516) with no argument. The handler returns 1. A second request overwrites the first.

Addresses: `0x5e6140`, `0x5e6150`, `0x5e615b`, `0x5e6166`, `0x5e6181`, `0x5e618d`, `0x5e6194`, `0x5e61a4`, `0x605f30`, `0x4963a0`, `0x4963ac`, `0x4963b7`, `0x4963bd`, `0x4963c9`, `0x4963ce`, `0xb4e350`, `0xb4e354`, `0xb4e358`, `0xb4e35c`
Confidence: verified
Proof: one
Test: none

`0x605f30` is true for a unit with health at or below 0, or for a player whose `PLAYER_FLAGS` has the ghost bit 0x10, so a
ghost is refused although its health is positive. `0x5e616b` pushes the source line `0x649` for the object lookup, not an argument of
`0x468550`. The event has the one fire site `0x4963ce`; `0x703e50` takes the id in `ecx` and no stack argument. Nothing clears the GUID
or the zone; the deadline alone is zeroed by the reset block of `0x48fbf0` (`0x49014a`), which runs when the interface is
loaded.

## The Lua functions

`GetSummonConfirmTimeLeft()` pushes one number, the milliseconds left (`0x4963e0`: deadline minus now, 0 when unset or past)
divided by 1000 and truncated, 0 when nothing is pending. `GetSummonConfirmSummoner()` pushes one string: the summoner's cached name,
or the empty string; a miss starts a name query whose callback `0x48b710` does nothing, so the name shows only when the dialog
asks again. `GetSummonConfirmAreaName()` pushes `AreaName_lang[locale]` of the zone (`+0x2c + 4*locale` of the AreaTable row), or the empty string for a
negative or oversized id or a missing row. None takes an argument or raises.

Addresses: `0x48b660`, `0x48b669`, `0x48b670`, `0x48b677`, `0x4963e0`, `0x48b6a0`, `0x48b6c3`, `0x48b6e1`, `0x48b6ec`, `0x48b710`, `0x48b720`, `0x48b727`, `0x48b72f`, `0x48b73c`, `0x48b744`, `0x48b753`, `0x55f080`, `0xc0e228`, `0xc0e048`, `0xc0e04c`
Confidence: verified
Proof: one
Test: none

The division is the unsigned multiply by `0x10624dd3` and `shr 6`. `0x55f080(0xc0e228, guid, &guid, 0x48b710, 0, 0)` returns the
record's name pointer when it has been answered, 0 otherwise, and sends the name query for an unknown GUID. The pushed pointer
is `0x882748` (the shared empty string) on a miss, so the binding never returns nil, except for a row whose name pointer is itself null.
The area bounds are signed compares against `0xc0e04c`.

## Answering

`ConfirmSummon()` reads no argument, returns no value and sends opcode 0x2ac followed by the latched summoner GUID, 8 bytes,
with no accept flag. It checks nothing: it sends again on every call, with the deadline passed, or with GUID 0 when no request
ever arrived, and it clears nothing.

Addresses: `0x48b770`, `0x48b7a8`, `0x48b7b0`, `0x48b7b5`, `0x48b7ba`, `0x48b7c5`, `0x48b7d0`
Confidence: verified
Proof: one
Test: none

The body has no compare apart from the data store's own teardown test. `tables/bindings.tsv` lists four summon functions and none declines a
request; the dialog's cancel only hides it. The server ends a request by its own delay, which the client receives in the
request and turns into the deadline.
