#!/usr/bin/env bash
# Writes the disassembly of .text of WoW/WoW.exe (or $BENILLA_RE_WOW_EXE) to dumps/disasm.txt
# (ignored by git).
# Findings cite instruction addresses of this file: GNU objdump, Intel syntax, address and
# instruction, no raw bytes. It follows the real instruction stream: objdump's sweep restarts at
# every function row of symbols/functions.tsv and at every jump-table entry, and the jump tables'
# bytes are left out (`cargo xtask listing`, xtask/src/listing.rs).
# Usage: tools/disasm.sh   (OBJDUMP=<path> to choose the binary)
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/check-exe.sh"
check_exe

# The listing has no fixed hash: it depends on the binutils version's formatting and on
# symbols/, since every function row restarts the sweep. Its addresses are what findings cite, and
# those do not change.

find_objdump() {
  local c
  for c in "${OBJDUMP:-}" i686-w64-mingw32-objdump x86_64-w64-mingw32-objdump gobjdump objdump; do
    [ -n "$c" ] && command -v "$c" >/dev/null 2>&1 || continue
    # Only GNU objdump, and only one built with the PE target.
    "$c" --version 2>&1 | head -1 | grep -q 'GNU' || continue
    "$c" -i 2>&1 | grep -q '^pei-i386' || continue
    command -v "$c"; return 0
  done
  return 1
}

OD=$(find_objdump) || {
  echo "error: no GNU objdump with the pei-i386 target found; see CONTRIBUTING.md, Setup" >&2; exit 1; }
echo "using $("$OD" --version | head -1) at $OD"

mkdir -p "$ROOT/dumps"
OUT="$ROOT/dumps/disasm.txt"
EXE_PATH="$(cd "$(dirname "$EXE")" && pwd)/$(basename "$EXE")"
( cd "$ROOT" && BENILLA_RE_WOW_EXE="$EXE_PATH" cargo xtask listing "$OD" "$OUT.tmp" )
mv "$OUT.tmp" "$OUT"

sum=$(sha256_of "$OUT")
echo "wrote dumps/disasm.txt: $(wc -l < "$OUT" | tr -d ' ') lines, SHA-256 $sum"
