#!/usr/bin/env bash
# Builds the capture rigs into tools/capture/build/:
#   player/imm32.dll   the player rig
#   hook/imm32.dll     the hook rig
#   imm32_orig.dll     a copy of the real imm32.dll, which both rigs forward to
#   imm32.def          the forwarders, written from that copy's exports
#   forwardtest.exe    the injection test
#   hooktest/          the hook mechanism test: hooktest.exe and a test build of the hook rig
#   probe.exe          the outside-read probe
#
# Needs the 32-bit MinGW-w64 cross compiler (i686-w64-mingw32-gcc; `brew install mingw-w64`,
# `sudo apt install gcc-mingw-w64-i686`) and Python 3, and the real 32-bit imm32.dll of the system
# the client runs on: IMM32_DLL names it, else the script looks beside the `wine` it finds
# (WINE, else `wine` on the PATH) in lib/wine/i386-windows/.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
out="$here/build"
cc="${CC:-i686-w64-mingw32-gcc}"
objdump="${cc%gcc}objdump"
command -v "$cc" >/dev/null || { echo "build.sh: no $cc on the PATH (see the comment above)" >&2; exit 1; }

imm32="${IMM32_DLL:-}"
if [ -z "$imm32" ]; then
    wine="$(command -v "${WINE:-wine}" || true)"
    if [ -n "$wine" ]; then
        wine="$(cd "$(dirname "$wine")" && pwd -P)/$(basename "$wine")"
        candidate="$(dirname "$wine")/../lib/wine/i386-windows/imm32.dll"
        [ -f "$candidate" ] && imm32="$candidate"
    fi
fi
[ -n "$imm32" ] && [ -f "$imm32" ] || {
    echo "build.sh: set IMM32_DLL to the 32-bit imm32.dll the client loads" >&2
    echo "          (Wine: <wine>/lib/wine/i386-windows/imm32.dll; Windows: C:\\Windows\\SysWOW64\\imm32.dll)" >&2
    exit 1
}

mkdir -p "$out/player" "$out/hook" "$out/hooktest"
cp "$imm32" "$out/imm32_orig.dll"
python3 -I "$here/imm32_def.py" "$out/imm32_orig.dll" "$out/imm32.def"

flags=(-O2 -Wall -Wextra -std=gnu11 -static-libgcc)
for rig in player hook; do
    "$cc" "${flags[@]}" -shared -o "$out/$rig/imm32.dll" \
        "$here/rig.c" "$here/$rig.c" "$out/imm32.def"
done
"$cc" "${flags[@]}" -o "$out/forwardtest.exe" "$here/forwardtest.c" -limm32
"$cc" "${flags[@]}" -o "$out/probe.exe" "$here/probe.c"
# The hooktest build of the hook rig skips the build check; hooktest.exe keeps the 0x400000 base
# without relocation, and -O0 with frame pointers makes its functions begin with push ebp.
"$cc" "${flags[@]}" -DRIG_TEST_ANY_IMAGE -shared -o "$out/hooktest/imm32.dll" \
    "$here/rig.c" "$here/hook.c" "$out/imm32.def"
"$cc" -O0 -fno-omit-frame-pointer -Wall -Wextra -std=gnu11 -static-libgcc \
    -Wl,--disable-dynamicbase,--image-base,0x400000 \
    -o "$out/hooktest/hooktest.exe" "$here/hooktest.c" -limm32
cp "$out/imm32_orig.dll" "$out/hooktest/"

# A control on the result: each rig forwards every export the .def lists.
for rig in player hook; do
    exported="$("$objdump" -p "$out/$rig/imm32.dll" | grep -c 'imm32_orig\.' || true)"
    wanted="$(grep -c ' = imm32_orig\.' "$out/imm32.def")"
    [ "$exported" = "$wanted" ] || {
        echo "build.sh: $rig/imm32.dll forwards $exported exports, the .def lists $wanted" >&2
        exit 1
    }
done
echo "build.sh: built in $out ($wanted exports forwarded)"
