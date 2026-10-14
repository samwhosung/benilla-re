# ffxeffects: overview

The full-screen effects: the glow (with its underwater variant) and the death tint, drawn as a
chain of passes between the world and the screen. The code only prepares each pass; the blur,
glow and tint are the device's fragment programs.

It is one object file, `0x6ca9d0` through `0x6cdf20`, which carries the source path
`FFXEffects.cpp` (`0x86f1c4`); the passes have no assertions, so the first reference to the path is
`0x6cbed3`. Its initializers are the twelve slots `0x82cdb4` to `0x82cde0`. The classes are
`FFX::Pass` (the type tag `.PAVPass@FFX@@`, `0x86f228`) and its subclasses, one per program:
`FFXBox4`, `FFXGauss4`, `FFXGlow`, `FFXGlowWave` and three death programs. The effect objects own
lists of passes; `0x6cd890` and `0x6cda70` bracket the world paint with the current effect.

The docs split by topic: `objects` (the pass and effect objects, their lists, start-up, shutdown and the bracket around the world paint), `targets` (the render targets, the pass draw state and the quad) and `passes` (the blurs, the glow combine and its underwater variant, the drunk byte readers and the death passes). This doc holds the object file and its static initializers.

Roots: `0x6cbbc0`, `0x6cbc30`, `0x6cbc90`, `0x6ccb20`, `0x6ccb90`, `0x6ccc00`, `0x6ccc70`, `0x6ccce0`, `0x6ccd40`, `0x6ccda0`, `0x6ccde0` and `0x6cced0`.

## Static initializers

The twelve initializer slots each hold a thunk that calls a body and registers an exit stub with
`_atexit`. The bodies set: the wave map descriptor fields (`0x6cbbd0`); the glow wave periods
`0xce89c4 = 3174` and `0xce89c8 = 2805` (`0x6cbc40`); the scales `0xce89bc = 1.0` and `0xce89c0 =
0.88` (`0x6cbca0`); the screen descriptor (`0x6ccb30`); the full and twin targets (`0x6ccba0`), the half
pair (`0x6ccc10`) and the quarter pair (`0x6ccc80`); the position array `0xce8ab8` (`0x6cccf0`) and
UV array `0xce8a98` (`0x6ccd50`); the draw descriptor `0xce8a88` (`0x6ccdb0`); and the two identity
matrices `0xce8a48` and `0xce8a08` (`0x6ccdf0`, `0x6ccee0`).

Addresses: `0x6cbbc0`, `0x6cbc30`, `0x6cbc90`, `0x6ccb20`, `0x6ccb90`, `0x6ccc00`, `0x6ccc70`, `0x6ccce0`, `0x6ccd40`, `0x6ccda0`, `0x6ccde0`, `0x6cced0`, `0x6cbbd0`, `0x6cbc40`, `0x6cbca0`, `0x6ccb30`, `0x6ccba0`, `0x6ccc10`, `0x6ccc80`, `0x6cccf0`, `0x6ccd50`, `0x6ccdb0`, `0x6ccdf0`, `0x6ccee0`, `0xce8a88`, `0xce8a98`, `0xce8ab8`, `0x82cdb4`, `0x82cde0`, `0x6ccb60`, `0x6ccb70`, `0x6ccbd0`, `0x6ccbe0`, `0x6ccc40`, `0x6ccc50`, `0x6cccb0`, `0x6cccc0`, `0x6ccd10`, `0x6ccd20`, `0x6ccd70`, `0x6ccd80`, `0x6ccea0`, `0x6cceb0`, `0x6ccf90`, `0x6ccfa0`, `0x6cbc00`, `0x6cbc10`, `0x6cbc60`, `0x6cbc70`, `0x6cbcc0`, `0x6cbcd0`
Confidence: verified
Proof: one
Test: none

Each thunk is `call body; jmp` the registrar (`push stub; call 0x409aef; pop; ret`), except
`0x6ccda0`, a `jmp` to its body. The stubs (`0x6cbc10`, `0x6ccb70` and the like) set one bit of a
flag byte (`0xce899c`, `0xce8b94`, `0xce8b58`) and do nothing else. The draw descriptor holds 4, 0,
the words 4 and 0, and 3, which `0x58a830` takes as counts and the strip type. The slot run
`0x82cdb4` to `0x82cde0` is followed by `0x82cde4`, which opens `0x6cdf50`'s file; the slot below
it, `0x82cdb0`, is `0x6ca170`, in the Warden file. The file's extent rests on its variables: a scan of
the listing and of every dword of the image finds 331 references to `0xce898c` through `0xce8bb4`, all
in `0x6ca9d0` to `0x6cdf20`; the control is `0x6ca960`, the last Warden function, which reads
variables just below them.
