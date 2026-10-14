# ui: the route frame type

`TaxiRouteFrame` is a frame type the stock interface declares for the taxi map (`TaxiRouteFrame.h`, `0x495ba0` and
`0x506950` to `0x506ee0`). The type is registered, its class adds no field, it replaces three virtual functions and one
layout thunk, and the replacements do nothing a script or a player can see. `Texture:SetTexCoord` has an eight-number form
that sets four independent corners. The route data is `ui/taxiroute`.

Roots: `0x495ba0`, `0x506a60`, `0x506ac0`, `0x79beb0`

## Class and size

`TaxiRouteFrame` is the last of the eight type names registered by `0x495940`, with flag 0, so both a frame element and
`CreateFrame("TaxiRouteFrame", ...)` resolve it (`ui/framexml#frame-types`). Its factory `0x495ba0` allocates 0x314
bytes, the size the plain frame factory allocates, and runs `0x506950`: the plain frame constructor, then the class's two
vtable words and the instance cell. The class adds no field.

Addresses: `0x495940`, `0x4959b7`, `0x4959b9`, `0x4959be`, `0x4959c3`, `0x495ba0`, `0x495baa`, `0x495bbd`, `0x6eec10`, `0x6eec38`, `0x506950`, `0x50695a`, `0x50695f`, `0x506965`, `0x50696c`, `0x769090`, `0x7692e5`, `0x7692eb`, `0xbe0b14`
Confidence: verified
Proof: one
Test: none

The registration is `push 0` at `0x4959b7` and the factory at `0x4959b9`. The plain factory `0x6eec10` allocates 0x314
(`0x6eec38`) and calls the same base constructor `0x769090`, whose last own store is at offset 0x310. The constructor
`0x506950` is straight-line: after the base call it writes `+0x00` (`0x50695f`), `+0x24` (`0x506965`) and the cell
(`0x50696c`), so no offset of the class's own is written.

## Overridden slots

The route frame's 36-slot vtable and 11-slot layout vtable are the plain frame's with four words replaced: primary slot 0
(the deleting destructor `0x506980`), slot 17 (`0x506a60`, emit a layer), slot 35 (`0x506ac0`, update the scale) and
layout slot 1 (`0x506ee0`, the thunk to the destructor). The emit override runs the plain emit, then for layer 0 registers
the paint callback `0x506a90`; the callback tests its argument and jumps to a function that is one `ret`, so nothing is
drawn. The scale override runs the plain update and, when it reports a change, zeroes the line count and returns 1.

Addresses: `0x808640`, `0x808610`, `0x81c498`, `0x81c46c`, `0x506980`, `0x506986`, `0x50698b`, `0x506a60`, `0x506a70`, `0x506a75`, `0x506a7a`, `0x506a81`, `0x506a90`, `0x506a92`, `0x506a94`, `0x506aa0`, `0x506ac0`, `0x506ac7`, `0x506acc`, `0x506ad0`, `0x506ee0`, `0x506ee0`, `0x76b3a0`, `0x76ac90`, `0x773110`
Confidence: verified
Proof: one
Test: none

A word-by-word comparison of the two 36-slot tables gives slots 0, 17 and 35 and nothing else, and the two 11-slot tables
differ in slot 1 only; each table ends where the next constant begins (a float after the first, a zero word after the second). The
emit override registers the callback with `0x773110` when the layer argument is 0 (`0x506a75`). The thunk `0x506ee0`
subtracts 0x24 and jumps to `0x506980`. The rest, the hit test, the input slots and the script slots, is the plain
frame's.

## Frame identity

`GetObjectType()` of a route frame returns `Frame`. `IsObjectType` is true for `Frame` and `Region`, ignoring case, and
false for `TaxiRouteFrame`. The type name exists only in the registration table. The class has no method table of its
own, so a method call resolves through the frame's chain.

Addresses: `0x484780`, `0x484800`, `0x4847b0`, `0x4847c3`, `0x4847d9`, `0x4847e9`, `0x701d80`
Confidence: verified
Proof: one
Test: none

The route frame keeps the plain frame's slots 4, 6 and 7, so it answers with the plain frame's type test. `0x4847b0` compares
the name with `Frame`, then `Region`, and answers 1 on a match (`0x4847e9`). `GetObjectType` is a load and a `ret`. Of the
method-table registrar `0x701d80`'s callers, none lies in the route frame's code (`0x506600` to `0x506f20`) or its
factory.

## Module state

The class owns a line array record (capacity, count, buffer, grow step) at `0xbe0a9c`. The static initializer writes zero
to each word (`0x5067c0`), the destructor writes zero (`0x506a19`, `0x506a1f`, `0x506a25`) and `0x506ab0` and `0x506ac0`
zero the count; no write stores another value. The destructor frees the buffer if one is set. The instance cell
`0xbe0b14` is written by the constructor and cleared by the destructor and read nowhere. No `call` or `jmp` targets
`0x506ab0` and no section stores its address; the control `0x506a90` has one stored copy. The route data that the interface draws comes from the taxi map's own arrays, not from this record.

Addresses: `0x5069b0`, `0x5069b5`, `0x5069bb`, `0x5069d1`, `0x5069e0`, `0x506a0d`, `0x506a50`, `0x5067c0`, `0x506a19`, `0x506a1f`, `0x506a25`, `0x506ab0`, `0xbe0a9c`, `0xbe0b14`, `0x4dd360`, `0x7693b0`
Confidence: verified
Proof: one
Test: none

The record has five references to its count and buffer words, all inside `0x506600` to `0x506f20`; a scan of the image
for each address finds the same sites and no pointer in a table. The destructor loops over `count` elements of 0x18 bytes
calling `0x4dd360`, which is a single `ret`, then frees the buffer (`0x506a0d`) and calls the plain destructor
(`0x506a50`).

## Route line textures

`Texture:SetTexCoord` takes four numbers, minX, maxX, minY, maxY, or eight, the x and y of the upper left, lower left,
upper right and lower right corners in that order. With four it expands to the rectangle corners. Any other count raises
`Usage: %s:SetTexCoord(minX, maxX, minY, maxY) or SetTexCoord(ULx, ULy, LLx, LLy, URx, URy, LRx, LRy)`. Both forms store
the same eight floats at `+0x104` to `+0x120` of the texture.

Addresses: `0x79beb0`, `0x79bf58`, `0x79bf61`, `0x79bf6a`, `0x79bfb3`, `0x79bff9`, `0x79c007`, `0x79c065`, `0x7704c0`, `0x770410`, `0x770570`, `0x7705b0`, `0x79c080`, `0x79c113`
Confidence: verified
Proof: one
Test: none

The argument count is `lua_gettop` less 1 (`0x79bf58`, `0x79bf5d`); 4 takes `0x79c007`, 8 takes `0x79bfb3`. The eight-number path
stores arguments 2 to 9 in order (`0x79bfd0` to `0x79bff0`) and calls `0x7704c0` (`0x79bff9`). The four-number path builds
the rectangle and calls `0x770410` (`0x79c065`), which stores upper left (minX, minY), lower left (minX, maxY), upper
right (maxX, minY) and lower right (maxX, maxY). When the flag at `+0x124` that `SetTexCoordModifiesRect` stores (`0x79c113`, in
`0x79c080`) is set, both then resize the layout rect (`0x770570`, `0x7705b0`).
