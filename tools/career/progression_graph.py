"""Print a progression state machine: states, handler records and traced Lua."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from vault import hash64
from vault_dump import Db
from luadump import read_chunk, decode, OPS
from luacalls import calls

HANDLER = 0x1b08ce405acdd040


def handler_chunk(db, col):
    rec_v = db.where[(HANDLER, col)]
    blob = db.value(HANDLER, col, 0xc3cd4b4183274819)
    if not blob or blob[0] != 'blob':
        return None
    _, dep, off, size = blob
    data = rec_v.buffers[dep]
    main, _ = read_chunk(data, off)
    lines = []
    for p in main.protos:
        lines.extend(calls(p))
    return lines


def main():
    db = Db()
    want = sys.argv[1]
    machines = {}
    for col in db.collections_of('progression_statemachines'):
        r = db.record('progression_statemachines', col)
        machines[col] = r
    handlers = {}
    for col in db.collections_of(HANDLER):
        r = db.record(HANDLER, col)
        handlers[col] = r
    for col, r in machines.items():
        if r.get('Name') != want:
            continue
        print('MACHINE', r.get('Name'), 'initial', r.get('InitialState'))
        for ref in r.get('States', []):
            scol = int(ref[2], 16) if not ref[2].startswith('progression') else None
            srec = db.record('progression_states', scol)
            print('  STATE', srec.get('Name'), {k: v for k, v in srec.items() if k not in ('Name', 'StateMachine', 'Description')})
            for href in srec.get('deb2096fe58b1805', []) or []:
                hcol = int(href[2], 16)
                h = handlers.get(hcol, {})
                print('    HANDLER', h.get('Name'), {k: v for k, v in h.items() if k not in ('Name', 'StateMachine', 'c3cd4b4183274819')})
                for line in handler_chunk(db, hcol) or []:
                    print('        ' + line)


if __name__ == '__main__':
    main()
