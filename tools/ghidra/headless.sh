#!/usr/bin/env bash
# Sourced by setup.sh and decompile.sh: finds Ghidra's analyzeHeadless, from GHIDRA_INSTALL_DIR
# (the folder that holds `support/`), else Homebrew's, else the PATH.

find_headless() {
  if [ -n "${GHIDRA_INSTALL_DIR:-}" ] && [ -x "$GHIDRA_INSTALL_DIR/support/analyzeHeadless" ]; then
    echo "$GHIDRA_INSTALL_DIR/support/analyzeHeadless"; return
  fi
  if command -v brew >/dev/null 2>&1 && [ -x "$(brew --prefix ghidra 2>/dev/null)/libexec/support/analyzeHeadless" ]; then
    echo "$(brew --prefix ghidra)/libexec/support/analyzeHeadless"; return
  fi
  command -v analyzeHeadless && return
  return 1
}
