# cvar: defaults

Most registered settings give `Register` a literal default text, and `tables/cvars.tsv` lists it. Some settings give a text that the client builds at the call: formatted from a constant, a host value or the hardware row, or chosen by the client's locale. This doc resolves those. The camera settings are `uicamera/cvars`, the video options are `options`, and the hardware step that overwrites sixteen of them is `console/hardware`.

Roots: none, the rows of the registration sites belong to the systems that register

## Formatted defaults

A formatted default is text built by `SStrPrintf` just before the `Register` call; `SoundBufferSize` is the exception, for `0x457520` picks one of two literal strings.

| setting | site | text |
|---|---|---|
| `mouseSpeed` | `0x402c7b` | `%1.1f` of the host pointer speed times 0.1 |
| `SoundBufferSize` | `0x4571ca` | `50` when the byte `0xc4ec54` is set and the detection record has 1 at `+0x18`, else `100`; a literal, not formatted |
| `targetNearestDistanceRadius` | `0x48fd60` | `%f` of 10.0, so `10.000000` |
| `targetNearestDistance` | `0x48fda4` | `%f` of 41.0, so `41.000000` |
| `violenceLevel` | `0x6c5ae1` | `%d` of the entry of the locale table |
| `gxMultisample` | `0x63a950` | `%d` of the `multisample` column of the matched hardware row |
| `gxFixLag` | `0x63a9c1` | `%d` of the `fix-lag` column of the matched hardware row |

Addresses: `0x402c7b`, `0x402c5b`, `0x42cd60`, `0x82e994`, `0x884cb4`, `0x4571ca`, `0x457520`, `0x835e10`, `0x835e0c`, `0x48fd60`, `0x48fda4`, `0x48fd21`, `0x48fd65`, `0x80450c`, `0x804510`, `0x835160`, `0x6c5ae1`, `0x6c5aa6`, `0x835154`, `0x86c3f8`, `0x63a950`, `0x63a9c1`, `0x63a910`, `0x63a97b`
Confidence: verified
Proof: one
Test: none

`0x402c5b` formats `0x82e994` (`%1.1f`) with the value of `0x42cd60`, which is the stored host speed (`0x884cb4`) times 0.1 (`core/input`). `0x457520` returns the pointer to `50` (`0x835e10`) when `0x639a00` returns a set flag and the copy of the detection record has 1 at `+0x18`, the processor class, and `100` (`0x835e0c`) otherwise.
`0x48fd21` pushes 10.0 and `0x48fd65` pushes 41.0 as doubles for `%f` (`0x835160`); both register with callback `0x490720` in category 5. `0x6c5aa6` loads the locale index and indexes the dwords at `0x86c3f8`: 2 for every locale except index 1 (`koKR`), which holds 1; `0x835154` is `%d`. `0x63a910` and `0x63a97b` read `+0x54` and `+0x50` of the matched row.

## Locale defaults

Three settings pick one of two literal texts by the locale index at `0xc0e080`. `AutoInteract` defaults to `1` for the Korean client (index 1) and `0` otherwise. `gxWindow` and `gxMaximize` default to `1` for index 4 (`zhCN`) and `0` otherwise. The locale names, in index order, are `enUS`, `koKR`, `frFR`, `deDE`, `zhCN`, `zhTW`, `esES` and `xxYY` (`0x8558a4`).

Addresses: `0x603368`, `0x603374`, `0x603390`, `0x63a72f`, `0x63a738`, `0x63a889`, `0x63a8e0`, `0x63a8b9`, `0xc0e080`, `0x8558a4`, `0x82e748`, `0x82e570`
Confidence: verified
Proof: one
Test: none

`0x603368` compares the index with 1 and `0x603374` branches to pick `0x82e748` (`1`) or `0x82e570` (`0`); the call is `0x603390`. `gxWindow` and `gxMaximize` take their text from a test of the same index against 4 (`0x63a738`, `0x63a8b9`) and register at `0x63a889` and `0x63a8e0`. The table of locale names has eight pointers at `0x8558a4`; the order is read from the strings.

## Names with no setting

The image holds no string `autoLootDefault`, `environmentDetail` or `groundEffectDensity`, so no setting is registered under those names. `WorldDetail` occurs only inside the names of the two script verbs `SetWorldDetail` and `GetWorldDetail` and the usage text of the first; there is no setting of that name, and the verbs write `frillDensity` and `smallCull` (`options#world-detail-verbs`).

Addresses: `0x8423e8`, `0x488dd0`, `0x488d70`
Confidence: verified
Proof: one
Test: none

The scan is a case-insensitive search of the whole image file. The three absent names give no hit; `WorldDetail` gives three, two as the name strings that the script function table points to and one at `0x8423f2` inside the usage string at `0x8423e8`. The registrations of the settings are the rows of `tables/cvars.tsv`, which carry no such name.
