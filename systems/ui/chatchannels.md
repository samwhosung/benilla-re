# ui: chat channels

The client's list of joined channels, the ChatChannels rows that name them, the walk that joins the
zone channels by itself on every zone change, and the channel notify message that confirms, renames
and drops the slots. The Lua verbs that join, leave and list are `channelverbs`; the line a notice
becomes is `chatevents`.

The code is `ChatFrame.cpp`: the slot helpers `0x49b980` to `0x49bf30`, the walk `0x49a210`, the notify
handler `0x49bf80` and the list handler `0x49c690`. The message handlers are registered by the chat
start-up (`chatwindows#chat-shutdown`).

Roots: `0x49a210`, `0x49b980`, `0x49bf80`, `0x49c690`, `0x49bd50`

## Joined channel slots

The client keeps up to ten joined channels in an array of 0xa0-byte slots (count `0xb4fe00`, base
`0xb4fe04`): `+0x00` the slot's own 1-based position, 0 while the slot is free; `+0x04` the channel's
name (128 bytes); `+0x84` the member list header; `+0x94` the ChatChannels id, 0 for a custom
channel; `+0x98` the instance number from the join notice; `+0x9c` the state: 0 confirmed by the server,
1 waiting for the server, 2 renamed and waiting, 3 suspended. A free slot is reused before the array grows.

Addresses: `0xb4fe00`, `0xb4fe04`, `0x49b980`, `0x49b9b0`, `0x49b9c3`, `0x49b9c5`, `0x49b9d9`, `0x49b9e0`, `0x49ba01`, `0x49ba0f`, `0x49bad7`, `0x49baf9`, `0x49bb05`, `0x49be50`, `0x49bf30`, `0x4a30d0`, `0x4a2ca0`
Confidence: verified
Proof: one
Test: none

`0x49b980(name, id)` first searches by name (`0x49be90`) and returns if the slot exists. Otherwise it takes
the first slot whose `+0x00` is 0 (`0x49b9b0`), numbering it with its position (`0x49b9e0`); with none free
and ten used (`0x49b9c3`) it shows system message 0x199 (`0x49b9c5`) and makes no slot; else it grows
the array by one (`0x4a3db0`, `0x4a3df0`, `0x4a3e10`) and numbers the new slot `count`. Either way
the name is copied with the limit 0x80, `+0x94` is the id (`0x49ba01`, `0x49baf9`) and `+0x9c` is 1. The
instance number is written by no allocator, so a reused slot keeps the last one.

## Finding a slot

`0x49be90(name)` returns the slot whose name equals the argument without case, whatever its state.
`0x49bdf0(name)` returns the slot's number only when its state is 0. `0x49bef0(id)` returns the first
slot with that ChatChannels id. `0x49bf30(n)` returns slot n (1 to count, its `+0x00` equal to n) only
while its state is 0; `0x49be50(n)` is the same test returning the slot's name instead.

Addresses: `0x49be90`, `0x49bdf0`, `0x49bef0`, `0x49bf30`, `0x49be50`, `0x49bf5c`, `0x49bf68`, `0x49be6b`, `0x49be73`
Confidence: verified
Proof: one
Test: none

Every search is linear over `count` slots with `0x64a4c0(slot+4, name, 0x7fffffff)`, a whole-string
comparison ignoring case; `0x49be90` tests neither `+0x00` nor `+0x9c`. `0x49bf30` builds its mask
without a branch (`setne; dec; and`, `0x49bf5c` to `0x49bf68`); `0x49be50` checks `+0x00` and `+0x9c`
with two compares (`0x49be6b`, `0x49be73`) and returns `slot+4`.

## Confirming, renaming and dropping

The server's join notice confirms a slot: `0x49bb20(name, instance)` finds the first slot of that name
whose state is not 0, sets the state to 0, sets bit id-1 of the zone channel mask when the id is not 0 and
stores the instance. `0x49bc50(old, new)` renames the slot in place, copying the new name over the old and
setting the state to 1, or to 2 when it was not suspended. `0x49bcf0(name)` suspends a slot (state 3) and
touches nothing else. `0x49bbd0(name)` frees the slot: number 0, name emptied, state 1, id kept.

Addresses: `0x49bb20`, `0x49bb43`, `0x49bb63`, `0x49bb8b`, `0x49bbaf`, `0x49bbb8`, `0x49bbbf`, `0x49bc50`, `0x49bcb9`, `0x49bccc`, `0x49bcd3`, `0x49bcf0`, `0x49bd3a`, `0x49bbd0`, `0x49bc1b`, `0x49bc29`, `0x49bc35`, `0xb6e5e0`
Confidence: verified
Proof: one
Test: none

The state writes are `0x49ba0f`, `0x49bb05` and `0x49bc35` (1), `0x49bb8b` (0), `0x49bcd3`
(`(old != 3) + 1`, from `setne dl; inc edx` at `0x49bccf` and `0x49bcd2`) and `0x49bd3a` (3). The
confirm skips slots already at 0 (`0x49bb63`), so a repeated notice changes nothing. All of them call
`0x49bd50` after a change that can matter (`0x49bbbf`, `0x49bc41`). The drop has one caller, `0x49c5c2`.
So only a server notice whose channel name matches a slot's stored name frees it.

## Defence channel state

After a confirm or a drop `0x49bd50` recomputes whether the player is in a zone-scoped defence channel:
a slot with a number, state 0, a ChatChannels row whose flags include 0x10000 and 0x2. When the
answer differs from the stored one at `0xb6e5dc`, it stores it and, if `0xb71e84` is not negative,
calls `0x4c56e0` and fires `UPDATE_WORLD_STATES` (id 526). Nothing else sets that value.

Addresses: `0x49bd50`, `0x49bd72`, `0x49bd9b`, `0x49bda2`, `0x49bdb8`, `0x49bdc0`, `0x49bdcf`, `0x49bddc`, `0x4c56e0`, `0x703e50`, `0xb6e5dc`, `0xb71e84`
Confidence: verified
Proof: one
Test: none

The loop reads each slot's id through the row index (`0xc0df1c`, bounded by `0xc0df20`),
`mov eax,[eax+4]; test eax,0x10000; test al,2` (`0x49bd9b`, `0x49bda2`). `0x49bddc` ends in a tail jump to
`0x703e50` with `ecx = 0x20e`.

## ChatChannels rows

A ChatChannels row is 84 bytes: `+0x00` id; `+0x04` flags; `+0x0c` plus 4 times the locale index the
name pattern, a format with at most one `%s`; `+0x30` plus the same the shortcut, the word a player
types. Flag 0x1 marks a channel joined by default, 0x2 a zone channel, 0x10 a city-only channel,
0x20 a channel whose `%s` is the city name in a capital, 0x10000 a defence channel and 0x20000 the
guild recruitment channel. Shipped data has six rows: General, Trade, LocalDefense, WorldDefense,
LookingForGroup and GuildRecruitment.

Addresses: `0xc0df14`, `0xc0df18`, `0xc0df1c`, `0xc0df20`, `0x542120`, `0x5421ce`, `0x542206`, `0x49a488`, `0x4997e8`, `0x49a2d0`, `0x49a3b8`, `0x49a2f6`, `0x49ecec`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

The loader `0x542120` asserts 21 fields and record size 84, indexes rows by field 0 into the id map (`0xc0df1c`,
size `maxId + 1`, `0xc0df20`). Consumers fix the columns: the id at `0x49a488`, the flags at `0x4997e8`,
`0x49a2d0` and `0x49a3b8`, the pattern at `0x49a2f6` and the shortcut at `0x49ecec`. The flag meanings
are those tests: 0x1 at `0x4997e8`, 0x20 at `0x49a2d0`, 0x10 at `0x49a3b8`. The row values are install
data, read through those offsets.

## Composing a channel name

A zone channel's name is `0x64a7f0(buf, 0x100, pattern, S)`. S is the current zone text (`0xb4b404`),
except that for a row with flag 0x20 standing in an area whose AreaTable flags include 0x100, S is
the area name of the one AreaTable row whose flags include 0x200, cached at `0xb4e4f0`; the shipped
row is named `City`, so Trade in a capital is `Trade - City`. The zone text is the parent zone's name.

Addresses: `0x4985bc`, `0x4985fd`, `0xb4e4f0`, `0x49a2d0`, `0x49a2df`, `0x49a2ed`, `0x49a308`, `0x49a4af`, `0x49a4ea`, `0x49eccc`, `0x4948e9`, `0xb4b404`, `0xb4e314`, `0x64a7f0`, `0x67e57d`
Confidence: verified
Proof: one
Test: none

`0x4985bc` to `0x4985fd` is a reverse linear scan of the AreaTable rows (stride 100, `test [row+0x10],0x200`)
run once by the chat start-up, storing the last match. The test chain at `0x49a2d0` to `0x49a2f3`
is `test byte [chatRow+4],0x20`, `test ah,1` on the area's flags, a non-null `0xb4e4f0`, then the
name at `+0x2c` plus the locale. The zone text `0xb4b404` is stored by the zone text update at
`0x4948e9` from the top-level area's name; the indoor name override of the zone text does not reach
it. The test is on the row pointer, not the string.

## The zone channel mask

The zone channels the client joins are the bits of `0xb6e5e0`: bit id-1 for the ChatChannels row of that
id. The walk tests it per row. Writers: the settings file's `ZONECHANNELS` line (`0x498d83`); the
defaults without a file, which set the bit of each row with flag 0x1 (`0x4997fc`); the join confirm
(`0x49bbaf`); the leave verb, which clears it (`0x49f10a` to `0x49f11a`). A fresh character's mask is 0x200003,
the rows 1, 2 and 22.

Addresses: `0xb6e5e0`, `0x498d83`, `0x4997fc`, `0x49bbaf`, `0x49f10a`, `0x49f11a`, `0x49a494`, `0x499c1b`, `0x49a134`
Confidence: verified
Proof: one
Test: none

An image scan for the dword finds eight references: those four writers, the test `0x49a494` and the two
reads that write the mask to the settings file. The mask lies past the initialised data, so it is 0 until
the first of these runs.

## The zone walk

`0x49a210` joins and leaves the zone channels. It runs once when the settings load, on every change of
zone, when the active player's `UNIT_FIELD_FLAGS` change 0x100000, and when a cinematic ends. It returns at once while the chat is
not ready (`0xb6e5c8`) or when the zone is negative, past the AreaTable maximum or has no row; with an
empty zone text it skips the channels and goes to the guild recruitment tail. Otherwise it makes three
passes.

Addresses: `0x49a210`, `0x49a219`, `0x49a225`, `0x49a22b`, `0x49a243`, `0x49a24a`, `0x49a275`, `0x49a290`, `0x49a452`, `0x49a468`, `0x49a5b4`, `0x49a6a4`, `0x499a22`, `0x494931`, `0x5ff56e`, `0x48f1d0`, `0xb6e5c8`, `0xb6e5d0`, `0xb6e5e4`, `0xb4e314`
Confidence: verified
Proof: one
Test: none

The four callers are `0x499a22`, `0x494931` (after `0xb4b404` is stored and before `ZONE_CHANGED_NEW_AREA`),
`0x5ff56e` (the active player's flags watcher) and `0x48f1d0`; those three skip the call while the active
player's `UNIT_FIELD_FLAGS` has 0x100000, and `0x494931` and `0x5ff56e` also during a cinematic (`0xb4e310`). `0x49a24a` stores the zone id in `0xb6e5d0`
before the sign test. The empty-text exits `0x49a235`, `0x49a23d` go to `0x49a6a4`; the others to `0x49a6b7`.

## Walk, slots already held

For each used slot whose id names a row, the pass composes the new name. A suspended slot skips the
name comparison. If the new name equals the slot's the slot is left alone. Otherwise a confirmed slot
first sends a leave with the old name. A city-only row in an area whose flags lack 0x8 then suspends
the slot and sends nothing; any other row renames the slot (`0x49bc50`) and sends a join with the new
name and an empty password. A leave always precedes its join, slot by slot.

Addresses: `0x49a2b3`, `0x49a312`, `0x49a31c`, `0x49a332`, `0x49a35e`, `0x49a367`, `0x49a3b8`, `0x49a3c1`, `0x49a3cb`, `0x49a3dc`, `0x49a3e1`, `0x49a410`, `0x418190`, `0x418430`, `0x5ab630`
Confidence: verified
Proof: one
Test: none

`cmp [slot+0x9c],0; jne` at `0x49a35e` gates the leave, which builds opcode 0x98 (`push 0x98`) and the
slot's old name (`0x418430`) and sends it with `0x5ab630`. The join is opcode 0x97 with the new name and
the empty string `0x882748`. The renamed slot's state becomes 2, or 1 from suspended. A walk from one
zone to another therefore ends with the same slot numbers and the new names, sent as leave, join,
leave, join.

## Walk, rows not held

For each ChatChannels row whose mask bit is set, the second pass finds no slot with that id
(`0x49bef0`) and none with the composed name, registers a slot (`0x49b980`) and sends the join, after
the same city-only test, which registers the slot before it suspends it. The third pass registers and joins each
name in the deferred-join list, with id 0, then frees the list. The tail clears the one-shot flag
`0xb6e5e4` and runs the guild recruitment cascade (`channelverbs#guild-recruitment-cascade`) when it was set.

Addresses: `0x49a468`, `0x49a488`, `0x49a494`, `0x49a4cf`, `0x49a50d`, `0x49a512`, `0x49a527`, `0x49a536`, `0x49a5b4`, `0x49a5da`, `0x49a5e2`, `0x49a6ac`, `0x49a6b2`, `0x49bef0`, `0x49be90`
Confidence: verified
Proof: one
Test: none

`0x49a50d` registers with the row id and precedes the eligibility test at `0x49a512`, so Trade takes slot
2 outside a capital and is suspended at once with no join. `0x49a5da` registers with `xor edx,edx`.
`0xb6e5e4` has one reader (`0x49a6a4`), one writer of 0 (`0x49a6ac`) and the two writers of 1 inside the cascade.

## Zone changes keep channels

The leave notice for a renamed slot cannot match a slot, so the interface keeps its channel list. The
client renames the slot before the server answers; the leave notice's name then matches no slot, so the
handler passes the line with channel id 0, slot number 0 and name empty, and drops nothing; the join
notice finds the renamed slot in state 2 and reports `YOU_CHANGED`. A leave of a suspended slot reports
`SUSPENDED` and keeps the slot.

Addresses: `0x49bc50`, `0x49c07e`, `0x49c087`, `0x49c0cc`, `0x49c0d7`, `0x49c0e0`, `0x49c0ff`, `0x49c115`, `0x49c5b5`, `0x49c5c2`, `0x49a9fc`, `0x49aa27`, `0x49aa31`, `0x49b12f`
Confidence: verified
Proof: one
Test: none

`0x49c0cc` looks the name up (`0x49be90`) and takes `SUSPENDED` only for a found slot with state 3, else
`YOU_LEFT`, and sets the drop flag `[ebp-0xc]` at `0x49c115`; `0x49c5c2` drops by name after the event.
The chat line's own name lookup (`0x49aa27`) misses, the marshaller gets no slot (`0x49b12f`) and fires
with channel id, slot number and name 0, 0 and empty. `0x49c07e` to `0x49c090` choose `YOU_CHANGED` when the
slot's state is 2.

## The channel notify message

Opcode 0x99, handled by `0x49bf80`: a u8 notice, the channel name (at most 128 bytes), then the
fields the notice needs (`tables/channel_notices.tsv`). A notice above 31 shows an empty system line.
Each arm picks a token and a chat type and calls the chat line function `0x49a870` with the token as
the text and the channel name as the channel. Notices 0 and 1 read a GUID and show nothing unless the
channel is joined and confirmed. Notice 12 reads a GUID and two bytes and calls `0x49e910` instead of
showing a line. Notice 2 reads two u32, discarding the first, and a topic string when bytes remain.

Addresses: `0x49bf80`, `0x49bf95`, `0x49bfa8`, `0x49bfcd`, `0x49bfe5`, `0x49bff3`, `0x49c60c`, `0x49bffa`, `0x49c005`, `0x49c01b`, `0x49c036`, `0x49c041`, `0x49c050`, `0x49c065`, `0x49c24d`, `0x49c283`, `0x49c4df`, `0x49c4e4`, `0x49c595`, `0x49c5b0`, `0x49c5c7`, `0x49c5d3`, `0x49c5f8`, `0x49e910`, `0x49c430`, `0x49c457`, `0x49c115`, `0x49c13c`, `0x49c157`, `0x49c39a`, `0x49c3a9`, `0x49c42b`, `0x49c43a`, `0x49c446`, `0x49c452`, `0x49c461`, `0x49c46d`, `0x49c5b5`, `0x49c5c2`, `0x4190b0`, `0x418e30`, `0x418cb0`, `0x4191b0`, `0x5ab650`
Confidence: verified
Proof: one
Test: none

The registration is `0x498604` and `0x49861a` (`mov ecx,0x99; call 0x5ab650`). The default chat type is
`mov edi,0xa` (`0x49bfe5`); arms set `edi` to 0x0f, 0x10 or 0x12, notices 26 and 27 set 0x13 themselves (`0x49c430`,
`0x49c457`), the arms that enter at `0x49c4df` take 0x13 from `mov edi,0x13`, and those that enter at `0x49c4e4` keep 0x12. The shared tail `0x49c4e9`
resolves the first GUID's name through the name cache into the sender, and the second one's into the
fifth argument; `0x49c595` to `0x49c5b0` push the ten arguments. Notice 2 fires a second line, the topic, at
`0x49c5f8` with the same type when `[ebp+0xf] == 2` and the topic string is not empty. The drop flag is
set by notices 3 (YOU_LEFT only), 4, 5, 19, 26 and 27 and consumed after the event. The stores are `0x49c115` (3), `0x49c13c` (4), `0x49c446` (19 by the jump `0x49c3a9` to `0x49c43a`, 26 by falling through from `0x49c42b`) and `0x49c46d` (5 by the jump `0x49c157` to `0x49c461`, 27 by falling through from `0x49c452`).

## Channel list message

Opcode 0x9b, handled by `0x49c690`, builds the member list of a channel as a comma-separated string and
hands it to the chat line function twice, with chat type 0x11 (`CHANNEL_LIST`), once per part of the
list, with the stored channel name as the channel and every other argument 0.

Addresses: `0x49c690`, `0x49c9dc`, `0x49ca7b`, `0x49a870`, `0x498604`, `0x49862a`
Confidence: inferred
Proof: none
Test: none

The two calls to `0x49a870` are `0x49c9dc` and `0x49ca7b`, each with `edx = 0x11`. The member list
format and the split between the calls are not read here.
