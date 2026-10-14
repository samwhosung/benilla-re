# ui: talents

The talent Lua functions (`TalentInfo.cpp`, `0x4f2c00` to `0x4f3830`): which tabs the player has, how a tab's talents are
indexed, what `GetTalentInfo` returns, the prerequisites, the learn request, and the watcher that reports a change in the
character's points. The talent tooltip is `ui/tooltipspell#talent-tooltips`; the wipe confirmation is
`objectlayer/interaction#confirmation-dialogs`.

Roots: `0x4f2c00`, `0x4f3010`, `0x4f3040`, `0x4f3160`, `0x4f3200`, `0x4f34d0`, `0x4f36a0`, `0x4c6fb0`, `0x4c7000`, `0x4c7130`, `0x48dc40`

## Talent tabs

The tab list is built once, on the first call, from `TalentTab.dbc` in file order: a row is kept when both its race mask
(`+0x2c`) and class mask (`+0x30`) accept the player's race and class (0 accepts all, else bit race-1 or class-1 of the
mask). The order field of the row is not read. A tab record is 16 bytes: the row id, the points spent (`+4`), the talent
count (`+8`) and a pointer to the first `Talent.dbc` row of that tab (`+0xc`). `GetNumTalentTabs()` returns the count.
`GetTalentTabInfo(tab)` returns four values: the localized name, the icon (nil when the row has none), the points spent
and the background string; an invalid tab returns `nil, nil, 0, nil`.

Addresses: `0x4f2c00`, `0x4f2c06`, `0x4f2c1b`, `0x4f2c90`, `0x4f2e50`, `0x4f3010`, `0x4f3040`, `0x4f3077`, `0x4f3088`, `0x4f30c5`, `0x4f30f1`, `0x4f3117`, `0x4f3128`, `0x4f3134`, `0xbdcd24`, `0xbdcd28`, `0xbdcd88`
Confidence: verified
Proof: one
Test: none

The built flag is `0xbdcd88`, set before the walk (`0x4f2c1b`). The mask test `0x4f2e50` takes race and class as 1-based bits.
The second pass links the talent runs: it walks `Talent.dbc` once and starts a new run when a row's tab id differs
from the previous row's, so each tab's talents must be contiguous in the file.

## Talent indexing

`GetNumTalents(tab)` returns the talent count of a tab, 0 for an invalid tab. A talent is found by direct index from the
tab's first row, 84 bytes apiece; the client does not sort by tier and column, so the Lua order is the file order of the
tab's block. The row holds: the id (+0), tab id (+4), tier and column from 0 (+8, +0xc), nine rank spell ids (+0x10),
three prerequisite talent ids (+0x34), three prerequisite ranks (+0x40), a flags byte (+0x4c) and a required spell
(+0x50).

Addresses: `0x4f3160`, `0x4f3200`, `0x4f329e`, `0x4f32a7`, `0x4f32aa`, `0xc0d6e8`, `0xc0d6ec`
Confidence: verified
Proof: one
Test: none

`GetTalentInfo` bounds the talent index with `cmp eax,[edi+8]; jae` and computes `imul eax,eax,0x54` from the run base
(`0x4f329e` to `0x4f32aa`). Both indices are 1-based. A non-number raises `Usage: GetTalentInfo(tabIndex, talentIndex)`.

## Talent info

`GetTalentInfo(tab, index)` returns eight values: the name and icon of the highest known rank's spell (the first rank's
when none is known), the tier and column from 1, the current rank, the maximum rank, 1 when flag bit 0 is set else nil,
and 1 or nil for the required spell. The rank scans the rank slots from the last: the first nonzero slot whose spell, or a
higher rank of it, is known gives its position plus 1, else 0. The maximum is the last nonzero slot plus 1. The last value
reads only the required spell, 1 when it is 0 or known, and not the prerequisites.

Addresses: `0x4f3200`, `0x4f3210`, `0x4f32cf`, `0x4f32df`, `0x4f32f5`, `0x4f333c`, `0x4f334a`, `0x4f339e`, `0x4f33e8`, `0x4f3403`, `0x4f340e`, `0x4f341e`, `0x4f3429`, `0x4f3454`, `0x4f3462`, `0x4f3484`, `0x4f34a4`, `0x60c740`, `0x60c8d0`
Confidence: verified
Proof: one
Test: none

The name is at `+0x1e0 + locale*4` of the rank spell's record and the icon id at `+0x1d4`. An invalid tab or index returns `nil, nil, 1, 1, 0, 0, nil, nil`. The flag test is `test [ebx+0x4c],1`
(`0x4f33e8`). The required spell test is `0x4f3403` to `0x4f3429`. The invalid-index tail pushes two nils, two 1.0, two 0, two nils
and returns 8 (`0x4f34a4`).

## Prerequisites

`GetTalentPrereqs(tab, index)` returns three values for each populated prerequisite slot of the talent: the prerequisite's
tier and column from 1, and 1 or nil for whether the player knows the prerequisite at the required rank or higher. The
prerequisite row is found by id. A talent with no prerequisite returns nothing.

Addresses: `0x4f34d0`
Confidence: verified
Proof: one
Test: none

The slot ids are at `+0x34`, `+0x38` and `+0x3c` of the talent row and the ranks at `+0x40`, `+0x44` and `+0x48`; the
learned test scans the prerequisite's rank slots from the required rank up with the known and higher-rank tests.

## Learning

`LearnTalent(tab, index)` sends opcode 0x251 with the talent id (u32) and the requested rank (u32), which is the number of ranks the
player already knows (the 0-based index of the next rank). The only check before sending is that the talent is not at its
maximum rank; points and prerequisites are left to the server. A non-number raises `Usage: LearnTalent(tabIndex,
talentIndex)`.

Addresses: `0x4f36a0`, `0x4f373e`, `0x4f3777`, `0x4f378f`, `0x4f379b`, `0x4f37b9`, `0x4f37c7`, `0x4f37cc`, `0x4f37d1`, `0x4f37e0`, `0x60c740`
Confidence: verified
Proof: one
Test: none

The known-rank scan runs from the last slot down with the exact test `0x60c740` only (no higher-rank test) and leaves `ebx`
at the highest known index or -1; `0x4f378f cmp ebx,[ebp-4]; jge` refuses at the maximum. The packet is the opcode
(`0x4f379b`), the talent id as a u32 (`0x4f37c7`) and the rank `ebx + 1` as a u32 (`0x4f37d1`), both through `0x418190`, sent by
`0x5ab630`.

## Character points

The player's two point counters (talent and profession points, the words at `+0xe48` and `+0xe4c` of the player block) are
watched: `0x4c6fb0` registers `0x4c7000` for them, and `0x4c7130` removes it. When they change, the callback signals
`CHARACTER_POINTS_CHANGED` (264) with two numbers, the change of the first counter and of the second. The ammunition word is
watched alongside (`ui/inventory#alert-triggers`).

Addresses: `0x4c6fb0`, `0x4c6fce`, `0x4c6fd8`, `0x4c7000`, `0x4c7086`, `0x4c7098`, `0x4c7130`, `0x4c714a`, `0x4c7154`, `0x703f50`, `0x467e70`, `0x467fb0`
Confidence: verified
Proof: one
Test: none

The registration is for offset `0xe48`, length 8, kind 4 (the player block). The callback subtracts the old words from the
current ones, taking 0 for a player that is not the active one; the arguments are pushed so the first Lua value is the first
counter's change (`0x4c7086` to `0x4c7098`). `ConfirmTalentWipe()` finds the active player and runs the wipe dialog with a zero
GUID, which uses the remembered trainer (`0x48dc40`).
