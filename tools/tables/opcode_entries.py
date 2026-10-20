#!/usr/bin/env python3
"""tables/opcode_entries.tsv: for each opcode the client handles or sends, the entries of
`systems/` that read the code at that place.

One row per registration in opcodes.tsv (role `handler`, the site the registration call) and per
send in opcode_sends.tsv (role `send`, the send call). The function is the handler, or the
function that holds the send. An entry covers a function when the entry's `Addresses:` line holds
an address inside the function's extent. The generator reads the committed docs under systems/ and
the symbol table, and the two tables beside it (--tables DIR, default tables/).

`entries` lists the covering entries as `<doc>#<anchor>`, `|`-separated, the doc relative to
systems/ without `.md`. `basis` says how they were chosen: `opcode` when an entry's text names
the message (`CMSG_...`, as in the name column, with no identifier character either side, so
`SMSG_QUESTUPDATE_FAILED` does not match inside `SMSG_QUESTUPDATE_FAILEDTIMER`) or the opcode (as
`0x12e`, in any case, with or without leading zeros) and only those are listed;
`function` when entries cover the function but none names the opcode; `unread` when no entry
covers it and `entries` is empty. A `function` row may be only a census listing: an entry that
lists the function's address without describing what it sends. A match of a small value is a lead:
the text may use the same
number for something else, except after a `+` or `-` (a field offset). A send whose opcode is `?` has no opcode to match, so its basis
is `function` or `unread`. Rows are sorted by opcode (`?` last), role, then site.
"""

import csv
import re
import sys

from lib import ROOT, Functions, addr, parse_args, write_table

ADDRESS = re.compile(r'0x[0-9a-f]{6}')


def slug(heading):
    keep = ''.join(c for c in heading.strip().lower() if c.isalnum() or c in '_- ')
    return keep.replace(' ', '-')


def entries():
    """Every entry of every doc: (doc, anchor, addresses in its Addresses line, text)."""
    out = []
    for path in sorted((ROOT / 'systems').rglob('*.md')):
        doc = path.relative_to(ROOT / 'systems').with_suffix('').as_posix()
        if path.name == 'README.md':
            continue
        text = path.read_text(encoding='utf-8')
        for section in re.split(r'\n(?=## )', text)[1:]:
            title, _, body = section.partition('\n')
            line = re.search(r'^Addresses: (.*)$', body, re.M)
            if line:
                out.append((doc, slug(title[3:]), ADDRESS.findall(line.group(1)), body.lower()))
    return out


def read(path):
    with open(path, newline='', encoding='utf-8') as f:
        return list(csv.DictReader(f, delimiter='\t', quoting=csv.QUOTE_NONE))


def names_opcode(text, opcode):
    value = int(opcode, 16)
    forms = {'0x%x' % value, '0x%03x' % value, '0x%02x' % value}
    return any(re.search(r'(?<![0-9a-z+-])%s(?![0-9a-z])' % re.escape(f), text) for f in forms)


def names_message(text, name):
    """Whether the entry's text names the message: the name with no identifier character on
    either side, so a name that is another's prefix does not match the longer one."""
    return re.search(r'(?<![0-9a-z_])%s(?![0-9a-z_])' % re.escape(name.lower()), text) is not None


def main():
    args = parse_args(__doc__)
    fns = Functions()
    cover = {}   # function address -> [(doc, anchor, text)]
    for doc, anchor, addresses, text in entries():
        seen = set()
        for a in addresses:
            row = fns.holding(int(a, 16))
            if row is not None and row['address'] not in seen:
                seen.add(row['address'])
                cover.setdefault(row['address'], []).append((doc, anchor, text))

    rows = []
    for r in read('%s/opcodes.tsv' % args.tables):
        rows.append((r['opcode'], 'handler', r['site'], r['handler'], r['name']))
    for r in read('%s/opcode_sends.tsv' % args.tables):
        rows.append((r['opcode'], 'send', r['site'], r['function'], r['name']))

    out = []
    for opcode, role, site, function, name in rows:
        found = cover.get(int(function, 16), [])
        named = [e for e in found if (opcode != '?' and names_opcode(e[2], opcode))
                 or (name and names_message(e[2], name))]
        chosen, basis = (named, 'opcode') if named else (found, 'function' if found else 'unread')
        links = []
        for doc, anchor, _ in chosen:
            link = '%s#%s' % (doc, anchor)
            if link not in links:
                links.append(link)
        out.append((opcode, name, role, site, function, '|'.join(links), basis))
    out.sort(key=lambda r: (r[0] == '?', r[0], r[2], r[3]))
    print('%d rows: %d opcode, %d function, %d unread'
          % (len(out), sum(r[6] == 'opcode' for r in out), sum(r[6] == 'function' for r in out),
             sum(r[6] == 'unread' for r in out)), file=sys.stderr)
    write_table(['opcode', 'name', 'role', 'site', 'function', 'entries', 'basis'], out, args.out)


if __name__ == '__main__':
    sys.exit(main())
