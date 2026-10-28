"""Print a challenge's state graph: challenge_states and their Lua handlers."""
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from vault import hash64
from vault_dump import Db
from luadump import read_chunk
from luacalls import calls

STATES_FIELD = 0x3cb7ba2afc9bde42


def chunk_lines(db, cls, col):
    v = db.where.get((cls, col))
    if v is None:
        return ['<missing handler>']
    rec = db.classes[cls]['defs']
    for key, d in rec.items():
        if d['type'] == hash64('Attrib::Blob'):
            blob = db.value(cls, col, key)
            if blob and blob[0] == 'blob':
                main, _ = read_chunk(v.buffers[blob[1]], blob[2])
                out = []
                for p in main.protos:
                    out.extend(calls(p))
                return out
    return ['<no blob>']


def main():
    db = Db()
    name = sys.argv[1]
    col = hash64(name)
    rec = db.record('challenges', col)
    print('CHALLENGE', name, 'objectives', rec.get('Objectives'))
    states = rec.get(db.name(STATES_FIELD)) or rec.get('States') or []
    scls = hash64('challenge_states')
    hcls = hash64('challenge_messagehandlers')
    for ref in states:
        srec = db.record(scls, int(ref[2], 16) if all(ch in '0123456789abcdef' for ch in ref[2]) else hash64(ref[2]))
        print('  STATE', srec.get('Name'), {k: v for k, v in srec.items() if k != 'Name' and not isinstance(v, list)})
        for k, v in srec.items():
            if isinstance(v, list):
                for h in v:
                    if h[0] != 'ref' or h[1] != 'challenge_messagehandlers':
                        continue
                    hcol = int(h[2], 16) if all(ch in '0123456789abcdef' for ch in h[2]) else hash64(h[2])
                    hrec = db.record(hcls, hcol)
                    print('    HANDLER', hrec.get('Name'), {kk: vv for kk, vv in hrec.items() if kk != 'Name' and not isinstance(vv, tuple)})
                    for line in chunk_lines(db, hcls, hcol):
                        print('        ' + line)


if __name__ == '__main__':
    main()
