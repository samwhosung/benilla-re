# ui: input across a world transfer

What the player's input does while the loading screen is up and after it drops: which of the interface's handlers
the screen's veto starves, what the world frame still does at the frozen cursor, why nothing is pumped during
the blocking stretch, what a movement key held through a map change does, and which cursor shows. The screen's
veto is derived below (`0x4069e0` registers it, `0x406a50` is it); the transfer packets are in `collision/teleport`.

Roots: `0x765bf0`, `0x7660d0`, `0x481790`, `0x4908c0`

## What the veto starves

The top frame registers its input handlers on twelve categories, and the screen's veto, `0x406a50`, stops five of them: characters
(1), key down (8), key repeat (0xa), button down (0xb) and the mouse move (0xc). The move handler of the top is the
hover pick `0x7660d0`, so while the screen is up the pointer position the top holds and the frame it hovers do not
change. The other seven (9, 0xe, 0x10, 0xd, 0x1b, 2 and 0x1c) pass, so a release still completes what began before the screen. The vetoed events starve every binding and frame, never the held-key tables, which the dispatcher updates before it walks the handlers.

Addresses: `0x765bf0`, `0x765c04`, `0x765c1b`, `0x765c32`, `0x765c48`, `0x765c5f`, `0x765c76`, `0x765c8c`, `0x765ca3`, `0x765cba`, `0x765cd0`, `0x765cdd`, `0x765ce7`, `0x765cfe`, `0x7665c0`, `0x4069e0`, `0x4069f1`, `0x406a07`, `0x406a1d`, `0x406a33`, `0x406a49`, `0x406a50`, `0x7660d0`, `0x4245b0`, `0x4245d2`, `0x4245db`, `0x424710`, `0x4246ad`
Confidence: verified
Proof: one
Test: none

The passing categories are key up, button up, the wheel, the relative move, the input-method change, the activation change and the window size. `0x4069e0` registers the veto, `xor eax,eax; ret`, on categories 1, 8, 0xa, 0xb and 0xc with calls at `0x4069f1`, `0x406a07`, `0x406a1d`, `0x406a33` and `0x406a49`, each `push 0x41000000; push 0; mov edx,0x406a50; mov ecx,category; call 0x41fca0`. `0x765bf0` registers `0x765df0` for category 1, `0x765e80` for 0x1b, `0x765f10` for 8, `0x765fd0` for 9, `0x766070` for 0xa,
`0x7660d0` for 0xc, `0x766250` for 0xd, `0x7662c0` for 0xb, `0x766420` for 0xe, `0x7664f0` for 0x10, `0x7665c0` for 0x1c (`0x765ce7`) and `0x766650` for 2, twelve registrations, each through
`0x41fca0` (`0x765c04` to `0x765cfe`). `0x4245b0` runs the held-state update `0x424710` before the handler walk (`0x4245d2`,
`0x4245db`), and the walk stops at a zero answer (`0x4246ad`).

## The world frame under the screen

The world frame's update still runs on every frame the screen is up, because the layer walk calls the top's update
and that walks the frames. Its one gate is that the world frame is the top's hovered frame; it then picks at the
top's stored cursor point, which the veto froze, and sets the cursor from the result. Between the object manager's
teardown and the new local player's arrival the pick finds no object candidates and returns 0 on every frame, and
the update resets the cursor to its default. Before the teardown, and after the player exists, the pick is live at the frozen point.

Addresses: `0x481790`, `0x4817a8`, `0x4817ae`, `0x4817b1`, `0x4817c5`, `0x4817cb`, `0x481840`, `0x48184a`, `0x481855`, `0x481863`, `0x481875`, `0x481050`, `0x481063`, `0x481079`, `0x4810e6`, `0x481190`, `0x4812c1`, `0x4812c8`, `0x523d30`, `0x467700`, `0x401bf7`
Confidence: verified
Proof: one
Test: none

`0x481790` returns to its tail unless the top's hovered frame at `+0x7c` is the world frame (`0x4817ae`, `0x4817b1`). It reads the stored point at
`+0x1118` and `+0x111c` (`0x4817c5`, `0x4817cb`), calls the pick `0x481190` (`0x48184a`), and for an answer of 0 sets cursor 0x16 with a spell
target pending (`0x481855` to `0x481863`) and the default cursor `0x523d30` without one (`0x481875`). Nothing in it reads the screen's state. `0x481190` returns before tracing when the flag word is 0
(`0x4812c1`, `0x4812c8`); `0x481050` ORs `0x5c` in only when the local player's object resolves (`0x481063` to `0x4810e6`),
and the object manager that holds it is destroyed by `0x467700` at `0x401bf7` in the transfer's deferred body.

## Nothing is pumped during the load

The window messages are pumped in one place: `0x42c9f0`, called only from the input step `0x423920`, which the
engine loop calls once per iteration after it has run the due deadlines. A world change is posted as such a
deadline, so its whole body runs inside one iteration, before the input step, and does not return to the pump until
the load ends. The screen still moves its bar during that stretch because each progress setter paints directly (`0x406900`),
not through the layer walk. Messages that arrive wait in the window queue and the ring, and the first pumped
iteration afterwards drains and delivers them all in order; the veto then decides, category by category, which survive.

Addresses: `0x42c9f0`, `0x41fe50`, `0x422540`, `0x42cafa`, `0x42cb34`, `0x42cb60`, `0x42cb6a`, `0x423920`, `0x423954`, `0x423983`, `0x420d0c`, `0x420d23`, `0x428510`, `0x401ba5`, `0x401bae`, `0x401bc0`, `0x4200a0`, `0x406900`, `0x430290`, `0x66ae10`
Confidence: verified
Proof: one
Test: none

The four message functions of the import table are used in `0x42c9f0` (`0x42cafa`, `0x42cb34`, `0x42cb60`, `0x42cb6a`), in `0x430290`
and `0x66ae10`, which no call or stored address reaches, and in four thunks. `0x42c9f0` has two calls, in `0x423920`
(`0x423954`, `0x423983`), and `0x423920` is called from the loop (`0x420d23`) and from `0x41fe50` and `0x422540`, which nothing reaches. The loop drains the deadlines with `0x428510`
(`0x420d0c`) and then runs the input step. `0x401b00` posts `0x401bc0` with `0x4200a0` (`0x401ba5` to `0x401bae`).

## A held key across a transfer

A movement key held through a transfer does not start moving the player again. The transfer does not touch the
held-key tables, but the arrival of the new local player runs the enter-world cascade, which clears every movement bit.
A key pressed while the screen is up is vetoed, and once the screen drops the operating system's repeats of it arrive
as key-repeat events, because the table already holds the key down, and the world frame ignores those. So the player
stands still until the key is released and pressed again. A key released during the screen delivers its release
and finds the bits already clear.

Addresses: `0x4908c0`, `0x4908c7`, `0x4908ce`, `0x490935`, `0x49093c`, `0x5144c0`, `0x5144d2`, `0x5143e0`, `0x514b70`, `0x490a80`, `0x490a8d`, `0x4248a2`, `0x4248a5`, `0x4248b1`, `0x4248b3`, `0x766070`, `0x46ff90`, `0x42d0cb`, `0x423cc5`
Confidence: verified
Proof: one
Test: none

`0x4908c0` returns at once when the once-per-world latch is set (`0x4908c7`) and otherwise reaches `0x5144c0` through the
input object `0x5143e0` returns (`0x490935`, `0x49093c`). `0x5144c0` calls `0x514b70` with the mask −1, which removes every bit
(`0x5144d2`), and stores the time. `0x490a80` clears the latch (`0x490a8d`), so a transfer runs both. `0x4248a5` tests for a key-down event
and `0x4248b1` for a key the held table already holds; `0x4248b3` then stores category 0xa, whose handler `0x766070` calls the world
frame's repeat slot, the base stub `0x46ff90`. The held tables are cleared only by the window's activation event (`0x42d0cb`, `0x423cc5`).

## The cursor during a transfer

The cursor shown over the screen is the device cursor, composited with the frame, so it appears over the opaque
screen and over the hand-painted frames. The transfer announcement sets it to the plain pointer, index 1, as both the
current and the default cursor before the screen goes up, and nothing sets it again on a frame. The device's
show and hide is reached from mouselook, the cinematics, the glue cursor calls and the end of leaving the world, never from the screen itself.

Addresses: `0x401900`, `0x40194c`, `0x495920`, `0x495927`, `0x495932`, `0x6e4940`, `0x6e49f0`, `0x6e49f5`, `0x6e49ff`, `0x523d20`, `0x523c20`, `0x523d30`, `0x523790`, `0x58b590`, `0x46db65`, `0x46db72`, `0x48ed94`, `0x48f189`, `0x50fdf7`, `0x50fe4c`, `0x764da6`, `0x490bb6`, `0x490bbb`
Confidence: verified
Proof: one
Test: none

`0x401900` calls `0x495920` (`0x40194c`), which calls `0x495190` and `0x6e4940` (`0x495927`, `0x495932`). The tail of `0x6e4940`
stores 1 as the default cursor with `0x523d20` (`0x6e49f5`) and sets cursor 1 with `0x523c20` (`0x6e49ff`), which does nothing when the
index is unchanged. The shape loader `0x523790` has six references, all in the cursor module. The device's show and hide `0x58b590` has eight
callers: the glue cursor pair (`0x46db65`, `0x46db72`), the cinematic start and stop (`0x48ed94`, `0x48f189`), mouselook
(`0x50fdf7`, `0x50fe4c`), `0x764da6`, and the end of leave-world, which shows the cursor (`0x490bb6`, `0x490bbb`); none is in the screen.
