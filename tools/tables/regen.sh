#!/bin/sh
# Regenerate every table the exe derives into DIR (default tables/) from your own WoW.exe (build 5875) at
# WoW/WoW.exe, or at $BENILLA_RE_WOW_EXE. The generators that read instructions use
# dumps/disasm.txt; tools/disasm.sh writes it if it is missing, the listing the findings cite.
# lua_globals is a capture, not derived from the exe, and is not regenerated here (tables/README.md).
# Usage: tools/tables/regen.sh [DIR]   (cargo xtask check --exe passes a scratch DIR and compares)
set -eu
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
out=${1:-tables}
exe=${BENILLA_RE_WOW_EXE:-WoW/WoW.exe}
[ -f "$exe" ] || { echo "regen: $exe not found; see docs/CONTRIBUTING.md, Setup" >&2; exit 1; }
[ -f dumps/disasm.txt ] || tools/disasm.sh
mkdir -p "$out"
for name in opcodes opcode_sends spell_failed messages events glue_events event_sites cvars dbc_stores dbc_schemas bindings chat_types chat_groups channel_notices unit_gates tutorial_triggers calls event_args verb_events glue_surface; do
    python3 "tools/tables/$name.py" --exe "$exe" --tables "$out" -o "$out/$name.tsv"
done
# reads the tables above and the committed docs
python3 tools/tables/opcode_entries.py --exe "$exe" --tables "$out" -o "$out/opcode_entries.tsv"
