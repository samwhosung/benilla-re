# ui

The interface engine: frames and their layout, the script layer that runs the interface's Lua,
the widget types, and the game's own interface code that feeds them.

The system owns two groups of object files. The engine side is `ENGINE\Source\Frame` (`CSimpleTop`
to `CSimpleHyperlinkedFrame`, `0x764160` to 0x7a3a80), `ENGINE\SOURCE\FrameXML` (from 0x6ed930) and
`ENGINE\SOURCE\FrameScript` (`0x701940` to `0x706430`); the Lua 5.0 core between them belongs to
`lua`. The game side is `WoW\Source\Ui` (0x480380 to 0x509ff0), the interface files of
`WoW\Source\UIUtil` (`ScriptEvents.cpp` to `Tooltip.cpp`, 0x515880 to 0x536ea0), the LCD files
(from 0x728f00, ending before zlib at 0x730bc0), `NamePlateFrame.cpp` (0x7cb0f0 to 0x7cc310) and
`HealthBar.cpp` (from 0x7cc320). In all, about 9,800 functions. The frame and layout core is
documented in `frames`, `anchors`, `strata`, `visibility`, `input`, `script` and `methods`; the Lua
state, its library and the C function contract in `luastate`; loading the interface from toc and
XML files in `framexml` and `xmltypes`; add-ons and saved variables in `addons`; the widget types
in `button`, `editbox`, `fontstring`, `messageframe`, `simplehtml`, `slider`, `scrollframe`,
`colorselect`, `model`, `tooltip`, `minimap`, `hover` and `resizebounds`, by behaviour. The rest is not documented yet (`Not documented yet`); its rows are those other systems' findings cite.
None of the primitives here has a bit-exact test yet.

Roots: `0x764160`, `0x766800`, `0x766820`, `0x767040`, `0x767070`, `0x7670a0`, `0x767240`, `0x768f70`, `0x768f90`, `0x769030`, `0x769060`, `0x769360`, `0x76b2f0`, `0x76c130`, `0x76c150`, `0x76c490`, `0x76c4c0`, `0x76c7e0`, `0x76c800`, `0x76c820`, `0x76c860`, `0x76c890`, `0x773800`, `0x778300`, `0x7785d0`, `0x7785f0`, `0x778620`, `0x778650`, `0x701940`, `0x701960`, `0x701990`, `0x7019b0`, `0x705e30`, `0x705e50`, `0x705e70`, `0x705ea0`, `0x705ed0`, `0x7a17f0`, `0x7a2c10`, `0x7a2ec0`, `0x7a2ee0`, `0x7a2f10`, `0x7a2f40`, `0x7a2f70`

## Frame core object files

The frame and layout core is nine runs of code. The first six are bounded by the source paths
their functions pass and by their initializer slots; the last three by their method tables, their
initializers and the constants those read.

| run | first function | last function | file |
|---|---|---|---|
| top | `0x764160` | `0x767080` | `CSimpleTop.cpp` |
| layout | `0x7670a0` | `0x769070` | `CLayoutFrame.cpp` |
| frame | `0x769090` | `0x76c4d0` | `CSimpleFrame.cpp` |
| region | `0x76c4f0` | `0x76c8a0` | `CScriptRegion.cpp` and its initializers |
| frame methods | `0x773640` | `0x778660` | `CSimpleFrameScript.cpp` |
| script | `0x701940` | `0x705d20` | `FrameScript.cpp` |
| script functions | `0x705e30` | `0x7063d0` | no path; `CreateFrame` and the type tables |
| region methods | `0x7a11d0` | `0x7a2f50` | no path; the `Region` table |
| point getters | `0x7a2f70` | `0x7a3200` | no path; the +∞ of `0xcf550c` |

Addresses: `0x8783a4`, `0x878500`, `0x8785b8`, `0x878874`, `0x879504`, `0x8729b0`, `0x7642ba`, `0x767d01`, `0x769b44`, `0x76c588`, `0x7739ad`, `0x701a4e`, `0x703e23`, `0x827000`, `0x7a2f80`, `0x7a3160`, `0x7a3230`, `0x81d560`, `0x81d564`, `0x81d568`, `0x81d56c`, `0x81d570`, `0x81d574`, `0x81d578`
Confidence: verified
Proof: one
Test: none

The paths `CSimpleTop.cpp` (`0x8783a4`), `CLayoutFrame.cpp` (`0x878500`), `CSimpleFrame.cpp`
(`0x8785b8`), `CScriptRegion.cpp` (`0x878874`), `CSimpleFrameScript.cpp` (`0x879504`) and
`FrameScript.cpp` (`0x8729b0`) are pushed at `0x7642ba`, `0x767d01`, `0x769b44`, `0x76c588`,
`0x7739ad`, `0x701a4e` and `0x703e23`. In the initializer table `0x827000` each file's set is a
contiguous run of slots: `0x82d588` to `0x82d598` for the top, `0x82d59c` to `0x82d5b0` for the
layout, `0x82d5b4` to `0x82d5c8` for the frame, `0x82d5cc` to `0x82d5dc` after it, `0x82d62c` to
`0x82d640` for the frame methods, `0x82d0ec` to `0x82d0f8` for the script and `0x82d0fc` to
`0x82d10c` after it, `0x82d8a0` to `0x82d8b4` for the region methods and `0x82d8b8` to `0x82d8cc`
for the point getters. A set's +∞, 2π and 1/2π can lie apart from the rest: the layout's `0x7670a0`
precedes its +∞ `0x767240`. Each file reads its own copies of the header's +∞, π and 10: the region
methods' initializers read `0x81d560`, `0x81d564` and `0x81d568`. The point getters are a file of
their own: their `0xcf550c` initializer `0x7a2f80` and the five that follow the getters, `0x7a3150`
to `0x7a3200`, read `0x81d56c`, `0x81d570` and `0x81d574`, and the next file,
`CSimpleHyperlinkedFrame.cpp`, opens with `0x7a3220`, which reads `0x81d578` (`0x7a3230`), at slot
`0x82d8d0`.

## Static initializers

Each run opens with the shared header's initializers, its +∞ first (`systems/README.md`, Object
files): a thunk, a five-byte `jmp` or a
ten-byte `call` and `jmp`, and a body that stores one value in a global of the file: +∞ copied from
a constant, π + π, 1 over that, 10·10, or 0.5/10. The extra initializers are the layout queue
(`anchors`), the point getters' second +∞ in `0xcf550c`, the drag threshold 0.01·0.01 in `0xcf0c14`,
the event table of the script, and the method hash tables of the frame and region methods, each with
an exit handler. The initializers of the rest of the files are `initthunks`, `initfloats`, `initstores`
and `initexits`.

Addresses: `0x764160`, `0x764170`, `0x766800`, `0x766810`, `0x766820`, `0x766830`, `0x767040`, `0x767050`, `0x767070`, `0x767080`, `0x767240`, `0x767250`, `0x768f70`, `0x768f80`, `0x768f90`, `0x768fa0`, `0x769030`, `0x769040`, `0x769060`, `0x769070`, `0x769360`, `0x769370`, `0x76b2f0`, `0x76b300`, `0x76c130`, `0x76c140`, `0x76c150`, `0x76c160`, `0x76c490`, `0x76c4a0`, `0x76c4c0`, `0x76c4d0`, `0x76c7e0`, `0x76c7f0`, `0x76c800`, `0x76c810`, `0x76c820`, `0x76c830`, `0x76c860`, `0x76c870`, `0x76c890`, `0x76c8a0`, `0x773800`, `0x773810`, `0x778300`, `0x778310`, `0x7783d0`, `0x7783e0`, `0x7785d0`, `0x7785e0`, `0x7785f0`, `0x778600`, `0x778620`, `0x778630`, `0x778650`, `0x778660`, `0x701940`, `0x701950`, `0x701960`, `0x701970`, `0x701990`, `0x7019a0`, `0x7019b0`, `0x7019e0`, `0x7019f0`, `0x705e30`, `0x705e40`, `0x705e50`, `0x705e60`, `0x705e70`, `0x705e80`, `0x705ea0`, `0x705eb0`, `0x705ed0`, `0x705ee0`, `0x7a17f0`, `0x7a1800`, `0x7a2c10`, `0x7a2c20`, `0x7a2ce0`, `0x7a2cf0`, `0x7a2ec0`, `0x7a2ed0`, `0x7a2ee0`, `0x7a2ef0`, `0x7a2f10`, `0x7a2f20`, `0x7a2f40`, `0x7a2f50`, `0x7a2f70`, `0x7a2f80`, `0x7a3150`, `0x7a3160`, `0x7a3170`, `0x7a3180`, `0x7a3190`, `0x7a31a0`, `0x7a31c0`, `0x7a31d0`, `0x7a31f0`, `0x7a3200`, `0xcf0c14`
Confidence: verified
Proof: one
Test: none

A typical pair is `0x764160`, `jmp 0x764170`, and `0x764170`, `mov eax,[0x81c370]` (+∞, 0x7f800000)
and its store in `0xcf0bc0`. `0x766810` is `fld [π]; fadd st(0),st`, `0x766830` divides 1.0 by that
global, `0x767050` squares 10 and `0x767080` divides 0.5 by 10. `0x76b300` squares 0.01
(`0x81c468`) into `0xcf0c14`. `0x7019b0` calls `0x7019c0` and registers `0x7019f0`, which
destroys the event records at exit; `0x778300` and `0x7a2c10` build a hash table's lists through
`0x778310` and `0x7a2c20` and register `0x7783e0` and `0x7a2cf0`, which free them.

## Starts the listing hides

Two function starts lie where the disassembly listing is out of step after a jump table:
`0x768710` and `0x76c040`. A disassembly that starts at either decodes a normal prologue. The
listing also turns a byte of `0x704480`'s index table into `jo 0x704ae0`, a false jump onto a
start, and keeps the unexecuted fillers at 0x7643ee and 0x7645bd inside `0x764390` and `0x7645a0`.

Addresses: `0x768710`, `0x76c040`, `0x76a679`
Confidence: verified
Proof: one
Test: none

The listing decodes the tables at 0x7686ec (the end of `0x768550`) and 0x76c018 (the end of
`0x76bf70`) as instructions and rejoins the real stream inside the prologues. `0x768710` is called
from `0x76a679`; `0x76c040` is slot 0x6c of `0x81c498`, the dword at 0x81c504.

## Unreached functions

Nothing live reaches fifteen functions of the frame core: the event table's teardown `0x704060`, four
global helpers `0x7041a0`, `0x7041f0`, `0x7042a0` and `0x7043b0`, the varargs wrapper `0x704ef0`, a
list link `0x764ce0`, a click raise `0x765010`, the absolute point setter `0x767b50` and the three
slots of the vtable only it installs, `0x767bf0`, `0x767c00` and `0x767c10`, a sizing step
`0x768550` and its caller `0x76a630`, and `0x76a9d0`, which marks all five layers.

Addresses: `0x704060`, `0x7041a0`, `0x7041f0`, `0x7042a0`, `0x7043b0`, `0x704ef0`, `0x764ce0`, `0x765010`, `0x767b50`, `0x767bf0`, `0x767c00`, `0x767c10`, `0x767bab`, `0x81c42c`, `0x768550`, `0x76a630`, `0x76a9d0`, `0x7680e0`
Confidence: verified
Proof: one
Test: none

The census is two scans over the whole image: every `call` or `jmp` in the listing whose target is
the function, and every dword equal to its address at any byte alignment. Both find nothing for
eleven of them; `0x768550`'s one call is `0x76a649`, inside `0x76a630`. The dwords of `0x767bf0`,
`0x767c00` and `0x767c10` are slots 1, 2 and 0 of `0x81c42c`, whose one store, `0x767bab`, lies in
`0x767b50`; `0x767be0`, its slot 3, is also slot 3 of the base `0x81c43c`, which live destructors
install. Each function above ends
in `ret` or `jmp`, so nothing falls through into them. The control is `0x7680e0`, with 45 calls.
Every other function of the nine runs, apart from the base library of `FrameScript.cpp`, has a
call, a jump, a stored address read by live code, or a slot of a vtable live code installs.

## Nameplate frame functions

Two rows of `NamePlateFrame.cpp` are read here for other systems. `0x7cba30` is slot 15 of the
nameplate's vtable 0x81de50, the slot `CSimpleFrame::OnMouseMove` (`0x76b320`) fills in the base:
while a spell targets and wants a location but not a unit, it answers 0; otherwise it calls the
base. `0x7cba70`, which `0x6122e0` tail-calls with a unit's nameplate (the unit's `+0xe60`),
stores 5000 at the nameplate's `+0x514` and sets the colour of the texture at `+0x4fc` to
0xffff0000, opaque red.

Addresses: `0x7cba30`, `0x7cba70`, `0x76b320`, `0x6e48a0`, `0x6e6320`, `0x6e6180`, `0x6122e0`
Confidence: inferred
Proof: none
Test: none

The constructor stores the vtable at 0x7cb29a; its dword at 0x81de8c, slot 15, is `0x7cba30`, the
only reference to it. `0x7cba30` calls `0x6e48a0`, `0x6e6320` and `0x6e6180` and returns 0 only when
the first two answer nonzero and the third zero (`0x7cba51`); else it passes its argument to
`0x76b320`. `0x6122e0` loads the unit's `+0xe60` and, when it is nonzero, jumps to `0x7cba70`
(`0x6122ea`), the one reference to it. `0x7cba70` stores 0x1388 at `+0x514` (`0x7cba77`) and calls
0x77f750 on `+0x4fc` with a pointer to 0xffff0000 (`0x7cba88`, `0x7cba8f`), the function that sets a texture's
vertex colour. What the 5000 counts is not read here.

## Not documented yet

The rest of the system has no doc yet; the rest of ui is part of
[#3](https://github.com/samwhosung/benilla-re/issues/3), the rewrite of every system's findings.
These are its files in link order, with a census of their function starts:

| files | functions |
|---|---|
| regions and drawing: `CSimpleModel`, the textures and batches of `CSimpleRender.cpp`, regions, backdrop, `CSimpleFont`, the texture, font string and font methods, `CSimpleHyperlinkedFrame` | about 600 |
| widget types and their methods: button, edit box, check button, message frames, status bar, slider, scroll frame, HTML, colour select, movie frame | about 850 |
| the parts of `FrameXML.cpp`, `XMLTree.cpp` and the base library of `FrameScript.cpp` that `framexml` and `luastate` do not cover, the LCD files | about 700 |
| `WorldFrame.cpp`, `GameUI.cpp` | about 820 |
| `ChatFrame.cpp`, `WorldMap.cpp`, `BattlefieldInfo.cpp`, `MailInfo.cpp`, `ChatBubbleFrame.cpp` | about 890 |
| `SpellBookFrame.cpp` to `TradeFrame.cpp` | about 790 |
| `LootFrame.cpp` to `GuildInfo.cpp` | about 860 |
| `SkillInfo.cpp` to `ItemTextFrame.cpp` | about 900 |
| `ActionBarFrame.cpp` to `PetitionInfo.cpp` | about 820 |
| `CraftInfo.cpp` to `CharacterModelBase.cpp` | about 830 |
| `QuestTextParser.cpp`, `ScriptEvents.cpp`, the parts of `AddOns.cpp` that `addons` does not cover | about 590 |
| `Cursor.cpp`, `PortraitButton.cpp`, `Tooltip.cpp`, `NamePlateFrame.cpp`, `HealthBar.cpp` | about 670 |

Addresses: `0x76c8e0`, `0x6edb90`, `0x729dd0`, `0x496720`, `0x51ebe0`
Confidence: inferred
Proof: none
Test: none

The counts are a census of function starts per run: call targets, stored addresses and first
instructions after padding, minus targets reached by fall-through or a jump table, counting the
static initializers. The runs between known paths are bounded by their initializer sets and
method tables, which a doc of each settles at the bytes. The game-side files keep the link order;
`UIUtil`'s `Camera.cpp` and `InputControl.cpp` lie between `QuestTextParser.cpp` and
`ScriptEvents.cpp` and are not part of this system: they are `uicamera` and `input`.
