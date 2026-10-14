#!/usr/bin/env bash
# Runs the 1.12.1 client under Wine, with a capture rig or none (README.md).
#
#   launch.sh plain      the client as installed, nothing added
#   launch.sh player     with the player rig: player.csv
#   launch.sh hook       with the hook rig: calls.csv (set BENILLA_CAPTURE_HOOKS)
#   launch.sh trace      with an apitrace opengl32.dll: a .trace of the OpenGL calls
#   launch.sh test       runs forwardtest and hooktest in this Wine, without the client
#   launch.sh restore    puts back the KnownDLLs value a run killed before its cleanup left out
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
#   WOW_SNAPSHOT   1: copy WDB, WTF, Logs, Errors and Screenshots of WOW_DIR before the run and
#                  put them back after it, discarding what the client wrote there
#   BENILLA_CAPTURE_HOOKS, BENILLA_CAPTURE_FILTER, BENILLA_CAPTURE_HZ   passed to the rig
#
# The rig and imm32_orig.dll are copied into WOW_DIR for the run and removed when the client
# exits. The script waits for that (wineserver -w), so a loader that returns at once is fine.
# It refuses to start while any wineserver runs, so it never stops a program of yours, and
# puts back the prefix's KnownDLLs value for imm32 exactly as it found it. The client itself
# writes WDB, WTF and Logs in WOW_DIR on every run, plain included; see WOW_SNAPSHOT.
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

# The script stops no wineserver: it runs only when none does. pgrep sees every prefix's, which
# is broader than needed, but a running one cannot be told apart by prefix without its socket.
no_wineserver() {
    if pgrep -f wineserver >/dev/null 2>&1; then
        die "a wineserver is running (pgrep -fl wineserver): quit its programs, or stop it with
          '$WINESERVER -k' if nothing of yours runs in it, then start again"
    fi
}

# imm32 is in Wine's KnownDLLs, which load from the system directory whatever the DLL overrides
# say; the rig can take its place only once it is out of that list. KnownDLLs are read when a
# wineserver starts, so the change is made with none running and the script waits for the one
# `reg` starts to exit (and save the registry) before the client starts a fresh one.
#
# The value found is kept in $state until it is put back, so 'launch.sh restore' can finish the
# job for a run killed before its cleanup. state holds "absent", or the type and the data.
state="$out/.knowndlls-imm32"
knowndlls_changed=0
read_knowndlls() {
    local line
    line="$("$wine" reg query "$knowndlls" /v imm32 2>/dev/null | tr -d '\r' |
        awk '$1 == "imm32" { $1 = ""; sub(/^ +/, ""); print; exit }')" || true
    if [ -z "$line" ]; then echo absent; else echo "$line"; fi
}
write_knowndlls() {
    local saved="$1" type data
    if [ "$saved" = absent ]; then
        "$wine" reg delete "$knowndlls" /v imm32 /f >/dev/null 2>&1 || true
    else
        type="${saved%% *}"
        data="${saved#* }"
        data="${data#"${data%%[! ]*}"}"
        "$wine" reg add "$knowndlls" /v imm32 /t "$type" /d "$data" /f >/dev/null
    fi
    "$WINESERVER" -w 2>/dev/null || true
}
leave_knowndlls() {
    mkdir -p "$out"
    [ -f "$state" ] && die "$state exists: an earlier run did not put KnownDLLs back; run 'launch.sh restore'"
    local saved
    saved="$(read_knowndlls)"
    echo "$saved" >"$state"
    knowndlls_changed=1
    if [ "$saved" != absent ]; then
        "$wine" reg delete "$knowndlls" /v imm32 /f >/dev/null 2>&1 || true
    fi
    "$WINESERVER" -w 2>/dev/null || true
    echo "launch.sh: imm32 is out of the prefix's KnownDLLs until the run ends (it was: $saved)"
}
put_back_knowndlls() {
    [ -f "$state" ] || return 0
    local saved
    saved="$(cat "$state")"
    "$WINESERVER" -w 2>/dev/null || true
    write_knowndlls "$saved"
    [ "$(read_knowndlls)" = "$saved" ] || { echo "launch.sh: KnownDLLs imm32 is not back to '$saved'" >&2; return 1; }
    "$WINESERVER" -w 2>/dev/null || true
    rm -f "$state"
    echo "launch.sh: KnownDLLs imm32 is back as it was ($saved)"
}

need_build() {
    for f in player/imm32.dll hook/imm32.dll imm32_orig.dll forwardtest.exe \
        hooktest/hooktest.exe hooktest/imm32.dll; do
        [ -f "$build/$f" ] || { "$here/build.sh"; return; }
    done
}

# Copies files into WOW_DIR for the run and undoes it on exit. A file already there is used as
# it is when it is byte-identical to the one placed, and left; one byte-identical to another rig
# build (a run killed before its cleanup, say) is moved aside and put back at exit; anything else
# stops the run. No file that predates the run is removed.
placed=()   # files to remove at exit
aside=()    # "backup|original" pairs to put back at exit
rig_builds=("$build/player/imm32.dll" "$build/hook/imm32.dll" "$build/imm32_orig.dll"
    "$build/hooktest/imm32.dll")
place() {
    local from="$1" name="$2" target
    target="$wow_dir/$name"
    if [ -e "$target" ]; then
        if cmp -s "$target" "$from"; then
            echo "launch.sh: $target is already the file to place; using it and leaving it"
            return
        fi
        local mine ours=0
        for mine in "${rig_builds[@]}"; do cmp -s "$target" "$mine" && ours=1; done
        [ "$ours" = 1 ] || die "$target exists and is not a rig build; move it first (it is left as it was)"
        local backup
        backup="$(mktemp "$wow_dir/.$name.launch-XXXXXX")"
        mv "$target" "$backup"
        aside+=("$backup|$target")
    fi
    cp "$from" "$target"
    placed+=("$target")
}

# WOW_SNAPSHOT=1: the directories the client writes, copied before the run and put back after.
snapshot_dirs=(WDB WTF Logs Errors Screenshots)
snapshot=""
take_snapshot() {
    [ "${WOW_SNAPSHOT:-0}" = 1 ] || return 0
    snapshot="$(mktemp -d "${TMPDIR:-/tmp}/benilla-capture-snapshot-XXXXXX")"
    local d
    for d in "${snapshot_dirs[@]}"; do
        if [ -e "$wow_dir/$d" ]; then cp -Rp "$wow_dir/$d" "$snapshot/$d"; fi
    done
    echo "launch.sh: snapshot of ${snapshot_dirs[*]} in $snapshot"
}
put_back_snapshot() {
    [ -n "$snapshot" ] || return 0
    local d
    for d in "${snapshot_dirs[@]}"; do
        rm -rf "${wow_dir:?}/$d"
        if [ -e "$snapshot/$d" ]; then cp -Rp "$snapshot/$d" "$wow_dir/$d"; fi
    done
    rm -rf "$snapshot"
    echo "launch.sh: ${snapshot_dirs[*]} put back as they were before the run"
}

cleanup() {
    local f pair
    for f in "${placed[@]+"${placed[@]}"}"; do rm -f "$f"; done
    for pair in "${aside[@]+"${aside[@]}"}"; do mv -f "${pair%%|*}" "${pair#*|}"; done
    put_back_snapshot
    [ "$knowndlls_changed" = 1 ] && put_back_knowndlls
    return 0
}
trap cleanup EXIT
trap 'exit 130' INT TERM HUP

run_client() {
    [ -f "$wow_dir/$wow_run" ] || die "no $wow_run in $wow_dir: set WOW_DIR (and WOW_RUN)"
    echo "launch.sh: running $wow_run in $wow_dir; quit the client to finish"
    take_snapshot
    (cd "$wow_dir" && "$wine" "$wow_run") || true
    "$WINESERVER" -w || true
}

run_rig() {
    local rig="$1"
    need_build
    no_wineserver
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
    no_wineserver
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
    no_wineserver
    marker="$(mktemp)"
    place "$APITRACE_DLL" opengl32.dll
    export WINEDLLOVERRIDES="opengl32=n,b${WINEDLLOVERRIDES:+;$WINEDLLOVERRIDES}"
    run_client
    # apitrace names the trace after the process, WoW.trace or WoW.<n>.trace, and writes it
    # where the process's current directory or profile puts it; collect the ones by that name
    # this run wrote, and no other program's.
    found=0
    while IFS= read -r t; do
        mv "$t" "$out/" && found=$((found + 1)) && echo "launch.sh: $out/$(basename "$t")"
    done < <(find "$wow_dir" "${WINEPREFIX:-$HOME/.wine}/drive_c" -iname 'wow*.trace' \
        -newer "$marker" 2>/dev/null)
    rm -f "$marker"
    [ "$found" -gt 0 ] || die "no .trace written; a client killed rather than quit leaves none"
    ;;
test)
    need_build
    no_wineserver
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
    "$WINESERVER" -w 2>/dev/null || true
    exit "$status"
    ;;
restore)
    [ -f "$state" ] || die "no $state: no run left KnownDLLs changed (CAPTURE_OUT must be the run's)"
    no_wineserver
    put_back_knowndlls
    ;;
*)
    sed -n '2,28p' "$0" | sed 's/^# \{0,1\}//'
    exit 2
    ;;
esac
