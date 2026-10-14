# ffxeffects: pass objects and effects

The objects that make an effect: the `FFX::Pass` base and its subclasses, the effects that own
lists of passes, the pass list container, the effects' start-up and shutdown and the bracket that runs the
current effect around the world paint.

Roots: `0x6ccfc0`, `0x6cd1d0`, `0x6cc130`, `0x6cc690`, `0x6cd890`, `0x6cda70`, `0x6cde60`

## Pass and effect objects

A pass (`0x6cde70`, the `FFX::Pass` constructor) copies up to 8 source target pointers to `+4` and
keeps the destination target at `+0x24`, a dword at `+0x28` and a flag byte at `+0x2c`; a subclass
adds its program handles from `+0x30`. Its vtable is `0x81148c` (deleting destructor `0x6caa90`, two
pure slots) or a subclass's: slot 0 deletes, slot 1 asks whether the program loaded, slot 2 renders.
An effect holds its CVar record at `+4` and two pass lists, at `+8` and `+0x18`, each `{capacity,
count, data, quantum}`. Its slots `0x6cded0` and `0x6cdf20` run slots 1 and 2 over list A.

Addresses: `0x6cde70`, `0x6caa90`, `0x6cded0`, `0x6cdf20`, `0x86f228`, `0x6ca9d0`, `0x6caa30`, `0x6caa60`, `0x6caac0`, `0x6cac30`, `0x6cac90`, `0x6cacc0`, `0x6cacf0`, `0x6caf40`, `0x6cafa0`, `0x6cafd0`, `0x6cb000`, `0x6cb1f0`, `0x6cb290`, `0x6cb2c0`, `0x6cb2f0`, `0x6cb7e0`, `0x6cb890`, `0x6cb8c0`, `0x6cb910`, `0x6cba60`, `0x6cbaa0`, `0x6cbad0`, `0x6cbaf0`, `0x81148c`, `0x811480`, `0x811498`, `0x8114a4`, `0x8114b4`, `0x8114c0`, `0x8114cc`, `0x8114d8`, `0x8114e4`, `0x8114f0`, `0x8114fc`
Confidence: verified
Proof: one
Test: none

`0x6cde7c` stores `0x81148c` and `rep stos` zeroes the 8 source slots (`0x6cde89`); the loop at
`0x6cdea0` copies the pointers of the array argument, as many as its count argument; `+0x24`,
`+0x28` and `+0x2c` take the third to fifth arguments (`0x6cdeb4`, `0x6cdebe`, `0x6cdebb`).
Subclass vtables: Box4 `0x811480`, Gauss4 `0x811498`, Glow `0x8114a4`, GlowWave `0x8114b4`, death
`0x8114c0`, the combiner death pass `0x8114cc`, the wave map base `0x8114d8`; effects `0x8114e4`
(glow), `0x8114f0` (base) and `0x8114fc` (death). Each subclass has a constructor, a deleting
destructor, a destructor body and a readiness test at the addresses listed; the pass lists are
tagged `.PAVPass@FFX@@` (`0x86f228`). `0x6cded0` returns 0 for an empty list, else the
AND of slot 1 over the list with the effect's CVar integer (`[[this+4]+0x28]`, a null record
giving the list result alone); `0x6cdf20` calls slot 2 on each pass in order.

## Effect pass lists

`0x6cc130` builds the glow effect: it registers the CVar `ffxGlow` and fills list A with Box4 (full
target to quarter), Gauss4 (quarter) and Glow (full and quarter to the screen), and list B with
Box4, Gauss4 and GlowWave (wave map, full and quarter to the screen). `0x6cc690` builds the death
effect; the capability word `caps+0x98` picks its list: 5 gives the combiner pass alone, 0 or more
(other than 5) gives Box4, Gauss4 and the death pass, a negative value gives none. The glow effect
runs list B when `0x467d00()` is nonzero and the camera liquid state `0x672470(0)` is not 0xf, else
list A.

Addresses: `0x6cc130`, `0x6cc690`, `0x6cc5a0`, `0x6cc630`, `0x6cc4a0`, `0x6cc4d0`, `0x6cc560`, `0x6cc8c0`, `0x6cc8f0`, `0x467d00`, `0x672470`, `0x6cc375`, `0x6cc6de`, `0x6cc73e`
Confidence: verified
Proof: one
Test: none

The passes are allocated by `0x6462e0` at `0x3c` bytes (Box4, Gauss4, Glow), `0x4c` (GlowWave),
`0x40` (death) and `0x34` (combiner). `0x6cc375` appends a second Glow instead of the GlowWave only
when `0xce89f8` is nonzero and `caps+0x5c` is nonzero. `0x6cc5a0` returns the CVar's integer `+0x28`
nonzero ANDed with slot 1 of each pass in the chosen list; `0x6cc630` renders them in order. The
test is the liquid state against 0xf alone, so water, ocean, magma and slime (`terrain/camstate`) all select list B. The death effect has one list and none of the nine callers of `0x672470` is in its code, so a ghost gets neither the warp nor the blur floor. `0x6cc6de` and
`0x6cc73e` compare `caps+0x98` with 5 and with −1.

## Pass list container

The pass lists are growable arrays of pass pointers with the tag `.PAVPass@FFX@@`: `0x6cc940`
(`ecx` the list, an extra count and a flag) grows the capacity to hold `count + n`, rounded up to a
multiple of the quantum; `0x6cca30` sets the quantum (0x40 when the request is 0x40 or more, else the
largest power of 2 not above it, at least 1); `0x6cca70` rounds a size up to a multiple; `0x6cca90`
reallocates to a capacity, keeping the first `min(old, new)` pointers.

Addresses: `0x6cc940`, `0x6cca30`, `0x6cca70`, `0x6cca90`, `0x6cc968`, `0x6cc999`, `0x6cc9a1`, `0x6cc9c1`, `0x6cc9d6`, `0x6cc9f4`
Confidence: verified
Proof: one
Test: none

`0x6cc968` compares with 0x40 and stores 0x40 at `+0xc`; `0x6cc976` to `0x6cc97d` clears the lowest
set bit until one remains; `0x6cc999` is `div ecx` and `0x6cc9a1` adds `quantum − remainder`.
`0x6cc9c1` calls the reallocator `0x646320`, `0x6cc9d6` the allocator when the reallocation fails,
and the loop at `0x6cc9f4` copies the pointers.

## Start and shutdown

`0x6ccfc0` registers the CVars `ffx` (`full screen effects`) and `ffxRectangle` (`use rectangle
texture for full screen effects`), each with default `1` and the handler `0x6cde30`, looks up
`gxMultisample`, clears the target descriptors, and sets `0xce89f8` to 1 when `ffxRectangle`'s
integer is nonzero and `caps+0x98` is 6, 7, 8 or 10, else 0. `0x6cde30` prints `enabled` for a
nonzero `atoi` of the new value and `disabled` otherwise, and returns 1; the effects register
`ffxGlow` and `ffxDeath` with it too. `0x6cd1d0` destroys the textures of the seven target
descriptors (`0x58ad60` where a handle is set) and zeroes them.

Addresses: `0x6ccfc0`, `0x6cd1d0`, `0x6cde30`, `0x6cde60`, `0x6cd16f`, `0x6cd1ac`, `0x6cd390`, `0xce89f8`, `0xce89fc`, `0xce8a00`, `0xce8a04`, `0xce8bb4`, `0x46a700`, `0x481c46`, `0x481e4e`, `0x46a7a0`, `0x481cdb`, `0x481c63`, `0x481c8d`, `0x5de9c0`, `0xb4b350`, `0xb4b39c`
Confidence: verified
Proof: one
Test: none

The registrations are `0x6ccfe3` (`ffx`) and `0x6cd007` (`ffxRectangle`); the records go to
`0xce8a04` and `0xce8a00`, and `0x63de30` returns `gxMultisample`'s to `0xce89fc`. `0x6cd16f` tests
`[[0xce8a00]+0x28]`, `0x6cd179` to `0x6cd1aa` compare `caps+0x98` against 0xa, 6, 7 and 8, and
`0x6cd1ac` stores 1. The callers of `0x6ccfc0` are glue (`0x46a700`) and the world frame
(`0x481c46`), which then build the effects. `0x6cd1d0` is called from `0x481e4e` and jumped to from
`0x46a7a0`. `0x6cde60` stores the current effect (`0xce8bb4`) and returns the old one; its six
callers are glue, the world frame (`0x481cdb`, with the glow effect `0xb4b350` or the death effect
`0xb4b39c` its `0x481c63` and `0x481c8d` built) and `0x5de9c0`, which makes the death effect current
while `PLAYER_FLAGS` bit 0x10 is set and the glow effect otherwise. An effect is a single slot:
the death effect replaces the glow effect.

## Begin and end

`0x6cd890` runs before the world paint and `0x6cda70` after. Both do nothing unless the `ffx`
CVar's integer, the `pixelShaders` CVar's integer (`0xc7f360`), the current effect `0xce8bb4` and its
slot 1 are all nonzero. Begin calls `0x58a1e0(6, 1)` on entry and `(6, 0)` once the checks pass, and
resizes the targets by `0x6cdb40` when the window size changed. With `gxMultisample` 1 it binds the
full target and sets the viewport to the window's extents over the full target's allocation (at most
1); otherwise to the full target's size over the window's (at most 1). End unbinds the full target
(`gxMultisample` 1) or copies the backbuffer into it (`0x58ae00`), renders the effect through slot 2,
and sets the whole viewport.

Addresses: `0x6cd890`, `0x6cda70`, `0x6cd93a`, `0x6cd944`, `0x6cd98a`, `0x6cd9f5`, `0x6cdaa8`, `0x6cdad9`, `0x6cdae6`, `0x58af60`, `0x58a240`, `0x58ae00`, `0x589d30`, `0xc7f360`, `0x46fad3`, `0x48350e`, `0x48379d`
Confidence: verified
Proof: one
Test: none

`0xc7f360` is the record of the CVar `pixelShaders` (registered at `0x688712`). `0x6cd918` and
`0x6cd925` take `__ftol` of `rect3 − rect1` and `rect2 − rect0` of `0x58a240`, compared at `0x6cd92a`
with `0xce8ba4` and `0xce8ba8`. `0x6cd944` tests `gxMultisample` against 1. `0x6cd98a` to
`0x6cd9ab` divide the window extents by the full target's `+4` and `+8` and replace a quotient that
is not below 1.0 by 1.0; `0x6cd9f5` on gives `full/window` where the full target is smaller than the window, else 1.0. The
viewport arguments are `(0, x, 0, y, 0, 1)`. The call sites are glue (`0x46fad3`; `0x46fae0` jumps
to `0x6cda70`) and the world frame (`0x48350e`, `0x48379d`).
