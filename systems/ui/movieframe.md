# ui: the movie frame

How a `MovieFrame` plays an AVI file (`CSimpleMovieFrame.cpp`, `0x78c0d0` to `0x78d9db`): the file it reads, the decoder library
it hands the frames to, the audio stream that sets its clock, the subtitles, and the three Lua methods. The frame's handler names
and constructor are `ui/xmltypes`; the layer callback that draws it is `ui/frames`.

Roots: `0x78c360`, `0x78c7a0`, `0x78ca30`, `0x78cb00`, `0x78cf30`, `0x78d1a0`, `0x78d560`, `0x78ce40`, `0x78ced0`

## The file

`0x78c360(name)` opens `<name>.avi` and reads it whole. It requires the `RIFF` and `AVI ` tags, reads the `hdrl` list, and takes from the first
`vids` stream header the frame rate (`dwRate / dwScale`, stored as a float at `+0x33c`) and the frame count (`dwLength`, at `+0x348`), and from the
stream format that follows the width and the height (`+0x340`, `+0x344`). It then walks the `idx1` index in 16-byte entries: an entry whose chunk
id ends in `d` (video) is read into one buffer as a size and the data, and one ending in `w` (audio) is appended to a second. Any other chunk of the file is skipped.

Addresses: `0x78c360`, `0x78c370`, `0x78c39f`, `0x78c3e3`, `0x78c3fb`, `0x78c459`, `0x78c4a0`, `0x78c4d0`, `0x78c577`, `0x78c58e`, `0x78c5ba`, `0x78c5c3`, `0x78c5cd`, `0x78c616`, `0x78c620`, `0x78c650`, `0x78c6d0`, `0x78c716`, `0x78c68e`, `0x78c6ac`
Confidence: verified
Proof: one
Test: none

`0x78c370` formats `%s.avi` and `0x78c39f` opens it. `0x78c3e3` and `0x78c3fb` compare the tags; `0x78c459`, `0x78c4a0` and `0x78c4d0` look for the `hdrl`, `movi` and `idx1` chunks. `0x78c58e` tests `vids` and `0x78c5ba` to `0x78c5cd` store the rate and the frame count. `0x78c650` and `0x78c664` test the chunk id letters 0x64 (`d`) and 0x77 (`w`) of each index entry; `0x78c68e` and `0x78c6ac` allocate the two buffers.

## The decoder

The video is decoded by the imported library `DivxDecoder.dll`, with `InitializeDivxDecoder`, `SetOutputFormat`, `DivxDecode` and `UnInitializeDivxDecoder`. `0x78c7a0`
numbers a decoder instance (a counter, `0xcf511c`), initialises it with the file's width and height and sets its output format to 1. The width must be 1024 or 800, else the start fails.
It then allocates the output bitmap for that size, 4 bytes a pixel, and creates one texture per tile: 2 tiles for 1024 wide and 6 for 800 wide. `0x78c990` destroys the instance, frees the buffers and the textures.

Addresses: `0x78c7a0`, `0x78c7d6`, `0x78c7fe`, `0x78c815`, `0x78c828`, `0x78c85e`, `0x78c925`, `0x78c990`, `0x78c99f`, `0x78ca1a`, `0xcf511c`
Confidence: verified
Proof: one
Test: none

`0x78c7d6` calls `InitializeDivxDecoder(instance, width, height)` and `0x78c7fe` `SetOutputFormat(instance, 1, width, height)`; each returns 0 on success. `0x78c815` and `0x78c828` accept widths 0x400 and 0x320 and give the size class 1 and 0. `0x78c925` creates a tile texture and `0x78ca1a` destroys one; `0x78c99f` calls `UnInitializeDivxDecoder`.

## Decoding a frame

`0x78d560(show)` takes the next frame record from the video buffer (a size, then the data) and calls `DivxDecode(instance, record, 0)` with the output
bitmap, the data and its size. A nonzero answer fails the frame. With `show` set it then copies each tile's rectangle of the bitmap into its texture (`GxTexUpdateRect`), so a frame that is only
decoded, to keep the codec's state, is not shown.

Addresses: `0x78d560`, `0x78d570`, `0x78d5ca`, `0x78d5d3`, `0x78d5e7`, `0x78d628`, `0x58ac70`
Confidence: verified
Proof: one
Test: none

`0x78d570` advances the read pointer past the record. `0x78d5ca` calls `DivxDecode`; `0x78d5d3` tests its result and `0x78d5e7` the `show` argument. `0x78d628` updates one tile.

## Clock and frame choice

The clock is the audio stream's play time (`FSOUND_Stream_GetTime`) while there is audio, else the time since the movie started. The frame to show is the clock times the rate, rounded; it is nudged by a
one-frame offset that follows the audio clock when that differs from the wall clock by more than two frames. A target at or beyond the frame count ends the movie. A target equal to the
shown frame does nothing; a target ahead of the next frame decodes the frames in between without showing them, and the last one is shown.

Addresses: `0x78cf30`, `0x78cf50`, `0x78cf68`, `0x78cf7b`, `0x78cfa5`, `0x78cfc3`, `0x78cffd`, `0x78d016`, `0x78d038`, `0x78d046`, `0x78d05c`, `0x78d06a`, `0x78d094`, `0x78d0b7`, `0x7fdaa2`
Confidence: verified
Proof: one
Test: none

`0x78cf50` and `0x78cfc3` read the audio time, `0x78cf68` and `0x78cf7b` the wall time. `0x78cfa5` multiplies by 0.001, `0x78cfab` adds one half and `0x78cfb1` truncates. `0x78cfff` to `0x78d01e` adjust the offset by one frame. `0x78d046` sets the finished flag at `+0x318` when the target reaches the frame count and `0x78d05c` stops the movie. `0x78d094` decodes the skipped frames without showing them, and `0x78d0b7` the last with `show` set.

## Audio

`0x78ca30` opens the audio buffer as a memory stream (`FSOUND_Stream_Open`), sets an end callback and starts it. The callback `0x78c070` sets the finished flag when its stream is the frame's.
`0x78caa0` stops and closes the stream and frees the audio buffer. If the stream cannot be opened or played the movie runs without sound, because `StartMovie` ignores the result of this start (`0x78ce6f`).

Addresses: `0x78ca30`, `0x78ca4a`, `0x78ca60`, `0x78ca71`, `0x78c070`, `0x78c081`, `0x78caa0`, `0x78caae`, `0x78cabf`
Confidence: verified
Proof: one
Test: none

`0x78ca4a` calls `FSOUND_Stream_Open` with the loop-unaware flag word 0x8000, `0x78ca60` `SetEndCallback`, and `0x78ca71` `Stream_Play`. `0x78c081` stores 1 at `+0x318` when the callback's stream is the one at `+0x3b0`.

## Subtitles

`0x78cb00` reads `<name>.sbt`, skipping a UTF-8 mark, and takes its lines. A line starting with `0` is a caption time line of the form `HH:MM:SS:FF - HH:MM:SS:FF`, and the line after it is the caption text.
Each time is hours, minutes and seconds in milliseconds plus the frame field times 41 ms. A caption is the start, the end and a copy of the text (12 bytes). While a caption has not ended
the frame holds its index; when its end passes it runs `OnMovieHideSubtitle`, and when subtitles are enabled, or the movie has no audio, the caption whose range holds the clock runs `OnMovieShowSubtitle` with the text.

Addresses: `0x78cb00`, `0x78cb0e`, `0x78cb3a`, `0x78cb4d`, `0x78cb77`, `0x78cb8a`, `0x78cca4`, `0x78ccd9`, `0x78ccdf`, `0x78cd62`, `0x78cf30`, `0x78d0e0`, `0x78d0ed`, `0x78d108`, `0x78d121`, `0x78d189`, `0x78c2e0`
Confidence: verified
Proof: one
Test: none

`0x78cb0e` formats `%s.sbt` and `0x78cb3a` loads it; `0x78cb4d` skips a leading 0xef byte. `0x78cb77` tokenizes a line and `0x78cb8a` tests that it starts with `0` (0x30). `0x78cca4` to `0x78ccdf` convert the start and `0x78ccdf` multiplies the frames by 0x29; `0x78cd62` copies the text. `0x78d0ed` compares the caption end with the clock, `0x78d108` runs `OnMovieHideSubtitle` (`+0x330`) and `0x78d189` runs `OnMovieShowSubtitle` (`+0x328`) with the text.

## Starting and stopping

`StartMovie("name")` needs a string (`Usage: <frame>:StartMovie("filename")`). It runs the file reader, the decoder start, the audio start and the subtitle load, sets the playing flag at `+0x314`, and
clears the display three times to black. It returns nil when the file reader or the decoder start fails and 1 otherwise: the audio start and the subtitle load results are ignored. `StopMovie()` ends a playing movie: it destroys the decoder, the audio and the captions, clears the playing and finished flags and runs
`OnMovieFinished` (`+0x320`). The movie also ends by itself at the last frame or when the audio ends. `EnableSubtitles([flag])` stores the flag (true when absent) at `+0x31c`.

Addresses: `0x78d770`, `0x78d842`, `0x78d820`, `0x78d870`, `0x78d92a`, `0x78d940`, `0x78d9ce`, `0x78d9d3`, `0x78ce40`, `0x78ce4b`, `0x78ce5d`, `0x78ce6f`, `0x78ce77`, `0x78ce81`, `0x78ced0`, `0x78cedd`, `0x78cee4`, `0x78ceeb`, `0x78cf1e`, `0x78cda0`, `0x78dc60`, `0x78dc80`, `0x87ab4c`
Confidence: verified
Proof: one
Test: none

`0x78ce4b`, `0x78ce5d`, `0x78ce6f` and `0x78ce77` call the file reader, the decoder start, the audio start and the subtitle loader; only the first two end the start on a zero answer; the results of the audio start and the subtitle load are dropped and `0x78cebd` returns 1. `0x78ce81` sets the playing flag and `0x78cea3` to `0x78ceb7` clear the device. `0x78cedd` to `0x78ceeb` free the decoder, the audio and the captions, and `0x78cf1e` runs the finished handler. `0x78d9ce` reads the flag with `GetScriptFlagArg` and `0x78d9d3` stores it. `0x78dc60` registers the three methods of the table `0x87ab4c` (`StartMovie`, `StopMovie`, `EnableSubtitles`) in the MovieFrame method hash with `FrameScriptRegisterMethods`; `0x78dc80` unregisters them (`ui/scriptapi`).

## Drawing

The frame registers a layer callback (`0x78d650` registers layer 2 with `0x78d680`). When it runs for a playing movie it advances the clock with `0x78cf30` and, when a frame is ready, draws the tile textures with `0x78d1a0`:
it saves the viewport, sets a full-window viewport and an orthographic projection, draws a quad per tile scaled by the screen aspect, and restores the viewport. It then clears the device buffers.

Addresses: `0x78d650`, `0x78d660`, `0x78d672`, `0x78d680`, `0x78d68d`, `0x78d698`, `0x78d1a0`, `0x78d1ad`, `0x78d1cb`, `0x78d1e5`, `0x78d321`, `0x78d4d3`, `0x78d4f0`, `0x76b3a0`, `0x773110`
Confidence: inferred
Proof: none
Test: none

`0x78d660` registers the layer and `0x78d672` the callback for layer 2. `0x78d68d` calls the clock and `0x78d698` the draw. `0x78d1ad` reads the aspect, `0x78d1cb` and `0x78d1e5` save and set the viewport, `0x78d321` builds the orthographic matrix and `0x78d4d3` and `0x78d4f0` build and draw the quads. The geometry was not decoded further.
