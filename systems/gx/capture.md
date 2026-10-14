# gx: screen capture

How the client takes a screenshot. The device half (arming, readback, the pixel buffer) is gx's; the frame pass that drives it, the file naming and the completion callbacks sit in the screen-layer code at `0x442350` through `0x442a10`, whose object file has no owner here, and the TGA encoder belongs to `image`. This doc states that code as call order and arguments.

Roots: `0x589cb0`, `0x589cc0`, `0x589ce0`, `0x592130`, `0x592140`

## Capture request and frame order

The Lua bindings `Screenshot` (glue `0x46d3d0`, world `0x4893d0`) each load a completion callback (`0x46c520`, `0x4930b0`) into `ecx` and call `0x442a10`, which stores it in the one-slot request at `0x88560c`. The frame pass `0x442350`, registered on event category `0x11` (post-update), ends with the present block: if `0x885604` is non-zero the whole block is skipped and the request stays armed; if the slot is empty it calls `GxScenePresent(3)` (`0x58a960`); otherwise it arms the device (`0x589cb0`), presents (`0x58a960`, which reads the pixels back), runs `0x4426f0` (fetch, encode, write), calls the stored callback with `ecx` set to whether a buffer existed, frees the buffer (`0x589ce0`) and clears the slot.

Addresses: `0x442a10`, `0x442350`, `0x46d3d0`, `0x4893d0`, `0x46c520`, `0x4930b0`, `0x442a00`, `0x58a960`, `0x589cb0`, `0x589ce0`, `0x4426f0`, `0x88560c`, `0x885604`
Confidence: verified
Proof: one
Test: none

The request slot has four references: the read at `0x442683`, the indirect call at `0x4426a2`, the clear at `0x4426ad` and the write in `0x442a10`. A second `Screenshot` in a frame overwrites the first callback. `0x885604` is written only by `0x442a00`, called with 1 and 0 from the movie frame code at `0x78ce8b` and `0x78cf06`; a request made while a movie plays waits.

## Device readback

`0x592130` (slot `+0x40`, both backends) sets `[dev+0x2864]` to 1. The present of either backend latches it, and `CGxDevice::DoCapture` (`0x592140`) takes the device rectangle (`0x5922e0`), sets width and height to the truncated `+0xc` and `+8` floats at `[dev+0x2868]` and `[dev+0x286c]`, and calls slot `+0x48` with the rectangle `{0, 0, h, w}` and the buffer record at `+0x2870`. The record holds 32-bit BGRA, `w * h * 4` bytes, top row first. `0x5921a0` (slot `+0x44`) returns width, height and the buffer; `0x5921e0` frees it. Direct3D reads back before `Present` (`0x5a19f8`); OpenGL after the swap (`0x59ba55`).

Addresses: `0x592130`, `0x592140`, `0x5921a0`, `0x5921e0`, `0x599e90`, `0x58e0c0`, `0x59ba10`, `0x5a1910`, `0x589cc0`, `0x592440`
Confidence: verified
Proof: one
Test: none

`0x599e90` takes the back buffer with `GetBackBuffer` when `[dev+0x220] < 2`, otherwise resolves the multisampled target into `[dev+0x3a54]` with `StretchRect`, then `GetDesc` and `LockRect` with flags `0x810` and converts by the surface format `0x15` to `0x19` (jump table `0x59a118`). `0x58e0c0` calls `glReadPixels` with `GL_BGRA_EXT` (`0x80e1`) and unsigned byte, then swaps row pairs to put the top row first. The flag is written by `0x592130` (1) and `0x592440` (0), set to 0 at construction (`0x591b8a`) and read at `0x59ba1b` and `0x5a192c`.

## Screenshot file

`0x4426f0` fetches the buffer with `0x589cc0` and returns 0 when there is none. Otherwise it creates the directory `Screenshots` (`0x834c5c`, relative to the working directory, one non-recursive `CreateDirectoryW` whose result is ignored), formats the path `%s\WoWScrnShot_%s.tga` (`0x834c44`) from the directory and a local-time stamp `%m%d%y_%H%M%S` (`0x832f60`, built by `0x429450` with `strftime`), builds a TGA from the buffer with `0x5a43c0(buffer, w, h, 32, 0, 1, 0)`, strips its alpha with `0x5a4250`, writes it with `0x5a4810` and destroys it. It returns 1 whenever a buffer existed; the write's result is discarded.

Addresses: `0x4426f0`, `0x42a960`, `0x429450`, `0x64a7f0`, `0x834c44`, `0x834c5c`, `0x832f60`, `0x5a43c0`, `0x5a4250`, `0x5a4810`, `0x5a3600`
Confidence: verified
Proof: one
Test: none

The path is `Screenshots\WoWScrnShot_MMDDYY_HHMMSS.tga`. `0x5a4810` opens it through `0x42a460` with access `0x40000000` and disposition 2 (`CREATE_ALWAYS`), so two captures in one second overwrite one file; there is no loop or counter in `0x4426f0`, `0x5a4810` or `0x5a43c0`. A failed write closes and deletes the file (`0x5a499b`, `0x42a910`).

## TGA written

`0x5a43c0` accepts depth 32 or 24 and alpha bits 0 or 8, copies the buffer, and writes the header: image type 2 (uncompressed true colour), width and height, depth `0x20`, descriptor `0x20` (top-left origin), the footer `TRUEVISION-XFILE.` (`0x85c854`), and sets the pixel byte count `+0x3c` to `((depth + 7) >> 3) * w * h`, which is `4 * w * h`. `0x5a4250` then compacts the pixels in place from four bytes to three, sets the depth to `0x18` and the descriptor's alpha bits to 0, and never updates `+0x3c`. `0x5a4810` writes 18 header bytes, the identification field and colour map when present (neither is), `+0x3c` pixel bytes and 26 footer bytes.

Addresses: `0x5a43c0`, `0x5a4250`, `0x5a4810`, `0x85c854`, `0x5a445f`, `0x5a4471`, `0x5a42bc`, `0x5a494c`
Confidence: verified
Proof: one
Test: none

Every screenshot file is therefore `18 + 4 * w * h + 26` bytes, with `w * h` trailing bytes from the unconverted tail of the buffer, while its header says 24 bits per pixel. `0x5a4250` also sets the Storm error `0x8720012d` on this normal path. The image holds no `screenshotFormat` or `screenshotQuality` name and no JPEG or PNG signature string (a case-insensitive census finds six `screenshot` literals and one `scrnshot`; its control finds `gxResolution`).

## Completion events

The completion callback takes the success flag in `ecx` and fires a Lua event through `0x703e50`: the world callback (`0x4930b0`, only while the world UI exists, `[0xb4e36c]` non-zero) fires event 208 `SCREENSHOT_SUCCEEDED` when `ecx` is non-zero and 209 `SCREENSHOT_FAILED` otherwise; the glue callback (`0x46c520`) fires 14 `GLUE_SCREENSHOT_SUCCEEDED` or 15 `GLUE_SCREENSHOT_FAILED`.

Addresses: `0x4930b0`, `0x46c520`, `0x703e50`, `0xb4e36c`
Confidence: verified
Proof: one
Test: none

The event names are the strings at `0x853244`, `0x853230`, `0x837aa4` and `0x837a8c`. No screen-captured message text exists in the image; any on-screen text comes from the interface scripts.
