# ui: Tooltips of world objects

What the client does when the mouse comes to rest on a unit, a game object or a corpse in the world: which
objects can be hovered, how the tooltip is built and seated, what the builders for game objects and corpses
add, and the three script slots a `GameTooltip` offers the interface. The unit builder is in `tooltipunit`;
the hover state itself (the GUID globals, the fade and the notifies) is in `hover`.

The world hover publishes a GUID once, when the hovered object changes, and the publisher builds the content
at once, in C++. No Lua `SetUnit` is needed for the content. Where the tooltip is seated is the interface's
choice for units and corpses, and for game objects the engine's or the interface's by a column of the object's
template.

Roots: `0x492890`, `0x52aa20`, `0x52aef0`, `0x5295d0`

## Which objects are hovered

Before it publishes anything, the per-frame hover pass asks the object it picked, through the object's
slot 21, whether it can be a mouseover. A false answer publishes the cleared GUID, so nothing shows: no
tooltip, no brightening, no cursor change. For a game object the answer comes from its type handler; objects of
the fixed always-true and never-true types, and the template-driven ones, are in `objectlayer/gameobjecttypes`.

Addresses: `0x482962`, `0x482973`, `0x48297e`, `0x482982`, `0x482987`, `0x4829d4`, `0x4829e0`, `0x4829ed`, `0x4829f3`, `0x482090`, `0x5f8620`
Confidence: verified
Proof: one
Test: none

The pass resolves the picked GUID to an object with mask 1 (`0x48296e`), tests it (`0x482973` to `0x48297c`)
and calls `[vtable+0x54]` (`0x482982`). A zero (`0x482987`) goes to `0x4829ed`, which publishes the
all-zero GUID through `0x482090` (`0x4829f3`). `0x482090` is also called by the other branches, at
`0x4829e0`, with the GUID. For a game object slot 21 is `0x5f8620`, which asks the type handler's slot 3.

## The publisher

`0x492890(new GUID; previous GUID)` needs an active player. A previous or published GUID is torn down: the
globals are cleared, the old object's highlight bit cleared and the tooltip faded. The new GUID is stored,
the object highlighted if it has a model, and the tooltip built by the object's kind, from the type mask:
a unit (`0x09`, `0x19`), a game object (`0x21`) or a corpse (`0x81`); every other kind has no tooltip. A unit
runs the default-anchor script, builds, signals `UPDATE_MOUSEOVER_UNIT` and plays a highlight sound by its
reaction: hostile, friendly or neutral. A corpse runs the script, builds and plays the neutral sound, as does
a game object.

Addresses: `0x492890`, `0x4928ab`, `0x4928b4`, `0x4928ba`, `0x4928e8`, `0x492909`, `0x49291b`, `0x492938`, `0x49293d`, `0x49294c`, `0x49295e`, `0x492966`, `0x49296f`, `0x492975`, `0x49297c`, `0x492983`, `0x4929a9`, `0x4929ae`, `0x4929b3`, `0x4929bb`, `0x4929c0`, `0x4929c5`, `0x4929ca`, `0x4929dd`, `0x4929e6`, `0x492a7d`, `0x492aa3`, `0x492aa8`, `0x492ab8`, `0x492ac8`, `0x703e50`, `0x458030`, `0x6061e0`, `0x84330c`, `0x8432f0`, `0x8432d4`
Confidence: verified
Proof: one
Test: none

The kind is `(mask − 9)`, bounded by 0x78 (`0x492969` to `0x49296f`), mapped by the 0x79 bytes at `0x492ac8`
to an arm of the four-entry table at `0x492ab8`: `0x492983` (units), `0x4929f6` (game objects),
`0x492a7d` (corpses) and `0x492ab2` (no builder). The unit arm calls `0x529fe0` (`0x4929a9`), signals event
0x150 with `0x703e50` (`0x4929ae`, `0x4929b3`; event 0x150 is `UPDATE_MOUSEOVER_UNIT` in the name table at
`0xbe1198`) and asks `0x6061e0` for the unit's reaction toward the player (`0x4929bb`, `0x4929c0`): at most 1
plays `GAMEHIGHLIGHTHOSTILEUNIT` (`0x4929ca`), 4 or more `GAMEHIGHLIGHTFRIENDLYUNIT` (`0x4929e6`), and anything
else `GAMEHIGHLIGHTNEUTRALUNIT` (`0x492aa8`). The corpse arm calls `0x52aef0` (`0x492aa3`).

## The game object anchor

A game object is seated by its template. When the type 5 (generic) template's column that the object's slot
`+0x5c` reads is set, the publisher gives the tooltip to `UIParent` with the cursor anchor and offsets 0,
so that it follows the cursor, and then builds. When it is not set, which includes every other type, it runs
the tooltip's default-anchor script, which the interface uses to seat it, and then builds the same content.
No script, no owner: a tooltip without an owner does not show.

Addresses: `0x4929f6`, `0x4929f8`, `0x4929fa`, `0x4929ff`, `0x492a01`, `0x492a0a`, `0x492a1e`, `0x492a2d`, `0x492a32`, `0x492a37`, `0x492a42`, `0x492a48`, `0x492a56`, `0x492a59`, `0x492a68`, `0x5f8630`, `0x52ffe0`, `0x530a80`, `0x530a60`, `0x702690`, `0xb4b44c`, `0xb4b3c4`, `0x8432d4`
Confidence: verified
Proof: one
Test: none

`0x4929fa` calls slot `+0x5c` of the game object's table; only game objects define it, as `0x5f8630`, which
reads the data column named by id 0x13, found for type 5 only (`objectlayer/gameobjects#object-slots`). A
nonzero result (fall-through at `0x492a01`) takes `UIParent`'s `+0x24` as the owner (`0x492a0a`), or none when
`UIParent` is absent (`0x492a0f`), and calls `0x52ffe0(owner, 6, 0, 0)` on the tooltip global (`0x492a11` to
`0x492a1e`), then the builder (`0x492a2d`). A zero result goes to `0x492a42`, which runs the script slot at
`+0x444` with `0x702690` when it is set (`0x492a56`, `0x492a59`), then calls the same builder
(`0x492a68`) and plays the same sound. `0x530a80` refuses to show a tooltip without an owner or a line
and hides it, and `0x530a60` clears the owner on every hide.

## The script slots

A `GameTooltip` accepts three scripts of its own besides the frame's. `OnTooltipSetDefaultAnchor` is run
by the world hover before it builds a unit, corpse or non-floating game object tooltip; its job is to give
the tooltip an owner. `OnTooltipCleared` runs at the end of every clear of the tooltip's lines.
`OnTooltipAddMoney` runs with the sell price in copper, which the item builder computes. Each name selects a
word of the tooltip, and a script is looked up by name first as an ordinary frame script.

| script | slot |
|---|---|
| `OnTooltipSetDefaultAnchor` | `+0x444` |
| `OnTooltipCleared` | `+0x44c` |
| `OnTooltipAddMoney` | `+0x454` |

Addresses: `0x5295d0`, `0x5295db`, `0x5295e2`, `0x5295ef`, `0x5295f8`, `0x529618`, `0x529638`, `0x529644`, `0x64a4c0`, `0x76a0d0`, `0x530263`, `0x52e48c`, `0x854780`, `0x854794`, `0x8547a8`
Confidence: verified
Proof: one
Test: none

`0x5295d0(name)` first asks the frame's own lookup `0x76a0d0` (`0x5295db`), then compares the name with the
three strings in turn (`0x5295ef`, `0x529614`, `0x529634`) and returns the address of the slot
(`0x5295f8`, `0x529618`, `0x529638`), or 0. The clear runs `+0x44c` (`0x530263`) and the item builder's
sell price runs `+0x454` (`0x52e48c`).

## The game object tooltip

`0x52aa20(tooltip; GUID pointer)` clears the tooltip and finds the game object by mask 0x20; none hides the
tooltip. It remembers the GUID at `+0x370`, tints the tooltip's backdrop dark blue (`0xff002e5a`) if it has
one, and adds the object's template name in gold; an object with no template adds nothing. If the object has a
type handler, its slot 33 may add lines (the meeting stone adds its area and levels). Then the lock lines
follow, then the tooltip is shown whatever it holds.

Addresses: `0x52aa20`, `0x52aa40`, `0x52aa5e`, `0x52aa6c`, `0x52aa76`, `0x52aa8c`, `0x52aa92`, `0x52aaa5`, `0x52aaaa`, `0x52aab8`, `0x52aac0`, `0x52aac5`, `0x52aad4`, `0x52acec`, `0x52acf0`, `0x52acfa`, `0x5f8100`, `0x77f410`, `0xc0d3bc`, `0x529010`
Confidence: verified
Proof: one
Test: none

`0x52aa5e` resolves the GUID with mask 0x20 and a miss zeroes `+0xd0` and calls slot 0x84 (`0x52aa6c`,
`0x52aa76`). `0x52aa92` tests the frame's backdrop (`+0x1ac`) and `0x52aaaa` calls `0x77f410` with the color
`0xc0d3bc`, which `0x529010` fills with `0xff002e5a`. `0x5f8100` gives the name, or an empty string without a
template (`0x52aab8`); `0x5303b0` adds it with gold (`0x52aac0`). The handler call is `[handler vtable+0x84]`
with the tooltip (`0x52aad4`). The tail stores 1 at `+0xd0` and calls slot 0x88 (`0x52acf0`, `0x52acfa`).

## The game object lock lines

An object with the locked flag (`GAMEOBJECT_FLAGS` bit 1) adds `LOCKED`, red when the player has no way to
meet the lock, green when the opener is a key item or no skill spell, and the difficulty colour of a known
skill spell. One more line follows from the first slot of the object's lock row, when that slot's action is
valid for the object's state. A key slot gives `LOCKED_WITH_ITEM` with the key item's name, in white. A skill
slot gives `LOCKED_WITH_SPELL_KNOWN` with the lock type's name in the difficulty colour when the player knows
a spell that opens that lock type; else `LOCKED_WITH_SPELL` in red, but only for an object not flagged locked.

Addresses: `0x52aae2`, `0x52aaea`, `0x52aaec`, `0x52ab14`, `0x52ab19`, `0x52ab1d`, `0x52ab24`, `0x52ab31`, `0x52ab3d`, `0x52ab51`, `0x52ab5e`, `0x52ab68`, `0x52ab7e`, `0x52ab8e`, `0x52ab91`, `0x52ab98`, `0x52abc6`, `0x52abd8`, `0x52abe2`, `0x52abe9`, `0x52abf7`, `0x52abfd`, `0x52ac04`, `0x52ac2f`, `0x52ac37`, `0x52ac4a`, `0x52ac50`, `0x52ac69`, `0x52ac80`, `0x52aca6`, `0x52acab`, `0x52acb8`, `0x52acd9`, `0x52ace7`, `0x52ad10`, `0x52ad52`, `0x5f8180`, `0x5f81d0`, `0x5f83d0`, `0x529fa0`, `0xc0dad0`, `0xc0dad4`, `0x838044`, `0x854988`, `0x85499c`, `0x8549b0`, `0x84439c`
Confidence: verified
Proof: one
Test: none

The header needs bit 1 of `fields+0xc` (`0x52aae2` to `0x52aaea`). `0x5f83d0(spell, points, required,
item, slot)` (`0x52ab14`) returns nonzero for an unmet requirement and leaves red `0xc0d3a8` (`0x52ab19`); a
zero spell (`0x52ab1d`) or an item (`0x52ab24`) selects green `0xc0d420` (`0x52ab3d`), else `0x529fa0` with
the required value in `ecx` and the player's in `edx` gives the colour (`0x52ab31`). `LOCKED` is added at
`0x52ab51`. The lock row is `0x5f8180` (`0x52ab68`), its slot 0 action `+0x64` is checked by `0x5f81d0`
(`0x52ab7e`), and the type at `+0x4` (`0x52ab8e`) picks the key arm (`0x52ac80`) or the skill arm. The skill
arm calls `0x5f83d0` again (`0x52abc6`): a spell id shown known (`0x52abd8`, key `0x8549b0`); none and the locked
flag set exits with no line (`0x52abf7`), else red `0xffff0000` is set inline (`0x52abfd`) and key
`0x85499c` used (`0x52ac04`). The lock type name is `[0xc0dad0][lock +0x24]` bounded by `0xc0dad4`,
at `+4` plus the locale, with `UNKNOWN` for none (`0x52ac37`, `0x52ac4a`, `0x52ac50`). The key arm looks the
item up in the item cache with callback `0x52ad10` (`0x52aca6`); a miss adds no line and the callback builds
the tooltip again from the GUID stored at `+0x370` (`0x52ad52`). The name is formatted into `LOCKED_WITH_ITEM`
and added in white (`0x52acd9`, `0x52ace7`).

## The corpse tooltip

A hovered corpse shows `CORPSE_TOOLTIP` with its owner's name, in white. The name comes from the player
name cache; until the cache has it the tooltip has no line and is not shown, and when the name arrives the
cache calls back and the tooltip is built again.

Addresses: `0x52aef0`, `0x52aefd`, `0x52af1b`, `0x52af22`, `0x52af2e`, `0x52af3e`, `0x52af47`, `0x52af63`, `0x52af77`, `0x52af7c`, `0x52af86`, `0x52af9d`, `0x52afa5`, `0x52afb5`, `0x52afc8`, `0x52afe0`, `0x52b022`, `0x55f080`, `0x8549e8`, `0xc0e228`
Confidence: verified
Proof: one
Test: none

`0x52af1b` finds the corpse with mask 0x80; a miss hides the tooltip (`0x52af2e`). The corpse GUID is kept at
`+0x388`/`+0x38c` (`0x52af3e`, `0x52af47`) and its owner GUID is at the corpse's fields `+0` and `+4`. The name
is requested from `0xc0e228` with callback `0x52afe0` (`0x52af63`, `0x52af77`); a miss leaves the function at
`0x52afce` before anything is shown. With the name, `CORPSE_TOOLTIP` is formatted (`0x52af9d`) and added white
(`0x52afb5`) and the tooltip shown (`0x52afc8`). The callback rebuilds from the stored GUID (`0x52b022`).
