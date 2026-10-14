# glue: the select ghost look

What the character-select screen changes for a ghost character: the pass, the glow value, the
light and the kit.

Bit `0x2000` of a roster record's flag word `+0xfc` marks a ghost. Exactly four places read it:
the light callback `0x472150`, the kit `0x4727f0`, the effect swap at the tail of `0x472950` and
`GetCharacterInfo`. A ghost selection swaps the full-screen glow pass for the death pass, pins the
light state's glow value, replaces every light of the three scene models with one fixed
directional light, and dresses the body with a ghost mesh, a tint and an alpha. The effect classes
are `ffxeffects`; the roster, the build and the scene are `select` and `selectscene`.

Roots: `0x472150`, `0x4727f0`, `0x472780`, `0x46fa60`, `0x46fa90`, `0x46fad0`, `0x472110`, `0x4732a0`

## Ghost flag readers

The flag word `+0xfc` has four readers of bit `0x2000`: `0x472194` (the light callback),
`0x472d3b` (the kit, in the build), `0x472fd9` (the effect swap, in the build) and `0x4733ff`
(`GetCharacterInfo`, whose eighth value is 1 for a ghost and nil otherwise). Its other tests are
bit `0x4` (`0x46b57e`), `0x400` and `0x800` (`0x472c15`, `0x472c25`) and `0x4000`
(`0x46b5b1`, `RenameCharacter` at `0x4735ab`). The only client write, `0x472722`, clears `0x4000`,
so the ghost bit is what the wire set.

Addresses: `0x472194`, `0x472d3b`, `0x472fd9`, `0x4733ff`, `0x4735ab`, `0x46b5b1`, `0x472722`, `0x5b446c`, `0x472300`, `0x473520`
Confidence: verified
Proof: one
Test: none

Census: every `+0xfc` memory operand in `0x466000` to `0x476000` (eight), and every `test` of
`ah`, `ch` or `dh` with `0x20` or `0x40` there (six); no `+0xfd` byte form exists and no
`and`, `cmp` or `test` with immediate `0x2000` lies in the band (control: 141 `+0xfc` operands
image-wide, 138 by register). The parser stores the word with one 4-byte read (`0x5b446c`);
`0x472300` copies it with the record.

## Ghost pass and glow

A selection that passes the index test installs the glue death pass `0xb41468` for a ghost, else
the glue glow pass `0xb414c4`, through `0x6cde60`, and stores `0.15` (`0x838570`) or `0.40`
(`0x838574`) in the light state's glow value. This runs for every selection, a cached model
included. A failed model lookup leaves before it. Showing the frame writes 0.30 and the glow
pass (`0x46fa60`); no other glue code writes the value. The pair is built by the same two
constructors as the world's pair.

Addresses: `0x472fba`, `0x472fde`, `0x472fe9`, `0x472fef`, `0x472ff7`, `0x473002`, `0x473007`, `0x838570`, `0x838574`, `0x8380b4`, `0x6d48b0`, `0x6cde60`, `0x6cc130`, `0x6cc690`, `0x46a71e`, `0x46a74d`, `0xb41468`, `0xb414c4`, `0x46fa60`
Confidence: verified
Proof: one
Test: none

`0x6d48b0` returns `0xce9b60`, so `[esi+0x110]` is the light state's glow value. The writers
among its 41 callers are `0x46fa6e` (`0x8380b4`, 0.30f) and `0x472fef`, `0x473007`; the readers
are `0x6cb0e2`, `0x6cb557` and `0x6cb9e1`. The light table builder `0x6d2260` also stores to it by absolute
address (`0x6d231f`, `0x6d263d`, `0x6d2789`); it is called from the world's light update, and no function of
the glue band calls it. Each float has one reference, its read. `0x6cc130`
and `0x6cc690` each have two call sites, `0x46a71e`, `0x46a74d` and the world's `0x481c63`,
`0x481c8d`; `0x6cde60` six. `0x472a6d jne 0x472fba` sends a built record to the swap.

## Death pass colour

The death pass packs `0x6cb930`'s alpha from the glow value as `(f32(glow·255 + 512) >> 14) & 0xff`
and ORs the colour `0x5393a8`, so a ghost select runs the death combine with `0x265393a8`:
alpha 38, red 83, green 147, blue 168. The living glow value 0.40 packs to alpha 102 and is the
glow pass's own weight, with no cross-fade because the glue has no player to feed it. The death
pass is the one `0x6cb7e0` builds, whose single call site `0x6cc832` is inside `0x6cc690`.

Addresses: `0x6cb930`, `0x6cb9e6`, `0x6cb9ec`, `0x6cb9f9`, `0x6cba08`, `0x6cba17`, `0x6cba1a`, `0x6cb7e0`, `0x6cc832`, `0x7ffe58`, `0x8029cc`
Confidence: verified
Proof: one
Test: none

`0x6cb9e6 fld [eax+0x110]`, `fmul 255.0`, `fadd 512.0` (`0x8029cc`) and `fstp` to f32 narrow once;
`0x6cba08 shr eax,0xe` takes the byte; `0x6cba17 shl ecx,0x18` and `0x6cba1a or ecx,0x5393a8` build
the dword. 0.15f gives `0x44099000`, byte 38; 0.40f gives `0x44198000`, byte 102. The death
class's pass vtable, slot 2, is stored only by `0x6cb7e0`.

## Death combine shader

The death pass's pixel program, read from the shipped `Shaders\Pixel\FFXDeath.bls` (the
`!!ARBfp1.0` text at file offset `0x421c`), computes `luma = sat(dot(screen + w·blur², (0.299,
0.587, 0.144)))`, then `rgb = primary.rgb · sat(4·luma·(1 − luma)) + luma` and alpha 1. Here
`screen` is the full scene texture, `blur` the quarter-size blur, `w` the packed alpha byte over
255 and `primary.rgb` the packed colour. The blue weight is 0.144, and the dot product and the
mix factor are both saturated.

Addresses: `0x6cb930`, `0x6cb7e0`
Confidence: verified
Proof: one
Test: none

The program is `MUL R0.xyz,R0,R0` on the blur, `MAD R0.xyz,R0,primary.w,R1` with the screen,
`DP3_SAT R0.x,R0,c[0].yzww`, `MUL R0.y,R0.x,R0` after `ADD R0.y,-R0.x,c[0].x`, `MUL_SAT` by 4 and a
`MAD` into `result.color`. The three other `FFXDeath` files carry no ARB text.

## Ghost sun colour

The ghost light callback reads `LightParams.dbc` row id 3, fixed in the code, and takes `LightIntBand`
ids `18·3 − 17 + k` at time key 0: id 37 for the diffuse colour (`CGLight+0x3c`) and id 38 for the
ambient (`+0x30`). Both shipped rows have one key, so the colour never changes: diffuse (94, 153,
198), ambient (26, 56, 85), specular 0. Row 3 is the `Light.dbc` death slot: column 11 holds it in
59 of 374 rows, and the world's ghost arm selects that column with `mov ecx,4`.

Addresses: `0x472150`, `0x4721d3`, `0x4721e2`, `0x4721ed`, `0x4721f8`, `0x472204`, `0x472233`, `0x472251`, `0x6d6ad0`, `0x6d62e0`, `0x7bbf20`, `0x71b4a0`, `0x5de9df`, `0x6d4620`, `0xce9d7c`, `0xce9d80`, `0xce9da4`, `0xce9da8`
Confidence: verified
Proof: one
Test: none

`0x4721d3 cmp [0xce9d80],3` and `0x4721e2 mov esi,[idmap+0xc]` take the row; `0x6d6ad0` returns
`idmap[18·id + sub − 17]` of the `LightIntBand` store; `0x6d62e0` evaluates it at key `xor edx,edx`
(0). `0x7bbf20` loads bytes 0, 1, 2 and pops the last first, so byte 2 is red: the dword is
`0xAARRGGBB`. The values are read from the shipped `LightIntBand.dbc`: id 37 one key, 0x5e99c6;
id 38 one key, 0x1a3855. `0x71b4a0` zeroes `+0x0c` to `+0x50`, so specular stays 0.

## Ghost sun direction

The ghost light is a directional light with the raw direction (0, 0, −1) (`0x472280`). The stored
vector is a from-light direction, and both consumers negate it once: the order-2 harmonic
accumulator `0x71bce0` (`fchs` at `0x71be7c`, `0x71be81`, `0x71be86`) on the shader lane every
select mesh takes, and the fixed-function commit `0x59c820` (four `fchs` for a directional
position). The to-light vector is (0, 0, +1), directly overhead. An authored light is negated
before it is stored, so the two conventions differ by one negation.

Addresses: `0x472280`, `0x472287`, `0x71b6a0`, `0x71bf90`, `0x71bfa6`, `0x71bce0`, `0x71be7c`, `0x71be81`, `0x71be86`, `0x59c820`, `0x59c8fd`, `0x59c912`, `0x70baf0`, `0x71c4e0`, `0x7076be`
Confidence: verified
Proof: one
Test: none

`0x71b6a0` copies the three floats to `CGLight+0x24` and divides by the length when it exceeds
the epsilon `0x8029d4`: no `fchs`. `0x71b620(0)` makes the light type 0, so `0x71bfa6 jne` takes
`0x71c0d3`, which calls the ambient, harmonic and specular accumulators. `0x59c8fd cmp [esi-0x40],1.0`
passes a point as is and negates all four components otherwise. The matrix `0x71bce0` rotates
by is the scene's `+0x9c`, the live view matrix (`0x7076be` copies it each frame), which the normal
shares through `[model+0xfc]`.

## Fill callback installs

`0x472150` is installed as the light-fill callback of three models: the background scene
(`0x472144`, from `0x472110`), the character body (`0x472bac`) and the pet (`0x472ebd`). For a
living selection it returns at `0x4722b2` and changes nothing. For a ghost it clears the model's
light collector (`0x71bc30` zeroes 436 bytes), so every authored light is discarded, and adds the
one fixed directional light. A callback that `0x472110` installs replaces the scene model's
fog callback `0x76d680`, so the select background has no fog.

Addresses: `0x472110`, `0x472144`, `0x472bac`, `0x472ebd`, `0x472194`, `0x4721ce`, `0x4722b2`, `0x7134b0`, `0x71bc30`, `0x76d680`, `0x76cd30`
Confidence: verified
Proof: one
Test: none

`0x7134b0` stores the function at `[model+0x3bc]` and its argument at `+0x3c0`. `0x71bc30` opens
`mov ecx,0x6d; rep stosd` with `eax` 0. `0x472110` calls the frame's `SetModel` slot `+0x94`,
which installs `0x76d680` through `0x76cd30`, then installs `0x472150` over it for the scene model.

## Ghost kit

A ghost body is dressed from `SpellVisualKit` row 989: of nine offered attachment ids only id
0x13, from column `+0x14` holding effect 717, is filled; the other eight columns are −1 and
`0x472780` rejects them. Effect 717 is the model `Spells\Ghost_state.mdx`. The kit's four (kind,
value) pairs give kind 1, value `0x4b0cb9fd`: a tint of (140, 185, 253) over 255; and kind 14, value
0.5: an alpha of 0.5. Both are plain stores on the body (`+0x184` to `+0x18c`, `+0x180`), applied
at once.

Addresses: `0x4727f0`, `0x4727f6`, `0x47280f`, `0x472780`, `0x472797`, `0x472799`, `0x4728ec`, `0x4728f3`, `0x472939`, `0x4728a4`, `0x4728cb`, `0x710cf0`, `0x710cb0`, `0x472d49`, `0xc0d750`, `0xc0d74c`
Confidence: verified
Proof: one
Test: `glue::char_customize_clamp_difftest`, `glue::char_customize_color_difftest`

`0x4727f6 cmp [0xc0d750],0x3dd` and `0x47280f` read index entry 989. The tint is `_ftol` of the value
(0x8cb9fd) with `or eax,0xff000000`: the alpha byte is the top byte, so byte 2 is red. The
shipped row has `+0x14` 717, `+0x3c` to `+0x48` (1, 14, −1, −1) and `+0x4c` to `+0x58`
(`0x4b0cb9fd`, 0.5, 0, 0); `SpellVisualEffectName` 717 is `Ghost_state` and the file above. `0x710cb0`
is one store and `0x710cf0` three; `0x4727f0` has one call site.
`char_customize_clamp_difftest` runs `0x4728a4` to `0x47293e`; `char_customize_color_difftest` runs `0x4728ec` to
`0x472939`.

## Ghost kit scope

The tint and alpha go to the body only. The ghost mesh is a child of the body and takes them
through the per-frame combine `0x714260`, so it is as translucent and tinted as the body. The pet
hangs off the scene, not the body, and gets neither. Nothing clears the body's `+0x180` and
`+0x184`, and a built record never re-enters the kit, so a cached ghost body stays tinted and a
cached living one never is. The ghost changes nothing else: animation 0, no camera call, facing 0,
the same equipment test, and no ghost term in the pet gate.

Addresses: `0x714260`, `0x70ea00`, `0x4722c0`, `0x4722e0`, `0x713020`, `0x472bda`, `0x47303f`, `0x472c0f`
Confidence: verified
Proof: one
Test: none

`0x714260` stores `+0x1a0` as the passed multiplier times `+0x184`, `+0x1ac` as the passed
addend plus `+0x190` and `+0x19c` as the passed fade times `+0x180`; `0x714152` to `0x714158`
pass the parent's `+0x1a0`, `+0x1ac` and `+0x19c`. `0x713020` writes `+0x1cc`, `+0x1d0` and the child
links, nothing of `+0x180` to `+0x198`; the loader `0x70ea00` sets the defaults (alpha 1, tint 1,
emissive 0). `0x47303f push ebx` is unconditional.
The equipment loop reads `+0xfc` once (`0x472c0f`) for `0x400` and `0x800`.
