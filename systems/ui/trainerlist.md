# ui: trainer states, filters and order

Which trainer rows are visible, in what order, and how a row's state is worked out. The records and the Lua getters are
`ui/trainer`. This is the finalizer `0x4d8410`, the three row comparators and the state re-evaluation `0x4d7d40`.

Roots: `0x4d7d40`, `0x4d8410`, `0x4d85c0`, `0x4d8760`, `0x4d8850`, `0x4d7b90`, `0x4d7c30`, `0x4da260`, `0x4da3f0`

## Filters

The state filter `0xb73a1c` holds a bit per state (bit n for state n). Every list packet sets it to 3, available and
unavailable, or to 5, available and used, for a type 1 trainer; so a red row shows by default and the known rows are hidden.
`SetTrainerServiceTypeFilter("type", on [, exclusive])` names `available` (bit 0), `unavailable` (1) or `used` (2), or `all`,
which sets 7 and needs no second argument. A zero clears the bit, a nonzero third argument makes it the only bit, otherwise it
is added. The skill-line filter and the expanded mask are reset to all ones by every packet. Each change runs the
finalizer and signals `TRAINER_UPDATE` (0x136).

Addresses: `0x4da260`, `0x4da28e`, `0x4da2a9`, `0x4da2bb`, `0x4da2c2`, `0x4da2e2`, `0x4da31a`, `0x4da334`, `0x4da36f`, `0x4da390`, `0x4d8c90`, `0x4d8cb0`, `0x4d8cd0`, `0x4da3f0`, `0x4da6c0`, `0x4da740`, `0x4d75d9`, `0x4d75e5`, `0x4d75ef`, `0xb73a1c`, `0xb73a20`, `0xb73a24`
Confidence: verified
Proof: one
Test: none

A missing second argument raises `Missing on//off parameter in SetTrainerServiceTypeFilter`, another name `Bad service type in
SetTrainerServiceTypeFilter`. The mapper `0x4da390` compares the name case-insensitively with `available`, `unavailable` and `used` and returns 6 for
another name. The default is written at `0x4d75d9` (3) and `0x4d75e5` (5, only when the type is 1). `0x4d8c90` stores the state
mask, `0x4d8cb0` the skill-line filter and `0x4d8cd0` the expanded mask, each then calling `0x4d8410` and signalling 0x136.

## Hiding rows

The finalizer marks every row visible (`+0x34` is 1) and then hides some, each hide lowering the visible count. A service
row is hidden when its state bit is clear in the state filter. Any row of a group is hidden when the group has no
visible content, which a header counts as the group's per-state member counts seen through the live state mask and the
skill-line filter. A service row, but not a header, is also hidden when its group is collapsed. So a filter that empties a
group hides its header, and collapsing keeps it. The rows stay in the array behind the visible ones.

Addresses: `0x4d8410`, `0x4d8414`, `0x4d8429`, `0x4d8431`, `0x4d8447`, `0x4d8452`, `0x4d846f`, `0x4d84ac`, `0x4d84ba`, `0x4d84c6`, `0x4d8508`, `0x4d8528`, `0x4d8535`, `0x4d853d`, `0x4d8541`, `0x4d8546`, `0x4d8549`, `0x73f727`
Confidence: verified
Proof: one
Test: `ui::listorder_trainer_filter_hides_header_e2e`, `ui::listorder_trainer_mounts_e2e`, `ui::listorder_trainer_tradeskill_e2e`, `ui::listorder_trainer_tradeskill_filter_e2e`

A header's `+0x1c` is set when some state `s` has its bit in the state filter and a nonzero counter `hdr[+4+4s]`; its `+0x20` is
the collapsed flag from the expanded mask. The row test at `0x4d8528` has no header exemption; the exemption at `0x4d853d` guards
only the collapse test. The counters are built with the list and re-counted by `0x4d7d40`. The sort runs over the whole
array, with the visible flag as its first key.

Each test runs `0x4d8410` whole on a synthesized list, with the real `qsort`, comparators and collator, and reads the sorted rows; the two tradeskill tests also run the group-key slice `0x4d7781` to `0x4d7807`.

## Row order

The row comparator is chosen by the trainer type (`0x4d8561`): type 1 uses `0x4d8850`, type 2 `0x4d8760`, anything else
(classes, pets) `0x4d85c0`. Each starts with the visible flag (visible first) and ends with the localized spell name,
case-sensitively. The class comparator then orders by group position, the header row first, required level, required skill
value, name, then rank text. The mount comparator uses group position, header first, state ascending, then name. The
tradeskill comparator uses the group key, the header first, required skill value, then name; it has no level, state or rank
key. Headers sort by `0x4d7c30` (the key) for type 2, else `0x4d7b90`.

Addresses: `0x4d8561`, `0x4d85c0`, `0x4d85ce`, `0x4d85ef`, `0x4d86a0`, `0x4d86c6`, `0x4d86e9`, `0x4d86fe`, `0x4d8760`, `0x4d876f`, `0x4d878b`, `0x4d87d9`, `0x4d87e3`, `0x4d87f8`, `0x4d8850`, `0x4d885b`, `0x4d88eb`, `0x4d88f1`, `0x4d894e`, `0x4d79eb`, `0x4d7b90`, `0x4d7bab`, `0x4d7bb5`, `0x4d7c12`, `0x4d7c30`
Confidence: verified
Proof: one
Test: `ui::sortcmp_4d85c0`, `ui::sortcmp_4d8760`, `ui::sortcmp_4d8850`

Name compares are `0x64a480` for rows and `0x64a4c0` (case-insensitive) for header names. In the class comparator group
position is the index in the header array, which is name-sorted; the tradeskill comparator compares the raw key. The header
comparator `0x4d7b90` puts group -1 first, then headers with `+0x24` zero (a flag set when a group's rows all have a
nonzero point cost), then the skill line's display name. Type 3 (pet) uses the class comparators.

Bit-exact coverage: `ui::sortcmp_4d85c0` covers 5 ranges from `0x4d85c0` to `0x4d8758` of `0x4d85c0`; `ui::sortcmp_4d8760` covers `0x4d8760` to `0x4d87b7`, `0x4d87bb` to `0x4d87c6`, `0x4d87ca` to `0x4d884f` of `0x4d8760`; `ui::sortcmp_4d8850` covers `0x4d8850` to `0x4d88a8`, `0x4d88ac` to `0x4d88bd`, `0x4d88c1` to `0x4d89a3` of `0x4d8850`.

## State

When the window is open `0x4d7d40` works each row's state out again, replacing the server's. A row whose spell id is -1,
negative, above the highest or without a record keeps the sent state. Otherwise the state starts at 0 and each effect of the
sent spell, in order, may overwrite it. A learn effect (0x24) counts the taught spell and whether it or a higher rank is
known. A skill-step effect (0x2c) gives 2 when the player's step in that line reaches the effect's value. A pet-learn effect
(0x39) gives 1, and stops, with no pet or a pet below the row's level. When every learn effect is known the state is 2.

Addresses: `0x4d7d40`, `0x4d7d46`, `0x4d7da2`, `0x4d7dec`, `0x4d7df6`, `0x4d7e0e`, `0x4d7e5a`, `0x4d7e76`, `0x4d7f07`, `0x4d7f17`, `0x4d7f1e`, `0x4d7f4e`, `0x4d7fac`, `0x4d7fb9`, `0x4d7fc6`, `0x4d803c`, `0x6e3760`, `0x60c740`, `0x60c8d0`
Confidence: verified
Proof: one
Test: none

The skill-step value is the larger output of `0x6e3760` (`basePoints + 1` on every shipped effect). A skill line the player does
not have gives no write. The player is found with the active GUID and the pet through the player's summon field.
Later effects overwrite earlier writes; nothing after the effect loop overwrites a state of 1 or 3 from inside it except
the all-known rule.

## State from requirements

A row still at 0 takes its state from its requirements, in order. A required skill line the player has, with value (plus
the permanent bonus) below the required value, gives 1; a line the player lacks leaves 0. Each required ability must be
known, or a higher rank known, by the pet if the row taught a pet spell and otherwise the player; the first missing one gives
1. A required level above 1 with the player's level below it gives 1. A type 1 trainer reports 3 for a row whose taught spell
has a higher rank known, and 3 in place of 1 when the missing ability is exactly the previous rank of the taught spell. 

Addresses: `0x4d7fd2`, `0x4d7fdd`, `0x4d8067`, `0x4d8082`, `0x4d811a`, `0x4d811f`, `0x4d81ac`, `0x4d81c4`, `0x4d81e4`, `0x4d81fd`, `0x4d8215`, `0x4d8221`, `0x4d8239`, `0x4d8310`, `0x4d835c`, `0x4d8385`, `0x4d838e`, `0x4d8397`, `0x4d83a4`, `0x4d83ad`, `0x4d83b9`, `0x4d7e22`, `0x4d7e3e`, `0x6de040`
Confidence: verified
Proof: one
Test: none

The missing-line exit `0x4d8082` jumps past the compare, so a profession recipe the server sent as unavailable becomes
available when the player has no rank in its line, after the first re-evaluation. No filter carries state 3, so such a row never shows. The state-3 writes are `0x4d7e3e` (type 1,
the learn effect with a higher rank known) and `0x4d83a4` (type 1, player variant, a learn effect present, the ability's
skill-line-ability row found by race and class, and that row's `+0x20`, the next rank, equal to the taught spell). Every
other failing path of the ability test reaches `0x4d83b9`, which writes 1. The level test has no type variant.

## Group counts

After the states, every group's six counters are cleared and its `+0x24` set to 1, and each service row is counted into
its group by state. At a type 1 trainer a row now in state 2 gets the group key -1 first. The finalizer then runs and
`TRAINER_UPDATE` is signalled.

Addresses: `0x4d8252`, `0x4d8260`, `0x4d8272`, `0x4d8274`, `0x4d82a9`, `0x4d82b2`, `0x4d82bb`, `0x4d82bf`, `0x4d82f3`, `0x4d83c9`, `0x4d83ef`, `0x4d83f4`
Confidence: verified
Proof: one
Test: none

The counter increment is `hdr[+4+4*state]` at `0x4d83c9`. The known-talents group exists only if the list builder made one.
The window's first frame shows the sent states, because neither the packet handler nor the builder calls `0x4d7d40`.

## When states are re-evaluated

`0x4d7d40` has twelve callers. A pet spell list (opcode 0x179), a learned spell (0x12b), a removed spell (0x203), a
superseded spell (0x12c, twice when the player is present), the flat and percent spell modifiers (0x266, 0x267), a change of
the player's coinage, of a skill's value, maximum or permanent bonus, or of the player's level, and the arrival of an item
record the icon or description getter requested.

Addresses: `0x4bdaf5`, `0x4d914a`, `0x4d9dba`, `0x5ddf41`, `0x5de42d`, `0x5de450`, `0x5e64f1`, `0x5e9fb7`, `0x5ea25f`, `0x5edb80`, `0x6e999f`, `0x6e99bb`, `0x4bd990`, `0x4d9140`, `0x4d9db0`, `0x5ddf30`, `0x5de180`, `0x5e6330`, `0x5e9c20`, `0x5e9fe0`, `0x5edb60`, `0x6e9950`
Confidence: verified
Proof: one
Test: none

The coin watcher is registered for field offset `0xf70`, the skill-value watcher on `+0x84c`, the other skill watcher on
`+0x84e` and `+0x852`, the level watcher on the unit field at `+0x70`. The two item callbacks run only for a successful
item arrival. Learned and removed spells reach it through `0x5e9c20` and `0x5e9fe0`; the latter calls it only when its
second argument is 0.
