# ui: standalone bindings

Lua functions of `WoW\Source\Ui` that stand on their own: the interface reload, the unit combat flag and the default
language. The argument and error conventions they share are in `ui/luastate`; the gate on protected actions is
`ui/protected`.

Roots: `0x4884d0`, `0x517e10`, `0x49fcd0`

## Reloading the interface

`ReloadUI()` is `0x4884d0` and returns nothing. It calls `0x491380`, which asks the protected-action gate with tag
10 (`0x494a50`) and, only when that allows, sets the byte `0xb4b3f4`. The gate allows a client-context call, and a
script-context call only while an input event runs (an edit box enter script, a mouse handler or a key binding), so a
call from a macro or a timer is ignored without a message. Each frame the per-frame callback `0x495590` tests the byte
at its end (`0x49565b`): if it is set, it tears the interface down (`0x490bd0`) and then builds it again
(`0x48fbf0`), where `0x490124` clears the byte. The same trigger runs when the screen aspect changes.

Addresses: `0x4884d0`, `0x491380`, `0x494a50`, `0x495590`, `0x49565b`, `0x495664`, `0x495669`, `0x490bd0`, `0x48fbf0`, `0x490124`, `0x49138e`, `0xb4b3f4`
Confidence: verified
Proof: one
Test: none

The byte `0xb4b3f4` has three references in the image: the clear `0x490124`, the set `0x49138e` and the test `0x49565b`.
`0x491380` is `mov ecx,0xa; call 0x494a50; test eax,eax; je` before the store. `0x494a50` maps tag 10 through
`0x494b54` to arm 1, `0x494a92`, which returns 0 only when none of the three input flags is set while the context
word is nonzero (`ui/protected#protected-action-gate`). The teardown and rebuild are `ui/addons#saved-variables-written`
and `ui/framexml`; the rebuild opens a new Lua state at `0x48fe97`. The screen aspect path is `ui/uiscale`.

## Unit combat flag

`UnitAffectingCombat(unit)` is `0x517e10`. A first argument that is neither string nor number raises
`Usage: UnitAffectingCombat("unit")`. The unit token resolves through `0x515940`; an unresolved token returns nil,
exactly as a unit that is not in combat does. Otherwise it returns the number 1 when bit 19 (0x80000) of
`UNIT_FIELD_FLAGS` is set and nil when it is clear. The player's own flags are read the same way as any other unit's,
by the same token resolution.

Addresses: `0x517e10`, `0x517e18`, `0x517e21`, `0x517e41`, `0x517e4a`, `0x517e56`, `0x517e5e`, `0x517e73`, `0x515940`
Confidence: verified
Proof: one
Test: none

`0x517e10` checks `lua_isstring` (`0x6f3510`), converts with `0x6f3690` and resolves with `0x515940`. `0x517e4a` loads the
unit's field block `[unit+0x110]`; `+0xa0` is `UNIT_FIELD_FLAGS` (field 46, the block starting at field 6). `0x517e56 shr ecx,0x13`
and `test cl,1` select the bit, `0x517e5e` pushes 1.0 and `0x517e73` the nil.

## Default language

`GetDefaultLanguage()` is `0x49fcd0`, takes no arguments and returns one string, the active player's race language
name in the client's locale. With no player, a negative or past-the-end language id, or no language record it returns
nothing at all (zero values). The registered name of the language count function is the misspelling
`GetNumLaguages`; there is no correctly spelled one.

Addresses: `0x49fcd0`, `0x49fcf4`, `0x49fcfd`, `0x49fd05`, `0x49fd12`, `0x49fd1e`, `0x49fd2a`, `0x5ec890`, `0x843628`, `0x49fb30`
Confidence: verified
Proof: one
Test: none

`0x5ec890` returns field 0x20 of the player's race record, the language id; `0x49fcfd` bounds it, inclusively (`jg`), against the highest id `0xc0db4c`
(starts at -1, `0x53ba01`) that follows the language table `0xc0db48`, and `0x49fd12` picks the locale column `[0xc0e080]` at `+4 + 4n` of the record
(`Languages.dbc`). `0x6f3890` pushes the string, and a null name pointer pushes nil. The pair `0x843628`
holds the name `GetNumLaguages` with the function `0x49fb30`; the neighbouring pairs, `SendChatMessage` before it and
`GetLanguageByIndex` after, show the order name then function.
