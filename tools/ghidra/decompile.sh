#!/usr/bin/env bash
# Decompiles functions with the project tools/ghidra/setup.sh built, this repo's names, signatures
# and types applied, into one C file (default dumps/ghidra/decompiled.c), and prints it. An address
# inside a function decompiles the function that holds it. The project is opened read-only, so
# nothing is saved to it. Decompiler output is a lead, never a derivation (docs/METHOD.md).
#
# Usage: tools/ghidra/decompile.sh [-o FILE] ADDRESS [ADDRESS ...]
# BENILLA_RE_GHIDRA_PROJECT_DIR and BENILLA_RE_GHIDRA_PROJECT name another project than
# dumps/ghidra/benilla-re; GHIDRA_INSTALL_DIR names Ghidra as for setup.sh.
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/../check-exe.sh"
. "$(dirname "${BASH_SOURCE[0]}")/headless.sh"
check_exe

OUT="$ROOT/dumps/ghidra/decompiled.c"
if [ "${1:-}" = "-o" ]; then OUT="$2"; shift 2; fi
[ $# -ge 1 ] || { echo "usage: tools/ghidra/decompile.sh [-o FILE] ADDRESS [ADDRESS ...]" >&2; exit 2; }
HEADLESS=$(find_headless) || {
  echo "error: Ghidra 12.x not found; set GHIDRA_INSTALL_DIR (docs/CONTRIBUTING.md, Setup)" >&2; exit 1; }
PROJECT_DIR="${BENILLA_RE_GHIDRA_PROJECT_DIR:-$ROOT/dumps/ghidra}"
PROJECT="${BENILLA_RE_GHIDRA_PROJECT:-benilla-re}"
[ -f "$PROJECT_DIR/$PROJECT.gpr" ] || {
  echo "error: no Ghidra project at $PROJECT_DIR/$PROJECT.gpr; run tools/ghidra/setup.sh" >&2; exit 1; }

rm -f "$OUT"
LOG=$("$HEADLESS" "$PROJECT_DIR" "$PROJECT" -process WoW.exe -noanalysis -readOnly \
  -scriptPath "$ROOT/tools/ghidra" -postScript Decompile.java "$OUT" "$@" 2>&1) || {
  echo "$LOG" >&2; exit 1; }
[ -s "$OUT" ] || { echo "$LOG" >&2; echo "error: no output in $OUT" >&2; exit 1; }
echo "$LOG" | grep -E 'Decompile: ' | sed 's/^.*> //' >&2
cat "$OUT"
