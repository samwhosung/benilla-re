#!/usr/bin/env python3
"""tables/lua_globals.tsv: the global names the 1.12 interface defines, by origin.

This table is not derived from the exe. It is a capture of the running client's `_G`, attributed
against the interface files, and `cargo xtask check --exe` does not regenerate it (tables/README.md,
Captured tables). This generator takes the capture and the interface folders by path:

    BENILLA_RE_GLOBALS_CAPTURE   the capture file: a saved-variables file with a
                                 `["globals"] = { ... ["count"]` section of records holding
                                 `["name"]` and `["t"]` (the Lua type)
    BENILLA_RE_INTERFACE_DIRS    the interface folders, joined with the platform's path separator:
                                 the shipped UI's extracted folder, and the folder holding the
                                 extracted `Blizzard_*` addons. A folder with one `.toc` is an
                                 addon; a folder without one holds addon folders. Only the files
                                 the `.toc` lists and the `<Script file>` and `<Include file>`
                                 references of the XML reach are read.
    --drop NAME                  a global of the capturing addon, repeated: its saved variable, its
                                 holder table and its slash-command token. The instrument, not
                                 the client. Each must be in the capture, or the run stops.

or `--capture PATH` and `--interface DIR` (repeated) in their place; `-o PATH` writes the table
there instead of standard output.

Each name in the capture gets one origin:

  lua       Lua 5.0's own runtime (LUA_5_0 below: the client removes `_G`, `print`, `require`,
            `dofile`, `loadfile` and the `io`, `os`, `debug` and `coroutine` tables).
  engine    the C client provides it; tables/bindings.tsv covers the functions.
  framexml  the interface defines it: its Lua, its XML, or a `$parent`-composed child of one.

Attribution is by definition site in the interface files:

  - a name assigned or `function`-declared in Lua, including Lua inside an XML document, is the
    interface's; indented assignments count, because the interface leaks globals out of function
    bodies (`button = getglobal(...)`);
  - a `name="..."` on any XML element is the interface's, `virtual="true"` included (a virtual
    `<Font>` is a real font object); a `<Binding name=...>` is a key-binding command, not a global;
  - `<a defined name><a $parent suffix>` is a composed child (`ContainerFrame1Item16IconTexture`),
    and a numeric tail on the parent side is allowed (`ContainerFrame1` and `Item16` come from
    `name..i`);
  - LUA_5_0 wins over all of these: the interface's `string = getglobal(...)` would otherwise
    claim the standard table.

The twelve `Blizzard_*` addons are load-on-demand, so a capture holds their globals only if their
windows were opened. Every name the interface defines that the capture lacks joins with type `lod`
and origin `framexml`. The table answers "is this a 1.12 name?", not "was this name live in one
session".

Rows are `name`, `type` (the capture's Lua type, or `lod`), `origin`, sorted. The capture's own
globals, named by `--drop`, are not rows.
"""

import argparse
import os
import re
import sys

LUA_5_0 = {
    'assert', 'collectgarbage', 'error', 'gcinfo', 'getfenv', 'getmetatable', 'ipairs',
    'loadstring', 'math', 'next', 'pairs', 'pcall', 'rawequal', 'rawget', 'rawset',
    'setfenv', 'setmetatable', 'string', 'table', 'tonumber', 'tostring', 'type',
    'unpack', 'xpcall',
}

LUA_ASSIGN = re.compile(r'^[ \t]*([A-Za-z_][A-Za-z0-9_]*)\s*=[^=]', re.M)
LUA_FUNC = re.compile(r'^[ \t]*function\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(', re.M)
LUA_MULTI = re.compile(
    r'^[ \t]*([A-Za-z_][A-Za-z0-9_]*(?:\s*,\s*[A-Za-z_][A-Za-z0-9_]*)+)\s*=[^=]', re.M)
XML_ELEM = re.compile(r'<(\w+)([^>]*?)/?>', re.S)
XML_NAME = re.compile(r'\bname\s*=\s*"([^"]*)"')
PARENT_SUFFIX = re.compile(r'\bname\s*=\s*"\$parent([A-Za-z0-9_]*)"')
# Lua inside an XML document: an inline `<Script>` body and every `<Scripts>` handler element
# (`<OnLoad>`, `<PreClick>`, ...), matched by shape so no handler name is missed. A
# `<Script file=...>` is skipped: its file is read on its own.
XML_SCRIPT = re.compile(
    r'<((?:On|Pre|Post)\w+|Script)\b(?![^>]*\bfile\s*=)[^>]*>(.*?)</\1>', re.S)


def captured_globals(path):
    """The capture's globals as (name, Lua type). The section is the `["globals"] = {` table at
    one tab of indent; its `["count"]`, where present, must equal the records read."""
    recs, cur_t, inside, count = [], None, False, None
    with open(path, encoding='utf-8', errors='replace') as f:
        for line in f:
            if not inside:
                inside = line.rstrip('\r\n') == '\t["globals"] = {'
                continue
            if line.rstrip('\r\n') == '\t},':
                break
            m = re.match(r'\t\t\["count"\] = (\d+),', line)
            if m:
                count = int(m.group(1))
                continue
            m = re.match(r'\s*\["t"\] = "([^"]*)",', line)
            if m:
                cur_t = m.group(1)
                continue
            m = re.match(r'\s*\["name"\] = "(.*)",\s*$', line)
            if m:
                recs.append((m.group(1), cur_t))
                cur_t = None
    if count is not None and count != len(recs):
        sys.exit('lua_globals: the capture says %d globals and holds %d' % (count, len(recs)))
    return recs


XML_REF = re.compile(r'<(?:Script|Include)\s+file\s*=\s*"([^"]+)"', re.I)


def _norm(base, ref):
    """`ref` (backslashes allowed) relative to the folder `base`, as a normal path."""
    return os.path.normpath(os.path.join(base, ref.replace('\\', '/').strip()))


def _find(path):
    """`path`, or the file of that name in its folder that differs only in case; None if none."""
    if os.path.isfile(path):
        return path
    here, name = os.path.split(path)
    try:
        for n in os.listdir(here):
            if n.lower() == name.lower() and os.path.isfile(os.path.join(here, n)):
                return os.path.join(here, n)
    except OSError:
        pass
    return None


def _toc(folder):
    """The `.toc` file in `folder`, or None."""
    tocs = sorted(n for n in os.listdir(folder) if n.lower().endswith('.toc'))
    return os.path.join(folder, tocs[0]) if len(tocs) == 1 else None


def interface_roots(dirs):
    """The addon folders: a folder holding one `.toc`, or each subfolder that does."""
    roots = []
    for d in dirs:
        d = os.path.normpath(d)
        if _toc(d):
            roots.append(d)
            continue
        for n in sorted(os.listdir(d)):
            sub = os.path.join(d, n)
            if os.path.isdir(sub) and _toc(sub):
                roots.append(sub)
    return roots


def corpus(dirs):
    """The files the interface loads, as {path: text}, in load order: each addon's `.toc` lists
    the files, an XML file lists more with `<Script file>` and `<Include file>`, and a
    reference is followed from the folder of the file that holds it. A file on disk that no
    reference reaches is not read, so an extra file cannot add a name; the paths left out go to
    standard error. A referenced file that is missing stops the run."""
    files, loaded = {}, set()
    roots = interface_roots(dirs)
    if not roots:
        sys.exit('lua_globals: no folder under %s holds a .toc' % ', '.join(dirs))
    for root in roots:
        toc = _toc(root)
        with open(toc, 'rb') as f:
            lines = f.read().decode('utf-8', 'replace').splitlines()
        queue = [_norm(root, l) for l in lines if l.strip() and not l.lstrip().startswith('#')]
        while queue:
            ref = queue.pop(0)
            path = _find(ref)
            if path is None:
                sys.exit('lua_globals: %s is referenced and not on disk' % ref)
            key = os.path.normcase(path)
            if key in loaded or not path.lower().endswith(('.lua', '.xml')):
                continue
            loaded.add(key)
            with open(path, 'rb') as f:
                text = f.read().decode('utf-8', 'replace')
            files[path] = text
            if path.lower().endswith('.xml'):
                queue = [_norm(os.path.dirname(path), r) for r in XML_REF.findall(text)] + queue
    skipped = []
    for root in roots:
        for here, subs, names in os.walk(root):
            subs.sort()
            skipped += [os.path.join(here, n) for n in sorted(names)
                        if n.lower().endswith(('.lua', '.xml'))
                        and os.path.normcase(os.path.join(here, n)) not in loaded]
    if skipped:
        print('lua_globals: %d files no reference reaches, not read: %s'
              % (len(skipped), ', '.join(skipped[:10]) + (' ...' if len(skipped) > 10 else '')),
              file=sys.stderr)
    return files


def lua_defs(text, into):
    into.update(LUA_ASSIGN.findall(text))
    into.update(LUA_FUNC.findall(text))
    for group in LUA_MULTI.findall(text):
        into.update(n.strip() for n in group.split(','))


def definitions(files):
    """Every global the interface defines, and the `$parent` suffixes children compose with."""
    names, suffixes = set(), set()
    for path, text in files.items():
        if path.lower().endswith('.lua'):
            lua_defs(text, names)
        else:
            for _tag, body in XML_SCRIPT.findall(text):
                lua_defs(body, names)
            suffixes.update(s for s in PARENT_SUFFIX.findall(text) if s)
            for tag, attrs in XML_ELEM.findall(text):
                if tag.lower() == 'binding':
                    continue
                m = XML_NAME.search(attrs)
                if m and '$parent' not in m.group(1):
                    names.add(m.group(1))
    return names, suffixes


def composed(name, roots, suffixes, depth=0):
    """Is `name` `<a defined name><a $parent suffix>`, recursively?"""
    if depth > 8:
        return False
    for i in range(1, len(name)):
        head, tail = name[:i], name[i:]
        if tail not in suffixes:
            continue
        if head in roots or head.rstrip('0123456789') in roots:
            return True
        if composed(head, roots, suffixes, depth + 1):
            return True
    return False


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n\n')[0])
    ap.add_argument('--capture', default=os.environ.get('BENILLA_RE_GLOBALS_CAPTURE'))
    ap.add_argument('--interface', action='append', default=None)
    ap.add_argument('--drop', action='append', default=[])
    ap.add_argument('-o', '--out')
    args = ap.parse_args()
    dirs = args.interface
    if dirs is None:
        dirs = [d for d in os.environ.get('BENILLA_RE_INTERFACE_DIRS', '').split(os.pathsep) if d]
    if not args.capture or not dirs:
        sys.exit('lua_globals: set BENILLA_RE_GLOBALS_CAPTURE and BENILLA_RE_INTERFACE_DIRS '
                 '(or pass --capture and --interface); see the header of this file')
    for d in dirs:
        if not os.path.isdir(d):
            sys.exit('lua_globals: no folder at %s' % d)
    if not os.path.isfile(args.capture):
        sys.exit('lua_globals: no capture at %s' % args.capture)

    files = corpus(dirs)
    defined, suffixes = definitions(files)
    drop = set(args.drop)
    every = captured_globals(args.capture)
    absent = drop - {n for n, _ in every}
    if absent:
        sys.exit('lua_globals: --drop names not in the capture: %s' % ', '.join(sorted(absent)))
    recs = [(n, t) for n, t in every if n not in drop]
    if not recs or not defined:
        sys.exit('lua_globals: the capture holds %d globals and the interface %d definitions'
                 % (len(recs), len(defined)))
    rows, captured, tally = [], set(), {}
    for name, t in recs:
        captured.add(name)
        if name in LUA_5_0:
            origin = 'lua'
        elif name in defined or composed(name, defined, suffixes):
            origin = 'framexml'
        else:
            origin = 'engine'
        rows.append((name, t, origin))
        tally[origin] = tally.get(origin, 0) + 1
    lod = sorted(defined - captured)
    rows += [(n, 'lod', 'framexml') for n in lod]
    text = 'name\ttype\torigin\n' + ''.join('\t'.join(r) + '\n' for r in sorted(set(rows)))
    if args.out:
        with open(args.out, 'w', encoding='utf-8', newline='\n') as f:
            f.write(text)
    else:
        sys.stdout.write(text)
    print('%d files, %d captured, %d defined, %d lod; origins %s'
          % (len(files), len(recs), len(defined), len(lod), tally), file=sys.stderr)


main()
