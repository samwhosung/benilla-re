# collision: world

Movement_C.cpp, `0x6307a0` through `0x631270`: the movement code that reaches other objects. It
keeps the movement globals with the object manager, a list of the transport game objects whose
type handler moves them every frame, the transport's matrix and facing that the transforms and the
resolvers read, the object a `CMovement` holds at +0xa4, and the notices the movement code sends
to the camera. The functions from `0x630c20` on are the file's static initializers, in `overview`.

The rest of the client reaches the list link and unlink (from the game object's transport type
handlers), one camera notice (from the unit code), the transport matrix (two callers outside
collision) and the list teardown, which the C runtime runs at exit.

Roots: `0x630800`, `0x6308b0`, `0x630920`, `0x6309f0`, `0x630ac0`

## Movement globals

`MovementSetGlobals` (`0x630a30`) and `MovementGetGlobals` (`0x630a40`) are tail jumps to
`0x468600` and `0x4685e0`, which store ecx at, and return, +0xd4 of the current object manager
`0xb41414`; with no manager the store is skipped and the get returns 0. The globals are one
312-byte block that `MovementInitialize` (`0x616860`) allocates and sets: a copy of its
260-byte path argument at +0x00, two handles at +0x104 and +0x108, an empty list at +0x118 to
+0x120, bit 0 of +0x124 set, and the clock at +0x128 and +0x12c. `MovementIdleMoveUnits`
(`0x616800`) reads +0x128 as the time of the last pass. `MovementDestroy` (`0x616910`) closes the
handles, empties the list, frees the block and sets the globals to null.

Addresses: `0x630a30`, `0x630a40`, `0x468600`, `0x4685e0`, `0xb41414`, `0x616860`, `0x616800`, `0x616910`, `0x64a5a0`, `0x73f285`, `0x6462e0`
Confidence: verified
Proof: one
Test: none

`0x630a30` is `jmp 0x468600` and `0x630a40` is `jmp 0x4685e0`. `0x468600` loads `[0xb41414]`,
skips on zero and stores ecx at +0xd4; `0x4685e0` returns that field, or the zero it loaded. In
`0x616860`, `SMemAlloc` (`0x6462e0`) gets 0x138 bytes with the Movement.cpp path and line 0x2b0;
+0x104, +0x108, +0x110, +0x114 and the list are zeroed, the list terminator at +0x11c points at
itself with its next pointer marked by bit 0, +0x118 (the link offset) is 0; `0x64a5a0`
(`SStrCopy`) copies at most 0x104 bytes; the clock is `0x42c010`'s. `0x616910` passes +0x104
and +0x108 to `0x73f285` (`_fclose`), so the two are open files; who opens them is not read here.
The getter has 28 call sites, two of them in the shared movement code.

## Transport update list

The list at `0xc4e4b4` links game objects through a link at +0x208: the offset 0x208, then a
terminator whose next pointer carries bit 0. `0x6308b0(object)` unlinks the object if it is
linked and appends it at the tail; `0x630920(object)` unlinks it; a null object stands for the
terminator. Both are called only by the two transport type handlers of a game object. Each pass,
`0x630970(now, elapsed)` calls slot +0x74 of the type handler (+0x210) of every listed object with
(now, elapsed), head to tail. `MovementIdleMoveUnits` calls it once per pass in which the clock has
advanced. `0x630800`, which the initializer `0x6307f0` registers with `_atexit`, unlinks every node
and then the terminator.

Addresses: `0xc4e4b4`, `0x6308b0`, `0x630920`, `0x630970`, `0x630800`, `0x6307f0`, `0x409aef`, `0x616800`, `0x616820`, `0x63099b`
Confidence: verified
Proof: one
Test: none

`0x6308b0` and `0x630920` take the object in ecx and form its link as `[0xc4e4b4]` + object, or
the terminator `0xc4e4b8` for 0. The unlink stores the previous link into the next link, found as
next & ~1 at the end or next + (link − prev.next) inside, then clears both words. The append sets
the link to (last, last.next), last.next to the object, and the terminator's last to the link.
The callers set the vtables the community calls `CGGameObject_C_Type_MapObjTransport` and
`CGGameObject_C_Type_Transport`; each links from its slots +0x60 and +0x6c and unlinks in its
destructor and its slot +0x68. `0x616820` passes now in ecx and now − the last pass in edx, after `jle` skips a zero or negative
difference; `0x63099b call [eax+0x74]` passes the handler at +0x210, not null-tested. The two
`.rdata` dwords equal to `0x630800` lie in tables of small packed values (their neighbours are
0x330800 and 0x230800), not code pointers; `0x6307f0` pushes it as an immediate.

## Transport matrix

`0x630ac0(M, guid)` writes the row-vector 4×4 matrix of the transport game object guid, mapping a
transport-space point q to q·M. With t the vector the object's vtable slot +0x14 returns, and c and
s the float cosine and sine of the angle slot +0x18 returns, the rows are:

| row | values |
|---|---|
| 0 | c, s, 0, 0 |
| 1 | −s, c, 0, 0 |
| 2 | 0, 0, k, 0 |
| 3 | t.x, t.y, t.z, 1 |

k = float(float(1 − c) + c) is one unit in the last place below 1.0 for some negative c.
`0x468460` finds the object with the type mask 0x20, and no null test follows: a GUID naming no
game object faults. The slots return the type handler's position and facing, or without a handler
the descriptor's fields +0x24 and +0x30.

Addresses: `0x630ac0`, `0x468460`, `0x7bdc40`, `0x7bdd60`, `0x630af3`, `0x630b30`, `0x630b38`, `0x630b4d`, `0x630b56`
Confidence: verified
Proof: one
Test: none

`ret 8`, M in ecx. `0x630afa` to `0x630b2d` store the identity. `0x630b30 mov edx,[edi]` uses the
lookup's return (`0x630af3`) unguarded. `0x630b38` calls slot +0x14 with a local vector and
`0x7bdc40` adds it to row 3. `0x630b4d` calls slot +0x18, which takes no argument: the `push 1`
and the axis (0, 0, 1.0) pushed before it belong to `0x630b56`, `0x7bdd60(M, angle, axis, 1)`.
That builds the rotation R with `fsincos` (cosine and sine stored as floats; flag 1 skips the axis
normalisation): R[0] = c, R[1] = s, R[4] = −s, R[5] = c, R[10] = (z·z)·(1 − c) + c; then M = R·M,
whose element [0][1] is R's row 0 times M's column 1 and whose sixteen values are stored in order.
The game object vtable's slot +0x14 returns the handler's slot +0x44 or the descriptor position,
slot +0x18 the handler's slot +0x48 or the descriptor facing. 1 − c is rounded to float before c
is added back, which gives the k below 1.0.

## Transport queries

`0x630b70(guid)` returns the transport game object's facing, its vtable slot +0x18, unguarded.
`0x630a80(guid)` returns bit 3 of the dword at +0xc of the object's field block (+0x110), 0
when no game object has the GUID. `0x630ba0(point, guid)` returns the object's slot +0x84 for
the point, which forwards to the type handler's slot +0x54, unguarded. `0x6309b0(mover, guid)`
passes the `CMovement` to the type handler's slot +0x4c, which unlinks the movement's link at
+0x08 and links it into the handler's own list, and returns 1; it returns 0 when no game object
has the GUID.

Addresses: `0x630b70`, `0x630a80`, `0x630ba0`, `0x6309b0`, `0x468460`, `0x630b90`, `0x630aa4`, `0x630bc4`, `0x6309d1`, `0x6309e1`
Confidence: verified
Proof: one
Test: none

Each looks the GUID up with `0x468460` and the mask 0x20, passing the Movement_C.cpp path and a
line. `0x630b90 call [edx+0x18]` leaves the facing in st0 through `ret 8`. `0x630aa4` reads
`[eax+0x110]`, then `shr eax,3; and eax,1` on its +0xc. `0x630bc4 call [edx+0x84]` passes ecx's
point; the object's slot +0x84 calls `[handler+0x54]`, which for both transport handlers returns 1
without a model and otherwise a model test not read here. `0x6309d1` returns 0 on a null object;
`0x6309e1 call [edx+0x4c]` on the handler at +0x210. Both transport handlers' slot +0x4c unlink the
argument's +0x08 link and link it at the handler's list (link offset at +0x14, terminator +0x18).
That the bit tested by `0x630a80` is the game object's transport flag is inferred.

## Camera notices

`0x6309f0(guid, transport)`: when the active camera follows the object guid, it hands the camera
the new transport GUID through `0x511da0`, which turns the camera by the old transport's facing and
back by the new one's when they differ. `0x630bd0(arg, guid)`: when the active camera follows the
object guid, it makes two calls into the world and object layers with arg in ecx, not read here.
The active camera is `0x4818f0`'s return; the GUID it follows is its +0x88.

Addresses: `0x6309f0`, `0x630bd0`, `0x4818f0`, `0x511da0`, `0x6309fe`, `0x630a18`, `0x630be1`
Confidence: inferred
Proof: none
Test: none

`0x6309f0` is `ret 0x10`: `0x6309fe` and the next compare match the camera's +0x88 and +0x8c with
the first GUID, and `0x630a18` calls `0x511da0` with the second. `0x630bd0` is `ret 8` and compares
the same fields (`0x630be1`). `0x511da0` cites the source path Camera.cpp and, when the transport
changes, rotates by the old transport's slot +0x18 facing and by the negated new one; that the
returned object is the camera the player sees, and what the two world calls do, is inferred.

## Spline object

`0x630c40()` allocates the 112-byte object a `CMovement` keeps at +0xa4 (`SMemAlloc`, the
Movement_C.cpp path, line 121), zeroes +0x08 to +0x13, +0x34 to +0x53 and +0x58 to +0x6b, stores
the vtable 0x7ffd84 at +0x2c and 1 at +0x54, and returns it, or 0 when the allocation fails.
`0x630ca0(object)` unlinks its link at +0x64, frees the buffer of the record at +0x44, runs a no-op
over each 12-byte element of the array whose count and buffer are +0x38 and +0x3c, frees that
buffer and frees the object. `0x630d70(link,
offset)` returns the link after link: the terminator, bit 0 cleared, at the end; otherwise the
next node plus offset, or, for a negative offset, plus the link's own distance from its node.

Addresses: `0x630c40`, `0x630ca0`, `0x630d70`, `0x6462e0`, `0x646430`, `0x7ffd84`, `0x7c62c0`, `0x7c6310`, `0x630c76`, `0x630cb9`, `0x630ce7`
Confidence: verified
Proof: one
Test: none

`0x630c4e` passes 0x70 bytes and line 0x79. `0x630c76` stores 0x7ffd84, a vtable whose first slot
reads the count at +0xc minus 3. `0x630cb9` calls `0x630d70` with offset −1 and moves the
neighbours as in any unlink. `0x630ce7` calls a function that frees its object only when bit 0 of
its argument is set, here 0. `0x630d70` is `ret 4`: `test al,1; jne` and `test eax,eax` take the end
case; a negative offset becomes link − prev.next. The kinematics code allocates through
`0x7c62c0` and frees through `0x7c6310`; that the object is the movement spline is inferred
from that use.

## Empty notices

`MovementNotifyZoneMgr` (`0x6307a0`) is `ret 8`; the anchor snapshot `0x7c5cd0` passes it the
owner's GUID. `0x630c10` is `ret 0x10`; `0x7c5950` and `0x7c6790` pass it the owner's GUID and the
transport GUID being dropped. Neither does anything.

Addresses: `0x6307a0`, `0x630c10`, `0x7c5cd0`, `0x7c5950`, `0x7c6790`
Confidence: verified
Proof: one
Test: none

Each body is its one `ret`, followed by padding. `0x6307a0` has one call site and `0x630c10` two.

## Unreached functions

`0x630a50(guid)` looks up the game object and calls its vtable slot +0x7c with the vector in
ecx, without a null test, returning the vector; the slot forwards to the type handler's slot
+0x50, which writes zero for a map-object transport and the speed times the direction for a
transport. Nothing live reaches it.

Addresses: `0x630a50`, `0x514120`, `0x8500b8`
Confidence: verified
Proof: one
Test: none

The census is a scan for `call` and `jmp` targets and for the address as a dword anywhere in the
image, which finds nothing for `0x630a50`; the same scan finds `0x514120` in the table
`0x8500b8`. The function above, `0x630a40`, ends in `jmp`, and padding precedes `0x630a50`.
