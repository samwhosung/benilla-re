# spell: attack

How casting meets the melee swing and the ranged auto-repeat: the attack a cast starts, the queued strike that holds the in-flight slot, the routes into the auto-repeat and the sweep of a ranged shot.

An on-next-swing spell (Heroic Strike, Raptor Strike), a melee ability and a ranged shot are cast through the
same request as any spell (`cast`), but each also changes the attack or the auto-repeat state. The melee
swing itself (`0x5ecb70`, `0x6131a0`) belongs to `objectlayer`; this topic states what the spell side asks of
it, in which order, and what the server's replies leave in the cast slots (`handlers`). The state is the
in-flight slot `0xceca88`, the saved slot `0xcecaa8`, the auto-repeat spell `0xceac30`, and the last sent
spell `0xcead5c`.

Roots: `0x6e4b60`, `0x6e5200`, `0x6e5230`, `0x6ea080`, `0x6e2b60`

## Melee start after a cast

The cast request starts the melee swing at the end of a send when three conditions hold. First, the spell
has `Attributes` 0x404 or `AttributesEx` 0x200, and `AttributesEx2` bit 20 is clear (a spell with that bit
defers the start to the spell go, `go`). Second, the request reached its send, which only the send sets
(`0xcead5c` equals the spell id). Third, the player is not yet attacking. When the player is not attacking, the
request first asks `0x612df0` for a target; if that fails the whole cast returns 0 before anything is
sent. The swing starts on the cast's target guid, or on the selection or the nearest hostile when it is 0.

Addresses: `0x6e4ed6`, `0x6e4edf`, `0x6e4eeb`, `0x6e4efb`, `0x6e4f02`, `0x6e4f0a`, `0x6e4f13`, `0x6e4fe6`, `0x6e51b5`, `0x6e51b8`, `0x6e51c2`, `0x6e51cb`, `0x6e51de`, `0x6e585a`, `0xcead5c`, `0x6e5200`, `0x6e5230`, `0x612df0`, `0x60ecb0`, `0x6131a0`
Confidence: verified
Proof: one
Test: none

`0x6e4edb` clears the flag byte `[ebp-2]`; `0x6e4edf` calls `0x6e5200` and skips on 0. `0x6e4eeb` calls
`0x60ecb0`; nonzero (attacking) jumps past the pick. `0x6e4efb` pushes the guid pair at `[ebx+0xc]` and
calls `0x612df0`; `0x6e4f02 je 0x6e506e` returns 0. `0x6e4f0a` calls `0x6e5230` (bit 20 alone), and only 0
reaches `mov byte [ebp-2],1` (`0x6e4f13`). `0xcead5c` has three references: cleared at `0x6e4fe6`, set to
the spell id at `0x6e585a` just before the send, read at `0x6e51b8`. The tail then compares it with the
record's id (`0x6e51be`), tests the flag (`0x6e51c5`) and `0x60ecb0` (`0x6e51cb`) and calls `0x6131a0`
(`0x6e51de`). The melee start ends in `0x6ea080` (`objectlayer`, The melee attack), so this tail is how a spell
press ends a running auto-repeat: when the player is already attacking, the tail is skipped and the
auto-repeat runs on.

## Queued strike holds the slot

A spell pressed from a button, key or macro reaches the cast request with no item object, so an on-next-swing
spell is pushed into the in-flight slot, and `IsCasting` (`0x6e3d30`) is true while it waits for the swing.
The already-casting refusal exempts a new cast when the in-flight record has `Attributes` 0x404. A new cast
pushes the queued one into the saved slot and its cast result pops it back. Nothing the spell go or start
handlers do writes either slot; the cast result for the strike, or an abort, clears it.

Addresses: `0x6e4ff5`, `0x6e5002`, `0x6e500b`, `0x6e500f`, `0x6e5026`, `0x6e4d97`, `0x6e4dbe`, `0x6e4ad0`, `0x6e4af8`, `0x6e4b3d`, `0x6e72e7`, `0x6e3d30`, `0x6e5a90`, `0x6e58a2`, `0x6e58bd`, `0x6e4fd3`, `0x6e4fdc`, `0xceca88`, `0xcecaa4`, `0xcecaa8`
Confidence: verified
Proof: one
Test: none

`0x6e5002 test [edx+0x18],0x404` forces the push for such a spell; both paths reach `0x6e500b`, which tests the
item, and only a nonzero item continues to the item leg at `0x6e500f`. Of the 15 call sites of the entry `0x6e5a90` that forwards the item
(`cast`, Cast entries), 13 clear `edx` first; the other two are the item use `0x5d9258` and the game
object use `0x5f35c0`; the pet bar calls the request with a zero item. `0xceca88` has 14 references in the
image (the listing and the dword scan agree) and three writes: the push `0x6e4af8`, the pop `0x6e4b3d` and
the start's zero `0x6e72e7`; the `rep stos` at `0x6e4f80` ends 0x28 bytes below it. `0xcecaa4` has three
references: two writes in the request (`0x6e4fd3`, `0x6e4fdc`: 1 for an active-player caster with a null
item, else 0) and the read at `0x6e58a2` that gates the send's pop, so the player's own cast is not
popped by its send.

## Cast over a queued strike

A cast started over a queued strike pushes the strike into the saved slot and takes the in-flight slot. The
pop restores the strike and leaves the saved copy in place, so a second abort restores it again. The server's
cast result for the new spell finds the saved slot unmatched, restores the strike by the abort and keeps the
saved copy; the strike's own result clears the saved slot first when it is saved, or pops when it is in flight.

Addresses: `0x6e4ad0`, `0x6e4ad5`, `0x6e4adf`, `0x6e4af8`, `0x6e4b2c`, `0x6e4b3d`, `0x6e49d6`, `0x6e73ee`, `0x6e73f6`, `0x6e7408`, `0x6e741a`, `0xcecaa8`, `0xcecb20`
Confidence: verified
Proof: one
Test: none

`0x6e4ad5 cmp eax,ecx; je` ends a push of the in-flight spell. The push (`0x6e4adf` to `0x6e4af8`) copies the
in-flight id and item guid into `0xcecaa8` and `0xcecb20` and stores the new id; the pop (`0x6e4b2c` to
`0x6e4b3d`) loads them back and stores nothing in the saved slot. The pop is the abort's tail (`0x6e49d6`).
`0x6e73ee` compares the result's spell with `0xcecaa8` and `0x6e73f6` zeroes the slot and its guid;
`0x6e7408` compares with the in-flight id and `0x6e741a` aborts with `cl` and `dl` 0, after the saved-slot test.

## Routes into the auto-repeat

The auto-repeat spell id `0xceac30` is written in three places: the send's auto-repeat arm (`cast`, After the
send), the start's zero and `0x6ea080`'s clear. The send has five callers, all taking the pending cast
record, so a cast reaches the auto-repeat only as a request whose record has `AttributesEx2` bit 5. The
requests come from presses, from the chained cast (`handlers`, In-flight cast reply) and from the feed-pet
cast `0x6ea1e0`. A melee start never begins an auto-repeat; its tail (`0x5ecd8c`) ends one. `0x6ea080` has nine
callers: the action-button toggle `0x4e60da`, the melee start `0x5ecd8c`, `0x605906`, the validator
`0x6095ec`, the failure reporter `0x6e1cea`, the stop binding `0x6e6e9a`, the spell-failure handler
`0x6e8e0d`, the thunk `0x6e99d0` and the unreached `0x6ec38b`.

Addresses: `0xceac30`, `0x6e594c`, `0x6e730b`, `0x6ea16a`, `0x6e5349`, `0x6e5467`, `0x6e60d7`, `0x6e6177`, `0x6e694f`, `0x6e54f0`, `0x6e74aa`, `0x6ea266`, `0x4e60da`, `0x5ecd8c`, `0x605906`, `0x6095ec`, `0x6e1cea`, `0x6e6e9a`, `0x6e8e0d`, `0x6e99d0`, `0x6ec38b`, `0x6ea080`
Confidence: verified
Proof: one
Test: none

The dword scan of the image for `0xceac30` finds 11 references, all in code: three stores (`0x6e594c`,
`0x6e730b`, `0x6ea16a`) and eight reads (`0x6e1cd9`, `0x6e7455`, `0x6e7fed`, `0x6e9fd0`, `0x6ea086`,
`0x6ea120`, `0x6ea130`, `0x6ea14e`); the listing agrees. The five calls of `0x6e54f0` are `0x6e5349`,
`0x6e5467`, `0x6e60d7`, `0x6e6177` and `0x6e694f`. The accessor `0x6e9fd0` has six callers. The scan of the
listing for `call 0x6ea080` finds the nine sites above and no `jmp`.

## Ranged shot sweep

A ranged shot adds the ranged attack time to its category recovery, so a spell with both recoveries 0 still
has a cooldown record when the spell go inserts it. The add applies to a spell with `Attributes` bit 1 and
without `AttributesEx2` bit 17 (0x20000); the dword it adds (`+0x1e8` of the unit block) is the one Lua
`UnitRangedDamage` returns first, times 0.001. The record's category start is the go's receive time. The
cooldown query shows that timer only when the record's category equals the spell's own, or its
`SpellCategory` row has flag bit 1; category 0 (Auto Shot) skips the direct match.

Addresses: `0x6e2b60`, `0x6e2c29`, `0x6e2c2c`, `0x6e2c32`, `0x6e2c3b`, `0x6e2c41`, `0x6e2c47`, `0x6e845d`, `0x6e8498`, `0x6e12c0`, `0x6e12c3`, `0x6e13e0`, `0x6e153f`, `0x6e155a`, `0x6e155e`, `0x6e1563`, `0x6e1567`, `0x518910`, `0x518977`
Confidence: verified
Proof: one
Test: none

`0x6e2c2c test byte [eax+0x18],2` and `0x6e2c32 test dword [eax+0x20],0x20000` gate the add at `0x6e2c47`
(`cooldowns`, Category recovery adjustment). The go's own-cooldown leg calls `0x6e2b60` on the category
recovery (`0x6e845d`) and inserts at `0x6e8498`; `0x6e12c3` returns without a record only when the
recovery, the category recovery, the held byte and the global time are all 0, so the add is what keeps the
record. In `0x6e13e0`, `0x6e155a test edx,edx; je 0x6e1563` skips the direct match for category 0, and
`0x6e1563 test esi,esi; je 0x6e15c6` leaves when the `SpellCategory` row is null. `0x518977` reads
`[[unit+0x110]+0x1e8]` and `0x518987 fild` it before the multiply by 0.001 (`0x801360`).
