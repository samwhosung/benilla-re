# ui: skill and trade skill list order

The four comparators that order the character skill list and the trade skill list: each list is a table of group records and
a table of row records, and each table is sorted by `qsort` over an array of record pointers with a comparator that answers
−1, 0 or 1. The comparators read plain fields of the records and a few `DBC` tables, so a list's order is fixed by their
keys. The pickers that build the lists are not in this entry.

Roots: `0x4d2ed0`, `0x4d3000`, `0x4d3070`, `0x4fd180`, `0x4fd380`, `0x4fd540`

## Skill groups

The skill list sorts its group records first. A group record starts with a category id. The comparator looks both ids up in the category table, and
orders the groups by the dword at `+0x28` of the category record, ascending; two equal keys answer 1, never 0. An id below 0 or above the table's
maximum has no record, and an id inside the range whose slot is null has none either: the comparator loads the slot unchecked
(`0x4d3024`), and it faults on a missing one.

Addresses: `0x4d2ed0`, `0x4d2fbf`, `0x4d2fc8`, `0x4d3000`, `0x4d3009`, `0x4d300d`, `0x4d3024`, `0x4d302b`, `0x4d3038`, `0x4d3040`, `0x4d3042`, `0x4d305c`, `0x73f727`
Confidence: verified
Proof: one
Test: none

`0x4d2ed0` sorts the group array with `0x4d3000` through `qsort` (`0x4d2fbf`, `0x4d2fc8`, element size 4). The comparator loads the two group pointers
from its two arguments and each category id from the first dword of the record (`0x4d3009`, `0x4d300d`), reads the category record from the table by id (`0x4d3024`, `0x4d302b`),
and compares `+0x28` (`0x4d3038`, `0x4d3040`); `setge` (`0x4d3042`) gives 1 for equal or greater keys. A missing record makes the comparator read address 0x28 and fault (`0x4d3038`, `0x4d3051`).

## Skill rows

The rows follow the groups. A row with its present flag sorts before one without it. Rows then order by the position of their group in the sorted
group table, ascending, then rows with a skill id of 0 first, then rows whose second flag is 0 before those whose flag is set, and last by the skill's localized
name, ascending, ignoring case. A pair with no skill record answers 0.

Addresses: `0x4d2fd3`, `0x4d2fd9`, `0x4d2fe2`, `0x4d3070`, `0x4d307e`, `0x4d3095`, `0x4d309e`, `0x4d30c7`, `0x4d30cc`, `0x4d30f0`, `0x4d30f8`, `0x4d3101`, `0x4d310d`, `0x4d311c`, `0x4d312d`, `0x4d3130`, `0x4d3148`, `0x4d315a`, `0x4d3172`, `0x4d317e`, `0x4d318d`, `0x4d3198`, `0x4d31a9`, `0x4d31ad`
Confidence: verified
Proof: one
Test: none

`0x4d2fe2` sorts the row array with `0x4d3070`. The comparator reads the present flag at `+0x08` of each row: neither present answers 0 (`0x4d319c`), and exactly one gives −1 to the present row
(`0x4d31a9`, `0x4d31ad`). Each row's group id at `+0x04` is searched for in the group table (`0x4d30c7` to `0x4d30e8`); different positions answer −1 for the smaller (`0x4d30f8`, `0x4d3101`). A row with a skill
id of 0 at `+0x00` goes first (`0x4d311c`, `0x4d312d`) and the flag at `+0x0c` orders the zero before a nonzero (`0x4d3130`). The skill table is read by id (`0x4d315a`), and its name at
`+0x0c + 4·locale` goes to the case-blind collation (`0x4d317e`, `0x4d318d`).

## Trade skill rows

The trade skill list orders its rows by the header they sit under. A row counts as listed when its flag at `+0x1c` is nonzero and its header's expanded dword at `+0x0c` is nonzero; listed
rows come first. Then rows order by the position of their header in the sorted header table, ascending, and a row whose id at `+0x00` is −1 (a header row)
comes before the others. Then rows order by `+0x04` ascending, by `+0x14` descending, and by the spell name from `Spell.dbc` at `+0x1e0`, ascending and ignoring case. A pair with no spell record answers 0.

Addresses: `0x4fd356`, `0x4fd36f`, `0x4fd380`, `0x4fd3c3`, `0x4fd3d9`, `0x4fd3de`, `0x4fd3e8`, `0x4fd3f6`, `0x4fd405`, `0x4fd417`, `0x4fd432`, `0x4fd44b`, `0x4fd452`, `0x4fd461`, `0x4fd475`, `0x4fd489`, `0x4fd4a7`, `0x4fd4b1`, `0x4fd4bb`, `0x4fd4d6`, `0x4fd4e6`, `0x4fd4f6`, `0x4fd506`, `0x4fd51b`, `0x4fd528`
Confidence: verified
Proof: one
Test: none

`0x4fd36f` sorts the row array with `0x4fd380`. For each of the two rows the loop over the header table (`0x4fd3c3`) finds the header whose first dword equals the row's `+0x08` and second equals its `+0x0c`, keeping
the last matching index (`0x4fd3de`, `0x4fd405`) and the listed flag (`0x4fd3e8`, `0x4fd417`). One listed row of two answers −1 for it (`0x4fd528`); two unlisted rows answer 0 (`0x4fd51b`). Different indexes answer
by `sbb` (`0x4fd44b`, `0x4fd452`). The spell table is read by id (`0x4fd489`); the keys are compared at `0x4fd4a7` (`+0x04`, ascending), `0x4fd4b1` (`+0x14`, `setl`: larger first) and `0x4fd4d6` (name).

## Trade skill headers

The header table orders by its first dword, ascending. Two headers with the same first dword order by the localized name of the matching record of the skill table, using the
name at `+0x4c + 4·locale`, or `+0x28 + 4·locale` when the first is empty, ascending and ignoring case; the matching record is the one whose first two dwords equal the header's. A
pair with no matching record, or with an empty name, answers 0.

Addresses: `0x4fd34e`, `0x4fd356`, `0x4fd540`, `0x4fd55b`, `0x4fd567`, `0x4fd5a1`, `0x4fd5ab`, `0x4fd5c4`, `0x4fd5ed`, `0x4fd5f3`, `0x4fd600`, `0x4fd624`, `0x4fd639`, `0x4fd63f`
Confidence: verified
Proof: one
Test: none

`0x4fd356` sorts the header array with `0x4fd540`. The comparator compares the first dwords of the two headers (`0x4fd55b`) and, when they differ, answers by `setg` (`0x4fd639`, `0x4fd63f`):
the greater is after. Otherwise it scans the records of 0x74 bytes for the two with the same first and second dword (`0x4fd5a1`, `0x4fd5ab`, `0x4fd5c4`), takes each name at `+0x4c + 4·locale`
(`0x4fd5f3`) or `+0x28 + 4·locale` (`0x4fd600`) and returns the collation (`0x4fd624`).
