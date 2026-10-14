# ui: The unit tooltip

The tooltip of a unit: who it describes, the health bar, the name, the subtitle and owner lines, the level
line and the offline and faction lines. The status lines that follow are in `tooltipunitstatus`. The same
builder serves `SetUnit` and the world hover; where the tooltip is seated is in `tooltipobject`. The line
machinery and the colours are in `tooltipcontent`.

The unit builder describes a GUID. It finds a live unit, and when there is none, the party frame's record of
a member or of a member's pet, so that a party member out of sight still has a tooltip. Everything it adds is
white or gold; the class colour and the guild line seen on a hovered player come from the interface scripts,
not from here.

Roots: `0x529fe0`, `0x529560`, `0x52fd30`

## Who the builder describes

`0x529fe0(this = tooltip; GUID pointer)` clears the tooltip and resolves the GUID three ways: the unit
object (type mask 8), the party member record, and the record of the party member whose pet has the GUID.
With none of the three it hides the tooltip and returns. Otherwise it stores the GUID at `+0x368` and
`+0x36c`, fills the health bar and adds the lines. Every path ends by showing the tooltip and returning 0.

Addresses: `0x529fe0`, `0x52a003`, `0x52a021`, `0x52a02b`, `0x52a036`, `0x52a042`, `0x52a04e`, `0x52a056`, `0x52a058`, `0x52a070`, `0x52a079`, `0x52aa02`, `0x52aa12`, `0x496400`, `0x496420`, `0x4e8820`, `0x4e8860`
Confidence: verified
Proof: one
Test: none

`0x52a021` calls `0x468460` with mask 8 on the GUID; `0x496400` (`0x52a02b`) returns the party member record
(`0x4e8820`, 0x148 bytes each from `0xbc70b0`) and `0x496420` (`0x52a036`) the record at `+0x80` of the member
whose pet GUID matches and whose flags bit 0 is set (`0x4e8860`). When all three are zero (`0x52a042` to
`0x52a049`) the tooltip's `+0xd0` is zeroed and slot 0x84 called (`0x52a050`, `0x52a058`). The GUID is
stored at `0x52a070` and `0x52a079`. The party record's bit 0 of `+0x8` means online; its words at `+0xa`
(health), `+0xc` (maximum health), `+0x12` (level) and `+0x14` (area) are the ones read below; the pet
record's words at `+0x5c` and `+0x5e` are its health and maximum health and its name starts at `+0x8`.

## The health bar

If the tooltip has a status bar (`+0x348`), the builder gives it the range 0 to maximum health and the
value health, from the unit's fields or else from the party records, and watches the unit's health field,
so that a change of health moves the bar. The bar's colour is not set here.

Addresses: `0x52a07f`, `0x52a085`, `0x52a095`, `0x52a0a8`, `0x52a0b7`, `0x52a0c3`, `0x52a0d2`, `0x52a0eb`, `0x52a0f4`, `0x52a106`, `0x52a115`, `0x52a12e`, `0x52a146`, `0x529560`, `0x529586`, `0x52958c`, `0x52959b`, `0x5295b1`, `0x467e70`, `0x783270`
Confidence: verified
Proof: one
Test: none

`0x52a08d` to `0x52a0a8` call `0x467e70(section 3, offset 0x40, GUID, length 4, 0x529560, bar, 0)`. The range
maximum is `fields+0x58` (`0x52a0b7`), the party word at `+0xc` (`0x52a0c3`) or the pet word at `+0x5e`
(`0x52a0d2`); `0x783270` sets the range (`0x52a0eb`). The value is `fields+0x40` (`0x52a0f4`), the party word
at `+0xa` (`0x52a106`) or the pet word at `+0x5c` (`0x52a115`), passed to slot 0x90 (`0x52a12e`); `0x52a146`
shows the bar. The watcher `0x529560` finds the unit by the GUID it is called with, sets the same range and
value (`0x52958c`, `0x52959b`, `0x5295b1`) and returns 1.

## The name line

The first line is the name in gold. For a live unit it is the decorated name: a player with a PvP rank
byte gets the rank title and a medal line, a civilian creature the civilian title, whatever the interface's
rank-title option holds. When the unit's name record has a realm string the title is `"%s - %s"` of the
name and the realm. The title is cut at 63 bytes. Without a unit the line is the name from the name cache,
else the party pet's name.

Addresses: `0x52a165`, `0x52a16a`, `0x52a174`, `0x52a187`, `0x52a18c`, `0x52a198`, `0x52a1ab`, `0x52a1bf`, `0x52a1db`, `0x52a1ea`, `0x52a1f2`, `0x52a202`, `0x52a304`, `0x52a30a`, `0x52a314`, `0x52a323`, `0x52a22c`, `0x609210`, `0x609370`, `0x55f080`, `0xc0e228`, `0x854980`
Confidence: verified
Proof: one
Test: none

`0x52a165` asks the name cache `0xc0e228` for the GUID, with no callback, and keeps the record. `0x609210`
is called for its realm out-parameter (`0x52a187`, result `[ebp-8]`); the decorated name comes from
`0x609370(buffer, 0x400, 1, 0)` at `0x52a1ab` or `0x52a1db`, with the flag the literal 1. With a nonempty realm
`SStrPrintf` joins them with `"%s - %s"` (`0x52a1bf`); otherwise the decorated name is the format itself
(`0x52a1ea`). The title buffer is 0x40 bytes (`0x52a1b6`). `0x52a202` adds it with gold `0xc0d3e8` and wrap 0.
No unit: `0x52a304` adds the name record's text in gold (`0x52a30a`), else the pet record's name at `+0x8`
(`0x52a314` to `0x52a328`).

## The subtitle and owner lines

A creature adds its subtitle in white, and a unit that has an owner adds a white line naming it. The owner
line is the owner's name inside one of four texts: `UNITNAME_TITLE_PET` for a charmed beast,
`UNITNAME_TITLE_MINION` for any other charmed unit, `UNITNAME_TITLE_GUARDIAN` or `UNITNAME_TITLE_CREATION`
for a unit made by a spell whose first effect summons a guardian or an object. A unit with no owner, or
whose owner is not in sight, has no such line.

Addresses: `0x52a207`, `0x52a216`, `0x52a21b`, `0x52a21f`, `0x52a22c`, `0x52a233`, `0x52a247`, `0x52a24c`, `0x52a264`, `0x6094c0`, `0x52fd30`, `0x52fd43`, `0x52fd47`, `0x52fd4d`, `0x52fd50`, `0x52fd64`, `0x52fd6b`, `0x52fd8b`, `0x52fd9d`, `0x52fdc9`, `0x52fdd5`, `0x52fe04`, `0x52fe16`, `0x52fe24`, `0x52fe2e`, `0x854140`, `0x52fe38`, `0x52fe44`
Confidence: verified
Proof: one
Test: none

The subtitle is `0x6094c0` and needs a unit that is not a player (`0x52a207` to `0x52a210`). `0x52fd30(unit, buffer,
size)` reads the charmed-by GUID at `fields+0x10` (`0x52fd4d`) and, when it is zero, the created-by GUID at
`fields+0x20` (`0x52fdc9`). A charmed-by owner picks the key `0x854140[1 + (creature type != 1)]`: pet for
type 1, minion otherwise (`0x52fd6b`, `0x52fd9d`). A created-by owner reads the spell at `fields+0x230` and
its first effect (`0x52fe04`); the effect number minus 42 indexes a byte table (`0x52fe44`) with a
three-way switch (`0x52fe38`): effect 42 gives guardian (`0x52fe24`, key 3), the object-summoning effects 50,
74, 87 to 90 and 104 to 107 give creation (`0x52fe2e`, key 4) and every other falls to the pet and minion
choice. The owner is found by its GUID with mask 8 (`0x52fd8b`) and named by `0x609210`.

## The level text

The level text is `??` for a unit of a level at least ten above the player's that is hostile to the player
(reaction 0 or 1), for a world boss (rank 3) and for a level of 0 or below; else it is the level. A player
who is drunk, that is whose `PLAYER_BYTES_3` byte 1 is nonzero, sees the level of a unit they can attack
lowered by that byte (at most 100) times 0.05, rounded to nearest, but to no less than 1. A party member
out of sight has the level from the party record, and a party pet `??`.

Addresses: `0x52a286`, `0x52a342`, `0x52a347`, `0x52a361`, `0x52a366`, `0x52a373`, `0x52a37c`, `0x52a37e`, `0x52a384`, `0x52a391`, `0x52a39a`, `0x52a3ad`, `0x52a3c6`, `0x52a3d5`, `0x52a3db`, `0x52a3e1`, `0x52a3e9`, `0x52a3f4`, `0x52a3fe`, `0x52a42c`, `0x52a43c`, `0x52a47f`, `0x52a483`, `0x605620`, `0x6061e0`, `0x606980`, `0x8029c8`, `0x85497c`, `0x835154`
Confidence: verified
Proof: one
Test: none

The text buffer is `[ebp-0x48]` (0x20 bytes), `??` copied from the string at `0x85497c` (`0x52a43c`) or `"%d"`
(`0x835154`, `0x52a483`). `0x6061e0` with the unit as the subject (`0x52a342`) must give at most 1 (`0x52a347`)
and the player's level must be at most the unit's minus 10 (`0x52a361`, `0x52a366`); rank 3 (`0x605620`,
`0x52a373`) and a level of 0 or below (`0x52a37e`) also give `??`. The drunk byte is at `[player fields block +
0x1d]` (`0x52a391`, `0x52a39a`), the attack test is `0x606980` (`0x52a3ad`), the byte is cut to 100
(`0x52a3c6`), multiplied by the float 0.05 (`0x8029c8`, `0x52a3d5`), added to 0.5 (`0x52a3db`) and
truncated (`0x52a3e1`); the result is subtracted from the level (`0x52a3e9`) and kept at 1 or more
(`0x52a3f4`). A unit that is absent takes the party record's level word (`0x52a3fe`), or `??` for a pet
record (`0x52a42c`).

## The class text

A dead unit, one with health 0 or the dead dynamic flag (bit 5), has the class text `CORPSE`. A player has the
race and class names joined by `"%s %s"`. A creature has its creature type's name, shown only when the
player's reaction to it is below 4 and the type is not 10 (unspecified). A player whose race or class has no
row leaves the class text unwritten.

Addresses: `0x52a29c`, `0x52a2a1`, `0x52a2ad`, `0x52a2b2`, `0x52a2c1`, `0x52a2c6`, `0x52a2d9`, `0x52a2de`, `0x52a2e5`, `0x52a2ee`, `0x52a2fc`, `0x52a411`, `0x52a44c`, `0x52a462`, `0x52a499`, `0x52a4ab`, `0x52a4c3`, `0x52a4eb`, `0x52a4f1`, `0x52a503`, `0x52a520`, `0x52a532`, `0x52a54d`, `0x52a551`, `0x52a568`, `0x52a572`, `0x605570`, `0x6061e0`, `0x854974`, `0x82ef08`, `0xc0de2c`, `0xc0de30`, `0xc0dee0`, `0xc0dee4`, `0xc0def4`, `0xc0def8`
Confidence: verified
Proof: one
Test: none

The dead flag `[ebp-0xc]` is set from the unit's health and dynamic flags (`0x52a29c` to `0x52a2b2`) or from
the party record's online bit and health word (`0x52a411`). The corpse text is the string at `0x854974`
(`0x52a462`). `0x52a2c1` asks for the creature type; type 10 skips the name (`0x52a2c6`). For other types
`0x6061e0` is called with the player as the subject (`0x52a2d9`) and a result under 4 (`0x52a2de`) reads the
creature-type store (`0xc0de2c`, last index `0xc0de30`; `0x52a2ee`, `0x52a2fc`), whose name is at
`[row + locale*4 + 4]` (`0x52a4c3`). A player's race and class rows come from the id maps `0xc0dee0` and
`0xc0def4` (`0x52a503`, `0x52a532`) by `fields+0x78` and `fields+0x79`, or by the name record's words at
`+0x138` and `+0x140` (`0x52a4f1`, `0x52a520`); the names are at `[row + locale*4 + 0x44]` for the race and
`+0x14` for the class (`0x52a54d`, `0x52a551`), joined at `0x52a568`. A missing row skips the join
(`0x52a53f` to `0x52a545`), leaving the buffer as it was.

## The level line, type and template

The type text is `PLAYER` for a player, and otherwise the word of the creature's rank: elite for ranks 1 and
2, boss for 3, nothing for 0 and 4. The template depends on which of the class and type texts are nonempty.
The line is white. Nothing in the builder marks a unit tapped; a tapped colour is the interface's.

| class text | type text | key |
|---|---|---|
| empty | empty | `TOOLTIP_UNIT_LEVEL` |
| empty | set | `TOOLTIP_UNIT_LEVEL_TYPE` |
| set | empty | `TOOLTIP_UNIT_LEVEL_CLASS` |
| set | set | `TOOLTIP_UNIT_LEVEL_CLASS_TYPE` |

Addresses: `0x52a579`, `0x52a58f`, `0x52a59b`, `0x52a5af`, `0x52a5b3`, `0x52a5ba`, `0x52a5d1`, `0x52a5db`, `0x52a5ef`, `0x52a5f6`, `0x52a613`, `0x52a621`, `0x52a63d`, `0x52a64c`, `0x52a671`, `0x52a681`, `0x52a6ab`, `0x52a6dd`, `0x854158`, `0x85496c`, `0x854670`, `0x854678`
Confidence: verified
Proof: one
Test: none

The type text is `PLAYER` (`0x85496c`) when the unit is a player or the name record exists (`0x52a579` to
`0x52a5ad`); otherwise `0x605620` indexes the word table `0x854158` (`0x52a5ba`), whose entries are empty,
`ELITE`, `ELITE`, `BOSS`, empty. The class text is `[ebp-0x108]` and the type text `[ebp-0x88]`;
`0x52a5f6` to `0x52a66d` choose by their first bytes. The keys are loaded at `0x52a621`, `0x52a64c`,
`0x52a681` and `0x52a6ab`, filled with the texts by `SStrPrintf` and added in white at `0x52a6dd`.

## The offline and faction lines

A party member whose record says offline adds `PLAYER_OFFLINE` in white after the level line, and the build
ends there. Otherwise, for a live unit and an active player, the faction line adds the name of the unit's
reputation faction in white, unless a flag of the matching reputation slot hides it. The line shows only
for a unit that does not have the hide-faction creature flag (type flag `0x10`). The faction is the one the
unit's faction template names; of its four reputation slots, the first whose race mask contains the player's
race and whose class mask contains the player's class decides, and a slot with both masks 0 never matches.

Addresses: `0x52a6e2`, `0x52a6e9`, `0x52a700`, `0x52a708`, `0x52a721`, `0x52a72e`, `0x52a743`, `0x52a750`, `0x52a761`, `0x52a773`, `0x52a77e`, `0x52a789`, `0x52a79b`, `0x52a7a9`, `0x52a7c0`, `0x52a7d0`, `0x52a7e5`, `0x52a7fc`, `0x52a806`, `0x52a819`, `0x52a81f`, `0x52a82c`, `0x52a83b`, `0x612610`, `0xc0dd3c`, `0xc0dd40`, `0xc0dd50`, `0xc0dd54`, `0x847490`
Confidence: verified
Proof: one
Test: none

`0x52a6e9` tests bit 0 of the party record's `+0x8`; clear adds `PLAYER_OFFLINE` and returns (`0x52a700`
to `0x52a72b`). The gate `0x612610` (`0x52a743`) needs the type flags without `0x10` or no creature record.
The template id is `fields+0x74` (`0x52a750`), bounded by `0xc0dd40` and read from the table `0xc0dd3c`
(`0x52a761`, `0x52a773`); its `+0x4` is the faction id, bounded by `0xc0dd54` and read from `0xc0dd50`
(`0x52a789`, `0x52a79b`), whose `+0x4` must be at least 0 (`0x52a7a9`). The four slots have race masks at
`faction + 8 + 4i`, class masks at `+0x18 + 4i` and flags at `+0x38 + 4i`; the race test is
`1 << (race − 1)` (`0x52a7d0` to `0x52a7e5`) and the class test `1 << (class − 1)` (`0x52a7ef` to `0x52a806`). Flag bit 2
(value 4) of the first matching slot suppresses the line (`0x52a81f`); else the name at
`[faction + locale*4 + 0x4c]` is added with `0xc0cf60` and wrap 0 (`0x52a82c` to `0x52a83b`).
