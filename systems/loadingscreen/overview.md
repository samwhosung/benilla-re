# loadingscreen: overview

The loading screen the client raises while a world loads: a layer above the interface that covers
the frame with the map's art, or with the world map and an animated transport route, fills a
progress bar from three load fractions, swallows the player's input, and ends when the player object
arrives. The glue's world entry, the transfer packets and the transport tick raise it; the loaders
feed its progress.

The system is one object file, `LoadingScreen.cpp`, with the container templates it instantiates and
the initializers of its run. The docs split by topic: `screen` (raising and ending the layer, the
input veto, the progress, the paint, the plain screen, the bar and the tip) and `route` (the transfer
entries, the taxi tables, the route ribbon and the world map tiles). This doc holds the object file,
its static objects and its containers. It draws through `gx`; the tip text is chosen in
`glue/flow#game-tip-choice`.

Roots: `0x4078e0`, `0x408400`, `0x408420`, `0x4084b0`, `0x4088f0`, `0x408a30`, `0x408d80`, and the static initializers `0x4060d0`, `0x4060f0`, `0x406110`, `0x406140`, `0x406170`, `0x4061a0`, `0x4061d0`, `0x406200`, `0x406230`, `0x406270`, `0x406300`, `0x406350`, `0x4063a0`, `0x406400`, `0x4064f0`, `0x4065a0`, `0x408570`, `0x4085a0`, `0x4085d0`, `0x408600`, `0x408630`, `0x408660`, `0x408690`, `0x4086c0`, `0x4086f0`, `0x408720`, `0x408750`, `0x408780`, `0x4087b0`, `0x4087e0`, `0x408800`, `0x408830`, `0x408860`, `0x408890`, `0x408ca0`, `0x408cd0`, `0x408e70`

## Object file

`LoadingScreen.cpp` runs from `0x4060d0` through `0x408f30`: its run of static initializers, the
screen's code from `0x406630` to `0x4083f9`, and the array templates it instantiates. Its path
string `0x82f1a4` is passed at `0x407304` and `0x407e14`. A thunk of the previous file, `0x4060b0`, precedes it, and the next file opens at `0x408fc0` with its own +∞ thunk. All 144 functions are
reached.

Addresses: `0x4060d0`, `0x408f30`, `0x407304`, `0x407e14`, `0x82f1a4`, `0x827000`, `0x8271c0`, `0x827250`, `0x4060b0`, `0x408fc0`
Confidence: verified
Proof: one
Test: none

The run in the initializer table `0x827000` is the slots `0x8271c0` to `0x827250`. It opens with
the +∞, 2π and 1/2π thunks `0x4060d0`, `0x4060f0` and `0x406110`, which read the file's own copies
of the header constants; the neighbouring slots' +∞ thunks, `0x4060b0` and `0x408fc0`, read other
copies. The run is not in address order: from `0x8271cc` its slots point at thunks after the
code (`0x408570` to `0x408e70`, the last in slot `0x82721c`), and the file's own statics fill `0x827220` to `0x827250`.
`0x407304` and `0x407e14` push the path for `0x468460`. The census lists every function of the
range as live.

## Shared header constants

The run opens with +∞ (`0x4060d0`), 2π (`0x4060f0`) and 1/2π (`0x406110`), then runs the shared
header's constants: the squares 10², 11.111111², 5.5555553², 10², 5², 10², 30², 120², 20², 10²,
0.5², 3², 100², then the float 5.0, then 50², 100², 40², 1.5², 10² and 0.5/10; and five products
(−21 · −1/36, that · −255, 18 · 1/36, that · 256, and 1/4.1666665). Each slot is a 5-byte `jmp`
to a body that stores one float in a global of the file.

Addresses: `0x4060d0`, `0x4060e0`, `0x4060f0`, `0x406100`, `0x406110`, `0x406120`, `0x408570`, `0x408580`, `0x4085a0`, `0x4085b0`, `0x4085d0`, `0x4085e0`, `0x408600`, `0x408610`, `0x408630`, `0x408640`, `0x408660`, `0x408670`, `0x408690`, `0x4086a0`, `0x4086c0`, `0x4086d0`, `0x4086f0`, `0x408700`, `0x408720`, `0x408730`, `0x408750`, `0x408760`, `0x408780`, `0x408790`, `0x4087b0`, `0x4087c0`, `0x4087e0`, `0x4087f0`, `0x408800`, `0x408810`, `0x408830`, `0x408840`, `0x408860`, `0x408870`, `0x408890`, `0x4088a0`, `0x408ca0`, `0x408cb0`, `0x408cd0`, `0x408ce0`, `0x406140`, `0x406150`, `0x406170`, `0x406180`, `0x4061a0`, `0x4061b0`, `0x4061d0`, `0x4061e0`, `0x406200`, `0x406210`
Confidence: verified
Proof: one
Test: none

The bodies are `mov eax,[c]; mov [g],eax` for +∞, `fld; fadd st(0),st; fstp` for 2π,
`fld 1.0; fdiv; fstp` for the quotients, `fld; fmul; fstp` for the squares and products, and
`mov [g],0x40a00000` for 5.0 (`0x4087f0`). The same set runs in every object file that includes the
header (`models/overview#shared-header-constants`).

## Static objects

Eight statics are built by the run's own slots, each a slot function that calls a body and then
registers a destructor with `_atexit`. The bodies store white (`0xffffffff`) as the route colour;
the unit quad's texture coordinates (0, 1), (1, 1), (0, 0), (1, 0); zeroes for the 12 world map
tiles' arrays; an empty route vertex array and two empty taxi arrays; the ship quad's offsets
(∓0.01, ∓0.015, 0) in the order (−,−), (+,−), (−,+), (+,+) and coordinates (0, 1), (0.5, 1), (0, 0.5),
(0.5, 0.5). The vertex and taxi destructors free their buffers; the rest return at once. `0x408e70` counts the file as one more user of a shared library
object and calls `0x5b71c0` on the first.

Addresses: `0x406230`, `0x406240`, `0x406250`, `0x406260`, `0x406270`, `0x406280`, `0x4062e0`, `0x4062f0`, `0x406300`, `0x406310`, `0x406330`, `0x406340`, `0x406350`, `0x406360`, `0x406380`, `0x406390`, `0x4063a0`, `0x4063b0`, `0x4063d0`, `0x4063e0`, `0x406400`, `0x406410`, `0x406440`, `0x406450`, `0x4064f0`, `0x406500`, `0x406580`, `0x406590`, `0x4065a0`, `0x4065b0`, `0x406610`, `0x406620`, `0x408e70`, `0x408e80`, `0x408ea0`, `0x408eb0`, `0x8828a4`, `0x8828b0`, `0x882b8c`, `0x882bac`
Confidence: verified
Proof: one
Test: none

A slot function is `call body; jmp register`, and the register routine is `push dtor; call
_atexit; pop ecx; ret` (`0x406250`). The tile arrays are zeroed as `0x30` entries of 12 bytes from
`0x8828f8` (`0x406310`) and of 8 bytes from `0x882be8` (`0x406360`); `0x406410` clears the growable
arrays `0x882b50` and `0x882b60`, `0x4063b0` the vertex array `0x8828e0`. `0x4063e0` frees its
buffer under the tag `.?AUDYNAMICELEMENTVERT@@`. `0x406450` destroys the taxi array through
`0x408440`, zeroes both headers and frees the pair buffer under the tag
`.?AULOADINGSCREENTAXIPATHINFO@?$CLoadingScreenShipInfo@VTaxiPathNodeRec@@@@`. `0x408e80` is `inc
[0xc2a3c4]; cmp eax,1`, then a `jmp` to `0x5b71c0` on 1.

## Growable arrays

The route vertices, the taxi paths, their leg lists and the pairs use one growable array: a
capacity, a count, a buffer and a granularity. A grow to `count + n` past the capacity, with its
second argument nonzero, rounds the request up to a multiple of the granularity. A granularity of
0 becomes the threshold (10 for vertices, 64 for the 4-byte lists, 16 for the outer array, 32 for
pairs) and is stored when the request reaches it; below it the largest power of two not above the
request serves that grow only.

Addresses: `0x4088c0`, `0x4088f0`, `0x408910`, `0x408a30`, `0x408a40`, `0x408b40`, `0x408d00`, `0x408d80`, `0x408ec0`, `0x408f30`, `0x4078e0`, `0x4084f0`, `0x408560`, `0x408400`, `0x408420`, `0x408440`, `0x4084b0`, `0x408510`
Confidence: verified
Proof: one
Test: none

`0x408910` reserves vertex storage (24 bytes each, the tag `.?AUDYNAMICELEMENTVERT@@`, threshold `0xa` at
`0x408939`), `0x408a40` appends a dword (threshold `0x40` at `0x408a5e`), `0x408d80` reserves dwords
(shared by 10 callers), `0x408b40` reserves outer entries (16 bytes, threshold `0x10`; `0x408ec0` copies
one) and `0x408d00` appends a zeroed pair (8 bytes, threshold `0x20` at `0x408d15`) after `0x408f30`
resizes. A resize copies the lower of the old count and the new capacity and frees the old buffer.
`0x4088f0` and `0x4084b0` free a dword buffer (and, for the second, the array with bit 0 of its
argument), `0x408440` destroys the outer array, `0x408400` and `0x408420` free vector and float buffers,
`0x408a30` clears four dwords, `0x4084f0` six, `0x4078e0` stores one float in a vector's three
components, `0x408510` frees and clears a dword array, and `0x4088c0` stores the placeholder vtable
`0x7ffdbc` over a cleared body.
