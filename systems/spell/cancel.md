# spell: cancel

Cancelling: a cast in flight or a pending targeting spell, a queued next-swing spell, auto-repeat, a channel and an aura.

The cancel `0x6e4940` ends what the cast request (`cast`) started. It drops a targeting word or
asks the server to cancel the in-flight spell, restores the saved in-flight slot, resets the
cursors and fires the cast bar's stop events for the player. Its callers decide which events and
whether a packet goes out; the table below lists every one. The other cancels each send one
request: the next-swing cancel `0x6e6f30` at the end of a melee stop, the auto-repeat stop
`0x6ea080`, the channel cancel `0x6e9b70` and the aura cancel `0x6e7040`. All of them are in
`Spell_C.cpp`, and the client reaches each of them from outside spell.

Roots: `0x6e4940`, `0x6e6f30`, `0x6e7040`, `0x6e9b70`, `0x6e9fd0`, `0x6ea080`

## Cancelling a cast

`0x6e4940(failed, send, reason)`: with a targeting word up it clears the word and sends nothing;
otherwise, with a spell in flight and `send` set, it sends `CMSG_CANCEL_CAST` (0x12f), a u32
in-flight spell. Then it pops, releases the placement model and sets both cursors to Point. When
the record's caster is the active player it fires `SPELLCAST_STOP` (338) for `failed` 0, else
`SPELLCAST_INTERRUPTED` (340) for reason 0x23 or 0x24 and `SPELLCAST_FAILED` (339) for any other,
all without arguments; clears
the caster; and, when the active player's object exists and is a player, unless the record's
spell has `AttributesEx` 0x4, calls `0x614150(player, spell, 0)`. With `failed` set it last calls
`0x4fd790`, which ends a trade-skill repeat.

Addresses: `0x6e4940`, `0x6e4947`, `0x6e4955`, `0x6e495c`, `0x6e4963`, `0x6e496c`, `0x6e4970`, `0x6e4997`, `0x6e49ab`, `0x6e49d6`, `0x6e49e5`, `0x6e49f5`, `0x6e49ff`, `0x6e4a04`, `0x6e4a1e`, `0x6e4a24`, `0x6e4a43`, `0x6e4a50`, `0x6e4a5a`, `0x6e4aad`, `0x6e4ab7`, `0x6e4abc`, `0x6e4ac2`, `0x4fd790`, `0x614150`, `0x703e50`
Confidence: verified
Proof: one
Test: none

`fastcall(cl, dl)` with the reason on the stack, `ret 4`; `cl` is saved at `0x6e4947`, `dl` is read
at `0x6e496c` after two callees that leave `edx` alone. `0x6e4955` zeroes the word and the branch
skips the packet. `0x6e4970` puts `0x12f`, then `0x6e3d10()`; `0x6e49ab` sends. `0x6e49d6` pops;
`0x6e49f5` and `0x6e49ff` set cursor 1. `0x6e4a04` to `0x6e4a1e` compare `+0x08` with the active
player; `0x6e4a43` adds `setne` to 0x153, else 0x152; `0x6e4a50` signals. `0x6e4aad` reads
`+0x1c`, `AttributesEx`; a missing record still calls `0x614150` (`0x6e4ab7`). `0x6e4abc` tests
`cl` on every path. Event names are `tables/events.tsv` rows 338 to 340. The packet's name is the
server's name for 0x12f.

## Who cancels a cast

Each caller passes (`failed`, `send`, reason). `CMSG_CANCEL_CAST` goes out only for `send` 1.

| caller | arguments | condition |
|---|---|---|
| `0x6e4900`, `0x6e4920`, the attempt | 1, 1, 0x1c | a targeting word is up; for `0x6e4920`, cast by another caster |
| `0x6e6e80` (`SpellStopCasting`) | 0, 1, 0x1c | a spell in flight, no auto-repeat |
| `0x6e6f30` | 0, 1, 0x1c | the in-flight spell has `Attributes` 0x404 |
| `0x6e7330` (cast result) | 0, 0, 0x1c | the result is for the in-flight spell |
| `0x6e1a00` (failure) | 1, report's third argument is 0, reason | the failure is for the in-flight spell |
| `0x6e6900` | 0, 0, 0x17 | after its own send |
| `Script::Jump`, `0x515090` | 0, 1, 0x1c | the in-flight spell's `InterruptFlags` bit 0 |
| `Script::UseSoulstone`, `0x495920`, `0x5e99b0` | 1, 1, 0x1c | on their own paths |
| `0x5ecb70` | 1, 1, 0x1c | the caster's precast spell `+0xc8c` has `Attributes` 0x2 |
| `0x605860` | 1, 1, 0x65 | the unit is the record's target |
| `0x5d8660` | 1, 0, 0x1c | the item is the record's |
| `0x5e38c0` | 1, 0, 0x17 | the in-flight item guid matches |

Addresses: `0x6e4900`, `0x6e4920`, `0x6e6e80`, `0x6e6ec3`, `0x6e6f68`, `0x6e741a`, `0x6e2218`, `0x6e695a`, `0x513bd0`, `0x513c43`, `0x515090`, `0x51512a`, `0x48ad70`, `0x48ae6b`, `0x495920`, `0x495932`, `0x5e99b0`, `0x5e9a6c`, `0x5ecb70`, `0x5ecc56`, `0x5ecc85`, `0x605860`, `0x6058e0`, `0x5d8660`, `0x5d86c4`, `0x5e38c0`, `0x5e3ab3`, `0x6e5aed`
Confidence: verified
Proof: one
Test: none

The census is every `e8` or `e9` in the image whose target is `0x6e4940`: 18 sites, the 17 above
and the unreached `0x6e5aed`; the control is the same scan finding the three calls of `0x6e9b70`.
The arguments are the `mov cl`, `mov dl` and `push` before each. `0x513c43` and `0x51512a` test
`[record+0x54] & 1`; `0x5ecc85` follows `test [record+0x18],0x2` on the record of the precast spell `0x5ecc56` loads
from `[unit+0xc8c]`, bounded against the Spell store, and a nonzero id with no row reads address
0x18; `0x6058e0`
compares the unit's guid with `[0xceac60]`; `0x5d86c4` with `[0xceac48]`; `0x5e3ab3` with
`[0xcecab0]`. Reason keys: 0x1c ERROR, 0x17 DONT_REPORT, 0x65 TARGETS_DEAD. The callers' own
purposes outside spell are not read.

## Cancelling next-swing spells

`0x6e6f30` cancels an in-flight spell with `Attributes` 0x404 through `0x6e4940(0, 1, 0x1c)`. Then,
when the saved slot holds a spell with `Attributes` 0x404 and no item, it sends `CMSG_CANCEL_CAST`
for the saved spell itself and clears the saved slot and its guid. A saved spell with the bits and
an item is cleared without a packet; one without the bits is kept.

Addresses: `0x6e6f30`, `0x6e6f59`, `0x6e6f68`, `0x6e6f9d`, `0x6e6faa`, `0x6e6fd9`, `0x6e6fe8`, `0x6e6ff3`, `0x6e701a`, `0x5ecac0`
Confidence: verified
Proof: one
Test: `objectlayer::lowhp_warn_5ecac0_difftest`

The only reference is the tail jump at `0x5ecb63`, the end of `0x5ecac0`. `0x6e6f59` and
`0x6e6f9d` test 0x404; `0x6e6faa` ORs the saved guid's halves and skips the send when nonzero.
`0x6e6fd9` puts `0x12f`, `0x6e6fe8` the saved spell, `0x6e6ff3` sends. `0x6e701a` to `0x6e7026`
zero `0xcecaa8` and `0xcecb20` on both paths. Slices: lowhp_warn_5ecac0_difftest `0x5ecac0`-`0x5ecb63`.

## Auto-repeat stop

`0x6ea080` does nothing without an auto-repeat spell `0xceac30`. Otherwise it sends
`CMSG_CANCEL_AUTO_REPEAT_SPELL` (0x26d) with an empty body; for an active player clears `+0xd58`
bit 0x200 and calls `0x60d040(player, spell, 1, 1)`, `0x614150(player, spell, 0)`, `0x60f530` and
`0x5fd9e0(player, −1)`; pops when the spell is in flight; clears `0xceac30`; and fires
`STOP_AUTOREPEAT_SPELL` (436) with no arguments. `0x6e9fd0` returns `0xceac30`.

Addresses: `0x6ea080`, `0x6ea090`, `0x6ea0c6`, `0x6ea0d9`, `0x6ea105`, `0x6ea113`, `0x6ea12b`, `0x6ea139`, `0x6ea140`, `0x6ea149`, `0x6ea160`, `0x6ea16a`, `0x6ea170`, `0x6e9fd0`, `0x60d040`, `0x60f530`, `0x5fd9e0`
Confidence: verified
Proof: one
Test: none

`0x6ea090 je` exits on 0. The data store is built with its own vtable, whose first slot sets it
up; `0x6ea0c6` puts `0x26d` and `0x6ea0d9` sends, before any local change. `0x6ea105` tests the
type word's player bit; `0x6ea113 and eax,0xfffffdff`. `0x6ea160` is `0x6e4ad0(0, 0)` when
`[0xceac30]` equals `[0xceca88]`. `0x6ea170` signals `0x1b4`. Callers: `0x6e6e80`, `0x6e1a00`,
`0x6e8d80`, `0x6e99d0`, `0x6ec350` in spell and four outside it, nine call sites in all.

## Channel and aura cancels

`0x6e9b70(spell)` sends `CMSG_CANCEL_CHANNELLING` (0x13b), a u32 spell, and changes nothing local.
`0x6e7040(spell)` cancels an aura: for an active player it first calls `0x60d040(player, spell, 1,
1)` and `0x614150(player, spell, 0)`; with `AttributesEx` 0x2000 and not 0x4 it stops when there is
no player or `0x5ee290(player)` is nonzero; spell 2584 fires `AREA_SPIRIT_HEALER_OUT_OF_RANGE` (528)
with no arguments; then it sends `CMSG_CANCEL_AURA` (0x136), a u32 spell.

Addresses: `0x6e9b70`, `0x6e9b9e`, `0x6e9ba7`, `0x6e9bb2`, `0x513cac`, `0x515193`, `0x60dd00`, `0x60dd4a`, `0x6e7040`, `0x6e7075`, `0x6e707e`, `0x6e709e`, `0x6e70a3`, `0x6e70ad`, `0x6e70b6`, `0x6e70c3`, `0x6e70eb`, `0x6e70f4`, `0x6e70ff`, `0x5ee290`
Confidence: verified
Proof: one
Test: none

`0x6e9b9e` puts `0x13b`, `0x6e9ba7` the spell from `ecx`, `0x6e9bb2` sends. Its three callers pass a unit's
channel spell (unit field `+0x228`): `0x513cac` in `Script::Jump` and `0x515193` in `0x515090` the
active player's, when its `ChannelInterruptFlags` (`+0x5c`) has 0x8 and the player's guid matches
a guid the movement code keeps; `0x60dd4a` when the spell has `AttributesEx` 0x4000 and a unit test
of its own passes.
`0x6e709e test ah,0x20` and `0x6e70a3 test al,0x4` read `+0x1c`; a spell id with no record reads
address `0x1c`. `0x6e70b6 cmp esi,0xa18`; `0x6e70c3` signals `0x210`; `0x6e70eb` puts `0x136`.
