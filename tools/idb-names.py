#!/usr/bin/env python3
"""The community names of the client, extracted from the vmangos/idb IDA database of build 5875.

    python3 tools/idb-names.py PATH/TO/Wow.idb [-o dumps/idb-names.tsv]

The database is `1.12.1.5875/Wow.idb` in github.com/vmangos/idb. It is read with python-idb
(`pip install python-idb`), which reads IDA's files without IDA. The output is the file
`cargo xtask census --fill --idb` takes: one line per name a person gave, `address<TAB>kind<TAB>name`,
kind `func` for a function start and `data` for anything else, sorted by address, after a header
line. IDA's own labels (`sub_`, `loc_`, `dword_` and the rest) and FLIRT's `unknown_libname_` carry
no one's judgement and are left out.

A community name is a lead (METHOD.md, The binary decides): a row takes one only once the bytes
agree (CONTRIBUTING.md, Adding a name), and `census --fill` drops those the bytes contradict.
The file is derived from a third party's database, so it goes under `dumps/` and is never
committed.
"""

import argparse
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# IDA's labels for what no one named.
AUTO = ('sub_', 'loc_', 'locret_', 'byte_', 'word_', 'dword_', 'qword_', 'unk_', 'flt_', 'dbl_',
        'asc_', 'stru_', 'off_', 'seg_', 'algn_', 'j_nullsub_', 'nullsub_', 'def_', 'jpt_',
        'xmmword_', 'unknown_libname_')


def main():
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument('idb')
    ap.add_argument('-o', '--out', default=str(ROOT / 'dumps' / 'idb-names.tsv'))
    args = ap.parse_args()
    try:
        import idb
    except ImportError:
        sys.exit('idb-names: needs python-idb (pip install python-idb)')
    with idb.from_file(args.idb) as db:
        api = idb.IDAPython(db)
        funcs = set(api.idautils.Functions())
        rows = sorted((ea, 'func' if ea in funcs else 'data', name)
                      for ea, name in api.idautils.Names()
                      if name and not name.startswith(AUTO))
    if not rows or not any(kind == 'func' for _, kind, _ in rows):
        sys.exit('idb-names: no function names read from %s' % args.idb)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, 'w', encoding='utf-8', newline='\n') as f:
        f.write('address\tkind\tname\n')
        for ea, kind, name in rows:
            f.write('0x%06x\t%s\t%s\n' % (ea, kind, name))
    n = sum(1 for r in rows if r[1] == 'func')
    print('%d names (%d functions, %d data) -> %s' % (len(rows), n, len(rows) - n, out),
          file=sys.stderr)
    return 0


if __name__ == '__main__':
    sys.exit(main())
