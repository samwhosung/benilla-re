# ui: Unit tooltip status lines

The lines of a unit tooltip after the level line: PvP and resurrection, the lore block of a unit that has been
studied, the skinnable line and the civilian, leader and zone lines. The unit builder, its name and level
lines are in `tooltipunit`. These lines only exist for a live unit, except the zone line.

Roots: `0x529fe0`, `0x529a40`, `0x529ec0`

## PvP and resurrection lines

A player flagged for PvP, or any unit with the PvP unit flag (bit 12), adds `PVP_ENABLED` in white. A dead
player the active player may assist adds `RESURRECTABLE` in green. For a player the PvP test is the player
flags bit 8; the unit flag is tested only when that is clear or the unit is not a player.

Addresses: `0x52a840`, `0x52a848`, `0x52a851`, `0x52a85c`, `0x52a864`, `0x52a866`, `0x52a872`, `0x52a878`, `0x52a87a`, `0x52a88b`, `0x52a899`, `0x52a89f`, `0x52a8a4`, `0x52a8ac`, `0x52a8b4`, `0x52a8bc`, `0x52a8c1`, `0x52a8c7`, `0x52a8d6`, `0x52a8df`, `0x6066f0`
Confidence: verified
Proof: one
Test: none

`0x52a84b` to `0x52a851` test the unit's object type for a player (bit 4), `0x52a856` to `0x52a862` the
player flags (`[unit+0xe68]+0x8`, bit 8) and `0x52a866` to `0x52a875` the unit flags (`fields+0xa0`, bit 12);
either adds the line (`0x52a87a`, key `PVP_ENABLED`, white). `RESURRECTABLE` needs health at most 0
(`0x52a8a4`), a player unit (`0x52a8ac`), an active player (`0x52a8b7`) and `0x6066f0(player, unit)`
(`0x52a8bc`), and is green `0xc0d3ac` (`0x52a8c7`).

## The skinnable line

A creature with the skinnable flag (unit flags bit 26) adds `UNIT_SKINNABLE`. It is red, except that when the
active player's interaction spell has a skinning effect, it takes the difficulty colour of the player's
skinning value against the creature's level: the required value is 1 to level 10, then `10 * level − 100`
to level 19, then `5 * level`.

Addresses: `0x52a908`, `0x52a90e`, `0x52a914`, `0x52a917`, `0x52a91b`, `0x52a924`, `0x52a939`, `0x529ec0`, `0x529ecd`, `0x529ed5`, `0x529eed`, `0x529f00`, `0x529f06`, `0x529f21`, `0x529f30`, `0x529f4e`, `0x529f60`, `0x529f63`, `0x529f6f`, `0x529f74`, `0x529f7c`, `0x529f7f`, `0x529f92`, `0xb700e4`, `0x6e3760`, `0x529fa0`
Confidence: verified
Proof: one
Test: none

`0x52a914` shifts the unit flags by 26 and `0x52a924` skips a player. `0x529ec0(tooltip, unit, buffer, size)`
copies the text into the buffer (`0x529ed5`) and starts with the red at `0xc0d3a8` (`0x529eed`). It indexes
`0xb700e4` by the unit's player bit (`0x529f06`), looks the spell up (`0x529f1b` to `0x529f21`) and scans its
three effects for 95 (`0x529f30`). `0x6e3760` returns the effect's points (`0x529f4e`), which must be above 0.
The required value is chosen by the level: at most 10 gives 1 (`0x529f68`), below 20 gives
`(5 * level − 50) * 2` (`0x529f74`, `0x529f78`), else `5 * level` (`0x529f7c`); `0x529fa0` (`0x529f7f`) returns the
colour. The line is added by `0x5303b0` with wrap 0 (`0x529f92`).

## The civilian, leader and zone lines

A PvP-flagged creature adds `PVP_RANK_CIVILIAN` in green when it is a civilian the player is hostile to and
not trivial, and `PVP_RANK_LEADER` in white when it is a racial leader. A party or raid member who is not in
sight and not the active player instead adds the name of the area the party record names, in white. The
party or raid test does not suppress the civilian and leader lines of a unit in sight.

Addresses: `0x52a93e`, `0x52a943`, `0x52a94c`, `0x52a953`, `0x52a95e`, `0x52a962`, `0x52a96f`, `0x52a979`, `0x52a984`, `0x52a997`, `0x52a99b`, `0x52a99d`, `0x52a9a6`, `0x52a9b2`, `0x52a9c0`, `0x52a9cb`, `0x52a9d4`, `0x52a9e0`, `0x52a9ee`, `0x52a9f3`, `0x4918e0`, `0x67e460`, `0x612550`, `0x6125c0`
Confidence: verified
Proof: one
Test: none

`0x4918e0` (`0x52a943`) is the party or raid member test; zero goes to the unit lines (`0x52a997`), and so does
the active player's own GUID (`0x52a953`, `0x52a95a`). A member with no unit (`0x52a95e`) and a party record
(`0x52a962`) is named by `0x67e460(area word at +0x14, buffer, 0x400, 0)` (`0x52a96f`, `0x52a979`) and the line is
added white with wrap 0 (`0x52a988` to `0x52a995`); with no record the build ends. A unit in sight calls
`0x612550` (`0x52a99d`), adds `PVP_RANK_CIVILIAN` with `0xc0d3ac` (`0x52a9a6`, `0x52a9b2`), then `0x6125c0`
(`0x52a9cb`) and `PVP_RANK_LEADER` with `0xc0cf60` (`0x52a9d4`, `0x52a9e0`).

## The studied-creature lines

A unit with the special-info dynamic flag (`fields+0x224`, bit 4), which a creature has after it has been
studied, adds a block: its damage, armor and health, then one line per nonzero resistance, then the
pet diet and the pet abilities. The first block is `DAMAGE_TEMPLATE` of the unit's minimum and maximum
damage (the minimum truncated, the maximum rounded up), or `SINGLE_DAMAGE_TEMPLATE` when they are equal,
`ARMOR_TEMPLATE` with its armor, `HP_TEMPLATE` with its health and `RESISTANCE_TEMPLATE` with the value and the
school's name for each other resistance above 0. All are white.

Addresses: `0x52a8e4`, `0x52a8ea`, `0x52a8f1`, `0x52a903`, `0x529a40`, `0x529a55`, `0x529a5b`, `0x529a60`, `0x529a73`, `0x529a83`, `0x529a8a`, `0x529aab`, `0x529ad3`, `0x529ad8`, `0x529ae6`, `0x529b10`, `0x529b2f`, `0x529b3d`, `0x529b46`, `0x529b65`, `0x529b6a`, `0x529b71`, `0x529b83`, `0x529b8a`, `0x529bc6`, `0x529be9`, `0x529bf2`, `0x53f830`, `0x53f510`, `0x53f59e`, `0x855808`, `0x73fdf5`, `0x40a2b0`
Confidence: verified
Proof: one
Test: none

`0x52a8ea` tests bit 4 of `fields+0x224` and `0x52a903` calls `0x529a40(tooltip, unit, buffer, 0x400)`. The damage
floats are `fields+0x200` and `fields+0x204`: both truncated by `__ftol`, the second after `ceil`
(`0x529a55` to `0x529a7b`); equal values take the single template (`0x529a8a`). The armor is read at
`fields + 4*index + 0x254` with `index` from `0x53f830` (`0x529ad8`, `0x529ae6`), and 0 when not above 0. The
index is the damage-class row whose flag bit 0 is set (`0x53f59e`). The health is `fields+0x40` (`0x529b3d`).
The resistance loop runs over `fields+0x254` to `+0x26c` (`0x529b6c`, `0x529bf2`), skips the index of `0x53f830`
(`0x529b76`), requires a value above 0 (`0x529b83`) and a nonempty `RESISTANCE%d_NAME` (`0x529b8a`,
`0x529baa`), and prints the value then the name (`0x529bc6`).

## The studied-creature diet and abilities

After the first block, a creature whose family has a diet adds `PET_DIET_TEMPLATE` with the names of the
foods in the diet, separated by commas. A creature that is tameable (type flag bit 0 and a family) adds
`TAMEABLE`, then `PET_SPELLS_TEMPLATE` with the names of its pet abilities and their ranks, separated by commas, if
it has any; any other creature adds `NOT_TAMEABLE`.

Addresses: `0x529bfe`, `0x529c01`, `0x529c0e`, `0x529c1a`, `0x529c2e`, `0x529c39`, `0x529c60`, `0x529c79`, `0x529c86`, `0x529caa`, `0x529cbb`, `0x529ce3`, `0x529cf3`, `0x529d05`, `0x529d15`, `0x529d23`, `0x529d2d`, `0x529d44`, `0x529d51`, `0x529d56`, `0x529d5b`, `0x529d61`, `0x529d73`, `0x529d8a`, `0x529d94`, `0x529da5`, `0x529dbc`, `0x529de5`, `0x529e04`, `0x529e1c`, `0x529e2f`, `0x529e59`, `0x529e68`, `0x529e8b`, `0x529e99`, `0x529eaa`, `0x529eb0`, `0x6055e0`, `0xc0de7c`, `0xc0de80`, `0xc0dbe0`, `0xc0dbe4`, `0xc0de40`, `0xc0de44`
Confidence: verified
Proof: one
Test: none

`0x6055e0` gives the family (`0x529c01`); the family store (`0xc0de7c`, last index `0xc0de80`) row's `+0x1c` is the
diet mask (`0x529c2e`). The food store (`0xc0dbe0`, count `0xc0dbe4`, 0x28 bytes a row) is scanned for rows whose
bit `1 << (id − 1)` is in the mask (`0x529c79` to `0x529c89`); the name is at `[row + locale*4 + 4]` (`0x529caa`).
The line is skipped when no food matched (`0x529cd3`). The tameable test is the creature record's `+0x14`
bit 0 (`0x529d23`) and `+0x1c` (`0x529d2d`); failing either, or having no record, adds `NOT_TAMEABLE`
(`0x529e99`, `0x529eaa`). `TAMEABLE` is added at `0x529d44`; the ability row is `[0xc0de40][record + 0x28]`
bounded by `0xc0de44` (`0x529d56` to `0x529d73`); its four spell ids are at `+0x4` and are used when the
matching word at `+0x14` is at least 100 (`0x529d94`, `0x529dbc`). The name is at `[spell + locale*4 + 0x1e0]`
(`0x529de5`) with the rank at `+0x204` in `" (%s)"` (`0x529e04`, `0x529e2f`), and the line is added at `0x529e8b`
only if at least one ability was found (`0x529e59`).
