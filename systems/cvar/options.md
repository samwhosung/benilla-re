# cvar: options

The video options file registers 31 settings of the world's drawing, each with a callback that checks the
new text, writes a line to the console and stores the value where the drawing code reads it. A callback
that returns 0 keeps the setting from changing (`registry#setting-a-value`). The file has no source path:
it is placed by its run of static initializers, and the console's device start-up calls its one entry.

Roots: `0x688470`, `0x688400`, `0x688420`, `0x688430`, `0x688450`, `0x688460`, `0x688910`, `0x688920`, `0x688960`, `0x688a10`, `0x688a70`, `0x688ab0`, `0x688b10`, `0x688b70`, `0x688bd0`, `0x688c10`, `0x688c50`, `0x688c90`, `0x688ce0`, `0x688de0`, `0x688e20`, `0x688e90`, `0x688f00`, `0x688f40`, `0x688fb0`, `0x689000`, `0x689050`, `0x689090`, `0x6892e0`, `0x689330`, `0x689400`, `0x689450`, `0x6894e0`, `0x689510`

## Registration

`0x688470` registers the 31 settings in category 1 with flags 1, script-visible, no callback argument and the help line
`Video option: ...` or a short description; it returns nothing and keeps each record in a global. The order is `lod`, `fullAlpha`,
`doodadAnim`, `mapShadows`, `lodDist`, `SmallCull`, `DistCull`, `MaxLights`, `shadowLevel`, `texLodBias`, `trilinear`, `frillDensity`, `farclip`,
`nearclip`, `specular`, `mapObjOverbright`, `mapObjLightLOD`, `pixelShaders`, `particleDensity`, `unitDrawDist`, `waterLOD`, `baseMip`, `anisotropic`,
`textureLodDist`, `horizonfarclip`, `showfootprints`, `bspcache`, `triangleStrips`, `footstepBias`, `occlusion` and `spellEffectLevel`. The defaults are the rows of
`tables/cvars.tsv` for the sites `0x68848c` to `0x688900`. `ConsoleDeviceInitialize` calls it once, at `0x63a6fb`.

Addresses: `0x688470`, `0x68848c`, `0x688900`, `0x63a6fb`, `0x63a230`, `0xc7f314`, `0xc7f2a0`, `0xc7f354`, `0xc7f318`, `0xc7f344`, `0xc7f330`, `0xc7f320`, `0xc7f370`, `0xc7f350`, `0xc7f2f8`, `0xc7f2bc`, `0xc7f2f4`, `0xc7f304`, `0xc7f348`, `0xc7f2cc`, `0xc7f324`, `0xc7f310`, `0xc7f360`, `0xc7f368`, `0xc7f298`, `0xc7f2c0`, `0xc7f2f0`, `0xc7f2e4`, `0xc7f2d8`, `0xc7f34c`, `0xc7f364`, `0xc7f2ec`, `0xc7f308`, `0xc7f300`, `0xc7f31c`, `0xc7f2a4`, `0x688910`
Confidence: verified
Proof: one
Test: none

Each registration pushes the flags, the default text, the callback, the category and the two trailing arguments, puts the help line in edx and the name in ecx, then calls `0x63db90`; the first is `lod` at `0x68848c` and the last
`spellEffectLevel` at `0x688900`. The records go to the globals of the address list, in the order above, from `0xc7f314` (`lod`) to `0xc7f2a4` (`spellEffectLevel`).

## Switch settings

Eleven settings are switches. A switch parses the text with `SStrToInt` and writes one line to the console. The world
flag word `0xc7b2a4` holds nine of them (`terrain/world#world-flags`). `fullAlpha` and `doodadAnim` keep their bit
inverted: the bit is set while the option is off, and the line is written only when the bit changes.

| setting | callback | sets or clears | line |
|---|---|---|---|
| `lod` | `0x688920` | bit 0x4 | `Terrain LOD enabled.` or `disabled.` |
| `fullAlpha` | `0x688960` | bit 0x4000, set for 0 | `Full alpha on doodads enabled.` or `disabled.` |
| `doodadAnim` | `0x688a10` | bit 0x8000, set for 0; calls `0x6953d0` | `Doodad animation enabled.` or `disabled.` |
| `mapShadows` | `0x688a70` | bit 0x40 | `Terrain shadows enabled.` or `disabled.` |
| `trilinear` | `0x688ce0` | bit 0x800000; for any nonzero value with bit 31 of the word clear stores 4 through `0x449ac0` (`0x688cee`, `0x688d0d`) | `Trilinear filtering enabled upon restart.` or `disabled upon restart.` |
| `specular` | `0x688e20` | bit 0x8000000 | `Specular enabled`, `Specular disabled.` or the unsupported line |
| `mapObjOverbright` | `0x688e90` | byte `0xca809c` | `MapObjOverbright enabled`, `disabled.` or the unsupported line |
| `pixelShaders` | `0x688f40` | bit 0x10000000 | `Pixel shaders enabled.`, `disabled.` or the unsupported line |
| `showfootprints` | `0x6892e0` | bit 0x400 | `Showing foot prints.` or `Hiding foot prints.` |
| `triangleStrips` | `0x689400` | bit 0x10000 | `Triangle strips enabled on restart.` or `disabled on restart.` |
| `occlusion` | `0x6894e0` | nothing | `Using hardware occlusion test.` or `Disabled hardware occlusion test.` |

Addresses: `0x688920`, `0x688960`, `0x688a10`, `0x688a70`, `0x688ce0`, `0x688cee`, `0x688e20`, `0x688e90`, `0x688f40`, `0x6892e0`, `0x689400`, `0x6894e0`, `0x6953d0`, `0x449ac0`, `0x58a230`, `0xc7b2a4`, `0xca809c`, `0x688d0d`, `0x688e38`, `0x688e43`, `0x688ea8`, `0x688eb3`, `0x688f58`, `0x688f63`, `0x688975`, `0x688a25`, `0x63a6fb`, `0x402840`, `0x66f6cb`
Confidence: verified
Proof: one
Test: none

Each parses with `0x64ac60` and writes the line with `0x63cb50`; the flag writes are `or` and `and` on `0xc7b2a4`. `0x688975` tests bit 0x4000 (`test ah,0x40`) and `0x688a25` bit 0x8000
(`test ah,ah`): a nonzero value clears the bit when it is set, a zero value sets it when it is clear. `0x688ce0` loads the flags and tests the sign at `0x688d0d`. `specular`,
`mapObjOverbright` and `pixelShaders` ask the device capabilities (`0x58a230`) and accept the on value only when the word at `+0x98` is not negative
(`0x688e38` to `0x688e43`, `0x688ea8` to `0x688eb3`, `0x688f58` to `0x688f63`); otherwise they write the unsupported line and return 0. Every other path returns 1. The registrations run in `ConsoleDeviceInitialize` (`0x63a6fb`), before the start handler `0x402840` of the interactive context (`core/loop`) reaches the world initializer, whose `or` of `0x7100b73` into the word (`0x66f6cb`) sets bit 0x40 again; a stored `mapShadows` of 0 does not hold after start-up.

## Float range settings

Seven settings take a number in a range. A value outside it writes a line and returns 0, so the setting keeps its old value; the bounds are inclusive.
The terrain callbacks are described in `terrain/world#distance-settings` and `terrain/render`.

| setting | callback | accepts | stores or calls |
|---|---|---|---|
| `lodDist` | `0x688ab0` | 50 to 250 (outside: line, then 77) | `0x672680` |
| `SmallCull` | `0x688b10` | 0.001 to 2 | float `0x868620` |
| `DistCull` | `0x688b70` | 1 to 888.888916 | float `0x868624` |
| `texLodBias` | `0x688c90` | -1 to 1 | `0x672640`, a no-op |
| `particleDensity` | `0x688fb0` | 0.25 to 1 | `0x7adfb0` |
| `unitDrawDist` | `0x689000` | 100 to 300 | float `0x86796c` |
| `footstepBias` | `0x689450` | 0 to 1 | nothing |

Addresses: `0x688ab0`, `0x688b10`, `0x688b70`, `0x688c90`, `0x688fb0`, `0x689000`, `0x689450`, `0x672680`, `0x672640`, `0x7adfb0`, `0x868620`, `0x868624`, `0x86796c`, `0x688ac8`, `0x688adb`, `0x688af4`, `0x688b28`, `0x688b3b`, `0x688b89`, `0x688b96`, `0x688c9f`, `0x688caf`, `0x6894a0`
Confidence: verified
Proof: one
Test: none

The float settings compare in both directions: `fcomp` then `test ah,5; jnp` rejects a value below the lower bound (`0x688ac8`, `0x688b28`, `0x688c9f`), and `fcomp` then `test ah,0x41; je` rejects one above the upper
(`0x688adb`, `0x688b3b`, `0x688caf`). `SmallCull` reads its value with `sscanf` and compares with the double 0.001 and the bound 2.0; `DistCull` writes its error line with the double `888.888916` (`0x688b89`, `0x688b96`).
`lodDist`'s error line is followed by the value 77.0 (`0x429a0000`, `0x688af4`) and the call. `footstepBias` accepts 0 to 1 and prints `Footstep bias set to %f` (`0x6894a0`) but stores nothing. Nothing reads the globals `0x868620`, `0x868624` and `0x86796c`.

## Integer range settings

Seven settings take a whole number. A number outside the range writes a line and returns 0, except `shadowLevel`, which only reports, and `spellEffectLevel`, which clamps.

| setting | callback | accepts | stores or calls |
|---|---|---|---|
| `MaxLights` | `0x688bd0` | 1 to 4 | `0xc5a45c` |
| `shadowLevel` | `0x688c10` | 1 or less | nothing |
| `frillDensity` | `0x688de0` | 1 to 256 | `0x6725a0` |
| `mapObjLightLOD` | `0x688f00` | 0 to 2 | `0xca7ecc` |
| `baseMip` | `0x689090` | 0 or 1 | `0x589bf0` |
| `waterLOD` | `0x689050` | 0 only | zeroes `0xc89db8` |
| `spellEffectLevel` | `0x689510` | any, clamped to 0 to 2 | `0xc5a470`, `0x7adfb0` |

Addresses: `0x688bd0`, `0x688c10`, `0x688de0`, `0x688f00`, `0x689090`, `0x689050`, `0x689510`, `0x6725a0`, `0x589bf0`, `0x589c50`, `0xc5a45c`, `0xca7ecc`, `0xc89db8`, `0xc5a470`, `0x7adfb0`, `0x688bdc`, `0x688be1`, `0x688c1e`, `0x68952a`, `0x689535`, `0x68955b`, `0x689057`, `0x66f6f8`
Confidence: verified
Proof: one
Test: none

`MaxLights` compares the integer with 1 and 4 (`0x688bdc`, `0x688be1`); `shadowLevel` compares with 1 (`0x688c1e`) and rejects above it, but it stores nothing, since the value is read from the record. `baseMip` returns 1 without a call when the number
equals the device's current level (`0x589c50`). `spellEffectLevel` clamps (`0x68952a`, `0x689535`), stores the level (`0x68955b`) and passes 0.33, 0.66 or 1.0 (`0x3ea8f5c3`, `0x3f28f5c3`, `0x3f800000`) to `0x7adfb0`; the world
initializer copies the record's integer to the same global (`0x66f6f8`). `waterLOD` zeroes `0xc89db8` first (`0x689057`) and rejects any nonzero value. `0xc5a45c` is read nowhere.

## Distance and texture settings

Five settings belong to terrain's drawing and are described there: `farclip`, `nearclip`, `textureLodDist` and `horizonfarclip`
(`terrain/world#distance-settings`) and `anisotropic` (`terrain/render`). Their callbacks are `0x688d40`, `0x688d90`, `0x6891f0`, `0x689250` and
`0x689110`. `bspcache` creates or frees the BSP leaf cache. The `farclip` record has two readers beside the camera and the streaming window: the world initializer copies its value into the fog end, and the doodad draw ends its fade band at the world far clip less 50 when that is below 200.

Addresses: `0x688d40`, `0x688d90`, `0x6891f0`, `0x689250`, `0x689110`, `0x689330`, `0x693ca0`, `0x693ce0`, `0x693d20`, `0xca03e4`, `0x688868`, `0x689399`, `0x6893da`, `0x66fb82`, `0x66f890`, `0x66f899`, `0xc62964`, `0x683fb9`, `0x683fc6`, `0xc7b498`
Confidence: verified
Proof: one
Test: none

`bspcache` with a nonzero value on an existing cache writes `Enabling BSP node cache (already enabled, so clearing content.)` and clears it (`0x693d20`); with none it writes `Enabling BSP node cache (first time - starting up)`, allocates 0x919000 bytes
(`0x689379`), constructs it with `0x693ca0` (`0x689389`) and stores it at `0xca03e4`. Zero destroys it with `0x693ce0` and frees it, or writes `BSP node cache already disabled.`. The callback always returns 1. The registration at `0x688868` has the default 1 and applies it through this callback, so the cache is built at start-up; the stores of 0 to `0xca03e4` are the callback's own (`0x689399`, `0x6893da`) and the world shutdown's (`0x66fb82`). `0x66f890` loads the value from the `farclip` record and `0x66f899` stores it in the fog end `0xc62964`; `0x683fb9` compares the far clip `0xc7b498` with 200 and `0x683fc6` subtracts 50 from it for the fade band end, which is otherwise 150.

## Mip verbs

`SetTerrainMip(v)` sets `shadowLevel` to `1 - trunc(v)` and `GetTerrainMip()` returns `1 - shadowLevel`. `SetBaseMip(v)` sets `baseMip` to `trunc(1 - v)` and `GetBaseMip()` returns `1 - baseMip`. Neither setter checks the range; the setting's own callback does (`shadowLevel` at most 1, `baseMip` 0 or 1). A non-number raises the usage text. Both setters return no values.

Addresses: `0x488fb0`, `0x488fbd`, `0x488fc5`, `0x488fe0`, `0x489018`, `0x489022`, `0x489030`, `0x48903d`, `0x489050`, `0x489280`, `0x48928d`, `0x489290`, `0x4892b0`, `0x4892e2`, `0x4892f5`, `0x4892fb`, `0x489322`
Confidence: verified
Proof: one
Test: none

`0x488fbd` loads the integer of the `shadowLevel` record at `+0x28` and `0x488fc5` subtracts it from 1.0 (the constant at `0x8015b8` is the double 1.0). `0x489018` truncates the number, `0x489022` subtracts it from 1, `0x489030` formats it with `%d` and `0x489050` sets the record's text with the apply flag. `0x48928d` loads the float of the `baseMip` record at `+0x24` and `0x489290` subtracts it from 1.0. `0x4892f5` subtracts the argument from 1.0, `0x4892fb` truncates the result and `0x489322` sets the record's text.

## Texture filter opt-in

`trilinear` and `anisotropic` store one process-wide filter mode, 4 or 5 (the static value is 3), and a texture takes it only when its creation passes the force flag. Of the 42 calls of `TextureCreate`, 22 pass the flag and follow the setting: the character, world frame, taxi map, object, liquid, drift cloud, map, name, text, shadow, lightning, model, disc and loading bar textures. The other 20 pass 0 and keep the mode they pack themselves: the loading art, minimap, mists, ripples, footprints, glare and one interface texture, and rain and snow at mode 4. `trilinear` 0 stores no mode, so a mode of 4 or 5 stored before stays.

Addresses: `0x406891`, `0x4781f3`, `0x4782bb`, `0x479601`, `0x47987e`, `0x479917`, `0x479c09`, `0x47a025`, `0x481c21`, `0x4dbbb2`, `0x5f7b92`, `0x6147be`, `0x68abab`, `0x68e6f1`, `0x6c4c77`, `0x6c759e`, `0x6c8500`, `0x6d80c1`, `0x6ecd8b`, `0x71da72`, `0x76d03d`, `0x7e5b62`, `0x406eba`, `0x40750f`, `0x4ed862`, `0x4ee5e4`, `0x6747ac`, `0x677581`, `0x67a74d`, `0x68f90f`, `0x699d11`, `0x6da05a`, `0x6da764`, `0x770281`, `0x7e5052`, `0x67477e`, `0x677560`, `0x688d1f`, `0x688d33`, `0x68919f`, `0x449d90`
Confidence: verified
Proof: one
Test: none

The force argument is the second stack argument of `0x449d90`. The first 22 sites listed push 1. The next 13 push 0 or a register cleared by `xor`, and so do seven more: `0x406ef1`, `0x407575`, `0x4ed88b`, `0x4ed8b4`, `0x4ee784`, `0x4ee924` and `0x6747e8`. `0x67477e` and `0x677560` push the mode 4 into the flags packer of rain and snow. `0x688d1f` takes `trilinear` 0 to `0x688d33` without a store, and `0x68919f` stores 5 when `anisotropic` is above 1. The flags word `0x449ae0` applies the stored mode only for a forced texture (`gx/textures#texture-flags-word`).

## Settings nothing reads

Some video options change nothing the game draws. `fullAlpha` and `doodadAnim` set bits 0x4000 and 0x8000 of the world flag word `0xc7b2a4`, and no instruction tests those bits outside their own setters; `doodadAnim` also calls `0x6953d0`, a bare return. `texLodBias` calls `0x672640`, also a bare return. `SmallCull`, `DistCull`, `unitDrawDist` and `MaxLights` store into globals that no instruction reads.
`triangleStrips` is not one of them: the map load copies its bit 0x10000 into the strip switch `0xc96304` (`terrain/groupdraw`).

Addresses: `0xc7b2a4`, `0x688975`, `0x688a25`, `0x6953d0`, `0x672640`, `0x868620`, `0x868624`, `0x86796c`, `0xc5a45c`, `0x69437b`, `0x694391`, `0xc96304`, `0x5fc020`, `0x69a3ee`, `0x6941f0`
Confidence: verified
Proof: one
Test: none

131 instructions name `0xc7b2a4`. The loads are followed by tests of other bits or by the toggling functions; the only tests of bits 0x4000 and 0x8000 are `0x688975` and `0x688a25` in the setters. The control is bit 0x400, which `0x5fc020` and `0x69a3ee` test. `0x69437b` loads the third byte of the word, `0x694391` stores it masked with 1 into `0xc96304` while `0x6941f0` runs, and the terrain code reads that byte.
The four stores are the only references to their globals (`0x868620`, `0x868624`, `0x86796c`, `0xc5a45c`) in the image, by a scan of every dword and every operand; the control is the lodDist global, which `terrain` reads.

## World detail verbs

`SetWorldDetail(n)` sets two settings from the level `n`, 0 to 2: `frillDensity` to 16, 32 or 48 and `smallCull` to 0.07, 0.04 or 0.01. `GetWorldDetail()` reads only `smallCull` and answers the highest level `i` for which the setting is at most the table value, or 2 when it is above 0.07 or not a number. A level outside 0 to 2 raises `value must be in the range 0, 2`; a non-number raises the usage text. The registered pair, `frillDensity` 16 and `SmallCull` 0.04, is level 0's frill with level 1's cull, so a fresh client reads back level 1 while its frill is the level 0 value.

Addresses: `0x488d70`, `0x488d77`, `0x488d90`, `0x488d9e`, `0x488dd0`, `0x488e0a`, `0x488e11`, `0x488e19`, `0x488e1e`, `0x488e39`, `0x488e51`, `0x488e56`, `0x488e76`, `0x488e8e`, `0x488e9b`, `0x804518`, `0x804524`, `0x68854a`, `0x68862e`
Confidence: verified
Proof: one
Test: none

`0x488e0a` truncates the number, `0x488e11` and `0x488e19` reject a value below 0 or above 2 at `0x488e9b`. `0x488e1e` takes the frill from the dwords at `0x804518` (16, 32, 48), `0x488e39` finds `frillDensity` and `0x488e51` sets its text with the apply flag; `0x488e56` takes the cull from the floats at `0x804524`, which `0x488e76` finds as `smallCull` (the registered name `SmallCull` matches because the lookup folds case) and `0x488e8e` sets. `0x488d77` finds `smallCull`; `0x488d90` compares the value with each table entry in turn, `0x488d9e` keeps the index when the value is at most the entry (`test ah,0x41; jp` skips a greater or unordered compare, so a NaN keeps the start value 2), and no step stops the loop. The registrations `0x68854a` and `0x68862e` give the defaults 0.04 and 16.

## Unregistered callback

`0x688c50` is the callback of an alpha mip setting: it accepts an integer of 1 or less with `Alpha mip level changed upon restart.` and rejects
the rest with `Alpha mip level must be in range 0 - 1.`. No registration names it.

Addresses: `0x688c50`
Confidence: verified
Proof: one
Test: none

The census over every byte of `.text` for `call`, `jmp` and conditional rel32 targets, and over every dword of the image, finds no reference to `0x688c50`; the same scan finds the callers of `0x688c10`, the sibling the registration
does use, as its control (`unreached`).
