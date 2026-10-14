# ui: Item requirement, spell and set lines

The lines of an item tooltip that tell the player what the item needs and does: the race, class, level,
skill, spell, rank and reputation requirements, the use, equip and proc spells, the item set and the
cooldown. A requirement is white when the active player meets it and red when not. The builder is in
`tooltipitem`; the lines before these are in `tooltipenchant`, the description and the rest in
`tooltipitemfoot`.

Record fields: allowed classes `+0x30`, allowed races `+0x34`, item level `+0x38`, required level
`+0x3c`, skill `+0x40` and rank `+0x44`, spell `+0x48`, honor rank `+0x4c`, medal `+0x50`, faction `+0x54`
and standing `+0x58`, spells `+0x11c`, triggers `+0x130`, charges `+0x144`, item set `+0x1c0`. All of
these lines show for `nameOnly` too, and the builder ends the group at the cooldown line.

Roots: `0x52b650`

## Race and class lines

A record with at most one allowed race and at most one allowed class shows one line, `RACE_CLASS_ONLY`,
with the race name, a space and the class name. Otherwise the race line and the class line are separate:
`ITEM_RACES_ALLOWED` and `ITEM_CLASSES_ALLOWED` list the names of the allowed races or classes, separated
by commas, and show only when the mask leaves out at least one race or class of the table. A line is white
when the player's race or class is in it and red when not. Races the table marks as hidden (bit 0 of
their flags) never count.

Addresses: `0x52ce14`, `0x52ce1e`, `0x52ce24`, `0x52ce30`, `0x52ce3e`, `0x52ce44`, `0x52ce62`, `0x52ce6a`, `0x52ce72`, `0x52ce89`, `0x52ce8e`, `0x52cea4`, `0x52ceb6`, `0x52cec1`, `0x52ced3`, `0x52cef9`, `0x52cf01`, `0x52cf12`, `0x52cf20`, `0x52cf26`, `0x52cf42`, `0x52cf59`, `0x52cf78`, `0x52cf85`, `0x52cfa0`, `0x52cfc0`, `0x52cff0`, `0x52d03e`, `0x52d04c`, `0x52d0b5`, `0x52d0cb`, `0x52d0ea`, `0x52d12a`, `0x52d16d`, `0x52d1f8`, `0x52d20e`, `0x52d22d`, `0x52d26b`, `0x52d2b0`, `0xc0ded8`, `0xc0dedc`, `0xc0deec`, `0xc0def0`
Confidence: verified
Proof: one
Test: none

The test `mask & (mask − 1) == 0` for both masks (`0x52ce24`, `0x52ce3e`) picks the single-line case. It scans
the race table (`0xc0ded8`, count `0xc0dedc`, 0x74 bytes a row) for the first visible row whose bit
`1 << (id − 1)` is in the race mask (`0x52ce62` to `0x52ce77`), formats `"%s "` with its name at
`[row + locale*4 + 0x44]` (`0x52ce89`, `0x52ce8e`), then scans the class table (`0xc0deec`, count `0xc0def0`,
0x44 bytes a row) the same way and appends the name at `+0x14` (`0x52cef9`, `0x52cf01`, `0x52cf12`). The player's
race and class bytes are at `[player+0x110]+0x78` and `+0x79` (`0x52cea4`, `0x52cf20`). In the other case two
flags record whether every visible race and every class is allowed (`0x52cfa0` to `0x52d045`); each list
joins names with `", "` (`0x84480c`) and is skipped when its flag is set.

## Level and skill lines

A required level above 1 adds `ITEM_MIN_LEVEL`, red when it exceeds the player's level. A required skill
adds `ITEM_MIN_SKILL` with the skill name and rank when the rank is nonzero, else `ITEM_REQ_SKILL` with
the name; it is white when the active player has the skill at the rank or more (value plus bonus), red
otherwise. A record with flags bit `0x40000` shows the skill line only to a player who has the skill
with a value above 0.

Addresses: `0x52d2c9`, `0x52d2cf`, `0x52d2d9`, `0x52d311`, `0x52d31d`, `0x52d324`, `0x52d33d`, `0x52d348`, `0x52d37c`, `0x52d3a2`, `0x52d404`, `0x52d412`, `0x52d426`, `0x52d43f`, `0x52d44a`, `0x52d457`, `0x52d464`, `0x52d467`, `0x52d470`, `0x52d4a4`, `0x52d4b9`, `0x52d4c9`, `0x52d4fd`, `0x5ea3f0`, `0xc0d92c`, `0xc0d930`, `0x838044`
Confidence: verified
Proof: one
Test: none

`0x52d2cf` tests `+0x3c` above 1; the red is `jg` on the player's level at `[player+0x110]+0x70` (`0x52d311`,
`0x52d31d`). The skill key is `ITEM_MIN_SKILL` when `+0x44` is nonzero (`0x52d348`) and the name comes from
the skill store (`0xc0d92c`, last index `0xc0d930`) at `[row + locale*4 + 0xc]`, with `UNKNOWN` for a
missing row (`0x52d3a2`, `0x52d3a8`). `0x5ea3f0` returns the skill's index (`0x52d404`); a negative index
is unmet; the value and bonus words are read from the player's field block at `(index*3 + 0x213)*4` and
`index*0xc + 0x852` (`0x52d43f`, `0x52d44a`), and only when the tooltip's player is the active player
(`0x52d412` to `0x52d429`). The `0x40000` test is `0x52d467`, with the absent-skill and zero-value exits at
`0x52d472` and `0x52d4a4` to `0x52d4b9`.

## The known-recipe line

An item whose first spell teaches a spell shows `ITEM_SPELL_KNOWN`, in red with no condition, when the
player already knows the taught spell or a higher rank of it. For a spell that targets the pet the
pet is asked instead; with no pet nothing shows.

Addresses: `0x52d500`, `0x52d50e`, `0x52d52b`, `0x52d538`, `0x52d545`, `0x52d54f`, `0x52d559`, `0x52d56d`, `0x52d58c`, `0x52d596`, `0x52d5ad`, `0x52d5bb`, `0x52d5c4`, `0x52d5cb`, `0x52d5d5`, `0x60c740`, `0x60c8d0`, `0xc0d788`, `0xc0d78c`
Confidence: verified
Proof: one
Test: none

The first spell (`+0x11c`) is looked up in the spell store (`0xc0d788`, last index `0xc0d78c`). It must
have effect 36, learn spell, in its first effect (`cmp [row+0xf4],0x24`, `0x52d52b`). A spell whose first
effect's implicit target A or B (`+0x148`, `+0x154`) is 5 (`0x52d538` to `0x52d547`) takes the pet branch
(`0x52d56d`): the unit named by the player's pet GUID (`0x52d58c`, mask 8) is asked. Otherwise the player is
asked. Both ask `0x60c740` for the taught spell (the first effect's trigger spell, `+0x1b4`), then `0x60c8d0`
for a higher rank; either true adds the line with `0xc0d390` (`0x52d5c4` to `0x52d5d5`).

## Spell, honor rank and medal lines

Three more requirements reuse `ITEM_REQ_SKILL` with a different name. A required spell (`+0x48`) shows the
spell's name, white when the player knows the spell or a higher rank. A required honor rank (`+0x4c`)
shows the rank's name, white when the player's rank byte is at least that rank. A required medal
(`+0x50`) shows `PVP_MEDAL` followed by the number, looked up as a string, white when the player's medal
bit for that number is set.

Addresses: `0x52d5e6`, `0x52d5f7`, `0x52d603`, `0x52d618`, `0x52d62b`, `0x52d642`, `0x52d654`, `0x52d694`, `0x52d6a4`, `0x52d6ae`, `0x52d6b2`, `0x52d6b7`, `0x52d6ff`, `0x52d70d`, `0x52d71d`, `0x52d726`, `0x52d737`, `0x52d74e`, `0x52d763`, `0x52d79b`, `0x5efe60`, `0x84e32c`
Confidence: verified
Proof: one
Test: none

The spell line reads `[spell row + locale*4 + 0x1e0]` (`0x52d642`) and is white if `0x60c740` or `0x60c8d0`
succeeds (`0x52d618`, `0x52d62b`). The rank test compares the byte at `[player fields+0x102b]` with the
requirement (`0x52d6a4`, `0x52d6ae`); the text is `0x5efe60` of the rank (`0x52d6b7`). The medal test shifts
`1 << (medal − 1)` into the dword at `[player fields+0x1034]` (`0x52d70a` to `0x52d726`), builds the key
with `PVP_MEDAL%d` (`0x84e32c`, `0x52d726`) and resolves it (`0x52d74e`).

## The reputation line

A required faction (`+0x54`) shows `ITEM_REQ_REPUTATION` with the faction name and the standing label, white
when the player's standing with the faction is at least the minimum of the required rank (`+0x58`).

Addresses: `0x52d79b`, `0x52d7a9`, `0x52d7ae`, `0x52d7b8`, `0x52d7c0`, `0x52d7c8`, `0x52d7d7`, `0x52d7fa`, `0x52d800`, `0x52d80f`, `0x52d820`, `0x52d858`, `0x4d6370`, `0x612bf0`, `0x80928c`, `0xc0dd50`, `0xc0dd54`, `0x84b5dc`
Confidence: verified
Proof: one
Test: none

`0x4d6370` returns the standing for the faction in `ecx` (`0x52d7a9`) and is compared with
`0x80928c[rank]` (`0x52d7ae`). The faction row is `[0xc0dd50][faction]`, bounded by `0xc0dd54`
(`0x52d7c0`, `0x52d7c8`), and the name is at `[row + locale*4 + 0x4c]` (`0x52d7fa`), `UNKNOWN` for none
(`0x52d800`). The label key is `FACTION_STANDING_LABEL%d` of `rank + 1` (`0x84b5dc`, `0x52d7d7`), resolved
by `0x612bf0` for the player's gender (`0x52d80f`).

## Spell lines

Each of the five spell slots with a spell in the store adds a line of its description with the tokens
expanded. The slot's trigger prefixes it: 0 `ITEM_SPELL_TRIGGER_ONUSE`, 1 `ITEM_SPELL_TRIGGER_ONEQUIP`,
2 `ITEM_SPELL_TRIGGER_ONPROC`, formatted as `"%s %s"` with the text; any other trigger gives the text
alone. The line is green and wraps, except a spell that teaches a recipe (below), which is white. A spell
with no text adds nothing. When the slot has charges other than −1, a white line of `ITEM_SPELL_CHARGES`
follows, with the absolute count as the plural index and value.

Addresses: `0x52d975`, `0x52d98b`, `0x52d992`, `0x52d9c0`, `0x52d9d1`, `0x52d9da`, `0x52d9f4`, `0x52da01`, `0x52da06`, `0x52da24`, `0x52da2f`, `0x52da37`, `0x52da3d`, `0x52da44`, `0x52da47`, `0x52da66`, `0x52da90`, `0x52daba`, `0x52dae2`, `0x52daea`, `0x52db3a`, `0x52db49`, `0x52db4e`, `0x52db65`, `0x52db8f`, `0x52dd40`, `0x5075f0`, `0x82ef08`
Confidence: verified
Proof: one
Test: none

The loop runs five times over a cursor from `record+0x144` (`0x52d97a`, `0x52d98b`); the spell id is at the
cursor minus 0x28, the trigger at minus 0x14 and the charges at the cursor. `0x5075f0` expands the
description (`0x52da24`). The prefix switch is `0x52da37` to `0x52daba`; the three prefixed arms format
with `"%s %s"` (`0x82ef08`). The colour is the green at `0xc0d3ac` (`0x52db35`), or white for a recipe
(`0x52db2c`). The line is added with wrap 1 (`0x52db3d`, `0x52db49`). The charge count is chosen at
`0x52d9c0` to `0x52da06`: the tooltip's `+0x428` when its fields are valid and it is set or there is no object,
else the object's field at `flds+0x28+4*slot`, else the record's; a slot whose record charge is 0 has none.
The charges line is added at `0x52db8f`.

## Recipe products

A spell line whose spell teaches another spell that creates an item (first effect 36, then first effect
24) is a recipe. After its charges line the tooltip of the created item follows, built by the same
builder as an incremental call (p7 = 1, so it starts with a blank line), then a white line of
`ITEM_REQ_SKILL` with the reagent names: each reagent's item name, with its count in parentheses when
above 1, joined by commas. The reagent line is left out when the list is empty or an item is still
being fetched.

Addresses: `0x52daf5`, `0x52db1c`, `0x52db25`, `0x52dba7`, `0x52dbb1`, `0x52dbce`, `0x52dbee`, `0x52dc03`, `0x52dc27`, `0x52dc32`, `0x52dc4e`, `0x52dc73`, `0x52dc8e`, `0x52dca3`, `0x52dccb`, `0x52dcd1`, `0x52dcd8`, `0x52dcf0`, `0x52dd0e`, `0x52dd27`, `0x52dd35`, `0x52b650`, `0x55ba30`, `0x854b30`
Confidence: verified
Proof: one
Test: none

`0x52daf5` and `0x52db1c` test the two effects (36, then 24). `0x52dba7` loads the item id from the
learned spell's `+0x19c` and `0x52dbce` calls `0x52b650` with p1 the id, p4, p5, p6, p8 and p9 zero, p2
and p3 zero GUIDs and p7 = 1 (`0x52dbb1`). The reagent loop runs eight times over the spell's reagent ids
(`+0xa8`) and counts (`+0xc8`) (`0x52dbee`, `0x52dbf5`); it asks the item cache (`0x52dc27`), marks the pass
unfinished on a miss (`0x52dc32`), formats `"%s (%d)"` (`0x854b30`) for a count above 1 and appends with
`, ` (`0x52dc4e`). The final line is `"\n%s"` of `ITEM_REQ_SKILL` of the list (`0x52dcf0`, `0x52dd0e`),
white, wrap 1 (`0x52dd27`).

## Item set lines

An item of a set shows the set below the spells: a blank line, the header `ITEM_SET_NAME` with the set's
name, the number of members the player has and the number of members, the set's skill requirement if any,
then one line per member in the set, indented by two spaces: cream for a member the player has
equipped, gray for one not. The player is the unit named by p8 and p9 (equipped slots 0 to 18 are
scanned), else the active player's equipped items.

Addresses: `0x52d860`, `0x52d890`, `0x52d898`, `0x52d8a8`, `0x52d8b7`, `0x52d8c8`, `0x52d8dd`, `0x52d8e3`, `0x52d904`, `0x52d915`, `0x52d92c`, `0x52d935`, `0x52d954`, `0x52d96a`, `0x52dd58`, `0x52dd6f`, `0x52dd79`, `0x52ddbb`, `0x52ddd8`, `0x52ddf3`, `0x52de16`, `0x52df28`, `0x52df5a`, `0x52df66`, `0x52df7c`, `0x52df88`, `0x52dfa1`, `0x5f0d60`, `0x622270`, `0x5eaae0`, `0x559fe0`, `0xc0dbc0`, `0xc0dbc4`, `0xc0e2dc`, `0x854b2c`
Confidence: verified
Proof: one
Test: none

The set row is `[0xc0dbc0][record+0x1c0]` with the bound `0xc0dbc4` (`0x52d890`, `0x52d898`). The owner is the
object of GUID (p8, p9) with mask 0x10 (`0x52d8c8`). The 17 member ids are the dwords from set row
`+0x28` (`0x52d8dd`); a nonzero one counts toward the total (`0x52d8ec`) and is owned when the visible-item
record `0x5f0d60(owner, slot)` of any slot below 19 has the entry at `+0x8` (`0x52d904` to `0x52d915`) or, with
no owner, `0x622270(player+0x1d38, id, 1)` finds it (`0x52d92c` to `0x52d93a`). The skill flag is
`0x5eaae0` (`0x52d96a`). The blank line is `" \n"` at `0x854b2c`, gold, wrap 1 (`0x52dd6f`). The header
is `"%s (%d/%d)"` of the name at `[set row + locale*4 + 4]` (`0x52dda2`), gold, wrap 0 (`0x52ddc3`). The
skill line (`+0xac` skill, `+0xb0` rank) repeats the item's skill rule (`0x52ddd8` to `0x52df20`) with
the requirement from the set. Member names come from the item-name cache `0xc0e2dc` (`0x52df5a`), and a
miss skips the member.

## Set bonus lines

After the members comes a second blank line and the bonuses. The set has eight bonus slots, each a spell
and a count of members needed; they are shown in order of that count, equal counts in slot order. A
bonus is `ITEM_SET_BONUS` in green with the text when the set's skill is met and the player has at least
that many members, else `ITEM_SET_BONUS_GRAY` in gray with the count and the text. Both wrap.

Addresses: `0x52dfb9`, `0x52dfd0`, `0x52dfda`, `0x52dfe7`, `0x52dfed`, `0x52e000`, `0x52e008`, `0x52e04e`, `0x52e053`, `0x52e05d`, `0x52e072`, `0x52e093`, `0x52e0a1`, `0x52e0b5`, `0x52e0d4`, `0x52e0e5`, `0x52e5c0`, `0x73f727`, `0xc0d3e4`
Confidence: verified
Proof: one
Test: none

The eight slot numbers are written as bytes 0 to 7 (`0x52dfd0`) and sorted by `qsort(…, 8, 1, 0x52e5c0)`
(`0x52dfed`) with the set row parked at `0xc0d3e4` (`0x52dfe7`). The comparator reads the counts at set row
`+0x8c` and orders by count, then by slot number (`0x52e5dc`, `0x52e5e3`, `0x52e600`). The spell of slot
`i` is at `+0x6c` (`0x52e008`) and its text is expanded by `0x5075f0` (`0x52e04e`). Green needs the skill flag
(`0x52e053`) and `owned >= count` (`0x52e05d`).

## The cooldown line

When the tooltip's fields are valid and the remaining cooldown at `+0x408` is above 0, `ITEM_COOLDOWN_TIME` is
added in white with the time left, and the builder's return value is 1. Nothing else shows the cooldown.

Addresses: `0x52e0fa`, `0x52e0fd`, `0x52e103`, `0x52e107`, `0x52e10f`, `0x52e115`, `0x52e126`, `0x52e13b`, `0x52e140`, `0x52fa50`
Confidence: verified
Proof: one
Test: none

`0x52e0fd` tests `+0x440`, `0x52e107` loads `+0x408` and `0x52e10f jbe` skips a value of 0. The text is
`0x52fa50(ms, ITEM_COOLDOWN_TIME, 0, 1)` (`0x52e126`) and the add is white, wrap 0 (`0x52e13b`).
