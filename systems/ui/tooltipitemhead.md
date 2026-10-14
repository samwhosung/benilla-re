# ui: Item name, bind, lock and slot lines

The first lines of an item tooltip, from the gray equipped header to the slot and type line: what each
line says, the gate that shows it and its colour. The builder, its arguments and the instance fields are
in `tooltipitem`; the damage, armor and stat lines that follow are in `tooltipitemstats`.

In order the builder emits: the equipped header, the name, the petition lines, `ITEM_SIGNABLE`, then (not
with `nameOnly`) the area, the map, `ITEM_CONJURED`, the bind line, the unique line, `ITEM_STARTS_QUEST`
and the lock lines, then the slot and type line or `CONTAINER_SLOTS`. A line's text is the global string
of the key named here, resolved by `tooltipcontent#localized-lines`; white, red, green and gray are the
colours of `tooltipcontent#the-colours`.

Roots: `0x52b650`, `0x529fa0`

## Name and header

With p5 set the builder first adds `CURRENTLY_EQUIPPED` in gray. The name is built by `0x5d8b00` from
the entry and a random-property row, and is white with `nameOnly`, else the quality colour. With p7 set the
name text is `"\n%s"` of the name, which separates an appended tooltip from what is above it. The quality
indexes the colour table without a limit; no line is recoloured for an item the player cannot use.

The row comes from the item-random-property store: the tooltip's `+0x424` when its fields are valid and it
is nonzero, else the object's `ITEM_FIELD_RANDOM_PROPERTIES_ID`; a wrapped gift has none. A negative id or
an id past the store's last index gives none.

Addresses: `0x52b7b0`, `0x52b7b5`, `0x52b7bf`, `0x52b7c9`, `0x52b7cb`, `0x52b7d9`, `0x52b802`, `0x52b80f`, `0x52b829`, `0x52b830`, `0x52b843`, `0x52b855`, `0x52b86f`, `0x52b874`, `0x52b87b`, `0x52b89a`, `0x52b8b3`, `0x52b8d0`, `0x52b8e1`, `0x5d8b00`, `0x5303b0`, `0xc0dbd4`, `0xc0dbd8`, `0xc0d3c8`, `0xc0d3c4`
Confidence: verified
Proof: one
Test: none

The header is `0x703bf0` of `CURRENTLY_EQUIPPED` with gray `0xc0d3c4` through `0x5303b0` (`0x52b843` to
`0x52b850`). `0x52b7b5` tests the valid flag `+0x440`; `0x52b7bf` reads `+0x424`; the object path
(`0x52b802`) reads the field at `flds+0x98` (`0x52b80f`). The row index is checked against `0xc0dbd8`
(`0x52b7cb`, `0x52b81b`) and the row read from `0xc0dbd4` (`0x52b7d9`, `0x52b829`). The id passed to
`0x5d8b00` is the row's first dword (`0x52b859`). The p7 prefix is built at `0x52b87b` to `0x52b8ab` from
the string `"\n%s"`. The colour is `lea ecx,[eax*4+0xc0d3c8]` with `eax` the record's `+0x1c` (`0x52b8d0`).

## Petition lines

An item the record marks signable (flags bit `0x2000`) with a live object shows its petition. The
builder looks the petition up in the petition cache by the petition id in the object's first instance
field (`flds+0x40`). A miss ends the build there and returns 0 without showing the tooltip. A hit adds
the title, `PETITION_TITLE` or, with bit 0 of the petition's byte at `+0x1110`, `GUILD_CHARTER_TITLE`
(white, wraps); then, when the name cache has the owner, `PETITION_CREATOR` or `GUILD_CHARTER_CREATOR`
with the name (white); then `PETITION_NUM_SIGNATURES` with the count at the object's `flds+0x44` when it
is nonzero (white). After these, `ITEM_SIGNABLE` is added for every signable record, in green.

Addresses: `0x52b8ff`, `0x52b902`, `0x52b918`, `0x52b91a`, `0x52b922`, `0x52b936`, `0x52b93b`, `0x52b944`, `0x52b946`, `0x52b951`, `0x52b95a`, `0x52b9b4`, `0x52b9bf`, `0x52b9cd`, `0x52b9d2`, `0x52b9d9`, `0x52ba1c`, `0x52ba32`, `0x52ba51`, `0x52ba5e`, `0x52ba8b`, `0x52ba9b`, `0x52baa0`, `0x52bab1`, `0x567d60`, `0x55f080`, `0xc0e0fc`, `0xc0e228`
Confidence: verified
Proof: one
Test: none

The test is the template flags bit read from the item's own slot-0 getter (`0x52b8ff`, `0x52b91a`). The
lookup `0x567d60(ecx = 0xc0e0fc; id, p3, 0x52e530, tooltip, 1)` returns the petition record (`0x52b93b`);
a zero ends the function (`0x52b944` to `0x52b94e`). The title lines use `push 1` as the wrap flag
(`0x52b9a4`); the creator line uses `0x55f080` on `0xc0e228` with the record's GUID at `+0x8`
(`0x52b9d2`), skips on a miss (`0x52b9d9`) and has wrap 0 (`0x52ba24`); the count line reads
`[flds+0x44]` (`0x52ba54`). `ITEM_SIGNABLE` is tested on the record's `+0x20` bit `0x2000` (`0x52ba9b`).

## Area, map and conjured lines

Two lines name where the item is restricted. The record's `+0x1c8` indexes the area table, and the
area's localized name (`+0x2c` plus the locale) is added in white; the record's `+0x1cc` indexes the map
table, and the map's localized name (`+0x10` plus the locale) is added in white. An index of 0 or below,
past the table's last index, or with no row adds nothing. `ITEM_CONJURED` follows in white when the
record's flags have bit 2. None of the three shows with `nameOnly`.

Addresses: `0x52babe`, `0x52bac3`, `0x52bac9`, `0x52bacc`, `0x52bad8`, `0x52bae0`, `0x52baf3`, `0x52bb01`, `0x52bb09`, `0x52bb15`, `0x52bb1d`, `0x52bb30`, `0x52bb3e`, `0x52bb46`, `0x52bb51`, `0x52bb7d`, `0xc0e048`, `0xc0e04c`, `0xc0daa8`, `0xc0daac`
Confidence: verified
Proof: one
Test: none

`0x52bac3 jne 0x52bfad` skips the whole region to the slot line for `nameOnly`. The area row is
`[0xc0e048][id]` with the count word `0xc0e04c` (`0x52bad8`, `0x52bae0`) and the name is read at
`[row + locale*4 + 0x2c]` (`0x52baf3`); the map row is `[0xc0daa8][id]` bounded by `0xc0daac`
(`0x52bb15`, `0x52bb1d`) and the name at `[row + locale*4 + 0x10]` (`0x52bb30`). Both lines use white
and wrap 0. The same area table gives the zone name to the character and summon-confirmation scripts.

## The bind line

At most one bind line is added, in white. A bound instance (`0x5da2c0`) shows `ITEM_SOULBOUND`, or
`ITEM_BIND_QUEST` when the record's bonding is 4 or 5. Otherwise the record's bonding picks the key:

| bonding (`+0x194`) | key |
|---|---|
| 0 | none |
| 1 | `ITEM_BIND_ON_PICKUP` |
| 2 | `ITEM_BIND_ON_EQUIP` |
| 3 | `ITEM_BIND_ON_USE` |
| 4, 5 | `ITEM_BIND_QUEST` |

A bonding of 6 or more adds no line unless the instance is bound.

Addresses: `0x52bb82`, `0x52bb8f`, `0x52bb9b`, `0x52bba1`, `0x52bba6`, `0x52bbb0`, `0x52bbce`, `0x52bbed`, `0x52bbf5`, `0x52bbfd`, `0x52bc02`, `0x52bc05`, `0x52bc0b`, `0x52bc12`, `0x52bc19`, `0x52bc37`, `0x52bc55`, `0x52bc76`, `0x52bc7e`, `0x52bc90`, `0x52e4fc`, `0x5da2c0`
Confidence: verified
Proof: one
Test: none

A zeroed buffer (`0x52bb84`) receives `ITEM_SOULBOUND` or `ITEM_BIND_QUEST` for a bound object
(`0x52bb8f` to `0x52bbe5`). A bonding of 0 (`0x52bbf5`) skips to the emit test (`0x52bc76`); a filled buffer
(`0x52bbfd`) is emitted as it is; else `bonding − 1` above 4 skips (`0x52bc05`) and the rest jumps through
the five-entry table at `0x52e4fc` (`0x52bc0b`): `0x52bc12`, `0x52bc37`, `0x52bc55`, then `0x52bc19` twice.
The emit is `0x5303b0` with white and wrap 0 (`0x52bc90`).

## The unique and quest lines

A record whose maximum count (`+0x5c`) is above 0 adds `ITEM_UNIQUE` when it is 1 and
`ITEM_UNIQUE_MULTIPLE` with the count otherwise, in white. A record whose start-quest id (`+0x1a8`) is
nonzero adds `ITEM_STARTS_QUEST` in white.

Addresses: `0x52bc98`, `0x52bc9d`, `0x52bca2`, `0x52bca9`, `0x52bcb8`, `0x52bcc7`, `0x52bcfa`, `0x52bd12`, `0x52bd17`, `0x52bd26`, `0x52bd52`
Confidence: verified
Proof: one
Test: none

`0x52bc9d jle` skips a count of 0 or less; `0x52bca2 cmp eax,1` picks `ITEM_UNIQUE`
(`0x52bca9`) or `ITEM_UNIQUE_MULTIPLE` (`0x52bcc7`) whose text is formatted with the count by `0x64a7f0`
(`0x52bcfa`). Both end in `0x5303b0` with `0xc0cf60` and wrap 0 (`0x52bd12`). `0x52bd17` reads `+0x1a8`.

## The lock lines

A record or instance with a lock shows `LOCKED`, then a line naming what opens it. The lock row is
the lock store's row at the record's `+0x1ac`, else at the tooltip's `+0x430`. The lines show for an
object whose unlocked bit (bit 2 of `flds+0x3c`) is clear, and for a valid block with a lock id. The
`LOCKED` colour is red when the player has no opener, green for a key item or a lock without keys, and
the skill colour for a known spell. The second line is `LOCKED_WITH_ITEM` (white) for a key item and
`LOCKED_WITH_SPELL_KNOWN` with the lock type's name for a skill, in the skill colour.

Addresses: `0x52bd57`, `0x52bd5f`, `0x52bd78`, `0x52bd86`, `0x52bdb3`, `0x52bdc0`, `0x52bdc6`, `0x52bdd2`, `0x52bdde`, `0x52be0c`, `0x52be11`, `0x52be1c`, `0x52be29`, `0x52be35`, `0x52be3d`, `0x52be58`, `0x52be5d`, `0x52be61`, `0x52be68`, `0x52be98`, `0x52beae`, `0x52bec2`, `0x52bed9`, `0x52bee0`, `0x52bef4`, `0x52bf00`, `0x52bf19`, `0x52bf30`, `0x52bf60`, `0x52bf72`, `0x52bf89`, `0x52bfa1`, `0x52bfa6`, `0x5da5e0`, `0x529fa0`, `0xc0dae4`, `0xc0dae8`, `0xc0dad0`, `0xc0dad4`, `0xc0d420`, `0xc0d3a8`
Confidence: verified
Proof: one
Test: none

Every `LOCKED` line sets the builder's return value to 1 (`0x52bfa6`). `0x52bd57` loads the id and the store bounds (`0xc0dae4`, `0xc0dae8`). The show test is at `0x52bdb3`
to `0x52bdd2`. `0x5da5e0(lock row, record +0x38; spell, points, skill, item, slot)` (`0x52be0c`)
returns nonzero for a lock the player cannot open. A zero return with a spell and no item takes the
skill colour from `0x529fa0` (`0x52be29`); a zero return otherwise takes the green at `0xc0d420`
(`0x52be35`); the default is `0xc0d3a8`. The first key slot's type at `lock + 4` picks the branch
(`0x52be5d`): 1 is the item branch (`0x52bf30`), 2 the skill branch (`0x52be6e`), anything else adds
only `LOCKED`. The item branch asks the item cache for the key's entry at `lock + 0x24` and, on a hit,
adds the key's name (`+0x8`). The skill branch calls `0x5da5e0` again (`0x52be98`), reads the lock type
name from `0xc0dad0` at `[row + locale*4 + 4]`, with `UNKNOWN` for a missing row (`0x52bef4`, `0x52bf00`).

## The skill colours

`0x529fa0` maps a skill requirement and the player's value to a colour pointer by difficulty. The
requirement is in `ecx` and the value in `edx`.

| value minus requirement | pointer | colour |
|---|---|---|
| 100 or more | `0xc0cf50` | gray `0xff808080` |
| 50 to 99 | `0xc0d420` | green `0xff40c040` |
| 25 to 49 | `0xc0cf18` | yellow `0xffffff00` |
| 0 to 24 | `0xc0d3a4` | orange `0xffff8040` |
| below 0 | `0xc0d3a8` | red `0xffff2020` |

Addresses: `0x529fa0`, `0x529fa3`, `0x529fad`, `0x529fba`, `0x529fc7`, `0x529fd0`, `0xc0cf50`, `0xc0d420`, `0xc0cf18`, `0xc0d3a4`, `0x529090`, `0x5290d0`, `0x529110`, `0x529150`
Confidence: verified
Proof: one
Test: none

The function adds 100, 50 and 25 to `ecx` in turn (`0x529fa0`, `0x529fad`, `0x529fba`) and compares `edx`
with each, then with `ecx` itself (`0x529fc7`). Each pointer is filled by a one-instruction initializer:
`0x529090` (gray), `0x5290d0` (green), `0x529110` (yellow), `0x529150` (orange).

## The slot and type line

A record with a row in the item-subclass table shows a two-cell line: the slot on the left, the subclass
name on the right. The row is found by a linear scan for the record's class and subclass. The left cell
is the global string of the inventory type's `INVTYPE_` key, except for class 6 (projectile), where it is
the localized name of row 6 of the item-class table. The right cell is the row's localized display name,
omitted for a cloak (type 16) and when the row's display flags have bit 0. An empty cell is dropped; two
empty cells add nothing.

Addresses: `0x52bfad`, `0x52bfc8`, `0x52bfde`, `0x52bfe5`, `0x52bfee`, `0x52c087`, `0x52c08b`, `0x52c091`, `0x52c097`, `0x52c09d`, `0x52c0bc`, `0x52c0c1`, `0x52c0ca`, `0x52c0cf`, `0x52c0dc`, `0x52c0fd`, `0x52c103`, `0x52c10f`, `0x52c128`, `0x52c12d`, `0x52c1a5`, `0x52c1ef`, `0x52c1f6`, `0x52c21b`, `0xc0db90`, `0xc0db94`, `0xc0dc24`, `0xc0dc28`, `0x83ddb0`
Confidence: verified
Proof: one
Test: none

The scan walks `0xc0db90` for `0xc0db94` rows of 0x74 bytes (`0x52bfee`), matching `row[0]` to the record's
class and `row[4]` to its subclass (`0x52bfde`, `0x52bfe5`). The right cell test chain is `0x52c087`,
`0x52c08b` (type 0x10), `0x52c091` (`row+0x14` bit 0), and the name at `[row + locale*4 + 0x28]`
(`0x52c09d`). Class 6 compares `0xc0dc28` (the last index) with 6 (`0x52c0c1`), takes the row pointer
`[0xc0dc24 + 0x18]` (`0x52c0cf`) and the name at `[row + locale*4 + 0xc]` (`0x52c0dc`) without a global
string lookup. Every other class reads `0x83ddb0[inventory type]` (`0x52c103`), looks it up (`0x52c10f`)
and clears the cell when the string is empty (`0x52c12d`). The two-cell add is `0x530270` (`0x52c1ef`); the
single add is `0x5303b0` (`0x52c21b`).

## The slot line colours

The two cells are coloured independently. A proficiency miss reds the right cell, and an exception
reds the left. With no mask for the class nothing is red. A miss is a mask without the subclass's bit,
unless the item is a weapon (class 2) whose subclass row names a proficiency the mask has: that is the
exception. An off-hand weapon (type 22) the player cannot dual wield is also an exception. A lone right
cell takes the miss colour.

Addresses: `0x52c134`, `0x52c13e`, `0x52c143`, `0x52c152`, `0x52c154`, `0x52c15b`, `0x52c160`, `0x52c163`, `0x52c16d`, `0x52c171`, `0x52c173`, `0x52c17c`, `0x52c184`, `0x52c18c`, `0x52c195`, `0x52c19e`, `0x52c1b1`, `0x52c1c2`, `0x52c1ce`, `0x52c202`, `0x5e1e70`, `0x5eab70`
Confidence: verified
Proof: one
Test: none

`0x5e1e70` returns the class's mask (`0x52c13e`); `0x52c145` skips a zero mask; `0x52c152` shifts the
subclass bit and `0x52c156 jne` skips a present one. For class 2 (`0x52c15b`) the row's `+0x8` (`−1`
falls to `+0xc`, `0x52c163`, `0x52c17c`) is shifted into the mask (`0x52c16d`): a set bit sets the
exception at `[ebp-0x14]` (`0x52c173`), a clear bit or both `−1` is a miss in `esi` (`0x52c184`). Type
`0x16` calls `0x5eab70` (`0x52c195`) and a zero return sets the exception (`0x52c19e`). The push order of
`0x530270` puts the exception colour first (left, `0x52c1c2`) and the miss colour as the right colour
(`0x52c1b1`).

## Container slots

A bag, quiver or ammo pouch (inventory type 18) shows one line, `CONTAINER_SLOTS`, in white, in place of
the slot and type line: the record's slot count (`+0x64`, never tested) and the subclass row's
display name. A bag with no subclass row, or a row whose name is missing or empty, adds no line at all,
and neither does the ordinary slot line. The proficiency colours and the display-flag omission do not
apply.

Addresses: `0x52b752`, `0x52b754`, `0x52b75b`, `0x52b76f`, `0x52bff9`, `0x52bffe`, `0x52c004`, `0x52c006`, `0x52c012`, `0x52c016`, `0x52c01e`, `0x52c027`, `0x52c02c`, `0x52c04d`, `0x52c054`, `0x52c06c`, `0x52c082`, `0x52c218`
Confidence: verified
Proof: one
Test: none

`[ebp-0x48]` is `sete dl` of `cmp [rec+0x2c],0x12` (`0x52b754`, `0x52b75b`, `0x52b76f`) and the only
test (`0x52bff9`, `0x52bffe`). A missing row (`0x52c004`) or name (`0x52c016`, `0x52c01e`) jumps to
`0x52c220`, past the slot region. The line is formatted with the slot count and the name
(`0x52c054` to `0x52c06c`) and added at `0x52c218`. Quivers and ammo pouches also have type 18, so they
take this line with their own subclass name.
