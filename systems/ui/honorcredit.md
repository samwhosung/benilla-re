# ui: the honor credit message and rank badges

The server message that reports an honor kill or an honor award, what the client prints and floats for it, and the
rank badge textures the kill text carries. The log line and the floating text are also in `objectlayer/unitcombat#honor`;
the rank functions are `ui/honor`.

Roots: `0x6c8470`, `0x6c7f10`

## Honor credit message

`SMSG_PVP_CREDIT` (0x28c) is a u32 honor, a u64 victim GUID and a u32 victim rank. The handler reads them in that order
and does nothing without an active player; otherwise it calls the honor logger with the player, the GUID, the rank and
the honor. A victim GUID of zero is an award with no victim: the line `COMBATLOG_HONORAWARD` with the honor, the floating
combat text, and no kill text. A nonzero GUID whose object is not in the object manager prints nothing at all, no line,
no combat text and no kill text.

Addresses: `0x48f6c9`, `0x48f6d2`, `0x48f6dd`, `0x48f6e8`, `0x48f703`, `0x48f708`, `0x48f722`, `0x48f485`, `0x4912e5`, `0x48f690`, `0x625270`, `0x625288`, `0x62529f`, `0x6252ab`, `0x6253bb`, `0x6253ee`
Confidence: verified
Proof: one
Test: none

The handler `0x48f690` is registered for 0x28c (`0x48f485`) and cleared at `0x4912e5`; opcode 0x28c indexes its jump
table to the arm `0x48f6c9`. The reads are `GetUInt32` (`0x48f6d2`), `GetGuid` (`0x48f6dd`), `GetUInt32` (`0x48f6e8`). The
logger tests the two GUID halves (`0x625288`), looks the victim up under type mask 8 (`0x62529f`) and leaves at `0x6253ee`
when it is missing (`0x6252ab`); every path to `0x6253ee` skips the combat text.

## Credit rank name

A victim with honor above 0 (signed) gets the line `COMBATLOG_HONORGAIN`: the victim's name, a rank title and the honor.
The title is the text `PVP_RANK_<rank>_<side>`, with the side from the victim's faction template: 0 when the group mask
has bit 4, 1 when it has bit 2, otherwise the whole credit is dropped. A missing text also drops it. The rank is not
range checked, so rank 19 gives its title. The title is resolved for the active player's gender, not the victim's. A
victim with honor 0 or less gets `COMBATLOG_DISHONORGAIN`, the rank 0 and the text of `PVP_RANK_CIVILIAN` as the title.

Addresses: `0x6252b1`, `0x6252bb`, `0x6252c1`, `0x6252e4`, `0x6252ef`, `0x625301`, `0x62530c`, `0x62530f`, `0x625321`, `0x625327`, `0x625337`, `0x625341`, `0x625369`, `0x625370`, `0x625376`, `0x625387`, `0x625390`, `0x62539d`, `0x6253ad`, `0x612bf0`, `0x6072e0`, `0x626850`, `0x626960`
Confidence: verified
Proof: one
Test: none

The side reads the faction template field at `+0x74` of the victim's unit block, bounds it against the template table
(`0x6252e7`, `0x6252ef`) and tests the group mask word (`0x62530c`) for 4 (`0x62530f`) and 2 (`0x625317`);
`0x625321 js` leaves on -1. The title key is formatted into a 0x40 buffer (`0x625337`) and resolved by `0x612bf0` with
`this` still the active player (`0x625374`, `0x625376`), the victim's pointer being in another register. The line is
added by `0x626850(0x51, fmt, name, title, honor)` (`0x625390`). The floating text call is `0x626960(0x2e, honor)`
(`0x62539d`); the kill text `0x6072e0(honor, rank, title)` (`0x6253ad`) puts `HK` or `DK`, a colon and the title above
the player. The honor award arm does not call it.

## Rank badges

The client loads 15 badge textures, `PvPRank01` to `PvPRank15`, when the world text system starts, and each kill text
carries a badge for the victim's rank. The kill text passes its rank as the extra argument of a world text record of
category 5; no other category stores a rank. A record shows the badge of index rank minus 5 for ranks 5 to 19, and none
below 5 or above 19. A dishonorable credit has rank 0 and no badge.

Addresses: `0x6c8470`, `0x6c84d8`, `0x6c84f2`, `0x6c8505`, `0x6c8511`, `0x6c7f10`, `0x6c7f19`, `0x6c7f1c`, `0x6c7f25`, `0x6c7f28`, `0x6c8017`, `0x6c8570`, `0x6c862d`, `0x6c8639`, `0x6c863b`, `0x6c7840`, `0x6c86d4`, `0x86c6f8`, `0xce88dc`
Confidence: verified
Proof: one
Test: none

The path array holds 15 pointers, rank 1 first (`0x6c84f2`), and the loop steps 0x3c bytes (`0x6c8511`) storing handles at
`0xce88dc`. The record is built by `0x6c8570`, which stores `*extra` at `+0x7c` only when the category is 5
(`0x6c862d`, `0x6c863b`). `0x6c7f10` runs from the record build by a jump (`0x6c86d4`), compares the rank with 5 and
`rank - 5` with 15 (`0x6c7f1c`, `0x6c7f28`) and fetches the handle at index `rank - 5` (`0x6c8017`). The other three callers of `0x6c7840` pass a null
extra. The 15th texture is the leader rank's: the rank functions stop at 18 (`ui/honor#pvp-rank-info`).
