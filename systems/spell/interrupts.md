# spell: interrupts

What the player's own input does to a cast in flight: movement commands, jumping, and the stop-casting binding.

The cancel `0x6e4940` and the channel cancel `0x6e9b70` (`cancel`) are called from the movement code and from
a Lua binding. This topic states the conditions on those callers, which `cancel` lists but does not
decide: which commands interrupt, what the spell record must say, what is sent and what is left for the
server to answer. The movement dispatcher and the jump binding belong to `input`; the binding `0x6e6e80`
is spell's.

Roots: `0x6e6e80`, `0x6e9b70`, `0x6e4940`

## Movement commands cancel a cast

A command press that moves the player cancels the cast in flight: forward, backward, strafe left, strafe
right and auto-run (the command bits 0x10, 0x20, 0x40, 0x80 and 0x1000), and a press of both mouse buttons
(commands 0x1 and 0x2 held together). Turn and pitch commands (0x100 to 0x800) and every release do not.
The cancel runs when a spell is in flight, no spell awaits a target, and the record's `InterruptFlags`
(`+0x54`) has bit 0: it is `0x6e4940(0, 1, 0x1c)`, which sends `CMSG_CANCEL_CAST` and fires `SPELLCAST_STOP`.
The jump binding runs the same two steps before its own checks.

Addresses: `0x515090`, `0x5150c0`, `0x5150c6`, `0x5150ce`, `0x5150d6`, `0x5150df`, `0x5150f2`, `0x5150fd`, `0x51511e`, `0x51512a`, `0x513bd0`, `0x513c09`, `0x513c14`, `0x513c36`, `0x513c43`, `0x513e20`, `0x513e80`, `0x513fa0`, `0x514000`, `0x513de0`, `0x513ee0`, `0x513f40`, `0x514060`, `0x5140c0`, `0x6e4940`, `0x6e3d10`, `0x6e48a0`
Confidence: verified
Proof: one
Test: none

The command handlers push their command bit, a press of 1 and call `0x515090`: `MoveForwardStart`
`0x10`, `MoveBackwardStart` `0x20`, `StrafeLeftStart` `0x40`, `StrafeRightStart` `0x80`, `ToggleAutoRun`
`0x1000`, `TurnLeftStart` `0x100`, `TurnRightStart` `0x200`, `PitchUpStart` `0x400` and `PitchDownStart`
`0x800`. After the press handler returns nonzero (`0x5150c0`), `0x5150c8` skips a release, `0x5150ce`
tests `commands & 0x10f0`, and `0x5150d6 test bl,3` with both bits 0 and 1 of the held word `[edi+4]`
(`0x5150e2`, `0x5150ea`) takes the two-button case. `0x5150f2` reads `0x6e3d10`; `0x5150fd` skips the
cancel when `0x6e48a0` holds; the record is bounded by the store's maximum; `0x51511e test [rec+0x54],1`;
`0x515124` pushes 0x1c, `dl` 1, `cl` 0 for `0x6e4940`. `0x513c36` to `0x513c43` are the same test and call
in `0x513bd0`, entered through its own gate `0x494a50(0)` and the mover lookup.

## Movement cancels a channel

The same two callers then cancel a channel: when the active player's channel spell (`+0x228` of the unit
field block) has `ChannelInterruptFlags` (`+0x5c`) with bit 3 and the player is the mover the movement code
keeps, they call `0x6e9b70`. That sends `CMSG_CANCEL_CHANNELLING` with the spell id and changes nothing
locally: no event fires and the cast state stays, so the stop event comes from the channel update
handler when the server answers with 0 (`handlers`).

Addresses: `0x51512f`, `0x51514e`, `0x515154`, `0x515175`, `0x515180`, `0x51518b`, `0x515193`, `0x513c48`, `0x513c67`, `0x513c6d`, `0x513c8e`, `0x513c99`, `0x513ca4`, `0x513cac`, `0x6e9b70`, `0xc4da98`, `0xc4da9c`
Confidence: verified
Proof: one
Test: none

`0x51512f` resolves the active player with mask 0x10 and `0x515154` loads `[[player+0x110]+0x228]`; zero,
negative, above the store's maximum or a null record end the block. `0x515175 test [rec+0x5c],8`;
`0x515180` and `0x51518b` compare the player's guid with the dword pair `0xc4da98`, `0xc4da9c`. The
jump binding repeats the block at `0x513c48` to `0x513cac`. `0x6e9b70` builds the packet and sends it and
has no other effect (`cancel`, Channel and aura cancels).

## Stop casting binding

`SpellStopCasting` (`0x6e6e80`) returns one value. If the UI action gate `0x494a50(2)` refuses it returns nil. If an
auto-repeat spell runs it cancels that (`0x6ea080`), returns the number 1 and does nothing else. Otherwise, with a
spell in flight (a queued on-next-swing spell holds the slot, `attack`), it aborts with `0x6e4940(0, 1, 0x1c)` and
returns 1; else nil. The abort sends `CMSG_CANCEL_CAST` only when a spell is in flight, `dl` is nonzero and no targeting word is up (with a
word up, `0x6e4955` only clears it). With `cl` 0, as here, it fires `SPELLCAST_STOP`, only for a player caster. The binding never
reaches `0x6e9b70`, and does not read the targeting word itself; the abort does (`0x6e494a`, `0x6e48a0`).

Addresses: `0x6e6e80`, `0x6e6e88`, `0x6e6e91`, `0x6e6e9a`, `0x6e6e9f`, `0x6e6ea8`, `0x6e6eb4`, `0x6e6ec3`, `0x6e6ec8`, `0x6e6edd`, `0x6e6edf`, `0x6f3810`, `0x6f37f0`, `0x494a50`, `0x6e9fd0`, `0x6e3d30`, `0x6ea080`, `0x6e4940`
Confidence: verified
Proof: one
Test: none

`0x6e6e88` calls `0x494a50` with 2; a zero jumps to the nil exit `0x6e6edd`. `0x6e6e91` calls `0x6e9fd0`;
nonzero goes to `0x6e6e9a`, `0x6ea080`, then pushes the double 1.0 (`0x3ff00000` high word, `0x6e6e9f`) with
`0x6f3810` and returns 1 value. `0x6e6eb4` calls `0x6e3d30`; zero goes to the nil exit; else `0x6e6ebd`
pushes 0x1c with `dl` 1 and `cl` 0 for `0x6e4940` (`0x6e6ec3`) and pushes 1.0. The nil exit calls
`0x6f37f0` (`0x6e6edf`). All three paths set `eax` to 1, the count of values pushed. The binding's
call closure is `0x494a50`, `0x6e9fd0`, `0x6ea080`, `0x6e3d30`, `0x6e4940` and the two push functions.
Lua truthiness is the caller's: a pushed 1.0 is true and nil is false. The binding `ClearTarget`
(`0x489ff0`) pushes 1.0 when the selection `0xb4e2d8` is nonzero at entry, else nil, and always clears the
selection (`0x493540`).

In `0x6e4940`, `0x6e4968` tests the result of `0x6e3d30`, `0x6e496c cmp dl,bl` the send flag, and `0x6e4a37` tests the
saved `cl` to pick event 0x152, or 0x153 and 0x154.
