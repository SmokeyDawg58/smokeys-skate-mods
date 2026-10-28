"""Read-only audit of shipped BAM navigation records. No game execution.

python tools/inspect_living_world_navigation.py PATH/TO/worldbam.big [collection-limit] [Sim|Pres]
"""
import struct
import sys
from collections import Counter
from extract_big import parse_named, refpack


def word(data, at):
    return struct.unpack_from('>I', data, at)[0]


def audit(path, limit, stream_kind='Sim'):
    if stream_kind not in ('Sim', 'Pres'):
        raise ValueError('stream kind must be Sim or Pres')
    with open(path, 'rb') as archive:
        head = archive.read(16)
        archive.seek(0)
        entries = {n.lower(): (o, s) for n, o, s in parse_named(archive.read(word(head, 12)))}

        def read(name):
            offset, size = entries[name.lower()]
            archive.seek(offset)
            data = archive.read(size)
            return refpack(data) or data

        base = 'data\\content\\world\\stream\\bam\\'
        cmap = read(base + 'bam_' + stream_kind.lower() + '.xsm')
        totals = Counter()
        for i in range(min(word(cmap, 16), limit)):
            start = 32 + i * 104
            name = cmap[start+28:start+92].split(b'\0')[0].decode()
            count = word(cmap, start+24)
            stream = read(base + name + '.xsf')
            # Collections sharing an XSF need their own CMAP payload window.
            cursor = word(cmap, start+16)
            for _ in range(count):
                offset = cursor + word(stream, cursor+12)
                blob = stream[offset:offset+word(stream, cursor+8)]
                size, compressed = word(blob, 4), word(blob, 8)
                raw = blob[80:80+size]
                data = refpack(raw) if compressed else raw
                assert len(data) == word(blob, 0)
                table = word(data, 48)
                records = []
                for j in range(word(data, 36)):
                    off, size, kind = word(data, table+j*24), word(data, table+j*24+8), word(data, table+j*24+20)
                    records.append((off, size, kind))
                    if kind in (0xeb0022, 0xeb0013): totals[hex(kind)] += 1
                for off, size, kind in records:
                    if kind == 0xeb0013:
                        rain = data[off:off+size]
                        for label, count_at, pointer_at, stride in [('intersections',0x20,0x2c,608),('roads',0x24,0x30,160),('super_roads',0x28,0x34,64)]:
                            n, p = word(rain,count_at),word(rain,pointer_at)
                            if n == 0: continue
                            assert p + n*stride <= len(rain), (name,label,n,hex(p),len(rain),rain[:64].hex())
                            totals[label] += n
                            for k in range(n):
                                at=p+k*stride
                                if label=='roads':
                                    ns,ps=word(rain,at+0x78),at+word(rain,at+0x7c)
                                    assert ps+ns*224 <= len(rain)
                                    totals['segments']+=ns
                                    for seg in range(ns):
                                        curve=ps+seg*224
                                        assert curve+word(rain,curve+0x48)+word(rain,curve+0x44)*4<=len(rain)
                                if label=='intersections':
                                    np,pp=word(rain,at+0x254),at+word(rain,at+0x250)
                                    assert pp+np*112 <= len(rain)
                                    totals['passages']+=np
                    if kind != 0xeb0022: continue
                    nav = data[off:off+size]
                    np, ne, nc = struct.unpack_from('>3I', nav, 0x50)
                    for n, field, stride in [(np,0x5c,12),(ne,0x60,44),(nc,0x64,56)]:
                        assert word(nav, field) + n*stride <= len(nav)
                    for field in (0x68,0x6c):
                        oid = word(nav, field)
                        assert oid >> 22 == 0, hex(oid)
                        assert oid < len(records), oid
                    costs = [struct.unpack_from('>f',nav,word(nav,0x64)+c*56+48)[0] for c in range(nc)]
                    totals['cells'] += nc
                    if totals['examples'] < 3:
                        print(name, 'points/edges/cells', np,ne,nc,'cost range', (min(costs),max(costs)) if costs else None)
                        totals['examples'] += 1
                cursor += word(stream, cursor+16)
        print(dict(totals))


if __name__ == '__main__':
    audit(sys.argv[1], int(sys.argv[2]) if len(sys.argv)>2 else 284,
          sys.argv[3] if len(sys.argv)>3 else 'Sim')
