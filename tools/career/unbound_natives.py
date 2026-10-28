"""Natives in challenge/binding_table.rs (every native Lua::Binding table of
the debug executable) that the port does not register yet, per module.

    python tools/career/unbound_natives.py            summary per module
    python tools/career/unbound_natives.py Progression the module's unbound names and addresses

A registration is a ("Module", "Name", function) tuple in any
crates/skate2-core/src/challenge/*.rs file. Only natives the shipped scripts
call matter; tools/career/luacalls.py lists those per handler.
"""
import glob
import os
import re
import sys

root = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..', 'crates', 'skate2-core', 'src', 'challenge')
table = open(os.path.join(root, 'binding_table.rs'), encoding='utf-8').read()
EMPTY_NATIVE = 0x82784530

modules = {}
pattern = r'Module \{\s*name: (Some\("(\w+)"\)|None),\s*table: (0x[0-9a-fA-F]+),\s*entries: &\[(.*?)\],\s*\}'
for m in re.finditer(pattern, table, re.S):
    name = m.group(2) or '<globals>'
    modules.setdefault(name, []).extend(re.findall(r'Entry::Function\("(\w+)", (0x[0-9a-fA-F]+)\)', m.group(4)))

bound = set()
for path in glob.glob(os.path.join(root, '*.rs')):
    source = open(path, encoding='utf-8').read()
    bound.update(re.findall(r'\(\s*"(\w+)",\s*"(\w+)",\s*[\w:]+\s*\)', source))

wanted = sys.argv[1:]
for name in sorted(modules):
    if wanted and name not in wanted:
        continue
    natives = modules[name]
    unbound = [(n, a) for n, a in natives if (name, n) not in bound and int(a, 16) != EMPTY_NATIVE]
    print('%s: %d/%d bound' % (name, len(natives) - len(unbound), len(natives)))
    if wanted:
        for n, a in unbound:
            print('    %s %s' % (n, a))
