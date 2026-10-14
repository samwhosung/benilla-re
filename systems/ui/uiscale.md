# ui: The interface scale

How the `uiScale` and `useUiScale` options become the scale of the `UIParent` frame, which the frame
tree passes to every frame below it, and what the client does when the player does not set one.

`uiScale` is `Frame:SetScale` on the frame named `UIParent`. The effective-scale cascade it uses is in
`visibility` (`Scale`); the units and the screen size it changes are in `modelview`. The screen scale
globals (`screencoord`) do not change with it.

Roots: `0x490770`, `0x4907f0`, `0x492f70`, `0x494550`, `0x494590`

## The two options

`uiScale` (default `1.0`) and `useUiScale` (default `0`) are registered with category 5 and a change
callback each, `0x490770` and `0x4907f0`. Each callback also runs when the option is registered with a
value already set. The options are the pair stored at `0xb4e2b0` (`uiScale`) and `0xb4e2b4`
(`useUiScale`).

Addresses: `0x48fca9`, `0x48fcab`, `0x48fcad`, `0x48fcb2`, `0x48fcba`, `0x48fcc4`, `0x48fcc9`, `0x48fccd`, `0x48fcd2`, `0x48fcd7`, `0x48fcdf`, `0x48fce4`, `0x48fcfb`, `0x63db90`, `0x63dc3d`, `0x63dc4f`, `0x63dc58`, `0xb4e2b0`, `0xb4e2b4`
Confidence: verified
Proof: one
Test: none

The register call `0x63db90` takes the name in `ecx` (`0x8430cc` `uiScale`, `0x8430c0` `useUiScale`), the
default string (`0x82e92c` `1.0`, `0x82e570` `0`), category 5 and the callback. A call stores its result in
the next call's argument block: `0x48fcd7` stores `uiScale`'s handle and `0x48fcfb` `useUiScale`'s.
`0x63db90` stores the callback at `+0xbc` and calls it when the option already exists
(`0x63dc3d` to `0x63dc58`).

## Applying a scale

A change of `uiScale` is applied only while `useUiScale` is on. The value is capped, on windows whose
aspect `a` is under 4/3, to `0.75·a`; it is then raised to 0.64 if it is not above that, and passed to
`UIParent:SetScale` with the force flag, after which the layout queue is drained. A change of
`useUiScale` to on applies the current `uiScale` the same way; a change to off applies the automatic
scale. With `UIParent` absent nothing is set.

Addresses: `0x490770`, `0x490778`, `0x49077f`, `0x490787`, `0x49078f`, `0x49079b`, `0x4907a6`, `0x4907b7`, `0x4907c7`, `0x4907f0`, `0x4907fa`, `0x490801`, `0x490812`, `0x49086a`, `0x4908ad`, `0x494550`, `0x494556`, `0x494563`, `0x49456a`, `0x49457a`, `0x49457f`, `0x76ac10`, `0x768ed0`, `0xb4b44c`, `0xb4e30c`
Confidence: verified
Proof: one
Test: `ui::glue_48fbf0_finite_diff`, `ui::glue_490770_eps_gates`

`0x490770` returns without applying when `useUiScale`'s integer (`+0x28`) is 0 (`0x490781`). Otherwise it
converts the new text (`0x64aaa0`), multiplies the cached aspect `[0xb4e30c]` by 3 (`0x49079b`) and
compares with 4 (`0x4907a6` is taken for 4/3 and wider) and, when narrower, replaces the value by the
smaller of it and `0.75·a` (`0x4907b7`). `0x4907f0` parses its new value (`0x4907fa`); a nonzero value reads
the `uiScale` option's float at `+0x24` (`0x490812`) and applies the same cap, while zero reads the
device rectangle and calls `0x492f70` (`0x4908ad`). The sink `0x494550` compares with 0.64
(`0x804584`, `0x494556`) and stores 0.64 unless the value is above it (`0x494563`); `[0xb4b44c]` holds
`UIParent` (`0x49456a`), and it calls `0x76ac10(UIParent, v, 1)` (`0x49457a`) and drains the queue
`0x768ed0`.

## The fullscreen frame scale

`SetupFullscreenScale(frame)` sets the frame's own scale to the smaller of `0.75·a` and 1.0, where `a` is the
aspect of the `gxResolution` setting while `widescreen` is on and 4/3 otherwise, the same call `frame:SetScale` makes without the force flag. It reads no `uiScale`
and writes no anchor or size. A non-table argument raises `Usage: SetupFullscreenScale(frame)`, a table
without a frame behind it `SetupFullscreenScale(): Couldn't find 'this' in frame object`, and any other object
`SetupFullscreenScale(): Wrong object type, expected frame`. The call returns no value.

Addresses: `0x48c270`, `0x48c282`, `0x48c2b1`, `0x48c2e9`, `0x48c306`, `0x48c30e`, `0x48c31d`, `0x48c323`, `0x48c335`, `0x41ad70`, `0x41ad10`, `0x76ac10`, `0x6f4940`, `0x83e2b8`, `0x8012cc`, `0x8015b8`
Confidence: verified
Proof: one
Test: none

The binding is entry `SetupFullscreenScale` of the pair table at `0x83e2b8`, with no direct caller. It checks the argument
is a table (`0x48c282`), unwraps element 0 (`0x6f3bc0`, `0x6f3740`), rejects a null pointer (`0x48c2b1`) and
asks the object's type slot for the frame class id (`0x48c2e9`). `0x41ad70` loads `[0x832a4c]`, which `0x41ad10` sets
to `a` times `0.75` (`0x8012cc`); `fcomp` against 1.0 (`0x48c30e`) then takes the value when it is below 1.0 and
1.0 when it is equal, above or not a number (`0x48c31d`). It calls `0x76ac10(value, 0)` (`0x48c323`, `0x48c335`). A
widescreen window of 16:9 gets 1.0 and a 5:4 window 0.9375; with the widescreen option off the aspect is 4/3 and
the result is 1.0.

## The automatic scale

With `useUiScale` off, which is the default, `UIParent` is not left at 1.0. Its scale is 768 over the
window's height when the height is above 768, else 1.0; capped to `0.75·a` for an aspect under 4/3; and
raised to at least 0.9. It is applied when the display mode is set and when `useUiScale` is turned off.
The result is 0.9 at 1920×1080 and 2560×1440 and 0.96 at 1280×800.

Addresses: `0x492f70`, `0x492f8c`, `0x492fa1`, `0x492fb4`, `0x492fbc`, `0x492fc7`, `0x492fd3`, `0x492fe9`, `0x493005`, `0x493012`, `0x49301d`, `0x493028`, `0x49303a`, `0x4901e0`, `0x491380`, `0x804588`, `0x80458c`, `0x801360`, `0xb4e304`, `0xb4e308`
Confidence: verified
Proof: one
Test: `ui::glue_4901e0_aspect_ratio`, `ui::glue_492_492f70_recip_unit`

`0x492f70(force, width, height)` returns early when `force` is 0 and the width and height equal the
cached pair at `0xb4e304`/`0xb4e308` (`0x492f8c` to `0x492f99`), else stores them (`0x492fa1`). The scale
starts at 1.0 (`0x492fad`); `0x4901e0` gives the aspect (`gxResolution`'s width over height when
`widescreen` is on, else 4/3, `screencoord`); `cmp esi,0x300; jle` (`0x492fbc`) keeps 1.0 up to 768 and
`768.0 / height` (`0x80458c`) above it. The cap is as in the callbacks (`0x492fd3`, `0x492fe9`). The 0.9
floor (`0x804588`) is `v > 0.9` keeps, else `0x3f666666` (`0x493005` to `0x493012`). After `0x494550`
(`0x49301d`), a cached aspect that differs from the new one by 0.001 or more calls `0x491380`
(`CGGameUI::Reload`) and updates the cache (`0x49303a`).

## Reading the scale

`UIParent`'s effective scale is returned by one reader that the screen-size functions use; it is 1.0 when
`UIParent` does not exist. `GetScreenHeight()` is `768 / uiScale` at every aspect and `GetScreenWidth()`
is `768·a / uiScale`.

Addresses: `0x494590`, `0x494595`, `0x494599`, `0x49459d`, `0x48b480`, `0x48b4d0`, `0x490065`, `0x49006a`, `0x490077`, `0x76c760`
Confidence: verified
Proof: one
Test: `ui::luabind_48b_screen_getters`

`0x494590` loads `UIParent`'s `+0x7c` (`0x494599`) and falls back to 1.0 (`0x49459d`). `UIParent` is
the frame `0x76c760` finds by the name `UIParent` (`0x490065`, `0x49006a`) and `0x490077` stores. The
screen functions divide G48 (width: G44) by the reader and multiply by `0.75a · 1024`, then divide by G44
(`modelview#model-unit-to-pixel`).

Bit-exact coverage: `ui::luabind_48b_screen_getters` covers the whole of `0x494590`.

## SetupFullscreenScale

`SetupFullscreenScale(frame)` sets the frame's own scale to the screen aspect scale when that is below 1 and to 1 otherwise. A
non-frame argument raises `SetupFullscreenScale(): Wrong object type, expected frame`, an unresolved one `SetupFullscreenScale(): Couldn't find 'this' in frame object`, and a
missing one `Usage: SetupFullscreenScale(frame)`. The aspect scale is the aspect times 0.75, so it is 1 at 4:3 and wider screens give 1.

Addresses: `0x48c270`, `0x48c306`, `0x48c30e`, `0x48c323`, `0x48c335`, `0x41ad70`, `0x76ac10`
Confidence: verified
Proof: one
Test: none

`0x48c306` calls `GetScaledScreenAspect` and `0x48c30e` compares it with the double 1.0; `jp` after `test ah,5` jumps for equal or greater to the store of 1.0 at `0x48c330`, and the less case reaches `SetFrameScale` with the aspect at `0x48c323`.
