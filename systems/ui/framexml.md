# ui: loading the interface from toc and XML files

How the client turns `Interface\FrameXML\FrameXML.toc`, the glue's toc and each addon's toc into
frames and running Lua: the toc runner, the file loader with its `.lua` and XML arms, the elements
an XML file holds, templates, creating a frame from a node, the frame-type factories, script
elements, the load log and the sound suppressed while loading.

`FrameXML.cpp` (from 0x6ed930, ending before `XMLTree.cpp`) holds the toc runner `0x6edb90`, the
file loader `0x6ede10`, the frame creator `0x6ee280`, the template table and the factory table.
`XMLTree.cpp` builds the node tree with the linked expat 1.95.5. What a frame does with its own node
is in `frames` (`Loading from XML`), `anchors` and `visibility`; each widget type's own elements
are in `xmltypes`; the order a game interface is built in is here, under its builder in
`GameUI.cpp`.

Roots: `0x6edb90`, `0x6ede10`, `0x6ee280`, `0x6ee500`, `0x6ee6f0`, `0x6ee880`, `0x6ef070`, `0x6edb30`, `0x6edb40`, `0x6edb50`, `0x6f2a30`, `0x6f2c50`, `0x6f2cf0`

## Building the game interface

Among its steps, `CGGameUI::Initialize` runs, in this order: a fresh Lua state; the C functions; the
549 event names; the frame types of `GameUI.cpp`; `Logs\FrameXML.log` opened into a load status; the
signature check of `FrameXML.toc`, as the glue's (`glue/manager`), quitting with code 10 unless it
passes; an MD5 over `FrameXML.toc`, every file it loads and `Bindings.xml`, checked against the
signature's digest, quitting with 10 on a mismatch; the lookups of `UIParent` and `GameTooltip`;
every addon that is not loaded on demand; the saved variables. Each status line is then written to
the log as the log's format string, so a `%` in a line is read as a conversion: a fault of the
client.

Addresses: `0x48fbf0`, `0x48fe97`, `0x48fe9c`, `0x48feab`, `0x48feb5`, `0x48ff06`, `0x48ff4d`, `0x48ff57`, `0x48ff90`, `0x48ffed`, `0x490018`, `0x490026`, `0x490038`, `0x490043`, `0x49006a`, `0x490096`, `0x4900a3`, `0x4900b2`, `0x490187`, `0x4901a1`, `0x65ace0`, `0x842ecc`, `0x842f7c`, `0x842f9c`, `0x6f10f0`, `0x401560`
Confidence: verified
Proof: one
Test: `ui::glue_48fbf0_finite_diff`

`0x48fe97` calls `0x703b80`, `0x48fe9c` `0x490250`, `0x48feab` `0x703d90(0xbe1198, 0x225)`,
`0x48feb5` `0x495940`, `0x48ff06` `SLogCreate` on `0x842ecc`. `0x48ff4d` calls `0x6f10f0` on
`0x842f7c` and `0x842f9c`; `0x48ff57` jumps through `0x4901c8`: answers 0 to 2 add a line and fall
to `0x48ff90`, `0x401560(10)`, and 3 goes on. `0x48ffed` runs `0x6edb90` with the MD5 context;
`0x490018` loads `Bindings.xml` with it; `0x490026` finishes it and `0x490038` compares four dwords
with the signature's, quitting at `0x490043`. `0x49006a` and `0x490096` look up `UIParent` and
`GameTooltip` with `0x76c760`. `0x4900a3` calls `0x51f600`, `0x4900b2` `0x4913b0`. The loop at
`0x490187` passes each status line as the format of `SLogWrite` (`0x4901a1`); `0x65ace0` is
variadic and hands the list at `[ebp+0x10]` to `0x65ac20`.

## Running a toc

`0x6edb90(toc, md5, status)` reads the toc whole, adds its bytes to the MD5, skips a leading `EF BB
BF`, and splits the text at CR and LF, skipping empty lines and keeping at most 1,023 bytes of a
line. Each line that does not begin with `#` is appended to the toc's directory, up to and including
its last backslash, stripped of trailing spaces (not tabs), and loaded with `0x6ede10`. The result
of each load is ignored, and a progress callback runs after each line. It answers 0 only when the
toc does not open; then `Couldn't open %s` is reported.

Addresses: `0x6edb90`, `0x6edbe5`, `0x6edc28`, `0x6edc66`, `0x6edc71`, `0x6edcc6`, `0x6edcd9`, `0x6edd24`, `0x6edd51`, `0x6edd91`, `0x6eddf0`, `0x82edfc`, `0x6edb50`
Confidence: verified
Proof: one
Test: `ui::glue_geom_6edb90_progress_ratio`

`0x6edbe5` finds the last `\`; the prefix is copied when it is shorter than 0x103 bytes. The toc is
read with one byte of padding (`0x6edc28`) and hashed (`0x6edc66`); `0x6edc71` tests the three
bytes. `0x6edcc6` tokenizes with `0x82edfc` (CR, LF) into 0x400 bytes, `SStrTokenize` skipping
leading separators; an empty token ends the loop, `#` skips the
line (`0x6edcd9`), and `0x6edd24` trims spaces backwards. `0x6edd51`'s `eax` is overwritten before
any test. `0x6edd91` calls `[0xcee9b8]` with the count `0xcee9fc` over `0xceea3c`; `0x6eddf0`
answers 1. `0x6edb50` counts the lines not beginning with `#`, for that progress total.

## The signature digest

`0x6f1200(toc, file, digest)` makes the 16 bytes `CheckTocSignature` compares with a signature. It
starts an MD5 context with `MD5Init`, passes it to `0x6f1270` with the toc and the empty string,
and, when `file` is not null, loads that file with `SFile::Load`, adds it with `0x661dd0` and frees
it with `SFile::Unload`; then it calls `MD5Final` with `digest`. `0x661dd0(context, input, length)` is
MD5's update: it adds `8·length` to the 64-bit bit count at `+0x10` with a carry into `+0x14`, fills
the 64-byte block at `+0x18` from index `(count >> 3) & 63`, runs the block transform `0x661f00`
on it when full and then on each whole 64 bytes of the input in place, and keeps the rest.

Addresses: `0x6f1200`, `0x6f120f`, `0x6f121e`, `0x6f1238`, `0x6f124b`, `0x6f1254`, `0x6f125f`, `0x6f11dc`, `0x6f11eb`, `0x661dd0`, `0x661dfd`, `0x661e1b`, `0x661e34`, `0x661e4f`, `0x661f00`, `0x661f3e`, `0x661f63`
Confidence: verified
Proof: one
Test: none

`0x6f11dc` passes the toc in `ecx`, `CheckTocSignature`'s own `edx` and its output buffer;
`0x6f11eb` compares four dwords. `SFile::Load` gets `(0, file, &buffer, &size, 1, 1, 0)`
(`0x6f1238`). In `0x661dd0`: `jae` past `inc [ctx+0x14]` (`0x661dfd`) is the carry, and the high
word also gains `length >> 29`; `0x661e1b` `jb` takes the short copy when `length < 64 − index`;
`0x661e34` transforms the filled block, and the loop at `0x661e4f` runs while `i + 63 < length`.
`0x661f00` adds the MD5 constants 0xd76aa478 (`0x661f3e`) and 0xe8c7b756 (`0x661f63`).

## Loading one file

`0x6ede10(path, md5, status)` first collapses `..`: while the path holds a `..` with a separator on
both sides, it removes it with the directory before it; the first `..` not so bounded stops the
collapse. Then a path whose text from its last `.` is `.lua`, in any case, runs as a Lua file;
anything else is read, hashed and parsed as XML, and its top-level elements are loaded. A Lua file
runs as the chunk `@path`; a missing one reports `Error loading %s`. A compile error is reported and
then passed to the error handler; a run-time error goes to the handler and is then reported.

Addresses: `0x6ede10`, `0x6ede40`, `0x6ede73`, `0x6edea0`, `0x6edec2`, `0x6edeea`, `0x6edefe`, `0x6edf0f`, `0x6edf2f`, `0x6edaa0`, `0x6edabd`, `0x6edaed`, `0x704bc0`, `0x704be1`, `0x704c32`, `0x704ae0`, `0x704b31`, `0x704b42`, `0x704b68`, `0x704b8d`, `0x8710c8`, `0x833188`, `0x872e50`
Confidence: verified
Proof: one
Test: none

`0x6ede40` searches `..` (`0x833188`); `0x6ede73` requires `\` or `/` before it and two after,
`0x6edea0` walks back to the previous separator and `0x6edec2` copies the rest over it. `0x6edeea`
finds the last `.`, `0x6edefe` compares it with `0x8710c8`, and `0x6edf0f` calls `0x704bc0`; else
`0x6edf2f` calls `0x6edaa0`, which reads the file (`0x6edabd`) and hashes it (`0x6edaed`).
`0x704bc0` formats `@%s` (`0x704be1`), hashes when given a context (`0x704c32`) and runs
`0x704ae0`. On a compile failure `0x704ae0` reports the message (`0x704b31`) and calls the handler
with it, `lua_pcall(1, 0, 0)` (`0x704b42`); otherwise it runs the chunk under the handler
(`0x704b68`) and reports a failure (`0x704b8d`).

## The elements of a file

The root's tag is not checked. Each child is: `Include`, whose `file` is appended to
the including file's directory and loaded through `0x6ede10`, so `..` and `.lua` work;
`Script`, whose `file`, used as written when it holds a backslash and appended to the directory
otherwise, runs as a Lua file with no collapse, and whose body, when not empty, then runs as the
chunk `<path>:<Scripts>`; `Font`, a named font; or a frame element, registered as a
template when its `virtual` is `true` and created otherwise. Nothing limits nesting or detects an
include cycle. An including path with no `\`, or a directory of 0x104 bytes or more, makes
`Include` load a stale buffer, a fault of the client.

Addresses: `0x6edf77`, `0x6edf9b`, `0x6edfbe`, `0x6edfc5`, `0x6edfd2`, `0x6ee000`, `0x6edffb`, `0x6ee00d`, `0x6ee017`, `0x6ee03d`, `0x6ee070`, `0x6ee0bf`, `0x6ee0d4`, `0x6ee0ff`, `0x6ee10f`, `0x6ee127`, `0x6ee14c`, `0x6ee158`, `0x6ee162`, `0x6ee190`, `0x6ee1bd`, `0x6ee1c4`, `0x6ee1e1`, `0x6ee1e6`, `0x8710c0`, `0x8710b8`, `0x871084`, `0x87106c`, `0x871044`, `0x871074`, `0x87108c`, `0x87104c`, `0x871020`
Confidence: verified
Proof: one
Test: none

`0x6edf77` takes the root's first child; each tag is compared ignoring case. `Include` without
`file` reports `0x87108c`; with it, `0x6edfbe` finds the last `\` of the current path and
`0x6edffb` appends before the recursion `0x6ee00d`. No `\` (`0x6edfc5`) or a directory of 0x104
bytes or more (`0x6edfd2`) jumps straight to `0x6ee000`, which loads `[ebp-0x33c]`, written only
by the arm it skips. `Script`: `0x6ee070` tests `file` for `\`,
`0x6ee0bf` appends otherwise, `0x6ee0d4` runs the file, then `0x6ee0ff` formats `0x871074` and
`0x6ee10f` runs the body. `Font` needs a `name` (`0x87104c` otherwise): `0x783870` then `0x783c30`.
Any other tag: `virtual` equal to `true` and a `name` go to `0x6ee500` (`0x871020` without a name);
else `0x6ee280` with no parent and then `LayoutPendingFrames` (`0x6ee1e6`). The recursion carries no
depth count.

## Templates

A template is a node kept by name in its own table, with the context word current at its
registration. Names compare ignoring case. A name already held by the client's own code (context 0)
cannot be registered again; one held by an addon is replaced only by the client's own code; a
second addon's fails with `Virtual object named %s already exists`. `inherits` names one template:
there is no list, and a miss reports `Couldn't find inherited node: %s` and loads the node alone.
A hit re-enters the frame's own load with the template's node first, so a template's regions and
scripts are applied before the frame's own.

Addresses: `0x6ee500`, `0x6ee561`, `0x6ee594`, `0x6ee5a4`, `0x6ee5c1`, `0x6ee6bb`, `0x6ee6c1`, `0x6ee6f0`, `0x64b3f0`, `0x64a4c0`, `0x769820`, `0x76984a`, `0x76985c`, `0x769861`, `0xceea1c`, `0x871188`, `0x8786a8`
Confidence: verified
Proof: one
Test: none

`0x6ee500(node, name, context, status)` hashes the name with `SStrHashHT`, which upper-cases `a`
to `z`, and compares with `SStrCmpI` (`0x6ee561`) in `0xceea1c`. On a hit `0x6ee594` reads the
held context: 0, or a nonzero new context, reports `0x871188` (`0x6ee5a4`); otherwise `0x6ee5c1`
frees the old node. `0x6ee6bb` stores the node and context and `0x6ee6c1` detaches the node from its
file's tree. `0x6ee6f0` answers the node or 0. `0x769820` reads `inherits`, looks it up
(`0x76984a`) and calls its own slot 8 with the template (`0x76985c`), or reports `0x8786a8`
(`0x769861`); `0x6ede10` passes `[0xceeac0]` as the context.

## Creating a frame from XML

`0x6ee280(node, parent, status)` finds the factory by the element's tag, ignoring case; an unknown
tag reports `Unknown frame type: %s`. The parent is the frame named by the node's `parent`, or by
the first `parent` up its `inherits` chain; a name that finds no frame reports `Couldn't find frame
parent: %s` and leaves the frame parentless; with no `parent`, or an empty one on the node or a
template up the chain, the argument is used. The factory constructs the frame under that parent,
before it is named. Then the node is applied in three steps: name and loading flag, the frame's
load, and the finishing step that creates the child frames and runs `OnLoad`.

Addresses: `0x6ee280`, `0x6ee2eb`, `0x6ee32c`, `0x6ee356`, `0x6ee382`, `0x6ee390`, `0x6ee3a7`, `0x6ee3b9`, `0x6ee3c2`, `0x6ee3c4`, `0x6ee3e8`, `0x6ee3ef`, `0x6ee3f4`, `0x6ee40b`, `0x6ee414`, `0x6ee4d6`, `0x6ee4e1`, `0x6ee4ea`, `0xcee9d8`, `0x871124`, `0x87111c`, `0x871110`, `0x8710f0`, `0x8710d0`
Confidence: verified
Proof: one
Test: none

The tag is hashed (`0x6ee2eb`) into `0xcee9d8` and compared (`0x6ee32c`); a miss reports
`0x871124` at `0x6ee356`. `0x6ee382` reads `parent`; when absent, the loop `0x6ee390` to
`0x6ee3c2` reads `inherits`, looks it up (`0x6ee3a7`) and reads the template's `parent`
(`0x6ee3b9`). An empty one ends the walk and the argument is used (`0x6ee3c4`); a nonempty
name goes to `0x76c760` (`0x6ee3e8`), whose answer is stored even when 0
(`0x6ee3ef`), with `0x8710f0` reported. `0x6ee40b` calls the factory with the parent in `ecx`; a
null result reports `0x8710d0` (`0x6ee414`). Slot 0x20 (`0x6ee4d6`), the layout part's slot 8
(`0x6ee4e1`) and slot 0x24 (`0x6ee4ea`) follow.

## Frame types

A frame element names a factory registered by name. `FrameXML.cpp` registers `Button`,
`CheckButton`, `EditBox`, `Frame`, `MessageFrame`, `Model`, `ScrollFrame`, `ScrollingMessageFrame`,
`Slider`, `SimpleHTML`, `StatusBar`, `ColorSelect` and `MovieFrame`; the game interface registers
`WorldFrame`, `GameTooltip`, `Minimap`, `PlayerModel`, `DressUpModel`, `TabardModel`, `LootButton`
and `TaxiRouteFrame`; the glue registers `ModelFFX`. Registering a taken name does nothing.
`WorldFrame` alone is one-shot: its factory is removed after it creates its first frame.

Addresses: `0x6ee880`, `0x6ee8e3`, `0x6ef070`, `0x495940`, `0x46a4fe`, `0x6ee439`, `0x6ee4c8`, `0x6eeab0`, `0x6eec10`, `0x6ef000`, `0x4959d0`
Confidence: verified
Proof: one
Test: none

`0x6ee880(name, factory, flag)` answers 0 on a name already present (`0x6ee8e3`) and otherwise
stores the factory at `+0x18` and the flag at `+0x1c` of a new entry. `0x6ef070` makes 13 calls with
flag 0, `Button` with `0x6eeab0` through `MovieFrame` with `0x6ef000`; `0x495940` makes eight,
`WorldFrame` (`0x4959d0`) with flag 1; `0x46a4fe` registers `ModelFFX` with 0. In `0x6ee280` a set
flag (`0x6ee439`) unlinks the entry and frees it (`0x6ee4c8`).

## The frame factories

Each of the 13 factories that `FrameXML.cpp` registers allocates one object of a fixed size under that file's tag and runs the constructor of its class on it, answering the
object or 0 when the allocation fails. The sizes and constructors are: `Button` 0x4dc (`0x7786a0`), `CheckButton` 0x4e8 (`0x785010`), `EditBox` 0x448 (`0x7799b0`),
`Frame` 0x314 (`0x769090`), `MessageFrame` 0x378 (`0x785640`), `Model` 0x3dc (`0x76c8e0`), `ScrollFrame` 0x344 (`0x786a40`), `ScrollingMessageFrame` 0x3c4 (`0x787670`
with the extra argument 8), `Slider` 0x338 (`0x789400`), `SimpleHTML` 0x374 (`0x789dd0`), `StatusBar` 0x334 (`0x782d80`), `ColorSelect` 0x340 (`0x78b220`) and `MovieFrame`
0x3e0 (`0x78c0d0`).

Addresses: `0x6eeab0`, `0x6eeadf`, `0x6eeaf8`, `0x6eeb30`, `0x6eeb5f`, `0x6eeb78`, `0x6eeba0`, `0x6eebcf`, `0x6eebe8`, `0x6eec10`, `0x6eec3f`, `0x6eec58`, `0x6eec80`, `0x6eecaf`, `0x6eecc8`, `0x6eecf0`, `0x6eed1f`, `0x6eed38`, `0x6eed60`, `0x6eed8f`, `0x6eeda8`, `0x6eedd0`, `0x6eedff`, `0x6eee1a`, `0x6eee40`, `0x6eee6f`, `0x6eee88`, `0x6eeeb0`, `0x6eeedf`, `0x6eeef8`, `0x6eef20`, `0x6eef4f`, `0x6eef68`, `0x6eef90`, `0x6eefbf`, `0x6eefd8`, `0x6ef000`, `0x6ef02f`, `0x6ef048`, `0x6462e0`
Confidence: verified
Proof: one
Test: none

Every factory is 109 bytes (111 for `ScrollingMessageFrame`) of the same shape: it pushes the size, the path `FrameXML.cpp` and a line number to `SMemAlloc` (`0x6462e0`), and on a nonzero answer calls the constructor with
the object. The calls to `SMemAlloc` are at `0x6eeadf`, `0x6eeb5f`, `0x6eebcf`, `0x6eec3f`, `0x6eecaf`, `0x6eed1f`, `0x6eed8f`, `0x6eedff`, `0x6eee6f`, `0x6eeedf`, `0x6eef4f`, `0x6eefbf` and `0x6ef02f` in the order of the list.

## Child frames and names

A frame's `Frames` element is loaded by its finishing step: first the children of the template it
inherits, recursively, then its own, each created with the frame as the parent argument. A name
beginning `$parent` takes the name of the nearest named ancestor (`frames`), and so does a frame
named by an anchor's `relativeTo` or `SetPoint`'s relative frame, against the anchoring frame's
ancestors; `parent` and `SetParent` names are looked up as written.

Addresses: `0x76a060`, `0x76a085`, `0x76a092`, `0x76a09e`, `0x76a0b5`, `0x76c700`, `0x76c71c`, `0x76c744`, `0x87879c`
Confidence: verified
Proof: one
Test: none

`0x76a060(node, status)` reads `inherits`, looks it up (`0x76a085`) and recurses with the template
(`0x76a092`), then finds `Frames` (`0x76a09e`) and calls `0x6ee280` on each child with the frame in
`edx` (`0x76a0b5`). `0x76c700`, the layout part's slot 0x28, expands the name with `0x76c5b0` on
its frame (`0x76c71c`) before `0x76c760` (`0x76c744`); `0x6ee280` calls `0x76c760` directly.

## Script elements

Each child of `Scripts` sets the handler of its tag. Setting releases the slot's function first, so
a frame's own handler replaces its template's, and only one `OnLoad` runs. An empty body stores 0,
no handler, which `GetScript` answers as nil. A body that fails to compile stores −1 and leaves the
compiler's message on the Lua stack, unreported; firing such a slot calls nil, an error each time.
A tag the frame has no slot for reports `Unknown script element %s`. The input a handler needs is
enabled even for an empty body; Lua's `SetScript` enables nothing.

Addresses: `0x769ef0`, `0x769f10`, `0x769f44`, `0x769f6b`, `0x7025c0`, `0x7025ec`, `0x7025f6`, `0x70263c`, `0x702656`, `0x704c70`, `0x704c87`, `0x704c9f`, `0x774780`, `0x7748b9`, `0x7748d0`, `0x7749e6`, `0x878778`
Confidence: verified
Proof: one
Test: none

`0x769ef0` passes each child's tag and body to `0x7025c0` (`0x769f10`); a failure reports `0x878778`
(`0x769f44`); the enables follow a success (`0x769f6b` onward). `0x7025c0` releases (`0x7025ec`);
a null or empty body stores 0 (`0x7025f6`, `0x702656`); else it stores `0x704c70`'s answer
(`0x70263c`), which is `luaL_ref` of the chunk or −1 (`0x704c9f`) with the message still pushed.
`GetScript` (`0x774780`) pushes the registry value of the slot (`0x7748b9`), nil for 0 and −1.
`SetScript` (`0x7748d0`) releases (`0x7749e6`) and stores a reference with no enable call.

## The load status and its log

Each load reports into a status object: a list of lines with a severity, 0 for a trace, 1 for a
warning and 2 for an error, and the highest severity seen. A file's lines are prefixed by
`++ Loading file %s`, and a toc's by `** Loading table of contents %s`, only when the file reported
something or `FrameXML_Debug` is above 0; then they are merged into the caller's status.
`FrameXML_Debug(n)` sets that level to `n` truncated, and answers the level; at a level above 0
creating a frame and adding a template also trace a line.

Addresses: `0x41a000`, `0x419ed0`, `0x6ee200`, `0x6ee21c`, `0x6ee22d`, `0x6eddaf`, `0x6eddc8`, `0x6eddd9`, `0x6edb30`, `0x6edb40`, `0x488440`, `0x488475`, `0x6ee291`, `0x6ee6c6`, `0xceea30`, `0x87100c`, `0x870fec`, `0x871154`, `0x87116c`
Confidence: verified
Proof: one
Test: none

`CStatus::Add` (`0x41a000`) appends and `0x419ed0` puts a line first. `0x6ee200` reads the level
from `0x6edb40` and, when it is not above 0, tests the highest severity; the prefix is put first at
`0x6ee21c` and slot 8 merges at `0x6ee22d`; `0x6eddaf` to `0x6eddd9` do the same for a toc.
`0x488440` converts argument 1 when it is a number and stores it through `0x6edb30` (`0x488475`),
then pushes the level. `0x6ee291` and `0x6ee6c6` gate the traces `0x871154` and `0x87116c`.
`0xceea30` has two references, `0x6edb30` and `0x6edb40`.

## Sound while loading

Building the game interface, entering and leaving the world and shutting the interface down each
run inside a counter `0xb05fa0`: while it is above 0, playing an interface sound by name does
nothing. So `OnShow` and `OnHide` scripts that play sounds during those runs are silent; nothing is
queued.

Addresses: `0x458f50`, `0x458f60`, `0x458030`, `0x45804d`, `0xb05fa0`, `0x48fbfa`, `0x49016d`, `0x4908d5`, `0x490a56`, `0x490a94`, `0x490b64`, `0x490bd1`, `0x490cd9`
Confidence: verified
Proof: one
Test: none

`0x458f50` increments and `0x458f60` decrements `0xb05fa0`. `0x458030` returns at `0x45804d` when
it is nonzero, after the null and empty name tests and before any other. The census of calls finds
four pairs: `0x48fbfa` and `0x49016d` in `CGGameUI::Initialize`, `0x4908d5` and `0x490a56`,
`0x490a94` and `0x490b64`, and `0x490bd1` with the tail jump `0x490cd9`; neither is stored as a
dword.

## XML trees

A file is parsed by expat with no declared encoding, and the tree keeps for each element its tag,
its attributes as name and value pairs, its children in order and its character data concatenated
as one body. Every tag and attribute lookup ignores case and takes the first match. The parser
object is never freed.

Addresses: `0x6f2a30`, `0x6f2a3e`, `0x6f2a79`, `0x6f2a98`, `0x6f28a0`, `0x6f29c0`, `0x6f29d0`, `0x6f2b10`, `0x6f2c50`, `0x6f2cf0`, `0x882094`
Confidence: verified
Proof: one
Test: none

`0x6f2a30` creates the parser with a null encoding (`0x6f2a3e`), sets the start, end and character
handlers (`0x6f2a79`) and parses the whole buffer as final (`0x6f2a98`); neither exit frees it. The
start handler `0x6f28a0` makes a node (`0x6f2b10`: parent `+0x0`, first child `+0x4`, tag `+0x8`)
appended as the last child, and copies the attribute pairs to `+0x14` with their count at `+0x10`.
`0x6f29d0` appends character data to the body `+0xc`. `0x6f2c50` finds a child and `0x6f2cf0` an
attribute with `SStrCmpI`. The string `expat_1.95.5` is at `0x882094`.
