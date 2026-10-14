# ui: the reputation Lua functions

The twelve reputation functions registered by `0x4d6c50` (`0x4d64c0` to `0x4d6c6d`): what `GetFactionInfo` returns, the
watched faction, collapse and selection, and the rest. The list they read is `ui/reputation`; what the toggles send is
`ui/reputationwire`. Indexes are 1-based positions in the visible list.

Roots: `0x4d6c50`, `0x4d64f0`, `0x4d6820`, `0x4d6a50`, `0x4d6aa0`

## Faction info values

`GetFactionInfo(index)` raises `Usage: GetFactionInfo(index)` for a non-number. The entry at index-1 gives a faction id,
and the function returns eleven values: the localized name, the localized description, the standing id (rank + 1, 1
to 8), the bar minimum (the table entry of the rank), the bar maximum (the next entry) and the standing itself, all absolute;
then at war (the number 1 or nil), can toggle war (1 when the standing is at least -3000 and the peace-forced bit is clear),
is header (1 when a header object has the faction's id), is collapsed (1 for such a header that is collapsed) and is
watched. False is always nil.

Addresses: `0x4d64f0`, `0x4d6509`, `0x4d6534`, `0x4d6565`, `0x4d6576`, `0x4d6583`, `0x4d65a0`, `0x4d65b1`, `0x4d65ac`, `0x4d65ea`, `0x4d6608`, `0x4d6613`, `0x4d6633`, `0x4d6687`, `0x4d6699`, `0x4d66a7`, `0x4d665e`, `0x4d667d`, `0x4d5fa0`, `0x4d61b0`, `0x4d61e0`, `0x4d62f0`, `0x4d6370`, `0x4d63a0`
Confidence: verified
Proof: one
Test: none

The entry lookup `0x4d5fa0` answers 0 past the count but accepts an index equal to the count (`cmp ecx,[0xb73760]; jbe`,
`0x4d5fa0`), reading the pool entry after the last. The bounds are pushed as `T[rank]` then `T[rank + 1]` then the standing
(`0x4d65b1`, `0x4d65a0`, `0x4d65ac`). The toggle test is `cmp [ebp-4],0xfffff448` (`0x4d6608`) and `0x4d61e0`. The header test
scans the header objects for the faction id (`0x4d6640` to `0x4d6649`), so it is a header-array lookup rather than the entry's
header bit. A collapsed header is a position below 32 with a clear mask bit (`0x4d6699` to `0x4d66b4`). Every path returns
11 (`0x4d667d`, `0x4d6743`, `0x4d680e`). The toggle function `0x4d5fd0` also refuses in combat, which this value does not know.

## Unknown index

When the faction id has no `Faction.dbc` row, the function scans the header objects for it. A match with a key of -1
or more and a position below 32 gives the text `FACTION_OTHER` for key 0 and `FACTION_INACTIVE` otherwise as the name, an
empty description, standing id 1, three zeros for the bar, nil at war and nil can toggle, is header 1, is collapsed by the
mask and nil watched. Without a match the eleven values are nil, nil, 1, 0, 0, 0 and five nils.

Addresses: `0x4d66c6`, `0x4d66d8`, `0x4d674d`, `0x4d675b`, `0x4d6760`, `0x4d6767`, `0x4d676c`, `0x4d676e`, `0x4d6773`, `0x4d67d8`, `0x4d67f3`, `0x4d6743`, `0x703bf0`
Confidence: verified
Proof: one
Test: none

The two names are chosen at `0x4d676c je` (key 0) and `0x4d676e` (the other key), then resolved by `0x703bf0`. The
description is the empty text (`0x4d6781`). An index past the entry count asks `0x4d5fa0` for faction 0, which matches the
key-0 header when the list has one, so such an index answers like the Other header.

## Watched faction

`GetWatchedFactionInfo()` returns five values: the name, the standing id, the bar minimum, the bar maximum and the standing,
from the faction whose slot the player block holds at `+0x10c4`; a slot outside 0 to 63 stands for faction 0. An absent
player or a faction without a row gives nil and four zeros. `SetWatchedFactionIndex(index)` sends the slot of the entry's
faction, only when it differs from the player block's, and writes nothing locally. An index past the list asks for
faction 0, whose slot is -1 when no row has the id 0.

Addresses: `0x4d6820`, `0x4d684d`, `0x4d6853`, `0x4d6859`, `0x4d68a0`, `0x4d690a`, `0x4d6b60`, `0x4d6b97`, `0x4d6b9e`, `0x4d6240`, `0x4d6276`, `0x4d627e`, `0x4d62f0`, `0x4d6317`
Confidence: verified
Proof: one
Test: none

The watched slot is `[[player + 0xe68] + 0x10c4]` (`0x4d684d`, `0x4d6853`) and `0x4d5620` turns an index outside 0 to 63
into faction 0. The standing id is `rank + 1` (`0x4d68a0`). `GetWatchedFactionInfo` has no argument check and returns 5 on every
path (`0x4d6900`, `0x4d693e`). The sender `0x4d6240` compares the slot with the field (`0x4d6276`) and sends opcode 0x318 with
the slot; an absent player sends nothing.

## Collapse and selection

`CollapseFactionHeader(index)` and `ExpandFactionHeader(index)` collapse or expand the header of an entry. For an index
past the entry count, or an entry that is not a header, they set every header collapsed or expanded. Nothing is sent. The
mask is a process cell: it is reset to all expanded by every rebuild and every header registration, is not saved, and is
written nowhere else. `SetSelectedFaction(index)` stores the entry's faction id. `GetSelectedFaction()` returns the position
of that faction among the visible entries, or 0 when it is not visible.

Addresses: `0x4d6a50`, `0x4d6a8c`, `0x4d6aa0`, `0x4d6400`, `0x4d6406`, `0x4d6413`, `0x4d6476`, `0x4d647e`, `0x4d6452`, `0x4d646b`, `0x4d5479`, `0x4d5c34`, `0x4d6bb0`, `0x4d6bec`, `0x4d6c00`, `0x4d6c13`, `0x4d6c2d`, `0x4d6c30`, `0x4d537b`, `0xb73768`
Confidence: verified
Proof: one
Test: none

`0x4d6400(entry, collapse)` tail-jumps to the sorter after every change. An entry index at or above the count
(`0x4d6406 jae`) or a non-header entry (`0x4d6413 je`) reaches `0x4d6476`, which writes 0 for collapse and -1 for expand. The
mask has five writers: `0x4d5479`, `0x4d5c34`, `0x4d6452`, `0x4d646b`, `0x4d647e`. The selection cell is zeroed only by the
teardown (`0x4d537b`), so a rebuild keeps it, and it holds a faction id, so a re-sort keeps it. The getter scans only the
visible count (`0x4d6c07`) and `or eax,-1` then `inc` give 0 for none (`0x4d6c2d`, `0x4d6c30`).

## Other functions

`GetNumFactions()` returns the visible entry count. `IsFactionInactive(index)` returns 1 when the entry's slot has the
inactive bit, else nil. `FactionToggleAtWar(index)` asks for war when the faction is not at war and peace when it is, for
the entry's faction, ignoring faction 0. `SetFactionInactive(index)` and `SetFactionActive(index)` set or clear the inactive
bit through the same request path. Each takes a number or raises its usage text, such as `Usage: IsFactionInactive(index)`.

Addresses: `0x4d6c50`, `0x4d64c0`, `0x4d64c6`, `0x4d6950`, `0x4d69b0`, `0x4d6a00`, `0x4d6af0`, `0x4d6987`, `0x4d6994`, `0x4d69a3`, `0x4d69f4`, `0x4d6a44`, `0x4d6b27`, `0x84a0a8`
Confidence: verified
Proof: one
Test: none

`GetNumFactions` converts the visible count (`0x4d64c6`) and cannot raise. `FactionToggleAtWar` passes `!IsAtWar` to
`0x4d5fd0` (`0x4d6994` to `0x4d69a3`), `SetFactionInactive` passes 1 to `0x4d60f0` (`0x4d69f4`) and `SetFactionActive` 0
(`0x4d6a44`). The registrar loops the twelve pairs at `0x84a0a8` (`0x4d6c67 cmp esi,0x60`) through `0x704120`.
