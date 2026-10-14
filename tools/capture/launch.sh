#!/usr/bin/env bash
# Runs the 1.12.1 client under Wine, with a capture rig or none (README.md).
#
#   launch.sh plain      the client as installed, nothing added
#   launch.sh player     with the player rig: player.csv
#   launch.sh hook       with the hook rig: calls.csv (set BENILLA_CAPTURE_HOOKS)
#   launch.sh trace      with an apitrace opengl32.dll: a .trace of the OpenGL calls
#   launch.sh test       runs forwardtest and hooktest in this Wine, without the client
#   launch.sh restore    puts imm32 back in the prefix's KnownDLLs
#
# Settings, from the environment:
#   WOW_DIR        the client's install directory (default: the repo's WoW/)
#   WOW_RUN        what to run in it (default: WoW.exe; a loader, if your setup starts the
#                  client through one)
#   WINE           the Wine loader (default: wine)
#   WINESERVER     its wineserver (default: wineserver beside WINE, else on the PATH)
#   WINEPREFIX     the Wine prefix, as Wine reads it
#   CAPTURE_OUT    where the rig writes (default: tools/capture/out/)
#   APITRACE_DLL   for trace: apitrace's 32-bit Windows opengl32.dll
#   BENILLA_CAPTURE_HOOKS, BENILLA_CAPTURE_FILTER, BENILLA_CAPTURE_HZ   passed to the rig
#
# The rig and imm32_orig.dll are copied into WOW_DIR for the run and removed when the client
# exits. The script waits for that (wineserver -w), so a loader that returns at once is fine.
set -euo pipefail

here="$(cd "$(dirname "$0")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
mode="${1:-}"
wow_dir="${WOW_DIR:-$repo/WoW}"
wow_run="${WOW_RUN:-WoW.exe}"
wine="${WINE:-wine}"
out="${CAPTURE_OUT:-$here/out}"
build="$here/build"
knowndlls='HKLM\System\CurrentControlSet\Control\Session Manager\KnownDLLs'

die() { echo "launch.sh: $*" >&2; exit 1; }

command -v "$wine" >/dev/null || die "no Wine loader '$wine': set WINE"
if [ -z "${WINESERVER:-}" ]; then
    beside="$(dirname "$(command -v "$wine")")/wineserver"
    if [ -x "$beside" ]; then WINESERVER="$beside"; else WINESERVER=wineserver; fi
fi
command -v "$WINESERVER" >/dev/null || die "no wineserver '$WINESERVER': set WINESERVER"

# A Windows path for a host path, through the prefix's drive mapping.
windows_path() { "$wine" winepath -w "$1" 2>/dev/null | tr -d '\r'; }

# imm32 is in Wine's KnownDLLs, which load from the system directory whatever the DLL overrides
# say; the rig can take its place only once it is out of that list. A fresh wineserver reads the
# changed registry.
leave_knowndlls() {
    "$WINESERVER" -k 2>/dev/null || true
    "$wine" reg delete "$knowndlls" /v imm32 /f >/dev/null 2>&1 || true
    "$WINESERVER" -k 2>/dev/null || true
    echo "launch.sh: imm32 is out of the prefix's KnownDLLs; 'launch.sh restore' puts it back"
}

need_build() {
    for f in player/imm32.dll hook/imm32.dll imm32_orig.dll forwardtest.exe \
        hooktest/hooktest.exe hooktest/imm32.dll; do
        [ -f "$build/$f" ] || { "$here/build.sh"; return; }
    done
}

# Copies files into WOW_DIR for the run and removes them on exit. A file of that name that is
# not one of ours is left alone and stops the run.
placed=()
place() {
    local from="$1" name="$2" ours=0
    if [ -e "$wow_dir/$name" ]; then
        for mine in "$build/player/imm32.dll" "$build/hook/imm32.dll" "$build/imm32_orig.dll" \
            "${APITRACE_DLL:-/nonexistent}"; do
            cmp -s "$wow_dir/$name" "$mine" && ours=1
        done
        [ "$ours" = 1 ] || die "$wow_dir/$name exists and is not a rig file; move it first"
    fi
    cp "$from" "$wow_dir/$name"
    placed+=("$wow_dir/$name")
}
cleanup() { for f in "${placed[@]+"${placed[@]}"}"; do rm -f "$f"; done; }
trap cleanup EXIT

run_client() {
    [ -f "$wow_dir/$wow_run" ] || die "no $wow_run in $wow_dir: set WOW_DIR (and WOW_RUN)"
    echo "launch.sh: running $wow_run in $wow_dir; quit the client to finish"
    (cd "$wow_dir" && "$wine" "$wow_run") || true
    "$WINESERVER" -w || true
}

run_rig() {
    local rig="$1"
    need_build
    mkdir -p "$out"
    local dir
    dir="$(windows_path "$out")"
    [ -n "$dir" ] || die "cannot map $out to a Windows path"
    place "$build/$rig/imm32.dll" imm32.dll
    place "$build/imm32_orig.dll" imm32_orig.dll
    leave_knowndlls
    export BENILLA_CAPTURE_DIR="$dir"
    export WINEDLLOVERRIDES="imm32=n,b${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
    run_client
    echo "launch.sh: the rig wrote into $out:"
    ls -l "$out"
}

case "$mode" in
plain)
    run_client
    ;;
player)
    run_rig player
    ;;
hook)
    [ -n "${BENILLA_CAPTURE_HOOKS:-}" ] || die "set BENILLA_CAPTURE_HOOKS (README.md, The hook rig)"
    run_rig hook
    ;;
trace)
    [ -f "${APITRACE_DLL:-}" ] || die "set APITRACE_DLL to apitrace's 32-bit opengl32.dll"
    mkdir -p "$out"
    marker="$(mktemp)"
    place "$APITRACE_DLL" opengl32.dll
    export WINEDLLOVERRIDES="opengl32=n,b${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
    run_client
    # apitrace names the trace after the exe and writes it where the process's current
    # directory or profile puts it; collect every trace this run wrote.
    found=0
    while IFS= read -r t; do
        mv "$t" "$out/" && found=$((found + 1)) && echo "launch.sh: $out/$(basename "$t")"
    done < <(find "$wow_dir" "${WINEPREFIX:-$HOME/.wine}/drive_c" -name '*.trace' \
        -newer "$marker" 2>/dev/null)
    rm -f "$marker"
    [ "$found" -gt 0 ] || die "no .trace written; a client killed rather than quit leaves none"
    ;;
test)
    need_build
    leave_knowndlls
    export WINEDLLOVERRIDES="imm32=n,b${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
    unset BENILLA_CAPTURE_DIR BENILLA_CAPTURE_HOOKS BENILLA_CAPTURE_FILTER
    status=0
    for rig in player hook; do
        t="$out/test/$rig"
        rm -rf "$t" && mkdir -p "$t"
        cp "$build/forwardtest.exe" "$build/imm32_orig.dll" "$build/$rig/imm32.dll" "$t/"
        (cd "$t" && "$wine" forwardtest.exe) || status=1
    done
    t="$out/test/hooktest"
    rm -rf "$t" && mkdir -p "$t"
    cp "$build/hooktest/"* "$t/"
    (cd "$t" && "$wine" hooktest.exe) || status=1
    grep -h x87 "$t/hooktest-out/hook.log" 2>/dev/null || true
    exit "$status"
    ;;
restore)
    "$WINESERVER" -k 2>/dev/null || true
    "$wine" reg add "$knowndlls" /v imm32 /t REG_SZ /d imm32.dll /f >/dev/null
    "$WINESERVER" -k 2>/dev/null || true
    echo "launch.sh: imm32 is back in the prefix's KnownDLLs"
    ;;
*)
    sed -n '2,24p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
    ;;
esac
