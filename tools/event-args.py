#!/usr/bin/env python3
"""For each `SignalEvent2` site, the arguments its format declares against the ones its caller
passes.

    tools/event-args.py [--all]

`SignalEvent2` (0x703f50) is cdecl and variadic: the event id, a printf-style format, then the
values. Its marshaller reads one value per directive of the format, eight bytes for `%f` and four
for the others, so a format that declares more than the caller pushed hands the script whatever
dwords lie above the last push. tables/event_sites.tsv gives each site's format, which is a
declaration; this compares it with the caller.

What the caller passed is read from its cleanup: the first `add esp,N` or `lea esp,[esp+N]` after
the call, before any other call, jump or return, frees the id, the format and the values, so the
values are N/4 − 2 dwords. A cleanup larger than the format accounts for is shared with an earlier
call MSVC did not clean up separately; then the count is the pushes walking back from the call to
the previous call, which holds where no value is itself a call's result pushed in between. A site
with neither, or a tail jump (whose caller's caller cleans up), is listed for a reading by hand.

It prints the sites whose format declares more than the caller passed, then those it could not
settle, and with `--all` every site. The control is that every site the table lists as
`SignalEvent2` is a call or jump to 0x703f50 in the listing. Option: --disasm PATH.
"""

import csv
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent / 'tables'))
from lib import ROOT, Disasm, addr, parse_args  # noqa: E402

SIGNAL2 = 0x703F50
CLEANUP = re.compile(r'^(?:add\s+esp,0x([0-9a-f]+)|lea\s+esp,\[esp\+0x([0-9a-f]+)\])$')
DIRECTIVE = re.compile(r'%([a-zA-Z])')


def declared(fmt):
    return sum(2 if d == 'f' else 1 for d in DIRECTIVE.findall(fmt))


def cleanup(dis, k):
    for j in range(k + 1, min(k + 8, len(dis.ins))):
        _, _, mn, ops = dis.ins[j]
        m = CLEANUP.match('%s %s' % (mn, ops.replace(' ', '')) if mn == 'add'
                          else '%s %s' % (mn, ops))
        if m:
            return int(m.group(1) or m.group(2), 16)
        if mn in ('call', 'ret') or mn.startswith('j'):
            return None
    return None


def pushes(dis, k):
    n = 0
    for j in range(k - 1, max(k - 40, -1), -1):
        _, _, mn, _ = dis.ins[j]
        if mn in ('call', 'ret') or mn.startswith('j'):
            break
        if mn == 'push':
            n += 1
    return n


def main():
    def extra(ap):
        ap.add_argument('--all', action='store_true')
    args = parse_args(__doc__, disasm=True, extra=extra)
    dis = Disasm(args.disasm)
    with open(ROOT / 'tables' / 'event_sites.tsv', newline='', encoding='utf-8') as f:
        sites = [r for r in csv.DictReader(f, delimiter='\t', quoting=csv.QUOTE_NONE)
                 if r['dispatcher'] == 'SignalEvent2']
    want = '0x%x' % SIGNAL2
    bad = [r['site'] for r in sites
           if int(r['site'], 16) not in dis.at
           or dis.ins[dis.at[int(r['site'], 16)]][3] != want]
    if bad or not sites:
        sys.exit('event-args: control failed: %s are no call or jump to %s in the listing'
                 % (', '.join(bad) or 'no sites', want))
    hazard, manual, lines = [], [], []
    for r in sites:
        a = int(r['site'], 16)
        k = dis.at[a]
        want_n = declared(r['arg_format'])
        name = r['event_names'] or r['event_ids']
        if dis.ins[k][2] == 'jmp':
            manual.append('%s %s: a tail jump; the values are its caller\'s' % (r['site'], name))
            continue
        n = cleanup(dis, k)
        if n is not None and n // 4 - 2 <= want_n:
            have, via = n // 4 - 2, 'add esp,0x%x' % n
        else:
            have = pushes(dis, k) - 2
            via = ('%d pushes, cleanup 0x%x shared' % (have + 2, n) if n is not None
                   else '%d pushes, no cleanup' % (have + 2))
            if have < 0:
                manual.append('%s %s: %s' % (r['site'], name, via))
                continue
        line = '%s %s %r declares %d, passes %d (%s)' % (r['site'], name, r['arg_format'],
                                                       want_n, have, via)
        lines.append(line)
        if have < want_n:
            hazard.append(line)
    if args.all:
        print('\n'.join(lines))
        print()
    print('declares more than it passes: %d' % len(hazard))
    for line in hazard:
        print('  ' + line)
    print('to read by hand: %d' % len(manual))
    for line in manual:
        print('  ' + line)
    print('%d SignalEvent2 sites, every one a call or jump to %s in the listing' % (
        len(sites), addr(SIGNAL2)), file=sys.stderr)
    return 0


if __name__ == '__main__':
    sys.exit(main())
