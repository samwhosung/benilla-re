#!/usr/bin/env bash
# Sourced by the other tools: finds the repo root and the exe, WoW/WoW.exe or
# $BENILLA_RE_WOW_EXE, and refuses any build but 5875 (tools/exe.env).

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EXE="${BENILLA_RE_WOW_EXE:-$ROOT/WoW/WoW.exe}"
. "$ROOT/tools/exe.env"

sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d' ' -f1
  else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

check_exe() {
  [ -f "$EXE" ] || { echo "error: $EXE not found; copy your 1.12.1 WoW.exe there (docs/CONTRIBUTING.md, Setup)" >&2; return 1; }
  local size sum
  size=$(wc -c < "$EXE" | tr -d ' ')
  if [ "$size" != "$EXE_SIZE" ]; then
    echo "error: $EXE is $size bytes, build 5875 is $EXE_SIZE; the addresses here do not apply to it" >&2; return 1
  fi
  sum=$(sha256_of "$EXE")
  if [ "$sum" != "$EXE_SHA256" ]; then
    echo "error: $EXE has SHA-256 $sum, build 5875 is $EXE_SHA256; the addresses here do not apply to it" >&2; return 1
  fi
}
