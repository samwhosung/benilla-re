# glue: the select scene

How the character-select screen places its models: who selects a character, the scene's two
attachments, the facing, the pet and the animation.

The select screen draws one background scene, the model `+0x318` of the `ModelFFX` frame
`0xb42174`, and hangs the selected character's body on its attachment 0 and the pet on its
attachment 1. Every selection runs `0x472740`, then `0x472950`, which builds a record's models once
and re-attaches them on every later selection. The scene model comes from the GlueXML's
`SetCharSelectBackground`, one of six `UI_<Race>` models. The character's own composition, the
glow pass and the ghost look are `select` and `selectghost`; the pet's scale and the roster are
`select`.

Roots: `0x472740`, `0x472950`, `0x4730e0`, `0x473090`, `0x473470`, `0x473650`, `0x473670`, `0x4722c0`, `0x4722e0`

## Selection callers

`SetSelectedCharacter` (`0x472740`) compares nothing with the previous selection: every call clamps,
stores and builds. It has four callers, all calls. Two pass −1 after the roster count is zeroed, so
`0x472950` leaves at its count test, above the facing reset. The other two, the restore of
`lastCharacterIndex` and the Lua `SelectCharacter`, reach the reset. The shipped GlueXML calls the
binding from one wrapper only, and its row click and double click skip the call when the row is
the selected one, so a click on that row keeps the dragged facing.

Addresses: `0x472740`, `0x47252e`, `0x472563`, `0x4725c6`, `0x4734b7`, `0x472965`, `0x472971`, `0x472a61`, `0x473470`
Confidence: verified
Proof: one
Test: none

`0x472740` is 0x472740 to 0x472774; the rel32 census finds `0x47252e`, `0x472563` (in `0x4724d0`),
`0x4725c6` (in `0x472580`) and `0x4734b7`, no jump callers, and no stored dword holds the address
(control: the same scan finds one caller of `0x472950`, four of `0x472580`). The two −1 legs run
with the count at 0 (`0x47250c` zeroes it before the enumeration, `0x4725af` in the teardown), so
`0x472971 jge` exits. In `CharacterSelect.lua` the
binding has one live call, in `CharacterSelect_SelectCharacter`. Its other callers (the arrow keys, which
test only the roster size, `SELECT_FIRST_CHARACTER` and `UpdateCharacterList`) make no already-selected test;
`CharacterSelectButton_OnClick` and `_OnDoubleClick` test `id ~= CharacterSelect.selectedIndex`.
`selectedIndex` is the 1-based echo of event 8.

## Facing reset

The select facing `0xb4217c`, radians, is zeroed by every selection that passes the index test:
`0x472a61` stores 0 before the built test `0x472a6d`, and `0x47303f` re-applies `0x4730e0(0)` on
both legs when the scene model exists. `0x4730e0` stores nothing until the build counter
`0xb42178` is nonzero, a counter only the first build of a record raises and nothing clears. The
create screen keeps its facing across rebuilds: `0x470b70` re-reads `0xb420f4` and re-applies it
(`0x470c33`, `0x470c39`), and only `0x470441` zeroes it. The two cells are never shared.

Addresses: `0x472a61`, `0x472a6d`, `0x47303f`, `0x4730e0`, `0x4730ed`, `0x4730f6`, `0xb4217c`, `0xb42178`, `0x472fb2`, `0x470b70`, `0x470c33`, `0x470c39`, `0x470441`, `0xb420f4`, `0x470a50`
Confidence: verified
Proof: one
Test: none

The writers of `0xb4217c` are `0x472a61` and `0x4730f6`, of `0xb420f4` `0x470441` and `0x470a59`;
a text census over every addressing form and a scan of `.rdata` and `.data` for the two addresses
as stored dwords find no other (control: the neighbours `0xb42174` and `0xb42140` hit). `0xb42178`
has two references, the `inc` at `0x472fb2`, on the freshly built leg only, and the read at
`0x4730e6`. Both cells and the counter lie in the zero-filled tail of `.data`. `0x470a50` stores
unconditionally and gates only the model apply.

## Facing rotation

`0x4730e0` rebuilds the body's local matrix from identity on every call: translation (0, 0, 0),
rotation by the angle about (0, 0, 1), scale 1. The angle reaches `fsincos` raw, with no wrap and
no clamp, and the matrix is row-vector form with rows (c, s, 0), (−s, c, 0), (0, 0, 1): a
counter-clockwise turn by +θ seen from +Z. The Lua getters multiply by 180/π and the setters by π/180,
so the cells hold radians and the scripts see degrees. The pet's matrix is never touched.

Addresses: `0x4730e0`, `0x473179`, `0x710650`, `0x7bdd60`, `0x7bdb00`, `0x7bdb67`, `0x473650`, `0x473670`, `0x471910`, `0x471930`, `0x803a78`, `0x7ffaac`, `0x472f94`, `0x710620`
Confidence: verified
Proof: one
Test: `glue::lua_deg2rad_difftest`, `glue::lua_rad2deg_difftest`

`0x473179` calls `0x710650(model, pos, angle, axis, scale)` on `[[record+0x110]+0x38]`, which resets
`+0xbc` to identity, then translates (`0x7bdc40`), rotates (`0x7bdd60` to `0x7bdb00`) and scales.
`0x7bdb67 fld` of the angle feeds `0x7bdb6a fsincos`; the matrix entries are stored at `0x7bdbce` to
`0x7bdc2e`, with the axis normalisation skipped by the caller's flag. `0x710650` has three callers:
`0x470abc`, `0x473179` and the widget's own `0x76d235`. The only other local-matrix writer in the
screen is `0x472f94`, the pet's scale. The dereference of `+0x110` has no null test (`0x47311e` reads `[ecx+0x38]` with `ecx` the loaded
`+0x110`), so the call faults when the build counter `0xb42178` is nonzero and the selected record's
build left at the failed `CreatureModelData` lookup (`0x472a8a`), which leaves `+0x110` zero, and a
`SetCharSelectFacing` call (`0x4736a9`) follows.
`lua_rad2deg_difftest` runs the getters `0x471910` and `0x473650` up to `lua_pushnumber`; `lua_deg2rad_difftest`
runs the setters `0x471930` and `0x473670` from `lua_tonumber` to the store. `0x4730e0` and the matrix are not run.

## Facing drive constants

The shipped select script drags with `CHARACTER_ROTATION_CONSTANT` 0.6 degrees per UI unit, drag
right positive, adding the cursor change to the current facing each update while the button is
held; the rotate buttons add or remove `CHARACTER_FACING_INCREMENT` 2 degrees a frame while
pushed. The drag surface is the whole screen. The create script runs the same code and calls
`SetCharacterCreateFacing(-15)` on show and on every race click, never on a sex or class change.
`GetCursorPosition` returns its value times 1024 times the aspect times 0.75; if the device value
runs from 0 to 1, a unit is 1/768 of the screen height and 0.6 degrees a unit is 0.6 degrees a
pixel at 768 lines only.

Addresses: `0x46dad0`, `0x471930`, `0x473670`, `0x41ad10`, `0x41ad70`, `0x41ae40`
Confidence: inferred
Proof: none
Test: `glue::lua_model_scale_difftest`

`CharacterSelect.lua` defines the constant at line 4 and the update at lines 400 to 418; the
handlers sit on `CharacterSelectUI`, a `setAllPoints`, `enableMouse` frame. `0x46dad0` multiplies
`0x41ad70`'s `[0x832a4c]` (set by `0x41ad10` to the aspect times 0.75) by 1024 (`0x7ffd68`) and the
device value `[reg+0x1118]`, then divides out `[0x832a44]` (`0x41ae40`). `SetBackgroundModel`
in `GlueParent.lua` maps Gnome to Dwarf, Troll and no race to Orc and BloodElf to NightElf, so six scenes
serve every race.
`lua_model_scale_difftest` runs the arithmetic of `GetCursorPosition` from `0x46db05`.

## Scene attachments

The body and the pet are siblings under the scene model, on attachments 0 and 1. `0x712f70` stores
the attachment id at `+0x1d0`, the parent at `+0x1cc` and the attachment-array index, found through
the parent's lookup table, at `+0x1d4`. Each frame `0x714000` composes the child's matrix as
`child+0xbc` times the parent's bone matrix with the attachment position added to its translation;
`0x71439b` runs a full 4×4 multiply, so the bone's rotation and scale reach the child, not only its
position. The six race scenes each carry exactly two attachments and a lookup of [0, 1]; the main
menu and race-select models carry none.

Addresses: `0x712f70`, `0x714000`, `0x7140aa`, `0x7140d0`, `0x7140d8`, `0x714158`, `0x714260`, `0x71439b`, `0x74a7c0`, `0x710310`, `0x473039`, `0x47306e`, `0x473013`
Confidence: verified
Proof: one
Test: none

Attachment records are 0x30 bytes (`lea eax,[eax+eax*2]; shl eax,4` at `0x7140d0`), `{id, bone u16
at +4, position at +8}`; the bone matrix, 0x40 bytes, is `[parent+0x94] + bone·0x40`. The copy of
the parent's matrix into the child (`0x714198`) is reached only when this leg is not taken: no
model data, an unanimated parent or an index of `0xffff`. The count, lookup and positions are
measured from the six `UI_<Race>.m2` of the install (MD20 version 256).

## Per-race scene term

No race or sex enters any facing path of the exe. The race term is in the scene file: the attachment
bone of `UI_Human` carries a constant rotation of −16.5 degrees about +Z (keys at 0 and 33 on global
sequence 1, quaternion (0, 0, −0.14349, 0.98965)) and a uniform scale of 0.97, as does the pet's bone 5;
`UI_Tauren`'s bone scales by 1.04 and the pet's by 0.7324; the other four scenes' bones are identity.
The authored camera sits 18.181 degrees off the
stand spot's +X axis in Human and within 1.6 degrees of it in the rest, so a client that composes
the attachment's position only leaves Human 18 degrees turned.

| scene | camera azimuth | camera elevation | fov |
|---|---|---|---|
| `UI_Human` | −18.181° | 12.829° | 80° |
| `UI_Orc` | +0.632° | 2.877° | 65° |
| `UI_Dwarf` | +0.347° | 3.818° | 65° |
| `UI_NightElf` | −1.521° | 11.459° | 60° |
| `UI_Tauren` | +0.112° | 7.553° | 65° |
| `UI_Scourge` | 0.000° | 6.429° | 65° |

Addresses: `0x714000`, `0x714961`, `0x706a50`, `0x76cec0`, `0x707350`
Confidence: verified
Proof: one
Test: none

Azimuth and elevation are the first camera's eye (`posBase`) seen from attachment 0's position.
The bones are roots whose pivots equal the attachment positions; their flags are 0 or 0x200, so
`0x714961 test cl,7` never takes the billboard leg. The loader `0x706a50` rewrites an `.mdx` or
`.mdl` extension to `.m2`, takes `.m2` as it is and refuses any other or none, so the scripts'
`.mdx` names resolve to `.m2` files. Each scene has one camera, and `SetCamera(0)` indexes it raw. The frame's own `SetFacing` (`ui`) turns
the scene model, and no GlueXML file calls it. With the bone yaw dropped, the total angle between Human's
forward and the line to the camera goes from 12.937 to 22.126 degrees; its elevation is 12.829.

## Pet model chain

The pet gate is a nonzero pet display id and two rows resolving: `CreatureDisplayInfo` through
`+0x04` to `CreatureModelData`; no class, race or level field is read. The model is a plain
`CM2Model` made by `0x707350` from the model row's `+0x08` path, stored at `+0x114` once, scaled by
`diag(S, S, S, 1)` written over `+0xbc`, textured by `0x4797b0` and attached at 1. The build runs
once per record; every selection detaches both attachments first and re-attaches the cached
models, which the record keeps referenced while they are detached.

Addresses: `0x472d61`, `0x472d7d`, `0x472d87`, `0x472d9c`, `0x472e14`, `0x472e2a`, `0x472e4a`, `0x472e4f`, `0x473700`, `0x47370d`, `0x4722c0`, `0x4722e0`, `0x7130a0`, `0x713020`, `0x710390`, `0x71300d`, `0x473ad0`
Confidence: verified
Proof: one
Test: none

`0x472950` reads the record's bytes `+0x100`, `+0x102` (`0x472a73`, `0x472a7a`) and `+0x102` to
`+0x107` (`0x472ac7` to `0x472b18`), never `+0x101` (class) or `+0x108` (level), and the pet block holds only direct calls, none of which
receives the record. `0x472a44` and `0x472a49` detach after the index test (`0x472965`,
`0x472971`); `0x7130a0` walks the scene's child list and releases each child whose `+0x1d0` equals
the id and `0x713020` unlinks it and releases it through `0x7103a0`, which `0x71300d` (`0x710390`)
balanced at the attach. `0x473700` releases `+0x114` but zeroes only `+0x110`. `0x473ad0`'s fallback path copies whole
records and releases each source (`0x473b83`), but its one caller (`0x47236b`, the roster growth) runs
while the records are still as the append built them, `+0x110` and `+0x114` zero, so the release
frees nothing. Of the 23 shipped `CreatureFamily` rows, 22 have levels 1 and 60; row 28 has levels
0 and 0, so its scale is NaN.

## Pet textures

`0x4797b0(pet, displayRow, modelRow)` takes the directory of the model path, then for display-row
columns 6, 7 and 8, skipping an empty string, formats `"%s%s%s"` from the shared empty string, the
directory and the variation name, loads that texture (the loader appends `.blp`, or `.tga` where
that fails) and binds it to M2 texture type 11, 12 or 13 through `0x710ec0`. It reads nothing
else of the display row, so the extra-display-info path the world uses is not taken.

Addresses: `0x4797b0`, `0x4797dd`, `0x479812`, `0x47983c`, `0x479865`, `0x47987e`, `0x479891`, `0x710ec0`, `0x449d90`, `0x449590`, `0x8362d8`
Confidence: verified
Proof: one
Test: none

`0x479812 mov dl,0x5c` with `0x64a450` finds the last backslash and `0x479823` ends the string after
it. `0x479840` to `0x4798a4` loops `edi` 0 to 2 over `ebx` from `0x47983c add ebx,0x18`;
`0x479865 call 0x64a7f0` formats; `0x479891` passes `edi + 0xb`. `0x710ec0` matches that value
against the model's texture-type dwords. The column map is read from the stores: `CreatureDisplayInfo`
records are 0x30 bytes with the model id at `+0x04` and the scale at `+0x10`; `CreatureModelData`
0x40 bytes with the path at `+0x08` and the scale at `+0x10`; `CreatureFamily` 0x48 bytes with the
scales at `+0x04` and `+0x0c` and the levels at `+0x08` and `+0x10`.

## Idle animation loop

The body and the pet start with identical calls, `0x7121a0(-1, 0, -1, 0, 1.0, 1, 1)`: the whole
model, animation 0, a random variation, from the start, blended and persistent. No argument loops.

Both models register `0x473090` through `0x711bb0` as the sequence-completion callback. On a
completion (reason 0) it branches on the channel in `edx`: −1 or 0x1a replays animation 0 on the
whole model with a fresh random variation (`0x47309a`, `0x47309f`), so a pet idles by chaining Stand
variations; any other channel gets `0x7121a0(edx, −1, −1, 0, 1.0, 1, 1)` unless the animation is 15
(`0x4730a4`), which does nothing.

Addresses: `0x472bda`, `0x472eea`, `0x472b87`, `0x472e99`, `0x473090`, `0x47309a`, `0x47309f`, `0x4730a4`, `0x711bb0`, `0x7121a0`, `0x7122c1`, `0x719494`, `0x7194fd`, `0x70a280`, `0x7194f5`
Confidence: verified
Proof: one
Test: none

A variation plays once: the shipped Wolf and Imp chains have replay range (0, 0), and the variation
probabilities sum to 32767. The pushes at `0x472bb6` to `0x472bd8` and `0x472ec7` to `0x472ee8` match slot for slot. The callback
cell `[model+0x70]` is called from the per-frame advance at `0x7194fd` with reason 0 (`0x7194f5
push 0`) once a channel reaches its end, behind a latch (`0x7194bc`), and from `0x7122c1` with
reason 1 when a call replaces the channel; `0x473093` returns for a nonzero reason. Wolf (animation
chain 2, 8, 9, 10: 1333, 2667, 3000, 4000 ms; probabilities 27247, 2184, 3276, 60) and Imp (1, 11,
14: 833, 1234, 1667 ms; 24576, 1638, 6553) are read from the `.m2` files.

## Held weapons at select

The select build never sheathes a weapon. It calls `0x47a0c0(slot, 0, 0, isShield, 0)` for slots
15 and 16, so the sheath type is 0, `0x47a070(0)` returns −1 and `0x47a380(−1)` returns at once,
and the hand attachment decides: slot 15 attaches at 1, slot 16 at 2, or at 0 with the shield path
when `isShield` is set. Slot 17 (ranged) is skipped by the loop. Neck, finger and trinket slots
have no case in `0x478cb0` and draw nothing.

Addresses: `0x472c01`, `0x472c4f`, `0x472c54`, `0x472c81`, `0x472c91`, `0x47a0c0`, `0x47a070`, `0x47a380`, `0x47a138`, `0x47a11c`, `0x47a12f`, `0x47a15d`, `0x47a164`, `0x478cb0`
Confidence: verified
Proof: one
Test: none

`0x472c8c push 0` is the sheath type and `0x472c8a push 0` the hand-or-sheath selector. In
`0x47a0c0`, `0x47a138` sets `esi` 1, `0x47a11c` 2 and `0x47a12f` 0 for the shield path
(the shield model directory instead of the weapon one); `0x47a15d` to `0x47a164` copy `esi` to `edi` for a zero
selector, and `0x47a178` validates the hand attachment with `0x712cb0`. `0x47a070` is a five-way
switch whose case 0 is `mov eax,-1`.

## Select draw order

The select scene paints in the `ModelFFX` frame's own strata and level bucket, MEDIUM level 1
under `GlueParent` (MEDIUM, 0), and every visible region of the select screen is in a later
bucket, so the effect bracket around the scene never covers a UI pixel. The override `0x46fa90`
registers `0x46fad0` on layer 2 only when the frame has a model; the base's direct registration of
the scene draw `0x76d240` is not made.

Addresses: `0x46fa90`, `0x46fad0`, `0x76d240`, `0x773110`, `0x76fb00`, `0x765920`, `0x76ab65`, `0x6cd890`, `0x6cda70`
Confidence: verified
Proof: one
Test: none

`0x76d240` has one call site, `0x46fada`. The batch the flat walk `0x765920` passes at
`0x7659ea` is the bucket's own; `0x76fb00` drains the layer's callbacks after its quads and strings.
A frame's level is its parent's plus one (`0x76ab65`). In `CharacterSelect.xml` the `ModelFFX` has
no layers; the character name, logo, buttons and dialogs are in levels 2 to 5 and the DIALOG and
tooltip strata. The effect bracket's reading of earlier pixels depends on the `gxMultisample`
setting and is `ffxeffects`.
