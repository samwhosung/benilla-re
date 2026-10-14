# objectlayer: reaction

How one unit reacts to another: the rank on the reputation scale, 0 hated to 7 exalted with 3
neutral and 4 friendly, that the interface, the cursor, the nameplates and the attack and assist
tests all start from. The rank is always the first object's reaction toward the second, and the
two directions can differ.

The answer comes from a ladder: the same object, then rules between two player-controlled units,
then the active player's reputation, then the faction templates. A unit's faction template is
field 35, `UNIT_FIELD_FACTIONTEMPLATE`; its row in `FactionTemplate.dbc` comes from the table at
`0xc0dd3c`, bounded by `0xc0dd40`. A row holds the faction at `+0x4`, flags at `+0x8`, its group
mask at `+0xc`, the friend and enemy group masks at `+0x10` and `+0x14`, four enemy factions from
`+0x18` and four friend factions from `+0x28`. In every unit function below, `[object+0x110]`
points at field 6, the start of the unit section.

Roots: `0x6061e0`, `0x606530`, `0x6064c0`, `0x5f7fd0`, `0x606980`, `0x6066f0`, `0x6067d0`, `0x6067f0`, `0x606880`, `0x606ba0`, `0x606060`

## Unit toward unit

`0x6061e0(this = A, B)` returns A's reaction toward B. A unit toward itself is 4. When both carry
bit 3 of `UNIT_FIELD_FLAGS` (field 46), the player-controlled bit, each is mapped to the player
controlling it; a missing player gives 3. Two players in one duel give 4 on the same team and 1
on opposing teams. The active player and a party member, either way round, give 4. Two players
both flagged free-for-all, bit 7 of `PLAYER_FLAGS`, give 1. Otherwise the faction rules below
decide.

Addresses: `0x6061e0`, `0x606205`, `0x606217`, `0x60622f`, `0x60625d`, `0x60626b`, `0x606296`, `0x6062b0`, `0x4918d0`, `0x4e7f70`, `0x6062dd`, `0x606313`, `0x60631c`, `0x60632c`, `0x60634f`, `0x60635f`
Confidence: verified
Proof: one
Test: `objectlayer::unit_reaction_league_of_arathor_emissary`, `objectlayer::unit_reaction_tracks_the_reputation_ladder`, `objectlayer::unit_reaction_reputation_branch_is_gated`, `objectlayer::unit_reaction_player_to_npc_is_atwar_not_rank`, `objectlayer::unit_reaction_persuaded_bump_and_clamp`, `objectlayer::unit_reaction_self_is_friendly`

`0x606200` compares the two pointers and returns 4. `0x606217` and `0x60622f` test bit 3 of
`[[A+0x110]+0xa0]` and of B's. `0x606170` gives each controlling player; a zero one jumps to
`0x60635f`, 3. The duel rung reads the player block `[P+0xe68]`: both teams `+0x20` nonzero and
the arbiter GUIDs `+0x0`/`+0x4` equal (`0x60626b` to `0x606294`) return 4 for equal teams and 1
for different ones (`0x606296`: `setne`, `dec`, `and 3`, `inc`). When one player is the active player, the party rung asks
`0x4918d0` about the other (`0x6062dd`, `0x606313`) and returns 4 on a member (`0x60631c`);
`0x4918d0` tail-jumps to `0x4e7f70`, which returns 1 for a nonzero GUID equal to the active
player's or to one of four 8-byte party slots, else 0. The free-for-all rung tests bit 7 of `[[P+0xe68]+0x8]` on both and returns 1
(`0x60634f`).

## The active player's reputation

When A is controlled by the active player, B's faction decides by the player's reputation: a
forced reaction for that faction is returned as it is, uncapped; else a faction with a
reputation slot is 1 when the player is at war with it and 4 when not, whatever the standing.
Only when B's faction has no slot, or A is not the player's, do the templates decide.

Addresses: `0x606372`, `0x60638c`, `0x6063a4`, `0x6063b2`, `0x6063e0`, `0x6063e9`, `0x606400`, `0x60640c`, `0x606411`
Confidence: verified
Proof: one
Test: `objectlayer::unit_reaction_league_of_arathor_emissary`, `objectlayer::unit_reaction_tracks_the_reputation_ladder`, `objectlayer::unit_reaction_reputation_branch_is_gated`, `objectlayer::unit_reaction_player_to_npc_is_atwar_not_rank`, `objectlayer::unit_reaction_persuaded_bump_and_clamp`, `objectlayer::unit_reaction_self_is_friendly`

`0x606372` needs bit 3 on A and `0x606170(A)` equal to `0x468550` (`0x6063a4`). B's template id
`[[B+0x110]+0x74]` indexes `0xc0dd3c` (`0x6063b2` to `0x6063d3`); `0x4d6490(faction)` at
`0x6063e0` returns its rank directly (`0x6063e9`). `0x605fc0` (`0x606400`) then `0x4d61b0`
(`0x60640c`): `neg al`, `sbb`, `and -3`, `add 4` gives 1 at war and 4 otherwise (`0x606411`).

## Faction toward a unit

`0x606530(template, B)` is a faction template's reaction toward unit B. A missing row on either
side is 3. When B is controlled by the active player, three rules come first: a contested guard
template (flag `0x1000`) is hostile, 1, to a player flagged contested, bit 8 of `PLAYER_FLAGS`; a
forced reaction for the template's faction is returned; a faction with a reputation slot returns
the player's standing rank. Otherwise the templates are compared. `0x6061e0` calls it with A's
template and caps the result at 6; when A is persuaded by B (field 18 holds B's GUID) or B has bit
14 of its unit flags, a rank of 2 to 4 rises by one, for a reputation faction only.

Addresses: `0x606530`, `0x4d63a0`, `0x4d6490`, `0x4d61b0`, `0x606576`, `0x606588`, `0x6065b0`, `0x6065be`, `0x6065d2`, `0x6065e6`, `0x6065fe`, `0x606609`, `0x606615`, `0x606627`, `0x606426`, `0x606432`, `0x606439`, `0x606449`, `0x606464`, `0x606497`, `0x6064a0`, `0x6064aa`
Confidence: verified
Proof: one
Test: `objectlayer::unit_reaction_league_of_arathor_emissary`, `objectlayer::unit_reaction_tracks_the_reputation_ladder`, `objectlayer::unit_reaction_reputation_branch_is_gated`, `objectlayer::unit_reaction_player_to_npc_is_atwar_not_rank`, `objectlayer::unit_reaction_persuaded_bump_and_clamp`, `objectlayer::unit_reaction_self_is_friendly`

`0x606576` and `0x60657f` test both rows. `0x606588` tests B's bit 3, `0x6065b0` compares B's
controlling player with `0x468550`. `0x6065be` tests `[row+0x8]` for `0x1000` and `0x6065d2` bit
8 of `[[P+0xe68]+0x8]`. `0x6065e6` calls `0x4d6490`; `0x6065fe` `0x605fc0` and `0x606609`
`0x4d63a0`; `0x606615` calls `0x606640(rowA, rowB)`. `0x4d63a0` turns the standing into a rank
by thresholds: 42000 and up is 7, 21000 6, 9000 5, 3000 4, 0 3, −3000 2, −6000 1, below that 0.
`0x4d6490` searches a list of `(faction, rank)` pairs and stores the rank on a match; `0x4d61b0`
reads bit 1 of the flag byte of the faction's reputation slot, one of 64 records of 16 bytes. In `0x6061e0`, `0x606432` calls with
`[[A+0x110]+0x74]`, `0x606439` caps at 6, `0x606449` compares `[A+0x110]+0x30` with B's GUID and
`0x606464` tests bit 14 of B's flags; `0x6064a0` keeps the increment to ranks 2, 3 and 4.

## Templates compared

`0x606640(rowA, rowB)` returns 1, 3 or 4: hostile when A's enemy mask meets B's group or B's
faction is among A's enemies; friendly when A's friend mask meets B's group, B's faction is among
A's friends, B's friend mask meets A's group, or A's faction is among B's friends; neutral
otherwise. It reads B's enemies never, so hostility is only A's. Each list stops at its first
zero.

Addresses: `0x606640`, `0x606649`, `0x606662`, `0x606679`, `0x606693`, `0x6066aa`, `0x6066c6`, `0x6066df`
Confidence: verified
Proof: one
Test: `objectlayer::faction_template_reaction_truth_table`, `objectlayer::faction_template_reaction_list_mechanics`, `objectlayer::faction_template_reaction_fuzz`, `objectlayer::unit_reaction_league_of_arathor_emissary`

`0x606649` tests `[A+0x14] & [B+0xc]`, the loop at `0x606662` compares `[A+0x18..]` with
`[B+0x4]`, `0x606679` tests `[A+0x10] & [B+0xc]`, the loop at `0x606693` walks `[A+0x28..]`,
`0x6066aa` tests `[B+0x10] & [A+0xc]`, the loop at `0x6066c6` walks `[B+0x28..]` against
`[A+0x4]`. Each loop runs at most four entries and leaves on a zero next entry.

## Controlling player and reputation slot

`0x606170(unit)` gives the player that controls a unit: the unit named by its charmer, field 10,
or else its creator, field 14, or the unit itself when both are zero; the result counts only when
its type mask has the player bit, else 0. `0x605fc0(faction)` is true when the faction's
`Faction.dbc` row exists and its reputation index, `+0x4`, is not negative.

Addresses: `0x606170`, `0x60617d`, `0x606182`, `0x6061b8`, `0x6061bf`, `0x6061c6`, `0x605fc0`, `0x605fc4`, `0x605fdd`
Confidence: verified
Proof: one
Test: none

`0x60617d` tests `+0x10`/`+0x14` and falls back to `+0x20` (`0x606182`); a GUID goes to
`0x468460` with the unit mask 8 (`0x6061b8`), none keeps the unit (`0x6061bf`); `0x6061c6` tests
bit 4 of the type mask, 0x10. `0x605fc0` bounds the id by `[0xc0dd54]`, reads `[0xc0dd50]`, and
`setge` on `[row+0x4]` (`0x605fdd`).

## Corpses and game objects

A unit's reaction toward a corpse compares the unit's template with the corpse's, through the
template comparator alone, and is 3 when either is missing. A game object reacts as its creator
does when its creator, field 6, is a live unit; otherwise as its faction, field 20, through
`0x606530`; with no faction it is 3.

Addresses: `0x6064c0`, `0x6064e9`, `0x606511`, `0x60651b`, `0x5f7fd0`, `0x5f7ff2`, `0x5f8001`, `0x5f800b`, `0x5f801b`, `0x5f8025`
Confidence: verified
Proof: one
Test: none

`0x6064c0` reads A's template, the corpse's from `0x5d7120` (`0x6064e9`), and calls `0x606640`
(`0x606511`) or returns 3 (`0x60651b`). `0x5f7fd0` looks `[+0x110]` up as a unit (`0x5f7ff2`)
and calls `0x6061e0` (`0x5f8001`); else `[[+0x110]+0x38]` (`0x5f800b`) goes to `0x606530`
(`0x5f801b`); zero returns 3 (`0x5f8025`).

## Contest, free-for-all and duel tests

Three small tests read the same player state as the ladder. One is true when a unit's
controlling player has the contested flag, bit 8 of `PLAYER_FLAGS`; another, which nothing calls,
when it has the free-for-all flag, bit 7. A third, also never called, is true when two
player-controlled units belong to players in one duel on opposing teams: the same arbiter, both
teams nonzero and different. A player stands for itself there, without the controlling-player
lookup.

Addresses: `0x606060`, `0x606069`, `0x606080`, `0x6060a0`, `0x6060ed`, `0x60611f`, `0x606141`, `0x60614f`
Confidence: verified
Proof: one
Test: none

`0x606060` calls `0x606170` and returns bit 8 of `[[P+0xe68]+0x8]` (`0x606069`), 0 with no
player; `0x606080` returns bit 7. `0x6060a0(this, other)` needs a unit `other` and bit 3 of both
`UNIT_FIELD_FLAGS`, takes each as its own player when its type mask has 0x10 (`0x6060ed`) and
otherwise asks `0x606170`, then compares the arbiters (`0x606141`) and returns 1 for different
nonzero teams (`0x60614f`).

## Attacking

`0x606980(this = A, B)` decides whether A may attack B. Only a creature with type flag 0x2 may
attack a ghost player, bit 4 of `PLAYER_FLAGS`. Bits 1, 7, 16, 20 and 25 of B's `UNIT_FIELD_FLAGS`
each refuse. Bits 8 and 9 refuse across the controlled line both ways: B's 0x100 against a
controlled A, B's 0x200 against any other, and A's against B alike. Two uncontrolled units may
fight when either reaction is 1 or less; a mixed pair when A's is below 4. Two
controlled units may not at 4 or more; else they may when a controlling player is missing, both
are in one duel, B's player has the PvP flag, bit 12, or both are free-for-all.

Addresses: `0x606980`, `0x6069aa`, `0x6069b7`, `0x6069ff`, `0x606a05`, `0x606a8a`, `0x606a90`, `0x606aab`, `0x606ab8`, `0x606afb`, `0x606b03`, `0x606b37`, `0x606b45`, `0x606b59`, `0x606b7f`, `0x605f70`
Confidence: verified
Proof: one
Test: none

The ghost gate tests `[[B+8]+8]` bit 4 and `[[B+0xe68]+8]` bit 4, then `0x605f70(A)` (`0x6069aa`),
which tests bit 0x2 of `[[A+0xb30]+0x14]`. The five flag tests run `0x6069b7` to `0x6069ff`. The
cross tests run `0x606a05` to `0x606a8a`, keying on A's bit 3. Both uncontrolled: `0x606aab` and
`0x606ab8` return 1 on a reaction `<= 1`. A controlled against an uncontrolled B, or the reverse,
ends at `0x606b7f`: `setl` on `< 4`. Both controlled: `0x606b03` refuses at 4; the duel test
(`0x606b37`) compares only the arbiters, both teams nonzero; `0x606b45` tests bit 12 of
`[[P_B+0x110]+0xa0]`; `0x606b59` the two free-for-all bits.

## Assisting, cooperating and interacting

`0x6066f0(this = A, B)` lets A assist B when B is selectable (bit 25 clear) and A's reaction is 4
or more. A controlled B's dueling player must share A's duel and team; a
free-for-all one needs A's too; a missing player passes. An uncontrolled B passes for an
uncontrolled A. Else, with a live charmer or creator unit, B fails on its own bit 8 and takes
that unit's PvP flag; without one, it takes its own. A corpse needs a reaction of 4. Cooperating
needs two uncharmed units, not one, with equal group masks. Interacting needs a selectable B
with NPC flags, both reactions at least 3, and for a ghost A, a creature seen by ghosts.

Addresses: `0x6066f0`, `0x606706`, `0x60671e`, `0x60673e`, `0x606756`, `0x606784`, `0x6067a7`, `0x6067bd`, `0x605ff0`, `0x606022`, `0x60602c`, `0x60603e`, `0x6067d0`, `0x6067df`, `0x606ba0`, `0x606ba6`, `0x606bb7`, `0x606c02`, `0x6067f0`, `0x60681a`, `0x606835`, `0x60683d`, `0x60684f`, `0x60685c`
Confidence: verified
Proof: one
Test: none

`0x606706` tests B's bit 25, `0x60671e` the reaction. `0x60673e` takes both controlling
players; `0x606756` reads B's player's duel team and needs A's player's arbiter and team equal;
`0x606784` the free-for-all pair. `0x6067a7` tests A's bit 3 and calls `0x605ff0(B)` (`0x6067bd`),
which looks up the charmer, else creator, as a unit (`0x606022`): with one, B's bit 8 refuses
(`0x60602c`) and the owner's bit 12 answers (`0x60603e`); without, B's bit 12. `0x6067d0` calls
`0x6064c0`, `setge` at 4 (`0x6067df`). `0x606ba0` refuses the same object (`0x606ba6`) and a charmer
on either (`0x606bb7`), then compares `[row+0xc]` (`0x606c02`). `0x6067f0` tests a player A's ghost
bit and `0x605f70(B)` (`0x60681a`), B's bit 25 (`0x606835`), field 147 nonzero (`0x60683d`), and
both reactions `>= 3` (`0x60684f`, `0x60685c`).

## Interacting now

`0x606880(this = A, B)` adds the state to interacting: A not flagged 0x100000, uncharmed and
alive; when shapeshifted, only in a form whose `SpellShapeshiftForm.dbc` flags carry 0x8; B alive,
or allowed to be talked to dead, uncharmed and out of combat; then `0x6067f0`.

Addresses: `0x606880`, `0x606893`, `0x60689f`, `0x6068ab`, `0x6068b4`, `0x6068e3`, `0x6068f2`, `0x6068fb`, `0x606904`, `0x606914`, `0x606920`
Confidence: verified
Proof: one
Test: none

`0x606893` tests A's bit 20, `0x60689f` its charmer field 10, `0x6068ab` its health, field 22,
`> 0`. `0x6068b4` calls `0x60e9f0`; when true, the form, byte 2 of field 138 (`[+0x212]`), indexes
the `SpellShapeshiftForm.dbc` rows and `[row+0x2c]` must carry 0x8 (`0x6068e3`). B's health `<= 0` asks `0x613230`
(`0x6068fb`); B's charmer (`0x606904`) and `0x60ecd0` (`0x606914`) refuse; `0x606920` tail-calls
`0x6067f0`. The meanings of `0x60e9f0`, `0x613230` and `0x60ecd0` are their names, not read here.
