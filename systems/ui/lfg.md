# ui: looking for group

`PartyFrame.cpp` keeps the player's three looking-for-group slots and a comment, groups the dungeon rows into six
type lists, and exchanges two opcodes with the server: opcode 0x200 carries the slots out, opcode 0x1ff
carries a result list in and an inquiry out. The functions are `SetLookingForGroup`, `GetLookingForGroup`,
`LFGQuery` and their result readers; the party state they share a file with is `party`.

Roots: `0x4e7c70`, `0x4e96b0`, `0x4e95d0`, `0x4e9760`, `0x4e8dc0`

## The type lists

The table `LfgDungeons` (store `0xc0db2c`, 14 fields, 56-byte rows) is grouped by its type word at `+0x30`: the
start-up function `0x4e7c70` copies each row's id into the dynamic array of its type, six lists of 12 bytes at
`0xbc6f90`. Type 3 is the quest list, 20 slots filled from the quest log. `0x4e89c0` recounts the lists for the
player: a row counts when the player's level lies in `[MinLevel, MaxLevel]` (`+0x28`, `+0x2c`) and its side word
(`+0x34`) is negative or equals the player's side, 0 for a faction template with flag 0x4, 1 for flag 0x2, else -1. The
counts go to the six words at `0xbc6f18`, each list is sorted by `0x4e8b20`, and `UPDATE_LFG_TYPES` is signalled.

Addresses: `0x4e7c70`, `0x4e7d19`, `0x4e7e17`, `0x4e89c0`, `0x4e89f2`, `0x4e8a62`, `0x4e8a6c`, `0x4e8add`, `0x4e8b0e`, `0x4e8b20`, `0x4e8be0`, `0x4e8c5b`, `0x4e8c70`, `0x4e8c80`, `0xbc6f18`, `0xbc6f90`, `0xbc7624`, `0xc0db2c`, `0xc0db34`, `0xc0db38`
Confidence: verified
Proof: one
Test: `ui::sortcmp_4e8b20`, `ui::sortcmp_4e8c80`

`0x4e89c0` stores the player's level in `0xbc7624` and counts into the word at `0xbc6f18 + 4*type`. The type 3 count is
written by `0x4e8be0` instead (`0x4e8c5b`): it collects the nonzero quest ids of the 20 log slots, `[[player+0xe68] + 0x28 + 12*i]`, sorts them
with `0x4e8c80` and tail-jumps to the event signal (event 531, `0x4e8c70`). The row comparator `0x4e8b20` puts rows
that exist before missing ones, rows whose level range holds the player's level before the others, then orders by the
locale name, case-insensitive. The quest comparator `0x4e8c80` puts quests whose title is cached before the rest, then orders
by title (`+0x9c` of the cached record). `0x4e7e1e` zeroes the six counts.

Bit-exact coverage: `ui::sortcmp_4e8b20` covers `0x4e8b20` to `0x4e8b40`, `0x4e8b44` to `0x4e8b54`, `0x4e8b58` to `0x4e8bd3` of `0x4e8b20`; `ui::sortcmp_4e8c80` covers the whole of `0x4e8c80`.

## SetLookingForGroup

`SetLookingForGroup(type1, entry1, type2, entry2, type3, entry3, comment)` returns no value and never raises. For each of
up to three pairs it reads the type (a number, else it stops), rejects a type above 5, reads the entry
without a type check, rejects an entry not below the type's count, and stores a slot word. The first failing pair ends the loop;
the slots after it keep 0. The comment is a string read from argument 7, and only when argument 4 is a number or a string.
Then it calls `0x4e88c0(slots, comment)`.

Addresses: `0x4e96b0`, `0x4e96d5`, `0x4e96e7`, `0x4e96ee`, `0x4e9702`, `0x4e9707`, `0x4e9710`, `0x4e9713`, `0x4e9728`, `0x4e9731`, `0x4e973a`, `0x4e9741`, `0x4e974d`, `0x4e9754`
Confidence: verified
Proof: one
Test: none

Type and entry come from `lua_tonumber` and `__ftol`; the entry compare is unsigned (`jae` at `0x4e970e`), so a negative
entry fails. The type tests are `ja` at `0x4e96f1` and `jae` at `0x4e9705`, so types 0 to 5 pass. The gate is `lua_isstring(L, 4)`
at `0x4e9731` and the value `lua_tostring(L, 7)` at `0x4e9741`. There is no usage error and no raise from the function or
from `0x4e88c0`.

## The slot word

The word stored for a pair is `(type << 24) AND entry` (`shl esi,0x18; and esi,eax` at `0x4e9710`, `0x4e9713`). Every reader
decodes `(type << 24) OR entry`, with `shr 0x18` and a 24-bit mask in `GetLookingForGroup`. The entry is below the type's
count, a table row count or at most 20 quests, which cannot reach 2^24, so the AND is 0 for every accepted pair: the three slots stay 0, and
`LFGQuery` sends a zero criterion word.

Addresses: `0x4e9710`, `0x4e9713`, `0x4e9604`, `0x4e9618`, `0x4e9823`, `0x4e9826`, `0x4e8dfa`, `0x4e8e0c`
Confidence: verified
Proof: one
Test: none

`0x4e9823` and `0x4e9826` repeat the same two instructions in `LFGQuery`. The other readers use the OR layout: the
results handler tests byte 3 of the word at `+0x38` (`0x4e8dfa`) and masks the low 24 bits (`0x4e8e0c`).

## The slots packet

`0x4e88c0(slots, comment)` compares the three slot words with the stored ones at `0xbc70a0` and copies those that differ.
A nonnull comment that differs from the stored comment (`0xbc6e98`, 0x80 bytes) is copied by `SStrCopy` and forces a
send; an equal comment sends only when a slot changed. Nothing is sent otherwise. The packet is `u32 0x200`, the three
slot words as `u32`, and the stored comment as a NUL-terminated string, always present. Because the slots stay 0, only a
new comment sends.

Addresses: `0x4e88c0`, `0x4e88e0`, `0x4e88e7`, `0x4e88ef`, `0x4e8911`, `0x4e8918`, `0x4e8925`, `0x4e892c`, `0x4e8948`, `0x4e8956`, `0x4e896a`, `0x4e897f`, `0x4e898e`, `0xbc70a0`, `0xbc6e98`
Confidence: verified
Proof: one
Test: none

The opcode 0x200 is pushed at one place only, `0x4e8948`; the client registers no handler for it. The slots are read from
the globals after the copy. The function has one caller and no stored address.

## GetLookingForGroup

`GetLookingForGroup()` takes no argument and returns four values: for each slot a string or nil, then the comment.
A slot of type 3 gives the title of the quest of that id from the quest cache, nil while it is not cached, and no query is
sent because the callback is null. Any other slot gives `Name_lang[locale]` (`+4 + 4*locale`) of the `LfgDungeons` row of that id, nil for an id
that is negative, past `0xc0db38` or without a row. The comment is the stored string, empty when unset.

Addresses: `0x4e95d0`, `0x4e9604`, `0x4e961d`, `0x4e9635`, `0x4e9640`, `0x4e9651`, `0x4e965f`, `0x4e966c`, `0x4e968c`, `0x4e969f`, `0x562a40`, `0xc0e1b0`
Confidence: verified
Proof: one
Test: none

The function returns 4 (`0x4e969f`) and has no argument access. With the slots always 0, the first three values are nil
because no row has id 0. The readback is local: the 0x1ff handler never writes the slots or the comment.

## The results

`LFGQuery(type, entry [, extra])` raises `Usage: LFGQuery(param1 [, class])` without a number as argument 1; a type
of 6 or more or an entry not below the type's count sends nothing. Otherwise it sends `u32 0x1ff`, the criterion
word `(type << 24) AND entry` and the extra number (0 when absent). Opcode 0x1ff arrives at `0x4e8dc0`: `u32 count`
(clamped to 50), `u32 second`, then per result a name (cap 0x30), two `u32`, three slot words and a comment (cap
0x80), stored in 50 records. A type 3 slot asks the quest cache for its title, counted in `0xbc7620`;
`UPDATE_LFG_LIST` is signalled at once when nothing is outstanding, else when the last answer lands.

Addresses: `0x4e9760`, `0x4e977e`, `0x4e97b2`, `0x4e97be`, `0x4e97f9`, `0x4e9840`, `0x4e8dc0`, `0x4e8e40`, `0x4e8e4a`, `0x4e8e56`, `0x4e8e60`, `0x4e8e8e`, `0x4e8e9b`, `0x4e8ea8`, `0x4e8eba`, `0x4e8ef4`, `0x4e8efb`, `0x4e8f17`, `0x4e8f37`, `0x4e8f45`, `0x4e8f60`, `0x4e8d30`, `0x4e7cc7`, `0x563280`, `0xbc6fd8`, `0xbc7618`, `0xbc761c`, `0xbc7620`
Confidence: verified
Proof: one
Test: none

The record is 0x44 bytes of name and numbers then the 0x80-byte comment: `0x44 + 0x80 = 0xc4`, the size allocated by
`0x4e7c70` for each of 50 records. `GetLFGResults` pushes `+0` as a string, `+0x30` as a number and resolves `+0x34` through
the ChrClasses table. Before it overwrites the list, the handler walks the old records while queries are outstanding
and calls `0x563280(entry, 0x4e8f60, 0)` on the quest cache for each type 3 slot, then zeroes the counter. The callback thunk `0x4e8f60` jumps to `0x4e8d30`,
which decrements the counter and signals event 532 at zero. Event 389 (`UPDATE_LFG`) has no fire site (`tables/event_sites.tsv`). `0x4e7c70` registers opcodes 0x329 and 0x2cb beside 0x1ff.
