# spell: reasons

Which cast-failure reasons the client raises itself, what the combat chat says about a failure, and the failures that show nothing.

Every failure reaches the reporter `0x6e1a00` (`failures`) with a reason byte: from a refusal in the client's
own code or from the server's `SMSG_CAST_RESULT` (`handlers`). The reporter shows a message (`failures`, Message
id per reason) and also writes a line in the chat through the combat log formatter `0x62c360`. This topic
lists the local sources of the reasons, the line's text and its gates, and the paths that end before any
display. The reason numbers are the wire's; the keys they select are the `SPELL_FAILED_` names of
`failures`, Reason key switch.

Roots: `0x6e1a00`, `0x6e7330`

## Reasons raised locally

The client raises 53 of the 145 reasons itself, at 61 of the 63 calls of `0x6e1a00`. The other two take a byte
from elsewhere: the cast result handler passes the packet's reason, and the validator's form rung passes the
form check's out byte (0x3d or 0x56). Every other reason, 0x5d and 0x5e among them, reaches the screen only from
the server.

| source | calls | reasons |
|---|---|---|
| validator | 40 | 0, 4, 5, 0x12, 0x14, 0x16, 0x17, 0x1e, 0x28, 0x2e, 0x39, 0x3c, 0x3d, 0x3e, 0x41, 0x4b, 0x4c, 0x4d, 0x50 to 0x58, 0x5a, 0x60, 0x61, 0x63, 0x64, 0x8d, 0x8f |
| cast request | 9 | 9, 0xa, 0x12, 0x13, 0x18, 0x2d, 0x46, 0x61, 0x82, 0x83 |
| equipped-item check | 4 | 0x19, 0x1a, 0x1b, 0x2f |
| totem and reagent check | 2 | 0x5c, 0x78 |
| range pre-check, bind range check | 2 | 0x59, 0x76 |
| `SpellTargetUnit` | 1 | 0x59 |
| item validator | 4 | 0xa, 0xd, 0x84, 0x90 |
| cast result handler | 1 | the packet's byte |

Addresses: `0x6e73ac`, `0x609e64`, `0x6e4c68`, `0x6e4cbf`, `0x6e4cf2`, `0x6e4d31`, `0x6e4dd4`, `0x6e4097`, `0x6e40c1`, `0x6e4195`, `0x6e41e4`, `0x6e4350`, `0x6e4375`, `0x6e4737`, `0x6e4e3e`, `0x6e4eab`, `0x6e4ec4`, `0x6e5064`, `0x6e6089`, `0x6e6e20`, `0x496077`, `0x496106`, `0x49612c`, `0x496152`, `0x6e1a00`
Confidence: verified
Proof: one
Test: none

A scan of the listing for `call 0x6e1a00` and `jmp 0x6e1a00` finds 63 sites (no `jmp`); for each, the
nearest preceding load of `dl` gives its reason, and two are not constants (`0x609e64` reads `[ebp-0x18]`,
`0x6e73ac` reads `[ebp-0x1]`). Sites that share a call take two reasons: `0x6e4737` 0x59 or 0x76, `0x6e6089`
the same, `0x6e5064` 0x2d, 9 or 0xa, `0x6e4195` 0x19 or 0x1a, `0x6e4ec4` 0x12 (0x46 has its own call at `0x6e4eab`), `0x609dd1` 4 or 0x50 (5 is reported at `0x6098d6`) and
`0x609fbe` 0x4b or 0x55. No site loads 0x5d or 0x5e. The reasons only the server sends include every
target-state reason (0x65 to 0x75, 0x7f to 0x81) and 0x8e. The client passes 0x1c to its cancels (`cancel`)
but never reports it.

## Packets of a local refusal

The reporter aborts with `0x6e4940(1, !fromServer, reason)` whenever the reported spell is the in-flight one. The
abort sends `CMSG_CANCEL_CAST` only when a spell is in flight (`0x6e3d30`), `dl` is nonzero (`0x6e496c`) and no
targeting word is up; with a word up it only clears it. Either way it pops the slot and resets the cursors. For a
player caster it zeroes the caster guid and fires an event by `cl`: a refusal passes `cl` 1, so `SPELLCAST_FAILED`
(`SPELLCAST_INTERRUPTED` for 0x23 and 0x24), not `SPELLCAST_STOP`.
The player's auto-repeat spell is stopped first (`CMSG_CANCEL_AUTO_REPEAT_SPELL`), so no cancel follows. Reason
0x17 does neither.

Addresses: `0x6e1c67`, `0x6e1c7a`, `0x6e1cd9`, `0x6e1ce1`, `0x6e1cea`, `0x6e2201`, `0x6e2209`, `0x6e220e`, `0x6e2218`, `0x6e4940`, `0x6ea080`, `0x6ea14e`, `0x6ea154`, `0x6ea160`
Confidence: verified
Proof: one
Test: none

Reports of the in-flight spell: the item validator (0xa, 0xd, 0x84, 0x90, with the word up), `SpellTargetUnit`
(0x59), the request's 0x13, 0x82, 0x83 and 0x61 for a pressed spell already in flight (before the same-spell
bail at `0x6e4d43`), the arm (9, 0xa, 0x2d) and the binder's range failure. Refusals after the bail report a
spell not yet pushed. `0x6e1c7a` jumps over the auto-repeat leg for an active player that is missing or not a
player. `0x6e1cd9` compares `0xceac30` with the spell, `0x6e1ce1` leaves for reason 0x17, and `0x6e1cea` calls
`0x6ea080`, which pops the in-flight slot when `0xceac30` equals it (`0x6ea154`, `0x6ea160`) before clearing the
auto-repeat. `0x6e2201 cmp esi,[0xceca88]` follows (`esi` reloaded from the spell id at `0x6e21f4`); `0x6e220e`
loads the from-server byte and `sete dl`, and `0x6e2218` calls the abort with `cl` 1.

## Failure chat line

After it shows the message, the reporter writes a combat log line with `0x62c360(player, spell, text)`. The
text is the reason's own string when that is not empty, else the last message displayed. A local refusal
writes the line as the server's failure does. The key is one of four, by the spell's `Attributes` bit 4
and whether the caster is the active player: `SPELLFAILCASTSELF`, `SPELLFAILCASTOTHER`, `SPELLFAILPERFORMSELF`
or `SPELLFAILPERFORMOTHER`. For a reason that changes the message id (0x9, 0x18, 0x28, 0x3c, 0x41, 0x59 and
the item form of 0xa) the line carries the key text, where the message shows the id's own sentence. For 0x2 it
carries the bare power noun, and for 0x4d the message again.

Addresses: `0x6e21dd`, `0x6e21e2`, `0x6e21ee`, `0x6e21fc`, `0x6e1d9a`, `0x6e21d2`, `0x6e20fd`, `0x6e2113`, `0x62c360`, `0x62c4e2`, `0x62c4ee`, `0x62c4fb`, `0x62c502`, `0x62c509`, `0x62c512`, `0x62c582`, `0x62c5b3`, `0x4968a0`, `0x62aff0`, `0x5fa6d0`, `0x62b190`
Confidence: verified
Proof: one
Test: none

`0x6e1d9a` points `edi` at the first buffer; only the fill path repoints it, at `0x6e21d2`, which comes after
the print call. `0x6e21e2 mov al,[edi]` tests the first byte: nonzero pushes `edi`, zero calls `0x4968a0`
(`0x6e21ee`). The `0x4d` arm clears its buffer's first byte (`0x6e20fd`, `0x6e2113`). `0x4968a0` returns
`0xb4da40`, which `0x496720` fills at each display (`failures`, Display, log and events). `0x62c4e2` asks
`0x5fa6d0` of the caster and `0x62c4ee` `0x62aff0` (bit 4 of `Attributes`); `0x62c4fb`, `0x62c502`, `0x62c509`
and `0x62c512` load the four keys. The chat type is 0x50 when the caster's relation class (`0x5efea0`) is 0 or 1,
else `0x62b190` of the spell (`0x62c582`).

## Failure chat line gates

`0x62c360` writes nothing when any of these holds: no player or a spell id of 0; the caster's unit or the
active player's object is missing; the caster is not strictly within the combat log range of its relation
class (a local refusal has distance 0, so it passes); the spell id is negative, above the store's maximum or
has no record; the record's `Attributes` has bit 7 or 8 (0x180); the spell's name is empty. A missing format
string writes a warning line under chat type 0x19 instead.

Addresses: `0x62c386`, `0x62c38e`, `0x62c3dd`, `0x62c416`, `0x62c47d`, `0x62c497`, `0x62c4a3`, `0x62c4b7`, `0x62c4bd`, `0x62c4d7`, `0x62c530`, `0x62c538`, `0x62c5d1`, `0x6269f0`, `0x626810`
Confidence: verified
Proof: one
Test: none

`0x62c386` and `0x62c38e` test `ecx` and `edx` (player, spell) and jump to the exit `0x62c5d8`. `0x62c3d4`
looks the caster's unit up by guid with mask 8 and `0x62c40d` the active player with mask 0x10; a null jumps
to the exit. The range is `0x626810` of the relation class from `0x5efea0`, with the `CombatDeathLogRange`
CVar looked up first (`objectlayer`, The one-sided range test); `0x62c476 fcompp` with `test ah,0x41; jne
0x62c5d8` leaves when `r² ≤ d²` or the compare is unordered. `0x62c4bd test [rec+0x18],0x180; jne` and
`0x62c4d7 cmp byte [eax],0` are the record gates, with the name taken from the locale column at `+0x1e0`.
`0x62c530` and `0x62c538` send a null or empty format string to `0x62c5d1`.

## Failures that show nothing

Some failures end before the message: reason 0x17 (the reporter returns twice, before and after cancelling
an auto-repeat); 0x5c and 0x78 when the item is not in the cache (its callback shows the message later, with no
chat line); 0x84 on the same miss; 0x56 when the spell has no record, the form store has no records or no form
name is selected; and 0x2 when the power type byte is 5 or more or the noun text is null or empty. None
writes the chat line. A failure whose displayed text is empty fires no error event, because the error sink
returns on an empty text, though the line is still written.

Addresses: `0x4945b0`, `0x4945b4`, `0x6e1ce1`, `0x6e1ce4`, `0x6e1cf7`, `0x6e1cfa`, `0x6e1eab`, `0x6e1efc`, `0x6e1ffd`, `0x6e2016`, `0x6e20c6`, `0x6e212b`, `0x6e2134`, `0x6e216d`, `0x6e2176`, `0x6e224f`, `0x6e222f`, `0x6e2236`
Confidence: verified
Proof: one
Test: none

The jumps to the epilogue `0x6e224f` before the display call `0x6e21dd` are `0x6e1ce4` and `0x6e1cfa`
(0x17), `0x6e1eab` (cache miss for 0x5c and 0x78), `0x6e1efc` (0x84), `0x6e1ffd`, `0x6e2016` and
`0x6e20c6` (0x56), and `0x6e2134`, `0x6e216d` and `0x6e2176` (0x2); the `jl` at `0x6e212b` follows a
`movzx` and never jumps. `0x6e222f` and `0x6e2236` are after the line: the event is skipped while a spell
is in flight and for reason 0x61. `0x4945b0` (`CGGameUI::AddErrorMessage`) returns at `0x4945b4` when the
text is empty.
