# ui: the Frame and Region methods

How a Lua method call on a frame reaches the C++ object, and what the methods of the `Frame` and
`Region` tables do with it.

`CSimpleFrameScript.cpp` (`0x773640` to `0x778660`) holds the 68 methods of `Frame` and the
functions that install them; the run `0x7a11d0` to `0x7a2f60` holds the 19 methods every region
shares (`GetName`, `SetParent`, `SetPoint` and the rest) and their installers. A frame's method
lookup asks the `Frame` table first and then the `Region` table, so a frame answers both.

Roots: `0x778300`, `0x778560`, `0x778580`, `0x778590`, `0x7a2c10`, `0x7a2e70`, `0x7a2e90`, `0x7a2ea0`

## Method tables and lookup

The `Frame` methods are 68 pairs of a name and a function at `0x878ec0`, from `GetFrameType` to
`SetBackdropBorderColor`; the `Region` methods are 19 pairs at `0x87c9b8`, from `GetObjectType` to
`ClearAllPoints`. Each table is hashed by name into its own hash table, `0xcf4d38` and `0xcf54b4`.
A frame's slot 8 looks a name up in the `Frame` hash and, when it is not there, in the `Region`
hash.

Addresses: `0x878ec0`, `0x87c9b8`, `0x778560`, `0x778565`, `0x77856a`, `0x77856f`, `0x778580`, `0x778590`, `0x7785a7`, `0x7785c0`, `0x7a2e70`, `0x7a2e90`, `0x7a2ea0`, `0xcf4d38`, `0xcf54b4`, `0x701d80`, `0x701f20`, `0x702000`
Confidence: verified
Proof: one
Test: none

The two tables are read as pairs of a string pointer and a function, 68 and 19 of them.
`0x778560` pushes `0xcf4d38` and calls `0x701d80` with `edx` 0x44 and `ecx` `0x878ec0`
(`0x778565` to `0x77856f`); `0x778580` hands `0xcf4d38` to `0x701f20`. `0x778590` (slot 8 of
`0x81c498`) calls `0x702000` with `0xcf4d38` (`0x7785a7`) and, when it answers 0, `0x7a2ea0`
(`0x7785c0`), which calls `0x702000` with the `Region` hash. `0x7a2e70` and `0x7a2e90` install and
remove the `Region` table.

## Finding self

Every method takes the object from its first argument: the argument must be a table, else it
raises `Attempt to find 'this' in non-table object (used '.' instead of ':' ?)`; element 0 must hold
an object, else `Attempt to find 'this' in non-framescript object`; the object must pass the type
test for the class's id, else `Wrong object type for member function`. The `Frame` methods test
with the id `0xcf0c10`, numbered from the shared counter on first use.

Addresses: `0x775750`, `0x77575e`, `0x775769`, `0x775777`, `0x77577c`, `0x775781`, `0x77579c`, `0x7757a6`, `0x7757bd`, `0x775894`, `0xcf0c10`, `0x847ef8`, `0x847ec0`
Confidence: verified
Proof: one
Test: none

`Show` (`0x775750`) is the pattern: when `[0xcf0c10]` is 0 it increments `[0xceef6c]` and stores
it (`0x77575e` to `0x775769`); it reads the type of argument 1 (`0x775777`) and compares it with 5
(`0x77577c`), raising `0x847ef8` otherwise; it reads element 0 of the table (`0x77579c`,
`0x7757a6`) and raises `0x847ec0` for none; it calls slot 0x10 with the id (`0x775894`) and raises
the third message when it answers 0. The other methods of both tables repeat the sequence.

## What the Frame methods call

The `Frame` methods pass their arguments to the frame functions of the other topics. `Show` and
`Hide` set the shown flag and call slots 0x88 and 0x84 (`visibility`); `Raise` raises and `Lower`
does nothing (`strata`); `SetFrameStrata` and `SetFrameLevel` call the setters, refusing an unknown
stratum and a negative level; `RegisterEvent` and the other three event methods call the script
layer (`script`); `SetAlpha` and `SetScale` clamp or refuse as described in `visibility`;
`EnableMouse`, `EnableKeyboard` and `EnableMouseWheel` switch input kinds (`input`).

Addresses: `0x775750`, `0x775810`, `0x775a50`, `0x775b20`, `0x774360`, `0x774560`, `0x774a40`, `0x774b30`, `0x774c20`, `0x774cf0`, `0x774e90`, `0x775140`, `0x777070`, `0x776ec0`, `0x7771f0`
Confidence: verified
Proof: one
Test: none

The calls are read from each method's body: `0x775a50` calls `0x76a5b0` and `0x775b20` calls
`0x76a5c0`; `0x774360` calls `0x6f17d0` and `0x76a470`; `0x774560` calls `0x76a4f0`; `0x774a40`,
`0x774b30`, `0x774c20` and `0x774cf0` call `0x702140`, `0x702280`, `0x7023e0` and `0x7024f0`;
`0x777070` calls `0x76af00` or `0x76af50` with kind 2, `0x7771f0` with 3, and `0x776ec0` both
with kinds 1 and 0.

## Coordinates the methods convert

Methods that take or return sizes and positions convert between the units scripts use and the
layout's own: a value read from a script is multiplied through the screen scale, a value returned
is divided by it. `GetLeft`, `GetCenter` and the other edge methods first lay out a frame that is
pending and answer nil for an invalid rectangle; `SetWidth` and `SetHeight` call layout slots 0x14
and 0x18.

Addresses: `0x7a1810`, `0x7a1980`, `0x7a1aa0`, `0x7a1bc0`, `0x7a1ce0`, `0x7a1e00`, `0x7a1f20`, `0x7a2030`, `0x7a2150`, `0x775bf0`, `0x775d50`, `0x776020`, `0x7762a0`
Confidence: inferred
Proof: none
Test: none

Each of these calls the screen-coordinate pair `GetScaledScreenAspect` and `DivideByScreenScaleX`
or `MultiplyByScreenScaleX` per value; the edge getters call `0x768310`, `0x7680e0` and
`0x768320`. The exact conversion belongs to the screen-coordinate system and is not derived here.

## Region parent and anchor methods

`SetParent` takes a frame object or a frame name, refuses a parent that would make a loop, and
calls the object's slot 0x14. `SetPoint` takes a point name, an optional relative region or name,
an optional relative point and two offsets; it refuses an unknown point, a missing region, the
region itself and a region that depends on it. `SetAllPoints` does the same checks for one
relative region. `ClearAllPoints` clears every point.

Addresses: `0x7a1550`, `0x7a2540`, `0x7a2940`, `0x7a2b70`, `0x767ed0`, `0x7a2340`, `0x7a2260`, `0x7a1460`, `0x7a1390`, `0x7a1290`, `0x7a11d0`
Confidence: inferred
Proof: none
Test: none

The refusals are the error strings each formats: `Would create a loop parenting to %s`, `Unknown
region point`, `Couldn't find region named '%s'`, `trying to anchor to itself` and `%s is dependent
on this`; the dependency test is `0x768010` and the setters `0x767c70` through slot 0x28 and
`0x767db0`. `0x7a2b70` calls `0x767ed0`, a `jmp` to `0x767620`. The argument shapes are from the
usage strings.
