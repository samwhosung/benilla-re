#!/usr/bin/env bash
# Builds the Ghidra project of WoW.exe into dumps/ghidra/ (ignored by git):
#   1. imports WoW/WoW.exe (x86:LE:32:default, windows) and runs auto-analysis with the options
#      recorded in SetOptions.java,
#   2. creates the functions of symbols/functions.tsv that analysis missed and gives each row's
#      function the row's body (CreateFunctions.java),
#   3. applies the symbol table: names, signatures, structs, labels, comments (ApplySymbols.java).
#
# Usage: tools/ghidra/setup.sh            full build, replaces any earlier project
#        tools/ghidra/setup.sh --apply    only steps 2 and 3, on the project already built
# GHIDRA_INSTALL_DIR names the Ghidra folder (the one holding `support/`); otherwise Homebrew's
# is used, or analyzeHeadless on the PATH.
set -euo pipefail
. "$(dirname "${BASH_SOURCE[0]}")/../check-exe.sh"
. "$(dirname "${BASH_SOURCE[0]}")/headless.sh"
check_exe

HEADLESS=$(find_headless) || {
  echo "error: Ghidra 12.x not found; set GHIDRA_INSTALL_DIR (CONTRIBUTING.md, Setup)" >&2; exit 1; }

PROJECT_DIR="$ROOT/dumps/ghidra"
PROJECT=benilla-re
SCRIPTS="$ROOT/tools/ghidra"
SYMBOLS="$ROOT/symbols"
mkdir -p "$PROJECT_DIR"
LOG="$PROJECT_DIR/setup.log"

start=$(date +%s)
if [ "${1:-}" = "--apply" ]; then
  "$HEADLESS" "$PROJECT_DIR" "$PROJECT" -process WoW.exe -noanalysis \
    -scriptPath "$SCRIPTS" \
    -postScript CreateFunctions.java "$SYMBOLS" \
    -postScript ApplySymbols.java "$SYMBOLS" 2>&1 | tee "$LOG"
else
  rm -rf "$PROJECT_DIR/$PROJECT.gpr" "$PROJECT_DIR/$PROJECT.rep"
  "$HEADLESS" "$PROJECT_DIR" "$PROJECT" -import "$EXE" -overwrite \
    -processor x86:LE:32:default -cspec windows \
    -analysisTimeoutPerFile 7200 \
    -scriptPath "$SCRIPTS" \
    -preScript SetOptions.java \
    -postScript CreateFunctions.java "$SYMBOLS" \
    -postScript ApplySymbols.java "$SYMBOLS" 2>&1 | tee "$LOG"
fi
echo "setup: $(( $(date +%s) - start )) s; log in dumps/ghidra/setup.log"
grep -E '^INFO +(CreateFunctions|ApplySymbols)\.java> (CreateFunctions|ApplySymbols): ' "$LOG" | sed 's/^INFO [A-Za-z.]*> //' || true
