# glue: the realm list

The realm list screen's data: categories, population bands, the sort, and its ten script functions.

`RealmList.cpp` (`0x46e300` to `0x46f4a0`) turns the realm list the logon server sends into the
tabs and rows of GlueXML's `RealmList` and `RealmWizard`. net's parser fills the holder at
`0xc28118` with 0x134-byte `REALM_INFO` records. Glue groups them into categories: the
`Cfg_Categories.dbc` rows of the `Wow.ini` Region, built once at startup into the array at
`0xb41f7c`, each a 28-byte `RealmCategory` holding indices into the holder. Each time a list
arrives, the glue manager's update rebuilds the categories, computes the population statistics
behind the load column, sorts every category by the four sort records at `0xb41f40`, and raises
the list. Ten script functions (`0x837ea0`) read the categories; two `.data` cells, the selected
category `0x837e98` and the chosen category id `0x837e9c`, outlive every glue screen. Nothing here
touches the wire: `ChangeRealm` and `RequestRealmList` hand off to the glue manager.

Roots: `0x46e300`, `0x46e470`, `0x46e510`, `0x46f480`, `0x46f4a0`, `0x46ecf0`, `0x46ed10`, `0x46ed20`, `0x46ed30`, `0x46ee40`, `0x46f0e0`, `0x46f1f0`, `0x46f2c0`, `0x46f330`, `0x46f400`

## The realm record

A `REALM_INFO` record is 0x134 bytes. net's `CMD_REALM_LIST` parser `0x5b2230` writes it; this
file reads six fields. The parser replaces three exact populations and sets a flag for each: 600.0
becomes 0.0 with `0x20`, 200.0 becomes 0.001 with `0x40`, 400.0 becomes 8.0 with `0x80`. Every
statistic and band below sees the replaced value. `0x5ab930` returns record `i` of the holder, or
null when `i` is at or past its count.

| offset | type | holds |
|---|---|---|
| `+0x04` | `uint32_t` | realm type, matched against `Cfg_Configs` column 1 |
| `+0x08` | `uint8_t` | flags: `0x01` and `0x02` returned by `GetRealmInfo`, `0x20`, `0x40`, `0x80` the sentinels |
| `+0x09` | `char[256]` | name |
| `+0x12c` | `float` | population |
| `+0x130` | `uint8_t` | the account's characters on the realm |
| `+0x131` | `uint8_t` | category id, 0 meaning the first category |

Addresses: `0x5b2230`, `0x5b22b7`, `0x5b232d`, `0x5b2342`, `0x5b2357`, `0x5ab930`, `0x5ab97a`, `0xc28118`
Confidence: verified
Proof: one
Test: `net::realm_list_fill`

The parser takes `esi = [holder+0xc] + i·0x134` and stores the index byte at `+0x00` (`0x5b22b7`),
then passes `+0x04`, `+0x08`, `+0x09` (limit 0x100), `+0x109` (limit 0x20), `+0x12c`, `+0x130`,
`+0x131` and `+0x132` to its readers in that order. `0x5b232d` compares the raw dword with
`0x44160000` (600.0), stores 0 and ORs `0x20`; `0x5b2342` maps `0x43480000` (200.0) to `0x3a83126f`
(0.001) with `0x40`; `0x5b2357` maps `0x43c80000` (400.0) to `0x41000000` (8.0) with `0x80`. The
types are the loads in this file: `fadd dword [+0x12c]` (`0x46e5ee`), `movzx` of `+0x130`
(`0x46ef4e`), `test byte [+0x8]` (`0x46ea7c`), a dword load of `+0x4` (`0x46ea98`), a byte load of
`+0x131` (`0x46e5a9`). `0x5ab930` returns null when `index ≥ [holder+8]` unsigned (`0x5ab97a`),
else `[holder+0xc] + index·0x134`. `net::realm_list_fill` runs `0x5b2230` whole; `0x5ab930` has
no test.

## The category set

At startup `0x46e300` builds one `RealmCategory` per `Cfg_Categories.dbc` row whose column 1
equals the Region `0xb41da4`, in file order: `+0x00` the row's column 0 as its id, `+0x04` the
row, `+0x08` to `+0x14` a shared dword array of realm indices, `+0x18` the count in use. The array
of pointers is `{capacity 0xb41f74, count 0xb41f78, data 0xb41f7c}`; a Region no row names leaves
it empty. `CGlueMgr::Initialize` copies the Region from `0x882730`, which startup reads from
`Wow.ini`, section `WoW Config`, key `Region`. The function's tail writes the default sort (Sort
records). At glue shutdown `0x46e470` frees each category's index array, each category and the
pointer array, and zeroes the three globals.

Addresses: `0x46e300`, `0x46e336`, `0x46e3b7`, `0x46e3f3`, `0x46e3fe`, `0x46e410`, `0x46e470`, `0x837fb4`, `0x837fc8`, `0x46a51b`, `0x4024ac`, `0x5411b0`, `0xc0dfb4`, `0xb41da4`, `0xb41f74`, `0xb41f78`, `0xb41f7c`, `0x882730`, `0x46ad10`
Confidence: verified
Proof: one
Test: none

`0x5411b0` loads `DBFilesClient\Cfg_Categories.dbc` into the store `0xc0dfb4` (records at `+0`,
count at `+4`) with stride 0x30: column 0 at `+0x00`, column 1 at `+0x04`, the names from `+0x08`,
the row's ordinal at `+0x2c` (`0x54131e`). `0x46e300` counts the rows with `[row+4] == [0xb41da4]`
(`0x46e336`), when the count changed, resizes the pointer array through `0x46f550` (or frees it
through `0x46f510` for none) and stores the count. A second pass tests the same match (`0x46e3b7`),
allocates 0x1c bytes (tag `0x837fc8`, `RealmList.cpp`, line 126), zeroes `+0x08` to `+0x14`, and
stores `[row]` (`0x46e3f3`), the row (`0x46e3fe`) and 0 at `+0x18` (`0x46e410`). `0x4024ac` passes
`0x882730`, `Region` and `WoW Config` to the `Wow.ini` reader; `0x46a51b` stores it in `0xb41da4`.
`0x46e470`, called only from `0x46ad10`, frees `+0x10` with the shared arrays' tag `0x832e00`, each
category with `delete`, and the array with `.PAURealmCategory@@` (`0x837fb4`).

## The list rebuild

When a realm list arrives, `CGlueMgr::Idle` calls `0x46e510`. It empties every category and walks
the holder in order: a realm's category byte, or the first category's id when the byte is 0, joins
every category of that id, so a byte no category has puts the realm on no tab. The last realm
named like the `realmName` CVar, case-insensitively, records its id. Then come the statistics and
the sort, and `0x837e98` takes the index of the first category of the recorded id, unchanged when
none matches. Last it raises `OPEN_REALM_LIST` (9), or `GET_PREFERRED_REALM_INFO` (10) when the
session's character list is empty and `0x837e9c` is −1. With no categories, a byte of 0 reads
through a null array.

Addresses: `0x46e510`, `0x46e530`, `0x46e5b8`, `0x46e5d4`, `0x46e60b`, `0x46e62d`, `0x46e708`, `0x46e713`, `0x46e71d`, `0x46e73d`, `0x46bf96`, `0x46c0e2`, `0x5b42a0`, `0x837e98`, `0x837e9c`
Confidence: verified
Proof: one
Test: `glue::realm_load_stats_difftest`

`0x46bf96` and `0x46c0e2` are the two calls. `0x46e530` stores 0 at each `+0x18`; a null holder is
created first, as `0x5ab930` does. `0x46e5a9` reads `+0x131`; a zero loads `[[0xb41f7c]]`, the first
category's id (`0x46e5b8`). `0x46e5d4` is `SStrCmpI(realmName, +0x09)`; zero stores the id in a
local that starts at −1. `0x46e60b` compares the id with every category's `+0x00`, with no break; an
array whose size `+0x0c` equals `+0x18` grows by one through `0x4291f0` (`0x46e62d`) and `+0x0c`
takes the new size; the holder index goes to `[+0x10][+0x18]` and `+0x18` increments. After the
statistics and `0x46e750`, the scan at `0x46e708` stops at the first category of the id and
`0x46e713` stores its index. `0x46e71d` tests the word at `ClientConnection` `+0x1ae4`, the count of
the character list at `+0x1ae0` that `0x5b42a0` fills; non-zero, or `0x837e9c ≠ −1`, signals 9, else
10 (`0x46e73d`).
`realm_load_stats_difftest` runs `0x46e510` whole.

## Population statistics

Over the holder's n realms in order (every category, sentinels as replaced), population p:

- `s = 0`, then `s = f32(s + p)` per realm, the add at 53 bits;
- `m = f32(s / n)`;
- `a = 0`, then `a = f32((p − m)² + a)` per realm, from the stored `m`, all at 53 bits;
- `d = f32(sqrt(a / (n − 1)) · 0.6f)`, 0.6f being `0x3f19999a` (0.60000002384), at 53 bits.

`m` goes to `0xb41fb0` and `d` to `0xb41fb4`. With n ≤ 1, `m = 1.0` and `d = 0.0`. A NaN
population makes both NaN, and the band then reads 0 for every realm without a sentinel flag.

Addresses: `0x46e510`, `0x46e586`, `0x46e5e3`, `0x46e672`, `0x46e684`, `0x46e69f`, `0x46e6c4`, `0x46e6c9`, `0x46e6d7`, `0x8038d8`, `0xb41fb0`, `0xb41fb4`
Confidence: verified
Proof: one
Test: `glue::realm_load_stats_difftest`

`0x46e586` zeroes the sum.

```
46e5e3  fld   dword [0xb41fb0]
46e5ee  fadd  dword [esi+0x12c]
46e5f8  fstp  dword [0xb41fb0]
46e684  fdivr dword [0xb41fb0]   ; d8 3d, /7: st0 = sum / n
46e69f  fsub  dword [0xb41fb0]   ; d8 25, /4: p − mean
46e6c4  fdivr dword [ebp-0x4]    ; d8 7d, /7: acc / (n − 1)
46e6c7  fsqrt
46e6c9  fmul  dword [0x8038d8]
```

`0x46e672` `cmp ebx,1; jbe` goes to `0x46e6d7`, which stores `0x3f800000` and 0. n and n − 1 are
loaded by `fild qword` with the high dword 0, so they are exact. In the second loop `fld st(0); fmul
st,st(1)` squares, `fadd [ebp-0x4]` adds the float accumulator (0 at the start), and `fstp` rounds
it to float. The NaN consequence is the band's unordered path.
`realm_load_stats_difftest` runs `0x46e510` whole.

## The load band

`0x46ec60(realm)` returns in `st0` the first band that applies, with m and d the stored statistics
and p the population; `m − d` and `m + d` are formed at 53 bits and compared with the float p, so
a population exactly on an edge reads 0.

| condition | band |
|---|---|
| flags `0x20` | −3.0 |
| flags `0x40` | −2.0 |
| flags `0x80` | 2.0 |
| `m − d > p` | −1.0 |
| `m + d < p` | 1.0 |
| otherwise, NaN included | 0.0 |

GlueXML's `RealmListUpdate` names −3, −2 and 2 Recommended, New and Full, and the others Low, High
and Medium by sign.

Addresses: `0x46ec60`, `0x46ec67`, `0x46ec7c`, `0x46ec8a`, `0x46eca4`, `0x46ecac`, `0x46ecc7`, `0x46eccf`
Confidence: verified
Proof: one
Test: `glue::realm_load_classify_difftest`

`0x46ec67` `test al,0x20` loads −3.0 (`0xc0400000`); `0x46ec7c` `test al,0x40` loads −2.0;
`0x46ec8a` `test al,al; jns` loads 2.0 for bit `0x80`. `0x46eca4` is `fcomp` of `m − d` with p and
`0x46ecac` `test ah,0x41; jne`: only `m − d > p`, with C0 and C3 clear, falls through to −1.0
(`0xbf800000`); below, equal and unordered go on. `0x46ecc7` compares `m + d` with p and `0x46eccf`
`test ah,5; jp`: only C0 alone, `m + d < p`, falls through to 1.0 (`0x3f800000`); greater, equal and
unordered (C0 and C2 set) jump to the zeroed local. The six values are the only returns.
`realm_load_classify_difftest` runs `0x46ec60` whole.

## Sort records

The sort is four records `{key, descending}` at `0xb41f40`, keys 0 characters, 1 load, 2 name, 3
mode. `0x46e300` writes `{0,0} {1,0} {2,0} {3,0}` once per process, over the zeros of `.bss`. Only
`SortRealms` changes them, through `0x46e9b0(key)`: the key already first toggles its direction; any
other moves to the front keeping its own, the records ahead of it shifting back one. Then every
category is re-sorted and `OPEN_REALM_LIST` (9) fires. A refresh does not reset the records: the
rebuild only reads them, so a sort lasts until the process exits. `SortRealms(s)` maps `name`,
`load`, `characters` and `mode` case-insensitively, any other string or number to name; anything
else raises `Usgae: SortRealms("type")`, the binary's spelling.

Addresses: `0x46e300`, `0x46e430`, `0x46e9b0`, `0x46e9da`, `0x46e9f1`, `0x46ea03`, `0x46ea0a`, `0x46f330`, `0x46f35b`, `0x46a520`, `0xb41f40`, `0x838074`, `0x83807c`, `0x838088`, `0x838090`, `0x838098`
Confidence: verified
Proof: one
Test: none

`0x46e430`, `0x46e437` store `i` and 0 at `0xb41f40 + 8i` for `i < 4`. `0x46e9b0` finds the first
record of the key; at index 0, `0x46e9da` reaches `sete` of the stored direction, otherwise it keeps
`+4`, and `0x46e9f1` copies record `j − 1` to `j` down to 1. `0x46ea03`, `0x46ea0a` store the front
record; both exits call `0x46e750` and jump to `0x703e50` with 9. `0x46f35b` presets 2; `SStrCmpI`
with `0x838090`, `0x838088`, `0x83807c`, `0x838074` selects 2, 1, 0, 3. Census, every little-endian
occurrence in the image of `0xb41f40`, of its `+4` and of the shift's base 8 below: `0x46e433`,
`0x46e43a` (`0x46e300`), `0x46e7ca` (the comparator), `0x46e9b5`, `0x46e9d6`, `0x46e9ec`,
`0x46ea05`, `0x46ea0c` (`0x46e9b0`), and one static initializer's float store to that base alone;
the other six dwords occur nowhere. Control: the same scan finds `0xb41fa8` at `0x46ea4b`.
`0x46e300` has one caller, `0x46a520` in `CGlueMgr::Initialize`, whose one caller `0x402ad0` has one
call site, in `0x402840`.

## The realm comparator

`0x46e750` sorts each category's index array with the C runtime's `qsort` (`0x73f727`) and the
comparator `0x46e790`. That resolves both indices (a null either side returns 0) and walks the four
records in order; the first key that does not tie returns, negated when the record's descending
flag is set, and four ties return 0. Keys (jump table `0x46e998`; a key above 3 is skipped):

| key | ascending result |
|---|---|
| 0 characters | both counts 0: tie; −1 when a has more than b; +1 otherwise, equal counts included |
| 1 load | bands: −1 when a's is lower, +1 higher, tie when equal |
| 2 name | `SStrCmpI(a.name, b.name)` |
| 3 mode | rp first (non-RP before RP), then pvp (non-PvP first): Normal, PvP, RP, RP-PvP |

Mode reads each realm's pvp and rp from the last `Cfg_Configs` row of its type; a type with no row
is Normal. Key 0 is not antisymmetric: two realms with the same nonzero count each sort after the
other, and the name compare inside it is discarded.

Addresses: `0x46e750`, `0x46e790`, `0x46e7f4`, `0x46e998`, `0x46e826`, `0x46e841`, `0x46e846`, `0x46e871`, `0x46e888`, `0x46e894`, `0x46e8d3`, `0x46e92c`, `0x46e962`, `0x46e971`, `0x73f727`, `0xc0dfa0`
Confidence: verified
Proof: one
Test: none

`0x46e998` holds `0x46e814`, `0x46e85d`, `0x46e7fb`, `0x46e8a9`, entered at `0x46e7f4`. Key 0:
`0x46e826` goes to the next record when both bytes are 0; equal bytes call `SStrCmpI` (`0x46e841`),
whose `eax` the next block never reads:

```
46e846  mov  al,[ebx+0x130]
46e84c  cmp  [edi+0x130],al
46e852  sbb  esi,esi
46e854  and  esi,-2
46e857  inc  esi
```

Key 1: `0x46e871` sets 1 when a's band is above b's (`test ah,0x41`); `0x46e888` `test ah,5; jp`
returns −1 when below. Key 3: the scan from `0x46e8d3` over the `Cfg_Configs` store `0xc0dfa0`
(stride 0x14, type `+4`, pvp `+8`, rp `+0xc`) overwrites on every match; `0x46e92c` compares pvp,
then rp; `0x46e962` returns `rp_a ? 1 : −1` when rp differs, `0x46e971` `pvp_a ? 1 : −1`. `0x46e894`
negates when the record's `+4` is set. `qsort` gets width 4 and `[+0x18]` elements.

## Category ordinals

Script category arguments are 1-based ordinals over the non-empty categories. `0x46edf0(k)` returns
the stored index of the k-th (0-based) non-empty category, else 0, so an ordinal below 1 or past
the last reads category 0, empty or not. `GetSelectedCategory` returns the ordinal of the first
non-empty category whose index is at least `0x837e98`, signed, so −1 reads the first; with none,
1. `GetNumRealms(c)` returns the count of category `0x46edf0(c − 1)`, 0 past the array, and
without a number the sum over all. `GetRealmCategories` returns the non-empty categories' names in
the client's locale, `UNKNOWN` (`0x838044`) without a row; with none, the first category's name
alone, and with no categories it reads through null.

Addresses: `0x46edf0`, `0x46ee13`, `0x46ee29`, `0x46f400`, `0x46f439`, `0x46ed30`, `0x46ed47`, `0x46ed94`, `0x46f1f0`, `0x46f238`, `0x46f255`, `0x46f26a`, `0x837e98`, `0x838044`, `0xc0e080`
Confidence: verified
Proof: one
Test: none

`0x46edf0` counts from −1, increments for each category with `[+0x18] ≠ 0` (`0x46ee13`) and stops
when the count equals `ecx`; `0x46ee29` returns that index when `0 ≤ index < count`, and every other
exit is `xor eax,eax`. `0x46f400` counts the same way and at `0x46f439` `cmp eax,ebx; jge` pushes
the count plus 1; falling out pushes 1.0. `0x46ed30` tests argument 1 for a number: `0x46ed47` adds
every `[+0x18]`; `0x46ed94` maps `ftol(arg) − 1`. `0x46f238` reads `[row + 8 + 4·[0xc0e080]]`;
`0x46f255` handles no name pushed, and `0x46f26a` loads `[cats[0] + 4]`, which faults when the
array is empty. The return count is the names pushed, or 1.

## Realm by index

`GetRealmInfo` and `ChangeRealm` take `(category, index)` or a flat index alone; a first argument
that is not a number raises the usage error. With two numbers, the realm is entry `index − 1`,
unsigned, of category `0x46edf0(category − 1)`; none when the category is past the array or the
entry past its count. With one, a residual `r = index − 1` walks the categories in order: a
category whose count is above `r` answers with its entry `r` and the walk goes on without changing
`r`, so a later category also above `r` overwrites the answer; a category at or below `r`
subtracts its count. A holder that lost a realm answers null, which also overwrites.

Addresses: `0x46ee40`, `0x46ee8c`, `0x46eeba`, `0x46eebe`, `0x46eed3`, `0x46eee0`, `0x46ef24`, `0x46f0e0`, `0x46f14e`, `0x46f167`, `0x46f1b5`
Confidence: verified
Proof: one
Test: none

The flat walk starts at `0x46ee8c`: `0x46eeba` `cmp ecx,esi; ja` fetches through `0x5ab930` and
stores at `0x46eed3`, then loops with `esi` unchanged; `0x46eebe` `sub esi,ecx` otherwise. The pair
form (`0x46eee0`) takes `[cats + 4k]` when `k < count` and needs `[+0x18] > index − 1` (`0x46ef24`
`jbe`). `ChangeRealm` repeats both: the flat walk at `0x46f14e` and `0x46f167`, the pair test at
`0x46f1b5`.

## GetRealmInfo returns

`GetRealmInfo` returns eight values on every path that returns; the usage error raises instead.
The type columns come from the first `Cfg_Configs` row of the realm's type; with no row both are
nil.

| # | found | not found |
|---|---|---|
| 1 | name | nil |
| 2 | characters, `+0x130` | 0 |
| 3 | 1 for flag `0x01`, else nil | nil |
| 4 | 1 for flag `0x02`, else nil | nil |
| 5 | 1 when the name equals `realmName` case-insensitively, else nil | nil |
| 6 | 1 for the row's pvp column, else nil | nil |
| 7 | 1 for the row's rp column, else nil | nil |
| 8 | the load band | 0 |

Addresses: `0x46ee40`, `0x46ee59`, `0x46ef49`, `0x46ef68`, `0x46ef83`, `0x46efa3`, `0x46eff4`, `0x46f03d`, `0x46f01f`, `0x46f072`, `0x46f090`, `0x5ab7d0`
Confidence: verified
Proof: one
Test: none

`0x46ee59` pushes the usage string to `luaL_error`. `0x46ef49` pushes `+0x09`; `0x46ef4e` loads the
byte through `fild`; `0x46ef68` and `0x46ef83` test bits 1 and 2, pushing 1.0 (`0x3ff00000:0`) or
nil. `0x46efa3` compares with `0x5ab7d0`'s value. The `Cfg_Configs` scan stops at the first type
match (`0x46eff4` `je 0x46f03d`), where `setne` turns `+8` and `+0xc` into flags. Both found paths
call `0x46ec60` (`0x46f01f`, `0x46f072`) and return 8; `0x46f090` pushes nil, 0, five nils and 0 and
returns 8.

## ChangeRealm

`ChangeRealm(category, index)` with two numbers stores `0x46edf0(category − 1)` in `0x837e98`
before any range check. When a realm is found it sets `0x837e9c` to 0 if it was −1 and passes the
record in `ecx` to the glue manager's `0x46b210`; none found does nothing more. The flat form
stores no category. It returns no values.

Addresses: `0x46f0e0`, `0x46f188`, `0x46f1c8`, `0x46f1d1`, `0x46f1dd`, `0x46b210`, `0x837e98`, `0x837e9c`
Confidence: verified
Proof: one
Test: none

`0x46f188` stores `eax`, the mapped index, between `cmp eax,[0xb41f78]` and its `jae`. `0x46f1c8`
`cmp [0x837e9c],-1; jne` skips `0x46f1d1`, the store of 0; `0x46f1dd` calls `0x46b210`. Every exit
is `xor eax,eax`.

## The realm suggestion

`SetPreferredInfo(category, pvp, rp)` maps the ordinal (`0x46edf0`), reads pvp and rp as
integers, and calls `0x46ea20`. That stores the index in
`0x837e98` (above the count, 0), the category's id in `0x837e9c`, and pvp and rp in
`0xb41fa8` and `0xb41fac`, unread. Skipping realms with flag `0x01`, it suggests:

1. the category's first realm whose type's first `Cfg_Configs` row has the asked pvp and rp
   (a type with no row counts as 0 and 0);
2. else the category's first realm;
3. else the first realm matching as in 1 in any category.

A suggestion signals `SUGGEST_REALM` (18) with `"%d%d"`, the index plus 1 and the position plus 1;
none signals `OPEN_REALM_LIST` (9). A pvp or rp of 2 matches nothing.

Addresses: `0x46f2c0`, `0x46f309`, `0x46f323`, `0x46ea20`, `0x46ea30`, `0x46ea3e`, `0x46ea52`, `0x46ea7c`, `0x46eab2`, `0x46eada`, `0x46eb22`, `0x46eb44`, `0x46ebeb`, `0x46ec26`, `0x46ec42`, `0x6f1c10`, `0xb41fa8`, `0xb41fac`, `0x837e98`, `0x837e9c`
Confidence: verified
Proof: one
Test: none

`0x46f309` and the call after it read arguments 2 and 3 with default 0; `0x46f323` passes the index
in `ecx`, pvp in `edx`, rp on the stack (`ret 4`). `0x6f1c10` returns 0 for nil, 0 or 1 for a
boolean and `__ftol` of a number. `0x46ea30` `cmp edi,eax; jbe` keeps an index up to the
count. `0x46ea52` stores `[[cats + 4k]]`. Pass 1 skips `test byte [+8],1` (`0x46ea7c`), zeroes both flags
(`0x46ea88` to `0x46ea93`), skips the scan on an empty table (`0x46ea96` `jbe`), stops at the
first row of the type (`0x46eab2`), restores the zeros on a miss (`0x46eabc`, `0x46eabf`), and
needs both `setne` flags equal to the arguments (`0x46eada`, `0x46eadf`). Pass 2 takes the first realm whose bit is clear (`0x46eb22`). Pass 3
repeats pass 1's test, zeros included (`0x46eb9a` to `0x46eba5`, `0x46ebcb`), over every
category (`0x46ebeb`). The suggestions call `0x703f50` with 18
(`0x46eb44`, `0x46ec42`); `0x46ec26` signals 9. The only occurrences of `0xb41fa8` and `0xb41fac` in
the image are the stores at `0x46ea49` and `0x46ea58`.

## The selection cells

`0x837e98`, the stored category index, and `0x837e9c`, the chosen category id, are `.data` cells,
−1 in the file, that outlive every glue screen and glue script reload. `0x837e98` is written by the
rebuild, the suggestion and `ChangeRealm`'s pair form, and read only by `GetSelectedCategory`.
`0x837e9c` is written by the suggestion (the category's id), by `ChangeRealm` (0 when −1) and by the
character list load at `0x472556` (0, when the list has characters); the rebuild's tail and
`0x47253d` read it.

Addresses: `0x837e98`, `0x837e9c`, `0x46e713`, `0x46ea3e`, `0x46f188`, `0x46f407`, `0x46ea52`, `0x46f1d1`, `0x472556`, `0x46e72a`, `0x47253d`
Confidence: verified
Proof: one
Test: `glue::realm_load_stats_difftest`

Every little-endian occurrence in the image of `0x837e98`: `0x46e714`, `0x46ea40`, `0x46f189`,
`0x46f409`; of `0x837e9c`: `0x46e72c`, `0x46ea54`, `0x46f1ca`, `0x46f1d3`, `0x47253f`, `0x472558`.
Each lies inside the instruction the claim lists. Control: the same scan finds `0x837ea0` at
`0x46f48b`. The file holds `ff ff ff ff` at both.
`realm_load_stats_difftest` runs `0x46e510` whole.

## Script functions

The ten pairs at `0x837ea0` ({name, function}, no terminator) are registered by `0x46f480` from the
glue screens' script setup and unregistered by name by `0x46f4a0`. Each function takes `lua_State`
in `ecx` and returns its result count.

| name | arguments | returns |
|---|---|---|
| `RequestRealmList` | flag (default 0) | 0; calls `0x46b8d0` with the flag |
| `CancelRealmListQuery` | none | 0; calls `0x46b7e0` |
| `RealmListDialogCancelled` | none | 0; calls `0x46b810` |
| `GetNumRealms` | [category] | 1 |
| `GetRealmInfo` | category[, index] | 8 |
| `ChangeRealm` | category[, index] | 0 |
| `GetRealmCategories` | none | the names pushed, or 1 |
| `SetPreferredInfo` | category, pvp, rp | 0 |
| `SortRealms` | key | 0 |
| `GetSelectedCategory` | none | 1 |

These agree with `tables/bindings.tsv`, which leaves `GetRealmCategories`' arity open.

Addresses: `0x837ea0`, `0x46f480`, `0x46f4a0`, `0x46ecf0`, `0x46ed10`, `0x46ed20`, `0x704120`, `0x704160`, `0x46abc9`, `0x46acf9`, `0x46b8d0`, `0x46b7e0`, `0x46b810`
Confidence: verified
Proof: one
Test: none

`0x46f480` loads `[0x837ea0 + i]` into `ecx` and `[0x837ea4 + i]` into `edx` for `i` from 0 to
`0x48` in steps of 8 and calls `0x704120`; `0x46f4a0` passes each name to `0x704160`. Their callers
are `0x46abc9` and `0x46acf9`. The pairs, read from the image: `RequestRealmList` `0x46ecf0`,
`CancelRealmListQuery` `0x46ed10`, `GetNumRealms` `0x46ed30`, `GetRealmInfo` `0x46ee40`,
`ChangeRealm` `0x46f0e0`, `GetRealmCategories` `0x46f1f0`, `SetPreferredInfo` `0x46f2c0`,
`SortRealms` `0x46f330`, `GetSelectedCategory` `0x46f400`, `RealmListDialogCancelled` `0x46ed20`;
the dwords after the tenth are string text. `0x46ecf0` reads argument 1 through `0x6f1c10` with
default 0 and passes it in `ecx`; `0x46ed10` and `0x46ed20` are one call and `xor eax,eax`.
