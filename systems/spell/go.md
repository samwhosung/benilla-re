# spell: go

`SMSG_SPELL_GO`: its fields, the miss report, the caster's state, the visuals, the missiles and the
cooldowns the client starts itself.

The go path `0x6e7a70` (`Spell_C.cpp`) runs for opcode `0x132` from the dispatch `0x6e7640`
(`handlers`, Start and go dispatch). It decodes the hit and miss lists into stack arrays, reports
instant misses, updates the casting unit's cast state, plays the cast kit, launches missiles or
plays impacts by the record's speed, runs the player's own cast tail, and starts the cooldowns the
cast earns. Its helpers are the missile launcher, the impact legs, the miss reason rewrite, and
the two functions that hold an item cooldown until the item's record arrives from the server. The
state it writes is the player's and the pet's cooldown lists, the pending item cooldown hash and
the prune time; it writes none of the cast slots of `handlers` itself. The rest of the client reaches
this topic only through the item cache, which calls the record callback back.

Roots: `0x6e8830`

## Spell go packet

After the common fields come a 16-bit cast flags word; a hit count byte and that many 64-bit guids;
a miss count byte and that many entries of a 64-bit guid and a reason byte, with one more byte when
the reason is 11; the target block; and two dwords, ammo display id and ammo inventory type, when
the flags have `0x20`, else zeros. With no Spell record the handler returns after decoding. The
combat log source is the first guid when an object of it is a unit or a game object (type bits 3
or 5), else the second guid.

Addresses: `0x6e7a70`, `0x6e7aa4`, `0x6e7ab3`, `0x6e7ac9`, `0x6e7aed`, `0x6e7b07`, `0x6e7b71`, `0x6e7b80`, `0x6e7b92`, `0x6e7ba2`, `0x6e7c76`, `0x6e7c97`, `0x6e7cb8`, `0x6e7cf7`, `0x6e7d13`, `0x6e7d24`, `0x6e7d2c`, `0x40a280`
Confidence: verified
Proof: one
Test: none

`0x6e7aa4` reads the word, `0x6e7ab3` the count; `0x6e7ac9` allocates count × 8 on the stack and
`0x6e7aed` reads each guid. `0x6e7b07` reads the miss count; three allocations hold guids, reasons
and second bytes (`[ebp-0x24]`, `[ebp-0x50]`, `[ebp-0x68]`), each as a count-and-pointer pair.
`0x6e7b71` reads the guid, `0x6e7b80` the reason, `0x6e7b92 cmp reason,0xb`, `0x6e7ba2` the second
byte. `0x6e7c76` reads the target block into the record at `[ebp-0x170]`; `0x6e7c97` tests flag
`0x20`, `0x6e7cb8` and `0x6e7cc3` read the two dwords. `0x6e7cf7` exits to the cleanup on a null
record. `0x6e7d13` looks up the first guid with type mask 1; `0x6e7d24` and `0x6e7d2c` test bits 5
and 3 of the object's type dword.

## Inline miss report

Only for a record speed (`+0x94`) equal to 0: for each miss, the combat log line `0x62bab0`
(reason, spell id, source guid, target guid, 0); then, unless bit 8 of `+0x18` is set, `0x607140` on
the target unit (the source, the reason, the record) and `0x494720` (the target guid, the reason).
A reflect, reason 11, with a nonzero second byte, adds the same three calls with the second byte as
the reason and the casting unit, the second guid, as the target. A positive, negative or NaN speed
reports nothing here.

Addresses: `0x6e7d4e`, `0x6e7d54`, `0x6e7d73`, `0x6e7dcc`, `0x6e7d5c`, `0x6e7d5f`, `0x6e7d95`, `0x6e7d9a`, `0x6e7dd4`, `0x6e7de5`, `0x6e7ded`, `0x6e7df9`, `0x6e7e14`, `0x6e7e51`, `0x6e7e5f`, `0x62bab0`, `0x607140`, `0x494720`
Confidence: verified
Proof: one
Test: none

`0x6e7d4e fld [esi+0x94]`, `fcomp` with 0.0f, `0x6e7d5c test ah,0x44`, `0x6e7d5f jp 0x6e7e71`. Equal
sets C3 alone, an odd count, so the block runs; greater sets nothing and less sets only C0, both
masked to 0; unordered sets C3 and C2, an even count; all three jump past. `0x6e7d95` takes the
reason in `ecx`, the spell in `edx`, pushes 0, target, source. `0x6e7d9a test ah,1` on `+0x18`
skips both unit calls; `0x6e7d73` loads the record that `0x6e7dcc` pushes. `0x6e7dd4` is `0x607140` on the unit of type mask 8, `0x6e7de5` takes the guid
pointer in `ecx` and the reason in `edx`. `0x6e7ded` and `0x6e7df9` test 11 and a nonzero second
byte; `0x6e7e14`, `0x6e7e51` and `0x6e7e5f` repeat the calls for the second guid.

## Caster state at go

The handler reads the clock once; that time starts every cooldown below. It builds one list of hits
then misses, misses first for flag `0x400`. For a casting unit (second guid, type mask 8) it
computes `p = 0x6e4870(record, flags)` and calls `0x60d040` (spell, the unit's descriptor dword
`+0x228` is 0, p); `0x614150` (spell, 0) unless bit 2 of `+0x1c` is set while that dword is nonzero;
`0x614150` (auto-repeat spell, 0) while one runs; and, when p, `0x605780` with the combined list,
which refills the unit's chain-target array. p is 1 for bit 30 of `+0x20`, else 0 for bit 10 of
`+0x1c` or bit 28 of `+0x20`, else 0 only for flag bit 0 without flag bit 12.

Addresses: `0x6e7e71`, `0x6e7eab`, `0x6e7f8b`, `0x6e7fa1`, `0x6e7fb4`, `0x6e7fc5`, `0x6e7fca`, `0x6e7fe8`, `0x6e7fed`, `0x6e7ffb`, `0x6e800d`, `0x6e4870`, `0x42c010`, `0x60d040`, `0x614150`, `0x605780`
Confidence: verified
Proof: one
Test: none

`0x6e7e71` stores `OsGetAsyncTimeMs` at `[ebp-0x38]`. `0x6e7eab test ah,4` picks the copy order
into the array of hit + miss count entries. `0x6e7f8b` looks up the unit. In `0x6e4870`, `test
eax,0x40000000; jne` returns 1, `[ecx+0x1c] & 0x400` and `eax & 0x10000000` return 0, `test dl,1;
je` and `test dh,0x10; jne` return 1, else 0. `0x6e7fb4 sete dl` on `[[unit+0x110]+0x228]`.
`0x6e7fca test [edi+0x1c],4` and the reload of that dword gate `0x6e7fe8`; `0x6e7fed` reads
`0xceac30`. `0x6e800d` passes the count-and-pointer pair `[ebp-0x4c]`. `0x605780` empties
`unit+0xd44` and appends the list's guids from the last, skipping the unit's own (`0x6057bf`).

## Cast visual and dest effect

The visual is `0x60d450` (unit, record, a stack temporary) with a casting unit, else `0x6ec1e0`
(record). Without one the client prints `SPELLVISUALID_NOT_FOUND|%d` with the spell id and skips
the cast kit, the missiles and the impacts. With a unit, the kit named by the visual's dword `+0x8`
plays through `0x60edf0` (unit, record, kit, 0, 0). With a unit, the visual's dword `+0x18` zero and
`+0x30` nonzero, a 0x90-byte effect object is built at the target block's destination point with
the SpellVisualEffectName record `+0x30` names, the spell id and the unit's guid, and started.

Addresses: `0x6e802e`, `0x6e8038`, `0x6e8041`, `0x6e8056`, `0x6e807b`, `0x6e8088`, `0x6e8093`, `0x6e80af`, `0x6e80ba`, `0x6e80e9`, `0x6e813b`, `0x6e8143`, `0x6e82b4`, `0x60d450`, `0x6ec1e0`, `0x60edf0`, `0x6462e0`, `0x61f490`, `0x61fcf0`, `0x6210e0`, `0x44cbd0`, `0xc0d758`
Confidence: verified
Proof: one
Test: none

`0x6e8041` sends a null visual to `0x6e82a7`, `0x44cbd0(1, 2, format, spell)` (`0x6e82b4`), then to
the player tail at `0x6e81ac`. `0x6e8056` indexes the SpellVisualKit store's id map by `+0x8`;
`0x6e807b` pushes 0, 0, kit, record. `0x6e8088` and `0x6e8093` test `+0x18` zero and `+0x30`
nonzero; `0x6e80af` is `SMemAlloc(0x90)`, `0x6e80ba` its constructor. `0x6e80e9` indexes the
SpellVisualEffectName store by `+0x30`. `0x6e813b` passes, first to last, the destination point
`[ebp-0x134]` to `[ebp-0x12c]` by value, the clock, the record's spell id `[ebp-0x160]`, 0, the
effect record, the unit's guid, 0 and a callback; `0x6e8143` calls `0x6210e0` on the object.

## Missile and impact legs

A speed above 0 or NaN, with a casting unit, launches missiles: `0x6e8a50` for the hits; for any
misses `0x6e8d10`, which stores `0x624f20` (unit, reason) over each reason, leaving it unchanged,
then `0x6e8a50` for the misses with their reasons and second bytes; then `0x60c9b0` on the unit when
the visual's `+0x18` is nonzero and the cast kit is absent or its `+0x8` is not above 0. A speed of 0
or below plays impacts at once: `0x6e8bf0` for the hits, `0x6e8cb0` on the casting unit for each
reflect whose second byte is 0. Both play the kit the visual's `+0xc` names with `0x60edf0` (unit,
record, kit, 1, 0) and call `0x60ea70` (unit, 0) when `0x6ea280` gives 2.

Addresses: `0x6e8148`, `0x6e814b`, `0x6e8159`, `0x6e815c`, `0x6e816b`, `0x6e8187`, `0x6e819b`, `0x6e8219`, `0x6e823a`, `0x6e8253`, `0x6e8279`, `0x6e8293`, `0x6e829d`, `0x6e8bf0`, `0x6e8c74`, `0x6e8c7b`, `0x6e8c80`, `0x6e8c89`, `0x6e8cb0`, `0x6e8cea`, `0x6e8cf1`, `0x6e8d10`, `0x6e8d63`, `0x624f20`, `0x60c9b0`, `0x60ea70`
Confidence: verified
Proof: one
Test: none

`0x6e8159 test ah,0x41`, `0x6e815c jp 0x6e8219`: greater masks to 0 and unordered to C3 and C0,
both even, so both jump; equal (C3) and less (C0) are odd and fall to `0x6e816b`. `0x6e8219` exits
for a null unit. `0x6e823a` pushes 0, 0, the hit pair, the target record, inventory type, display
id, visual; `0x6e8279` passes the second-byte pair and the reason pair instead of the zeros.
`0x6e8293 test ecx,ecx; jg` on the kit's `+0x8`. `0x6e8187` and `0x6e818c` test reason 11 and a
zero byte. In `0x6e8bf0` and `0x6e8cb0` the kit is the SpellVisualKit id map at `[visual+0xc]`;
`0x6e8c74` and `0x6e8cea` play it, `0x6e8c7b` and `0x6e8cf1` call
`0x6ea280`, `0x6e8c80 cmp eax,2`, `0x6e8c89` calls `0x60ea70`. `0x6e8d63` stores `0x624f20`'s result over the reason; `0x624f20` is `mov eax,[ebp+0x8]; ret 4`.

## Missile launch

`0x6e8a50` (unit, record, visual, ammo display id, ammo type, target record, list, reasons, second
bytes) needs the unit and a nonzero visual `+0x18` or display id. It launches one missile per list guid when the list is nonempty, the target mask has `0x802`, and
the mask lacks `0x20` and `0x40` or bit 0 of the visual's `+0x3c` is set: it skips a unit for which
`0x605f30` is true and calls `0x60a3d0` (the guid, the target record, display id, ammo type,
is-hit, the reason or 1, the second byte or 0, visual, record, list, 0). Otherwise, with `0x20` or
`0x40` in the mask, it launches one missile at a zero guid, reason 1, the reasons pointer last.

Addresses: `0x6e8a50`, `0x6e8a7f`, `0x6e8a88`, `0x6e8a98`, `0x6e8aa2`, `0x6e8aab`, `0x6e8abc`, `0x6e8ac6`, `0x6e8ad4`, `0x6e8b24`, `0x6e8b3a`, `0x6e8b50`, `0x6e8b84`, `0x6e8ba2`, `0x6e8bd4`, `0x605f30`, `0x60a3d0`
Confidence: verified
Proof: one
Test: none

`ecx` is the unit, `edx` the record, seven stack arguments, `ret 0x1c`. `0x6e8a7f` and `0x6e8a88`
test `+0x18` and the display id. `0x6e8a98 sete al` on the reasons pointer is the is-hit flag.
`0x6e8aa2 test al,0x60` on the mask's low byte; `0x6e8aab` tests `+0x3c` bit 0 and leaves the
launch flag at 0 only when it is clear. `0x6e8abc`, `0x6e8ac6 test eax,0x802` and `0x6e8ad4` gate
the loop. `0x6e8b24` is `0x605f30` for a unit (type bit 3). `0x6e8b3a` loads the second byte,
`0x6e8b50` the default reason 1. `0x6e8ba2` returns when the location bits are clear;
`0x6e8bd4` passes a zeroed local guid.

## Player cast tail

For the player's cast: a first guid that is the player, with flags lacking `0x41`, calls `0x4fd7b0` (spell id). A spell with effect 33 or 59 in a slot of `+0xf4` and exactly one hit
reads that hit as a game object: type 3 calls `0x5ed5f0` (unit, the hit guid); types 0, 1 and 3
match each effect-33 slot's misc value (`+0x1a8`) against the type-2 index slots of the object's
Lock.dbc record and set the object's state (`+0x27c`) with `0x5f8bd0`: action 0 or 1 gives 0, 2
gives 1. Then, when `0x6e5230` holds, or `0x6e5200` with bit 3 of `+0x54`, and the unit's `+0xc48`
guid is 0, `0x6131a0` starts the unit's attack on the first hit, or on guid 0.

Addresses: `0x6e81ac`, `0x6e81b6`, `0x6e81d3`, `0x6e81dc`, `0x6e81de`, `0x6e81e7`, `0x6e81f9`, `0x6e8202`, `0x6e82c1`, `0x6e82df`, `0x6e82fc`, `0x6e830c`, `0x6e831b`, `0x6e832a`, `0x6e8350`, `0x6e8365`, `0x6e8382`, `0x6e83b5`, `0x6e83c0`, `0x6e83d4`, `0x6e83e0`, `0x6e83e7`, `0x6e83e9`, `0x6e83fe`, `0x6e8402`, `0x4fd7b0`, `0x5ed5f0`, `0x5f8180`, `0x5f8bd0`, `0x6e5230`, `0x6e5200`, `0x47bf60`, `0x6131a0`
Confidence: verified
Proof: one
Test: none

`0x6e81b6` and `0x6e81c1` compare the second guid; `0x6e81d3`, `0x6e81dc` the first; `0x6e81de test
flags,0x41`. `0x6e81f9`, `0x6e8202` scan three dwords from `+0xf4`; `0x6e82c1 cmp [ebp-0x20],1`;
`0x6e82df` type mask `0x20`. `0x6e82fc` reads `[[go+0x110]+0x3c]`; `jl`, `cmp 1; jle`, `0x6e830c cmp
3; jne`. `0x6e832a` gets the lock; `0x6e8350` keeps slots whose effect is `0x21`; `0x6e8365` needs
the lock's `+4+4k` equal to 2 and `0x6e8370` its `+0x24+4k` equal to the misc value; `0x6e8382`
takes `+0x64+4k`. `0x6e83b5` pushes 0 or 1. `0x6e83c0`, `0x6e83d4 test [edi+0x54],8`, `0x6e83e0`
`0x47bf60(unit+0xc48)`, `0x6e83e7 jne`, `0x6e83e9` to `0x6e83fe` the first hit or 0.

## Own cooldown start

When the second guid is the player's, the go starts the cast's cooldown on the player's list with
`0x6e12c0` (spell, item, t, recovery, category, t, category recovery, on hold, 0, 0), t the receive
time and on hold bit 25 of `+0x18`. When both guids match: item 0, recovery `+0x4c` through
modifier op 11, category `+0x8`, category recovery `+0x50` through `0x6e2b60`. When the first guid
is an item with an entry id: its record's slot among five spell ids at `+0x11c` gives recovery
`+0x158` if not negative, category `+0x16c` if positive and category recovery `+0x180` if not
negative, each else the spell's column, unmodified; no slot inserts nothing. A record not yet cached
pends through `0x6e8660`, and the handler ends.

Addresses: `0x6e8425`, `0x6e8431`, `0x6e8441`, `0x6e845d`, `0x6e846f`, `0x6e8493`, `0x6e8498`, `0x6e849d`, `0x6e84a9`, `0x6e84c2`, `0x6e84d8`, `0x6e84dd`, `0x6e84f0`, `0x6e8511`, `0x6e8523`, `0x6e8534`, `0x6e8566`, `0x6e8570`, `0x6e7e88`, `0x6e12c0`, `0x6e2b60`, `0x6e6af0`, `0x55ba30`, `0x4b31b0`, `0x4f93d0`, `0xcecaec`
Confidence: verified
Proof: one
Test: none

`0x6e7e88 shr ecx,0x19; and cl,1` makes the flag. `0x6e8425` looks up the first guid with type
mask 2; `0x6e8431` reads the descriptor's `+0xc`. `0x6e8441` compares the guids. `0x6e845d` is
`0x6e2b60(&v, record, 0)` on `+0x50`, `0x6e846f` is `0x6e6af0(record, &v, 0xb)` on `+0x4c`.
`0x6e8498` pushes 0, 0, flag, category recovery, t, category, recovery, t, 0, spell on
`0xcecaec`; `0x6e849d` flushes. `0x6e84a9` skips item 0. `0x6e84c2` is `0x55ba30` with the item
id, the first guid, the callback `0x6e8830`, 1 and 0; a null result reaches `0x6e84d8` and
`0x6e84dd jmp 0x6e863c`. `0x6e84f0` scans five ids; `0x6e8511`, `0x6e8523` and `0x6e8534` use
`jge`, `jg`, `jge`. `0x6e8566` inserts and `0x6e8570` adds `0x4f93d0`.

## Pet cooldown and pruning

After the player's legs, and for any caster, a casting unit whose owner guid is the player's starts
the spell's cooldown on the pet's list: spell, item 0, recovery `+0x4c` and category recovery
`+0x50` both through modifier op 11, category `+0x8`, the same time and on-hold flag. The owner is
the unit's descriptor guid `+0x10` past `+0x110`, or `+0x18` when that is 0. Then, when the receive
time less the prune time is not negative as a signed value, the prune time becomes t + 120000 and
`0x6e1910` (t) prunes both lists. A pended item cooldown skips both steps.

Addresses: `0x6e857a`, `0x6e858b`, `0x6e859a`, `0x6e85a2`, `0x6e85ad`, `0x6e85c5`, `0x6e85d1`, `0x6e85f2`, `0x6e85f7`, `0x6e85fc`, `0x6e8601`, `0x6e8606`, `0x6e860f`, `0x6e8611`, `0x6e862b`, `0x6e1910`, `0x4bce90`, `0xcecb04`, `0xcee8b8`
Confidence: verified
Proof: one
Test: none

`0x6e857a` needs the unit; `0x6e858b` reads `+0x14` and `+0x10`, `0x6e8598 jne` keeps `+0x10`, else
`0x6e859a` takes `+0x18`; `0x6e85a2` and `0x6e85ad` compare with the player. `0x6e85c5` and
`0x6e85d1` are `0x6e6af0(record, &v, 0xb)`. `0x6e85f2` loads `0xcecb04`; `0x6e85f7` inserts with the
same argument order as the spell leg; `0x6e85fc` and `0x6e8601` call `0x4b31b0` and `0x4bce90`.
`0x6e860f js` skips; `0x6e8611` adds `0x1d4c0`; the loop at `0x6e862b` runs twice from `0xcecaec`
in steps of `0x18`. `0xcee8b8` has no other writer and no file bytes, so the first go prunes.

## Pending item cooldowns

`0x6e8660` (item id, spell id, time, on hold) finds or adds the `ITEMCOOLDOWNHASHNODE` keyed by the item id in
the hash `0xceca60` and stores the spell at `+0x18`, the time at `+0x1c` and the flag at `+0x20`; a new
node also gets the key, and at `+0x14` the flag argument's high byte, 0 from the go. `0x6e8830`, the
item cache's callback, takes its item's node; with the record now cached and the spell among its
five, it inserts as the go's item leg does, with the pended time and flag, unless recovery and
category recovery are both 0. It flushes and frees the node. The category values need the
callback's first stack argument, 1 from the go, nonzero.

Addresses: `0x6e8660`, `0x55bdb0`, `0x6e867a`, `0x6e86a9`, `0x6e86c9`, `0x6e8735`, `0x6e8787`, `0x6e87c0`, `0x6e87ef`, `0x6e8806`, `0x6e8814`, `0x6e8830`, `0x6e8899`, `0x6e88d1`, `0x6e88f0`, `0x6e8943`, `0x6e8976`, `0x6e89a3`, `0x6e89a8`, `0x6e8a3a`, `0x55bf2a`, `0x55bf39`, `0xceca60`
Confidence: verified
Proof: one
Test: none

The hash keeps its bucket array at `+0x1c`, 12 bytes a bucket, and its mask at `+0x24`, -1 before
the first insert, which sets 3 and builds four buckets (`0x6e86c9`). A key's bucket is `key & mask`;
`0x6e86a9` matches `[node] == key`. Below a mask of `0x1fff`, an insert lowers the counter `+0x10` by
3, floored at 0, adds one for each node in the bucket and, past 13 (`0x6e8787`), regrows to
`2·mask + 2` (`0x6e87c0`). `0x6e87ef` allocates through slot 1 of the hash's vtable; `0x6e8806` to
`0x6e8814` store. In `0x6e8830`, `0x6e88d1` asks the cache again with no callback; `0x6e88f0`
scans `+0x11c`; `0x6e8943` tests `[ebx+0x8]`, which the cache passes as the stored argument
on its record's arrival (`0x55bf2a`, `0x55bf39`, in the receive `0x55bdb0`); `0x6e8976` skips two zeros; `0x6e89a3` inserts. The node is unlinked and
`0x6e8a3a` calls slot 0. The spell record lookup here has no null check before `+0x4c` (`0x6e8935`).
