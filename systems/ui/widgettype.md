# ui: widget types and names

What a widget says it is and how a script finds one by name: the type names and their chains, what
`GetObjectType` and `IsObjectType` answer, how a name string becomes a widget, and how `SetPoint`
and `SetAllPoints` use it.

Every widget class holds its Lua type name as a pointer to a string in `.data` and the chain of
names it answers to as a fixed list in its code. A widget's Lua table keeps it as element 0
(`script`). There is no name table for frames and regions: a name is a global in the Lua state
that holds the widget's table, published when the widget is named. Fonts also have a hash table.

Roots: `0x7a11d0`, `0x7a1290`, `0x76c700`, `0x76c760`, `0x76c5b0`, `0x7a2540`, `0x7a2940`, `0x76c650`

## Type chains

A widget answers one name and the names of its bases: 23 classes, each with a type name function
and a chain function. A `Texture` or `FontString` also answers `Region`, every frame class answers
`Frame` and `Region`, and a `Font` answers only `Font`.

| class | answers, in order |
|---|---|
| Region | Region |
| Font | Font |
| Texture, FontString | itself, Region |
| Frame | Frame, Region |
| EditBox, StatusBar, MessageFrame, ScrollFrame, ScrollingMessageFrame, Slider, SimpleHTML, ColorSelect, MovieFrame, GameTooltip, Minimap, Model | itself, Frame, Region |
| Button | Button, Frame, Region |
| CheckButton, LootButton | itself, Button, Frame, Region |
| PlayerModel | PlayerModel, Model, Frame, Region |
| DressUpModel, TabardModel | itself, PlayerModel, Model, Frame, Region |

Addresses: `0x76c480`, `0x76c460`, `0x77e520`, `0x77e500`, `0x773480`, `0x773430`, `0x7735d0`, `0x773580`, `0x484800`, `0x4847b0`, `0x778810`, `0x7787b0`, `0x785100`, `0x785090`, `0x495b60`, `0x495af0`, `0x779db0`, `0x779d50`, `0x782e80`, `0x782e20`, `0x7857f0`, `0x785790`, `0x786b40`, `0x786ae0`, `0x7879a0`, `0x787940`, `0x789510`, `0x7894b0`, `0x789f60`, `0x789f00`, `0x78b360`, `0x78b300`, `0x78c220`, `0x78c1c0`, `0x529440`, `0x5293e0`, `0x4ee090`, `0x4ee030`, `0x470030`, `0x46ffd0`, `0x505830`, `0x5057c0`, `0x5042d0`, `0x504240`, `0x502880`, `0x5027f0`
Confidence: verified
Proof: one
Test: none

A scan of `.text` for `mov eax,[g]; ret` where `g` holds a pointer to a short string finds exactly
these 23 type name functions (slot 7 of the vtable), and slot 6 beside each compares the argument
with the names of its chain. A model or button derived class lists its base's name: the chain is a literal list, not a walk of a
parent. `ScrollingMessageFrame` does not answer `MessageFrame`, a `Font` does not answer `Region`,
and `FontString` does not answer `Font`. No class answers `LayoutFrame`, `ScriptObject` or `Object`.

## Asking for a type

`GetObjectType()` returns the class's name string. `IsObjectType(name)` returns the number 1 when the
name is the class's or one of its bases', else nil, comparing the whole string without regard to
case (ASCII letters only); a number argument is accepted and read as its text. Any other argument
raises `Usage: %s:IsObjectType("TYPE")`, with `<unnamed>` for a widget without a name. The self checks
raise first.

Addresses: `0x7a11d0`, `0x7a1290`, `0x7a11f7`, `0x7a121c`, `0x7a1254`, `0x7a126f`, `0x7a1278`, `0x7a1315`, `0x7a1323`, `0x7a133f`, `0x7a135c`, `0x7a1364`, `0x7a136d`, `0x7a1382`, `0x64a4c0`, `0x414310`, `0x414333`, `0x6f3810`, `0x6f37f0`, `0x6f3510`, `0x6f3690`, `0x79fda0`, `0x79fe60`, `0x7a1100`, `0x7a2ea0`, `0x7a2e70`
Confidence: verified
Proof: one
Test: none

The self checks (`0x7a11f7` to `0x7a1254`): the argument must be a table, `lua_rawgeti(1, 0)` must give
a userdata, and the object's slot 4 must accept the lazily numbered Region id `[0xcf0c3c]`. The
slot 6 compare is `SStrCmpI` (`0x64a4c0` over `_strnicmp`, which folds `A` to `Z` only) with length
`0x7fffffff`. `lua_isstring` accepts tags 3 and 4 (`0x7a1315`). The hit pushes the number 1 with
`lua_pushnumber`, the miss `lua_pushnil`. The two methods are entries 0 and 1 of the 19-pair Region
table `0x87c9b8`, registered once by `0x7a2e70`; a Texture, FontString or Frame reaches it by a
second lookup (`0x7a2ea0`), and only `Font` has its own pair, `0x79fda0` and `0x79fe60`, with no fallback.
Method names are matched without folding case.

## Finding a widget by name

A name string is read as a global of the Lua state and must be a table whose element 0 is an object
that accepts the id asked. Any global that holds a widget's table resolves, a frame's published
name or an alias an addon assigned. A name that does not resolve raises for `SetPoint` and
`SetAllPoints`; each caller picks its id, and only the layout front end expands `$parent`.

Addresses: `0x76c760`, `0x76c777`, `0x76c77e`, `0x76c78d`, `0x76c7a9`, `0x76c7cf`, `0x6f3a40`, `0x6f3b00`, `0x6f7cf0`, `0x76c700`, `0x76c71c`, `0x76c744`, `0x76c5b0`, `0x76c6e0`, `0x6ee3e8`, `0x6ee280`, `0x7a1637`, `0x7910a4`, `0x790fa0`, `0x48d7bf`, `0x767964`, `0x767800`, `0x767949`
Confidence: verified
Proof: one
Test: none

`0x76c760(name, id)` pushes the name and calls `lua_gettable` (`0x6f3a40`, which goes through
`luaV_index` and so honours `__index`; the raw `lua_rawget` is `0x6f3b00`) on the globals index
`0xffffd8ef`, requires type 5, reads element 0 with `lua_rawgeti` and calls slot 4 with the id. The layout
slot 10 `0x76c700` expands `$parent` with `0x76c5b0` against the anchoring frame, then calls
`0x76c760` with the Region id; it is stored in 33 layout vtables and has no direct caller. `0x76c5b0`
replaces a leading `$parent`, any case, by the name of the first ancestor up `+0x9c` with a nonempty name, else `Top`. The ids are `[0xcf0c3c]` (Region) from the
layout path, `[0xcf0c10]` (Frame) from `SetParent` (`0x7a1637`) and the XML `parent` attribute
(`0x6ee3e8`), which do not expand `$parent`. The XML `relativeTo` attribute reaches slot 10
(`0x767964`) and does.

## SetPoint and SetAllPoints arguments

`SetPoint(point, relativeTo, relativePoint, x, y)` reads the second argument as a point name,
and the third as a frame name, a frame table, nil (the screen root) or a number, which means the
call has no frame and the number is the x offset; the relative frame then defaults to the parent
(the screen root with none). `relativePoint` defaults to `point`. Both offsets must be numbers or both are 0.
A table is not type checked. Failures raise: a frame not found, itself, a frame that depends on it,
an unknown point; a missing third argument is a usage error. `SetAllPoints` accepts a missing
argument and reads a number as a name.

Addresses: `0x7a2540`, `0x7a25d2`, `0x7a25eb`, `0x7a262f`, `0x7a264a`, `0x7a2654`, `0x7a26a8`, `0x7a26b3`, `0x7a26c9`, `0x7a26de`, `0x7a2710`, `0x7a271d`, `0x7a2764`, `0x7a27b2`, `0x7a2810`, `0x7a28ff`, `0x7a2940`, `0x7a29d9`, `0x7a29ea`, `0x7a2a04`, `0x7a2a61`, `0x7a2aa8`, `0x7a2af6`, `0x7a2b5b`, `0x768010`, `0x767c70`, `0x767db0`, `0x6f1840`
Confidence: verified
Proof: one
Test: none

`0x7a2540` raises `Usage` through `0x7a290d` unless argument 2 is a string or number
(`0x6f3510`) and argument 3 has type 4, 5, 0 or 3 (`0x7a25eb` to `0x7a2620`; absent is −1 and fails).
The default relative frame is slot 3 of the layout vtable, `0x76c6e0`: the parent's layout part or `[0xcf0bd8]`.
A string calls slot 10 (`0x7a26b3`), a table takes element 0 plus 0x24 with no type test, nil the
screen root. The failure legs format `%s:SetPoint(): Couldn't find region named '%s'`,
`trying to anchor to itself`, `%s is dependent on this` (`0x768010`, a walk over the nine points of the
candidate) and `Unknown region point`; each raises. `0x7a28ff` calls `0x767c70(…, 1)`. `0x7a2940` reads argument 2 with
`lua_isstring`, so a number becomes the name `"42"`, and ends with `0x767db0(…, 1)`; its messages begin `%s:SetAllPoints():`.

## Naming a widget

Naming a widget stores a copy of the expanded name and publishes the widget's table as a global
when that global is nil, so a name already taken stays with its owner and the later widget is
unreachable by name. Destroying the widget sets the global to nil whoever holds it. Fonts are also
kept in a hash table by name; no other class is.

Addresses: `0x76c650`, `0x76c662`, `0x76c67d`, `0x76c691`, `0x76c6a4`, `0x76c6b8`, `0x701bd0`, `0x701c5b`, `0x701c62`, `0x701c6c`, `0x701c88`, `0x701cb8`, `0x701cd0`, `0x783870`, `0x783886`, `0x783901`, `0x783c6c`, `0x784150`
Confidence: verified
Proof: one
Test: none

`0x76c650` ignores a null or empty name (assert 0x57, `0x76c6c5`), frees the old copy, expands
`$parent` (`0x76c691`), stores `SStrDupA` (`0x76c6a4`) and calls `0x701bd0` (`0x76c6b8`). `0x701bd0` probes the global
with `lua_gettable` (`0x701c62`) and sets it with `lua_settable` (`0x701cb8`) only when its type is
nil (`0x701c6c`); `false` blocks it as well. `0x701cd0` clears the global without checking its value. `[widget+0x98]` is touched by the
constructor, the destructor and these only. `0x783870(name, create)` finds or creates a font in a hash
table of its own, keyed by `SStrHashHT` and compared by `SStrCmpI`, and `0x784150` names the font, clearing
the old global and publishing the new. Of the string-hash calls in the frame files, three are in the
font table (`0x783886`, `0x783901`, `0x783c6c`).
