# ui: the reputation list

The reputation list (`ReputationInfo.cpp`, `0x4d5200` to `0x4d6d80`): the 64 slots the server fills, the list entries and
headers the panel shows, the standing arithmetic, the flag bits, which factions get a row, and how rows are sorted and
counted. The Lua functions are `ui/reputationlua`; the messages and requests are `ui/reputationwire`.

Roots: `0x4d5290`, `0x4d5430`, `0x4d5a70`, `0x4d5c40`, `0x4d6370`, `0x4d63a0`

## Reputation slots

The client keeps 64 reputation slots of 16 bytes: the faction id (`+0`), the flag byte the server sent (`+4`), the base
standing from `Faction.dbc` (`+8`) and the standing the server sent (`+0xc`). A slot's index is the faction's
`reputationListID`, field 1 of its `Faction.dbc` row, valid from 0 to 63. Each reader tests the index below 0x40. The two
writers of flags and standing do not, so a slot index from the wire above 63 writes outside the array. Every reference
to the array lies inside this file's code.

Addresses: `0x4d5360`, `0x4d5426`, `0x4d54ee`, `0x4d553c`, `0x4d5fc0`, `0x4d5fc3`, `0x4d6330`, `0x4d633b`, `0x4d5600`, `0x4d5620`, `0x4d5624`, `0x4d54e0`, `0x4d6004`, `0x4d610c`, `0x4d61b9`, `0x4d61e9`, `0x4d6220`, `0x4d637c`, `0xb73290`
Confidence: verified
Proof: one
Test: none

The teardown clears 0x100 dwords (`0x4d5426`). Each field has one writer: the id at `0x4d54ee`, the base at `0x4d553c`,
the flags at `0x4d5fc3` and the standing at `0x4d633b`. The flag byte has 13 references and the other fields 3, 2 and
2. `0x4d5600` maps a faction id to its `reputationListID` through the id map and returns -1 for an id without a row;
`0x4d5620` maps a slot back to its faction id, 0 outside 0 to 63. A reputation change does not write the base: it is
set on each rebuild.

## Faction rows

The reputation code reads these fields of a `Faction.dbc` record, which is 0x94 bytes: the id (`+0`), the
`reputationListID` (`+4`), four race masks (`+8`), four class masks (`+0x18`), four base standings (`+0x28`), the team
(`+0x48`), the name (`+0x4c`) and the description (`+0x70`). The name and the description are one column per locale,
picked by the locale index. The four `ReputationFlags` columns after the bases are read nowhere, so the server's flag
byte is the only source of flags.

Addresses: `0xc0dd48`, `0xc0dd50`, `0xc0dd54`, `0x4d54a7`, `0x4d54f4`, `0x4d5500`, `0x4d5536`, `0x4d5b35`, `0x4d5b5d`, `0x4d5e19`, `0x4d6565`, `0x4d6576`
Confidence: verified
Proof: one
Test: `ui::sortcmp_4d5dc0`

The stride is `imul edx,edx,0x94` (`0x4d54a7`). The loop of `0x4d5430` reads `[esi-0x10]` (race mask), `[esi]` (class mask)
and `[esi+0x10]` (base) from `esi = row + 0x18` stepping 4 (`0x4d54f4`, `0x4d5500`, `0x4d5536`). The team is read at
`0x4d5b35` and `0x4d5b5d`, the name at `0x4d5e19` and `0x4d6565`, the description at `0x4d6576`. No memory operand in the file
names the displacements of the flag columns.

Bit-exact coverage: `ui::sortcmp_4d5dc0` covers `0x4d5dc0` to `0x4d5e07`, `0x4d5e0b` to `0x4d5e6d` of `0x4d5dc0`.

## Standing and rank

A faction's standing is the slot's base plus the slot's wire standing, 0 for a faction without a slot. The rank is 0 to 7
by thresholds on the standing, each inclusive: 42000 and up is 7, 21000 is 6, 9000 is 5, 3000 is 4, 0 is 3, -3000 is 2,
-6000 is 1, below that 0. The table of rank minimums holds -42000, -6000, -3000, 0, 3000, 9000, 21000, 42000 and 43000.

Addresses: `0x4d6370`, `0x4d6383`, `0x4d6389`, `0x4d63a0`, `0x4d63a5`, `0x4d63e3`, `0x4d63f2`, `0x80928c`
Confidence: verified
Proof: one
Test: none

`0x4d6370` adds the two cells at `0x4d6383` and `0x4d6389`. `0x4d63a0` is a ladder of signed `jl` compares against
0xa410, 0x5208, 0x2328, 0xbb8, 0, -3000 (`0xfffff448`) and -6000 (`0xffffe890`); the last leg is `setge`, so -6000 is rank 1.
The table `0x80928c` has the same nine values and is read by the Lua functions for the bar bounds.

## Base standing pick

At each rebuild a slot's base is picked from the four (race mask, class mask, base) triples of its row, for the active
player's race and class bits (bit race-1 and class-1). A triple matches when its race mask is 0 or has the race bit and its
class mask is 0 or has the class bit, except that a triple with both masks 0 never matches. The loop does not stop at the
first match, so the last matching triple's base is the one that stays.

Addresses: `0x4d545f`, `0x4d5463`, `0x4d5503`, `0x4d5507`, `0x4d5509`, `0x4d550b`, `0x4d5512`, `0x4d551c`, `0x4d5520`, `0x4d5522`, `0x4d5531`, `0x4d5533`, `0x4d553c`, `0x4d5560`, `0x4d5564`
Confidence: verified
Proof: one
Test: none

Race and class are the bytes at `+0x78` and `+0x79` of the unit block (`0x4d545f`, `0x4d5463`). With a race mask of 0
the code tests the class mask: 0 skips (`0x4d550b je 0x4d555a`), otherwise the class test follows (`0x4d550d`). The loop counter
runs four times unconditionally (`0x4d5560`, `0x4d5564`), and each match stores the base again (`0x4d553c`).

## Flag bits

The flag byte of a slot is the server's. The client tests: bit 0 for visible, which gates membership; bit 1 for at war;
bit 2 for hidden, which suppresses the auto-reveal and the rank-change message; bit 3 for header, which becomes the
entry's header flag; bit 4 for peace forced, which blocks both war toggling and the toggle checkbox; bit 5 for inactive,
which moves the row under the Inactive header. Bit 6 and bit 7 are never tested.

Addresses: `0x4d554a`, `0x4d5728`, `0x4d5815`, `0x4d61c1`, `0x4d56db`, `0x4d6065`, `0x4d57df`, `0x4d5885`, `0x4d5acb`, `0x4d61e0`, `0x4d61f1`, `0x4d6210`, `0x4d6228`, `0x4d611a`, `0x4d6122`, `0x4d6126`
Confidence: verified
Proof: one
Test: none

The tests are `test [..],1` (`0x4d554a`, `0x4d5728`) and `test al,5` (`0x4d5815`, visible or hidden); `shr eax,1` (`0x4d61c1`); `test al,4`
at `0x4d57df` and `test [ebp-1],4` at `0x4d5885`; `shr edx,3` (`0x4d5acb`); `shr eax,4` (`0x4d61f1`) and `test dl,0x10`
(`0x4d6065`); `shr eax,5` (`0x4d6228`). The thirteen references to the flag byte are all in this file; none tests 0x40 or
0x80.

## Display entries and headers

The list is an array of 97 entries of 20 bytes, allocated at startup, with a count and a visible count, and an array of
32 header objects of 8 bytes. An entry is the slot (`+0`, -1 for the Inactive header), the faction id (`+4`), the header
flag (`+8`), the header's position (`+0xc`, set by the sort) and the header key (`+0x10`). A header object is a key and a
child count. The key is a faction id, 0 or -1. Neither array is bounds checked.

Addresses: `0x4d5290`, `0x4d52e5`, `0x4d52ef`, `0x4d5310`, `0x4d5319`, `0x4d5335`, `0x4d5337`, `0x4d533e`, `0xb73254`, `0xb73258`, `0xb73760`, `0xb736b0`, `0xb736c0`
Confidence: verified
Proof: one
Test: none

The sizes are the last of four pushes before `SMemAlloc` (`0x4d5319 push 0x14`, `0x4d533e push 0x8`); the pushes before them
are the line and the file name. The capacity is 0x61 (`0x4d52ef`), the header loop runs 0x20 times (`0x4d534e`).

## List membership

A faction gets a row at rebuild only when it passes three gates. Its `reputationListID` is 0 to 63; at least one of its
four triples matches the active player; the server has set its visible bit. The rebuild walks the `Faction.dbc` rows from
the last to the first. After the walk it adds the synthetic Inactive header and collapses it. The server can add a row
later with the visible message or a standing change, and those two paths do not test the masks. Adding a row also adds
its header.

Addresses: `0x4d5430`, `0x4d5451`, `0x4d5479`, `0x4d54d8`, `0x4d54e0`, `0x4d554a`, `0x4d5555`, `0x4d5579`, `0x4d55d9`, `0x4d55de`, `0x4d5a70`, `0x4d5a93`, `0x4d5b04`, `0x4d5ad8`, `0x4d5bf2`, `0x4d5bfb`, `0x4d5738`, `0x4d5826`, `0x4909d3`, `0x4d5694`
Confidence: verified
Proof: one
Test: none

`0x4d5555` is the only call of the adder in the rebuild, inside the accepting block `0x4d5533`, so a faction no triple
matches is not added whatever its flags say. The rebuild resets the expanded mask to all ones and the counts to 0
(`0x4d5479` to `0x4d5493`) and has two callers: the world-enter code `0x4909d3` and the initialize message (`0x4d5694`). The
adder returns 0 when the slot is already a row (`0x4d5a93`, `0x4d5b04`); on a new row it stores the slot, the faction id
and the header bit from flag bit 3 (`0x4d5acb` to `0x4d5ad8`). With the notify argument it calls the standing log (`0x4d5bfb`).

## Header rows

A row whose header bit is set is its own header: the key is its own faction id. Any other row gets the key of its team,
field 18 of its row, and when no header entry with that faction id exists yet a header is made for the team at the next
position. A header made for a team takes the team's own slot, so the team's own row is the same entry whenever it is added.
Two headers have no `Faction.dbc` row: key 0 (a team of 0) is named by the text `FACTION_OTHER` and key -1 (the rebuild's
Inactive header) by `FACTION_INACTIVE`. Adding a header key expands every header.

Addresses: `0x4d5c20`, `0x4d5c2e`, `0x4d5c34`, `0x4d5b16`, `0x4d5b31`, `0x4d5b6a`, `0x4d5b87`, `0x4d5bdc`, `0x84a0a4`, `0xb736b0`, `0xb736c0`
Confidence: verified
Proof: one
Test: none

`0x4d5c20` stores the key in the next header object, counts it and sets the mask to -1 (`0x4d5c34`), so every header
registration expands all. The synthesized header is written at `ptrArray[count + 1]` (`0x4d5b87`) with the slot of the
team (`0x4d5600`), the header bit 1 and the key; the team search is `0x4d5b6a` to `0x4d5b83`. The two special names are
read in the Lua function (`ui/reputationlua#unknown-index`).

## Sorting and counts

`0x4d5c40` counts the children of each header (entries with the header's key and no header bit; an inactive entry counts
under -1), sorts the header objects, then gives each entry its header's position in the sorted array. The visible count is
the entry count less every child of a collapsed header and less every header with no children. The entries are sorted last
and `UPDATE_FACTION` (372) is signalled. Headers with children come first, ordered by the localized name; a header without
a row comes after one with a row; two without rows order by the larger key first. Rows order by header position, a collapsed
child after everything, the header row before its children, then children by name.

Addresses: `0x4d5c40`, `0x4d5c72`, `0x4d5c8e`, `0x4d5c9c`, `0x4d5cc9`, `0x4d5ce0`, `0x4d5d36`, `0x4d5d68`, `0x4d5d86`, `0x4d5da3`, `0x4d5dab`, `0x4d5dc0`, `0x4d5e34`, `0x4d5e42`, `0x4d5e60`, `0x4d5e70`, `0x4d5ef4`, `0x4d5f02`, `0x4d5f10`, `0x4d5f70`, `0x4d5f81`, `0x73f727`, `0xb73764`
Confidence: verified
Proof: one
Test: `ui::sortcmp_4d5dc0`, `ui::sortcmp_4d5e70`

The entry position is written after the header sort (`0x4d5d36` follows `0x4d5cc9`), so the primary row key is the sorted
position. A collapsed child is a non-header with a position below 32 whose mask bit is clear (`0x4d5d49` to `0x4d5d68`);
the comparator `0x4d5f02` gives such a child +1 against anything that is not. Larger raw keys come first because
`0x4d5e34 setl; lea eax,[eax+eax-1]` returns +1 when a is less. Equal positions between a header and a child test
the header bit (`0x4d5f81`). Sorting reorders and recounts only; the entry count is untouched.

Bit-exact coverage: `ui::sortcmp_4d5dc0` covers `0x4d5dc0` to `0x4d5e07`, `0x4d5e0b` to `0x4d5e6d` of `0x4d5dc0`; `ui::sortcmp_4d5e70` covers `0x4d5e70` to `0x4d5f3d`, `0x4d5f41` to `0x4d5f4f`, `0x4d5f53` to `0x4d5f90` of `0x4d5e70`.
