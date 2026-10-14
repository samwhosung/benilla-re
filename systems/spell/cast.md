# spell: cast

The cast request: the attempt and its refusals, the pending cast, the in-flight spell, the targeting arm and the packets that ask for a cast.

A cast starts in the attempt `0x6e4b60`, reached for the player through `0x6e5a90` and for the pet
bar directly. It refuses locally, plays the activation sound, fills the pending cast record
`0xceac48`, records the spell as in flight and hands the targets to the arm `0x6e5250`. The arm
either sends at once through `0x6e54f0` or leaves a flag word up for the targeting cursor
(`targeting`), whose binder sends when the word empties. The in-flight spell `0xceca88` is one
slot with one saved slot behind it; the server's cast result (`handlers`) and the cancel
`0x6e4940` (`cancel`) restore the saved one. Outside spell, the client calls the attempt, the
player's cast entry, the feed-pet cast and its setter, the power divisor, the Lua registration pair
and three helpers. All of it is in `Spell_C.cpp`.

Roots: `0x6e4b60`, `0x6e5a90`, `0x6e6ef0`, `0x6e6f10`, `0x6e7130`, `0x6e9b60`, `0x6e9be0`, `0x6e9c20`, `0x6ea1d0`, `0x6ea1e0`, and the cache callback `0x6e59c0`

## The in-flight spell

`0x6e4ad0(spell, item)` with a nonzero spell other than the in-flight one pushes: the in-flight
spell `0xceca88` and its item guid `0xcecab0` move to the saved slot `0xcecaa8` and `0xcecb20`,
and the new spell and its item's guid, or 0, take their place. With spell 0 it pops: the saved
slot is copied back and stays as it was. A spell equal to the in-flight one changes nothing.
Every change ends in `0x4b3250`, which fires `CURRENT_SPELL_CAST_CHANGED` (261) with no arguments.
Only one level is saved: a second push overwrites the saved slot.

Addresses: `0x6e4ad0`, `0x6e4ad5`, `0x6e4adf`, `0x6e4b00`, `0x6e4b2c`, `0xceca88`, `0xcecaa8`, `0xcecab0`, `0xcecb20`, `0x4b3250`
Confidence: verified
Proof: one
Test: none

The function is `fastcall(ecx = spell, edx = item)`. `0x6e4ad5 cmp eax,ecx; je` returns; `test
ecx,ecx; je 0x6e4b2c` pops. The push stores the old spell at `0x6e4adf` and the guid pair, then
reads the item's guid through `[edx+0x8]` (`0x6e4b00`), or stores zeros. Every path tail-jumps to
`0x4b3250`, which calls a game UI function and jumps to `0x703e50` with `ecx = 0x105`. The
writers of the four globals in the image are this function and the system start `0x6e7150`, which zeroes them;
the cast result handler `0x6e7330` also clears the saved slot (`handlers`). All four lie in the
zero-filled part of `.data`.

## The pending cast record

`0xceac48` is a 256-byte record the attempt zeroes and fills before targeting: `+0x00` the guid
of the item cast with, else the caster's; `+0x08` the caster's guid; `+0x10` the spell id. The
binder (`targeting`) adds the target mask at `+0x14` (16 bits), an object guid at `+0x18`, an item
guid at `+0x20`, two positions at `+0x30` and `+0x3c`, and the arm a 128-byte text at `+0x70`.
The send reads the whole record. `+0x08` gates the cancel's events, which clear it.

Addresses: `0xceac48`, `0x6e4f72`, `0x6e4f88`, `0x6e4fa1`, `0x6e4fb1`, `0x6e5311`, `0x6e4a5a`, `0x6e54f0`
Confidence: verified
Proof: one
Test: none

`0x6e4f72` to `0x6e4f80` is `rep stos` of `0x40` dwords from `0xceac48`. `0x6e4f88` stores the
spell; `0x6e4fa1` the item's guid (`[item+0x8]`) or the caster's; `0x6e4fb1` the caster's at
`+0x08`. `0x6e5311` is the text's destination, `0xceac48 + 0x70`, size `0x80`. The other writers
are the system start `0x6e11e0`, the binder `0x6e5b40` and the cast result handler `0x6e7330`.
The send's reads are below (Cast request packets).

## Cast attempt refusals

`0x6e4b60(caster, spell, item, target guid)` returns 1 when the cast was sent or targeting began,
else 0. Its first checks, in order, each return 0:

| check | refused when | report |
|---|---|---|
| inputs | no caster, no record, `Attributes & 0x40` | none |
| `0x494a50(2)` | it returns 0 | none |
| profession | `Effect[0]` 47 | toggles a window, nothing sent |
| dead | `0x605f30` and not `Attributes` & 0x800000 | 0x13 |
| Attack | `Effect[0]` 78 | `0x6131a0` on the target, nothing sent |
| looting | `Effect[0]` 99, 127, or 53, 54, 92 | 0x82, 0x83, 0x61 |
| same spell | spell is the in-flight one | sound `igPlayerInviteDecline` |

Addresses: `0x6e4b60`, `0x6e4b85`, `0x6e4bac`, `0x6e4bb2`, `0x6e4bc1`, `0x6e4bce`, `0x6e4bd7`, `0x6e4c49`, `0x6e4c55`, `0x6e4c7a`, `0x6e4c90`, `0x6e4ca2`, `0x6e4cac`, `0x6e4cd4`, `0x6e4d0d`, `0x6e4d43`, `0x6e4d50`, `0x605f30`, `0x6126f0`, `0x6131a0`, `0x494a50`, `0x4f6000`, `0x4fc910`, `0x6de040`, `0x458030`, `0x4f6012`, `0x4fc92a`, `0x4fc997`
Confidence: verified
Proof: one
Test: none

The function aligns its frame and returns `ret 0xc`: `ecx` caster, `edx` spell, stack item and the
guid's two halves. Effect 47 with a nonzero `EffectMiscValue[0]` (`+0x1a8`) calls `0x4f6000`; with zero,
`0x6de040` maps race, class and spell to a SkillLine row for `0x4fc910`. Each toggles: the
profession already current clears `0xbdcfb8` and fires `CRAFT_CLOSE` (`0x4f6012`), or clears
`0xbde040` and fires `TRADE_SKILL_CLOSE` (`0x4fc92a`); another stores it and fires `CRAFT_SHOW`, or
`TRADE_SKILL_SHOW` only when `[0xbde04c]` is nonzero (`0x4fc997`). Looting is `0x6126f0` nonzero, the 64-bit value at
`+0x1d28` of the active player. Reports go through `0x6e1a00` with the spell in `ecx` and the
reason in `dl` (`failures`); the key names are SPELL_FAILED_CASTER_DEAD, DISENCHANT_WHILE_LOOTING,
PROSPECT_WHILE_LOOTING and SPELL_IN_PROGRESS (`0x6e23e0`). `0x6e4d43 cmp esi,[0xceca88]` passes
`igPlayerInviteDecline` to `0x458030` (`0x6e4d50`) and returns 0, with no report.

## Cast attempt gates

After the early refusals the attempt cancels a pending targeting spell, then checks in order:

| check | refused when | report |
|---|---|---|
| in flight | a spell is in flight without `Attributes` 0x404 | 0x61 |
| `0x6e4000` | reagents or totems fail | by the check |
| `0x6e40e0` | the equipped item fails | by the check |
| ranged | `Attributes` 0x2 and the caster's slot `+0x98` (2, 0) is null | 0x18 with item class and mask |
| combo points | `AttributesEx` 0x500000, the active player, combo byte 0 | 0x46 rogue, else 0x12 |
| attack pick | `0x6e5200` set, not attacking, `0x612df0` fails | none |
| `0x6e4440` | the target or range test fails | by the check |
| `0x6094f0` | the validator fails | by the check |

Addresses: `0x6e4d62`, `0x6e4d71`, `0x6e4d97`, `0x6e4dbe`, `0x6e4ded`, `0x6e4e03`, `0x6e4e13`, `0x6e4e21`, `0x6e4e50`, `0x6e4e5b`, `0x6e4e85`, `0x6e4e98`, `0x6e4edf`, `0x6e4eeb`, `0x6e4efb`, `0x6e4f26`, `0x6e4f3b`, `0x6e4000`, `0x6e40e0`, `0x6e4440`, `0x6094f0`, `0x612df0`, `0x60ecb0`, `0x5fa6d0`
Confidence: verified
Proof: one
Test: none

`0x6e4d62` calls `0x6e48a0` (word nonzero) and then `0x6e4940(1, 1, 0x1c)` (`0x6e4d71`). `0x6e4d97`
calls `0x6e3d30`; `0x6e4dbe test [record+0x18],0x404` reads the in-flight spell's record, and the
report names the new spell. `0x6e4ded` and `0x6e4e03` pass caster, record and 1, or 1 and 1. The
ranged report pushes 0, `+0xec`, `+0xe8`. The combo test runs only when `0x5fa6d0` holds
(`0x6e4e5b`) and reads byte `+0x1029` of the object at `+0xe68` (`0x6e4e85`); class is byte `+0x79`
of the unit fields (`0x6e4e98`). The pick passes `&guid` to `0x612df0` (`0x6e4efb`) unless
`0x60ecb0` holds. `0x6e4f26` passes record, guid and 1; `0x6e4f3b` record and item. Each refusal
is `je 0x6e506e`, the `xor al,al` exit.

## Cast attempt commit

A passing attempt plays the item's use sound (`0x457fb0(2, item)`), else `GAMEABILITYACTIVATE`
for `Attributes` 0x10 and `GAMESPELLACTIVATE` otherwise. It fills the record, sets `0xcecaa4` to
"no item or item flag 0x40, and the caster is the active player", zeroes `0xcead5c`, and pushes the
spell as in flight when the caster's slot `+0xb4` returns at least 0 or the spell has
`Attributes` 0x404, and there is no item or the item has flag 0x40. Then the arm runs. When it
returns 1 the attempt finishes; when it returns 0 the attempt refuses or raises the cursor
(Targeting refusal or cursor). It finishes by starting a melee attack when the spell was sent,
`0x6e5200` holds and `0x6e5230` does not, and calls `0x4b3250`.

Addresses: `0x6e4f48`, `0x6e4f6d`, `0x6e4fba`, `0x6e4fd3`, `0x6e4fe6`, `0x6e4ff5`, `0x6e5002`, `0x6e5026`, `0x6e5038`, `0x6e51b5`, `0x6e51c2`, `0x6e51cb`, `0x6e51de`, `0x6e51e3`, `0xcecaa4`, `0xcead5c`, `0x457fb0`, `0x5da2a0`
Confidence: verified
Proof: one
Test: none

The item flag is slot 0 of the vtable at `+0x110` of the item, `0x5da2a0` in both item vtables the
constructors store, which returns the item record's `+0x20`; `test al,0x40` at `0x6e4d8f` and
`0x6e501d`. `0x6e4ff5` is `call [vtable+0xb4]` with the spell; `jge` takes the push. `0x6e5026`
calls `0x6e4ad0(spell, item)`. `0x6e5038` passes caster, record and the guid. The attack start
at `0x6e51b5` requires `[0xcead5c]` equal to the spell, which only the send writes, the flag
`[ebp-0x2]` set at `0x6e4f13`, and `0x60ecb0` false (`0x6e51cb`); it calls `0x6131a0`
(`0x6e51de`). Both attribute predicates are in Attack predicates.

## Attack predicates

`0x6e5200(record)` is 1 when the record has `Attributes` 0x404, `AttributesEx` 0x200 or
`AttributesEx2` 0x100000, and 0 for none or a null record. `0x6e5230(record)` returns bit 20 of
`AttributesEx2`, 0 for a null record. The attempt runs its attack pick for the first, and starts a
melee attack after the send for the first when the second is 0.

Addresses: `0x6e5200`, `0x6e5207`, `0x6e5213`, `0x6e5218`, `0x6e5230`, `0x6e523a`
Confidence: verified
Proof: one
Test: none

`0x6e5207 test [ecx+0x18],0x404`, `0x6e5213 test ah,0x2` on `[ecx+0x1c]`, `0x6e5218 test
[ecx+0x20],0x100000`; any nonzero returns 1. `0x6e523a` is `shr eax,0x14; and eax,1` on
`[ecx+0x20]`. Both are `fastcall` with the record in `ecx`. `0x6e7a70` also calls both (`handlers`).

## The targeting arm

`0x6e5250(caster, record, guid)` loads the word `0xcecac0` from `Targets` (`+0x34`, 16 bits), then
switches on `EffectImplicitTargetA[0]` (`+0x148`), 1 to 63, through an 11-entry table. If the word
has 0x2000 and the text `0xcee838` is not empty, the text is copied to the record's `+0x70`, 0x2000
leaves the word and enters the mask. It fires `CURRENT_SPELL_CAST_CHANGED` through
`0x4b3250`; then a zero word sends at once. Target 16 returns 0. Otherwise a candidate is bound through `0x6e5b40`; with none bound, the active player is bound when the
`autoSelfCast` value is nonzero; else an unknown object whose spell's maximum range exceeds 100 is
sent by guid.

| implicit target | word |
|---|---|
| 1 | clears 0x400 |
| 5 | clears 0x8000 |
| 6, 53 | sets 0x80 |
| 16 | marks the cursor |
| 21, 45 | sets 0x100 |
| 23 | sets 0x800 |
| 25, 63 | sets 0x2 |
| 26 | sets 0x4000 |
| 35 | sets 0x8 |
| 57, 61 | sets 0x4 |

Addresses: `0x6e5250`, `0x6e525a`, `0x6e527b`, `0x6e5282`, `0x6e52ef`, `0x6e5302`, `0x6e5325`, `0x6e5338`, `0x6e5342`, `0x6e535b`, `0x6e5484`, `0x6e54b0`, `0xcecac0`, `0xcee838`
Confidence: verified
Proof: one
Test: none

The byte table `0x6e54b0` maps `target − 1`, 0 to 62, to cases 0 to 10 of `0x6e5484`; case 10 is
`0x6e52ef`, no change, and every target not listed takes it. Cases 0 and 1 test the bit before
clearing it. `0x6e5302` calls `SStrPrintf(+0x70, 0x80, "%s", text)`, then clears 0x2000 in the
word, ORs 0x20 into byte `+0x15`, the mask's high byte (`0x6e5325`), and empties the text.
`0x6e5338` compares the whole word with 0 and `0x6e5342` calls `0x6e54f0(0xceac48, 0)`. Target 16
sets `bl` (`0x6e52db`), tested at `0x6e535b`. The walk is in Arm candidates.

## Arm candidates

The candidate is the caster's main-hand item when the caster is a player and the spell has
`Attributes` 0x200 (none when the count at `+0x1d38` is 15 or less), else the guid passed in, else
the selection `0xb4e2d8`. A successful bind returns 1. Otherwise, when `autoSelfCast` is set, the
active player is bound and the bind's result returned. Otherwise a nonzero candidate the object
manager does not know, for a spell whose maximum range is above 100, goes in the record at `+0x18`
and is sent; that returns 1. Everything else returns 0.

Addresses: `0x6e5361`, `0x6e536e`, `0x6e5376`, `0x6e538b`, `0x6e5391`, `0x6e5393`, `0x6e539f`, `0x6e53bc`, `0x6e53c3`, `0x6e53d7`, `0x6e5400`, `0x6e5425`, `0x6e543f`, `0x6e5447`, `0x6e5454`, `0xb4e2d8`, `0xceac34`, `0x468460`, `0x6e5b40`, `0x6e3480`
Confidence: verified
Proof: one
Test: none

`0x6e5361` tests the type word's player bit, `0x6e536e` `Attributes` 0x200; `0x6e5376 cmp
[edi+0x1d38],0xf; ja` then reads `[[edi+0x1d3c]+0x78]` (`0x6e538b`) and
jumps past the passed guid (`0x6e5391`). `0x6e53bc` is `0x468460(guid,
1)`; `0x6e53c3` binds it. `0x6e53d7` reads `[[0xceac34]+0x28]`, the CVar record `0x6e7150` stores
for `autoSelfCast` (default "0"). `0x6e5425` is `0x468460(guid, 1)` again and a found object returns 0;
`0x6e543f` calls `0x6e3480(caster, spell, &min, &max, 0)` (`records`); `0x6e5447 fcomp 100.0; test
ah,0x41; jne` returns 0 for a maximum not above 100, NaN included. `0x6e5454` stores the guid at
`0xceac60` and sends with `dl` 0; the mask at `+0x14` holds at most the text bit, so the block
carries no guid.

## Targeting refusal or cursor

When the arm returns 0, the attempt refuses with 0x2d for a spell with `Attributes` 0x200; refuses
for a word with 0x80 with 0x0a when the selection is set and 0x09 when it is not; and otherwise
raises the cursor. The refusals clear the word. Raising the cursor sets the cursors to Cast and
UnableCast, clears `0xceca90` and `0xcecad4`, builds the placement model for the first
`Effect` among 50, 76, 81, 104 to 107, sets `0xceca90` for 81, and sets `+0x1100` of `0xcf0bd8` to 1.

Addresses: `0x6e5045`, `0x6e504b`, `0x6e5050`, `0x6e5057`, `0x6e5064`, `0x6e507b`, `0x6e5082`, `0x6e50a3`, `0x6e50ab`, `0x6e50b0`, `0x6e50ba`, `0x6e50c8`, `0x6e50e0`, `0x6e511d`, `0x6e5124`, `0x6e514e`, `0x6e5198`, `0x6e51ab`, `0xcecac0`, `0xceca8c`, `0xceca90`, `0xcecad4`, `0xcf0bd8`, `0x523c20`, `0x523d20`, `0x55ba30`
Confidence: verified
Proof: one
Test: none

`0x6e504b test ch,0x2` on `Attributes`; `0x6e5050` loads the spell for the report and `0x6e5057`
zeroes the word. `0x6e507b` reads the word's low
byte, `jns` skips; `0x6e50a3` picks 0x0a, then 0x09 for a zero `0xb4e2d8`. Keys: MAINHAND_EMPTY,
BAD_TARGETS, BAD_IMPLICIT_TARGETS (`failures`). `0x523d20(2)` sets the default cursor and
`0x523c20(0x16)` the current one (`0x6e50b0`, `0x6e50ba`); the names table gives index 2 Cast and
22 UnableCast. The effect scan is `0x6e50e0` to `0x6e510c`. `0x6e5124` builds the key, the spell id
low and its sign ORed with 0xb0000000 high, and asks the game-object cache for `EffectMiscValue[i]`
with the callback `0x6e59c0` (`0x6e514e`); a cached record's display info names the model, placed at
the caster's position, stored at `0x6e5198` (`targeting`, Placement model and rotation).

## Placement model callback

`0x6e59c0` is the game-object cache's callback for the request above. When its argument is still
the pending spell (`+0x10` of the record), it looks the record up again and, when it is there, builds
the placement model at the position of the record's caster and stores it in `0xceca8c`, without
releasing a model already there.

Addresses: `0x6e59c0`, `0x6e59d9`, `0x6e5a09`, `0x6e5a4a`, `0x6e5a77`, `0xceca8c`
Confidence: verified
Proof: one
Test: none

Its address is pushed at `0x6e5135`, the only occurrence in the image. It returns `ret 8`.
`0x6e59d9` compares the first stack argument with `[0xceac58]`. `0x6e5a09` repeats the cache
lookup with no callback; `0x6e5a4a` is `0x468460([0xceac50], 8)`; its slot `+0x14` fills the
position the model is built at, as at `0x6e517c`; `0x6e5a77` stores the model.

## Cast request packets

`0x6e54f0(record, flag)` sends one of three requests, each ending in the target block.

| opcode | when | body before the targets |
|---|---|---|
| `CMSG_USE_ITEM` (0xab) | `+0x00` differs from `+0x08` | u8 bag, u8 slot, u8 spell index |
| `CMSG_CAST_SPELL` (0x12e) | the caster is the active player | u32 spell |
| `CMSG_PET_CAST_SPELL` (0x1f0) | otherwise | u64 caster guid, u32 spell |

The target block is a u16 mask (`+0x14`); a packed guid of `+0x18` when the mask has any of
0x8a02; a packed guid of `+0x20` for 0x1010; three floats `+0x30` to `+0x38` for 0x20; three
floats `+0x3c` to `+0x44` for 0x40; and 128 raw bytes from `+0x70` for 0x2000. A packed guid is a
mask byte, then each nonzero byte of the guid from the lowest, mask bit i for byte i.

Addresses: `0x6e54f0`, `0x6e5513`, `0x6e551d`, `0x6e5577`, `0x6e55a7`, `0x6e554c`, `0x6e5580`, `0x6e57d8`, `0x6e57ec`, `0x6e57f8`, `0x6e5801`, `0x6e5808`, `0x6e5816`, `0x6e582d`, `0x6e583b`, `0x6e584f`, `0x6e585a`, `0x6e5863`, `0x6e5872`, `0x7e4e10`, `0x642ec0`, `0x642e10`, `0x418070`, `0x418130`, `0x418190`, `0x418370`, `0x4183d0`, `0x418570`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

Every request first releases `0xceca8c` and sets both cursors to Point, index 1 (`0x6e554c`); a
missing caster (`0x468460(+0x08, 8)`, `0x6e5577`) sends nothing (`0x6e5580`). `0x6e57d8` puts
`0xab`, then bytes at `0x6e57ec`, `0x6e57f8`, `0x6e5801`. `0x6e5808` asks `0x5fa6d0`; `0x6e5816`
puts `0x1f0` and `0x418370` the guid (`0x6e582d`); `0x6e583b` puts `0x12e`; `0x6e584f` the spell.
The opcode is a dword, as in every client request. `0x6e585a` stores the spell in `0xcead5c`.
`0x7e4e10` writes the block: `0x418130`, `test word,0x8a02`, `0x642ec0`, `test word,0x1010`,
`0x4183d0` thrice per position, and `0x418570(+0x70, 0x80)`, which copies 128 bytes. `0x642e10`
puts a 0 byte, puts each nonzero byte and patches the mask. `0x5ab630` sends (`0x6e5872`). The opcode names are the server's names for these values.

## Item use fields

For `CMSG_USE_ITEM` the bag is `0x5e13b0(owner, container guid)`, the slot is the item's index in
its container's slot list, and the spell index is the first i, 0 to 4, whose item spell is the
cast spell and whose trigger is 0. A missing item, container, owner, item record or matching spell
prints a console line and sends nothing. Before the packet the client plays a visual kit of the
spell on the caster through `0x60edf0`.

Addresses: `0x6e55ad`, `0x6e55c9`, `0x6e55df`, `0x6e5615`, `0x6e5630`, `0x6e5655`, `0x6e5687`, `0x6e56b8`, `0x6e56db`, `0x6e5730`, `0x6e5749`, `0x6e579e`, `0x6e57ce`, `0x5e13b0`, `0x63cb50`, `0x60edf0`, `0x6ec1e0`
Confidence: verified
Proof: one
Test: none

`0x6e55ad` is `0x468460(item, 2)`; `0x6e55df` looks up the item fields' `+0x08` (the container)
with mask 1; `0x6e5615` calls the container's slot `+0x10`, a count and a guid array, searched
from `0x6e5630`; `0x6e5687` looks up the fields' `+0x00` with mask 0x10. `0x6e56db` reads the item
record by the item's entry (`[+0x8]+0xc`) through `0x55ba30`. `0x6e5730` compares `+0x11c + 4i` with
the spell and `+0x130 + 4i` with 0. The lines are "Casting item not found", "Casting item's
container not found", "Casting item not found in container", "Active player not found",
"Casting item doesn't have stats" and "Casting item doesn't have spell used", through `0x63cb50`
(`0x6e55c9`, `0x6e5655`, `0x6e5749`). `0x6e579e` gets the SpellVisual row; `0x6e57ce` calls
`0x60edf0(spell, kit, 4, 0)` with the SpellVisualKit row the visual's `+0x04` names.

## After the send

After the packet, in order: when the caster is the active player, the flag is 0 and the spell is
not the caster's `+0xc8c`, `0x6ec220(caster)` and `0x4b3250`; when `0xcecaa4` is 0 and the spell is
in flight, a pop; the global cooldown starts in the pet's history when the caster's charmer, else
summoner, is the active player, else the player's; with `AttributesEx2` 0x20 or `Attributes` 0x2,
`0x611cf0(caster, 2, 1, 1)`. With `AttributesEx2` 0x20 (auto-repeat): `+0xd58 |= 0x200`, the spell
goes to `0xceac30`, `START_AUTOREPEAT_SPELL` (435) fires with no arguments, an active-player caster
gets `0x5ecac0`, and an in-flight spell equal to it is popped.

Addresses: `0x6e5877`, `0x6e5889`, `0x6e5898`, `0x6e58a2`, `0x6e58bd`, `0x6e58c2`, `0x6e58f3`, `0x6e58fb`, `0x6e591c`, `0x6e5922`, `0x6e5930`, `0x6e593b`, `0x6e5947`, `0x6e594c`, `0x6e5952`, `0x6e5957`, `0x6e5972`, `0x6e5976`, `0x6e5989`, `0xceac30`, `0x611cf0`, `0x5ecac0`, `0x6e2de0`, `0x6ec220`
Confidence: verified
Proof: one
Test: `objectlayer::lowhp_warn_5ecac0_difftest`

`0x6e58c2` reads unit field `+0x10`, else `+0x18`, and `0x6e58f3` sets `edx` 1 only when it equals the
active player; `0x6e58fb` is `0x6e2de0(spell, edx)` (`cooldowns`). `0x6e591c test [edi+0x20],0x20`,
`0x6e5922 test [edi+0x18],0x2`, `0x6e5930` the call. `0x6e593b or [esi+0xd58],0x200`; `0x6e5947`
loads the event id `0x1b3`, `0x6e594c` stores the spell and `0x6e5952` signals. `0x6e5957` to `0x6e5972` compare the caster's guid with
the active player; `0x6e5976` calls `0x5ecac0`, which sends `CMSG_ATTACKSTOP` when attacking and
ends in `0x6e6f30`. The pop is `0x6e4ad0(0, 0)` (`0x6e58bd`, `0x6e5989`). The attempt sets
`0xcecaa4` only for an active-player caster, so the first pop serves a pet's cast. Slices: lowhp_warn_5ecac0_difftest `0x5ecac0`-`0x5ecb63`.

## Cast entries

`0x6e5a90(spell, item, guid)` casts for the active player: `0x6e4b60(player, spell, item, guid)`.
`0x6e5b10(guid*)` binds the object the guid names (mask 1) through `0x6e5b40`. `0x6ea1d0(spell)`
stores the feed spell `0xcecad8`. `0x6ea1e0(item, unit)` casts it when the item and unit exist,
the unit's created-by-spell field (`+0x214`) is nonzero, the feed spell has a record, and the
unit's creator (`+0x20`) is the active player; when the cast returns 1 it binds the item.

Addresses: `0x6e5a90`, `0x468550`, `0x6e5aa4`, `0x6e5abe`, `0x6e5b10`, `0x6e5b2d`, `0x6ea1d0`, `0xcecad8`, `0x6ea1e0`, `0x6ea1fd`, `0x6ea207`, `0x6ea24f`, `0x6ea266`, `0x6ea274`, `0x4bd1d0`, `0x4bd385`
Confidence: verified
Proof: one
Test: none

`0x6e5a90` is `fastcall(ecx = spell, edx = item)` with the guid's halves on the stack, `ret 8`;
`0x6e5aa4` gets the active player from `0x468550` and `0x468460(guid, 0x10)`; `0x6e5abe` calls
with `edx = spell`. The pet bar calls `0x6e4b60` itself with the pet as caster and no item (`0x4bd385` in `0x4bd1d0`).
`0x6e5b2d` tail-jumps with the object in `ecx`. `0x6ea1fd` reads `+0x214`, `0x6ea207` the spell,
`0x6ea24f` compares `+0x20` and `+0x24` with the player's guid; `0x6ea266` is `0x6e5a90(spell, 0,
0)`; `0x6ea274` tail-jumps to `0x6e5b40` with the item. The writers of `0xcecad8` are `0x6ea1d0`
and the system start.

## Small spell helpers

`0x6e7130(power)` returns the display divisor of a power type: 1, 10, 1, 1, 1000 for 0 to 4, 1 for
a negative type, and the dword past the table for 5 and up. `0x6e9be0(record)` is 1 when an
`Effect` is 6, 35, 119, 128 or 129. `0x6e9c20(store)` reads into `this` eight bytes at `+0x00` and
`+0x08`, then four at each of `+0x10`, `+0x14`, `+0x18`, `+0x1c` and `+0x20`. `0x6e9b60` empties
the pet's spell history. `0x6e6ef0` and `0x6e6f10` register and unregister the five Lua functions
of `0x86f950` (`targeting`).

Addresses: `0x6e7130`, `0x6e7137`, `0x6e7139`, `0x86f978`, `0x6e9be0`, `0x6e9bea`, `0x6e9c20`, `0x630170`, `0x418e30`, `0x4190b0`, `0x419130`, `0x6e9b60`, `0x6e1880`, `0x6e6ef0`, `0x6e6f10`, `0x86f950`, `0x704120`, `0x704160`
Confidence: verified
Proof: one
Test: none

`0x6e7137 jl` returns 1; `0x6e7139` loads `[ecx*4+0x86f978]` with no upper bound; the sixth dword is
the text "Spel". `0x6e9bea` compares each of the three `Effect` dwords. `0x6e9c20` is `ret 4` with
`0x4190b0` twice, `0x418e30`, `0x419130` twice, `0x418e30` twice, which read 8, 4 and 4 bytes
(`push 8` or `push 4` in each); the community names call the third a float read; its one caller is `0x630170`, the
handler of opcode 0x1d6, which discards the result. `0x6e9b60` passes the pet's history to `0x6e1880`
(`cooldowns`). `0x6e6ef0` passes each pair to `0x704120`; `0x6e6f10` each name to `0x704160`.

## Unreached functions

`0x6e5ad0` casts the spell named by its text through `0x6e30a0` and `0x6e5a90`, or cancels with
(1, 1, 0x1c) for the text `none`. `0x6e9fe0(record)` returns bit 5 of `AttributesEx2`. Nothing
reaches either.

Addresses: `0x6e5ad0`, `0x6e5ade`, `0x6e5aed`, `0x6e5af8`, `0x6e9fe0`
Confidence: verified
Proof: one
Test: none

Two scans over the whole image: an `e8` or `e9` at any offset whose target is the function finds
none for either, and the little-endian dword of either address occurs nowhere. The control finds
15 calls of `0x6e5a90` and one stored `0x6e59c0`. Both follow a `ret` and padding. `0x6e5ade` is
`SStrCmpI(text, "none")`.
