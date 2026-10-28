"""Read-only extractor for the original named BIG archives (research aid).

Mirrors crates/skate2-formats/src/big.rs (named BIG* tables) and refpack.rs
(`RefpackBuf::Decompress` 0x82bc9140). Originals stay outside the repository:
pass an output directory outside the repository.

  python extract_big.py ARCHIVE.big OUTDIR [substring ...]
"""

import struct
import sys
from pathlib import Path


def parse_named(data):
    variant = data[3]
    width = 4 if variant in b"F4" else variant - ord("0")
    count, header_size = struct.unpack(">II", data[8:16])
    cursor, entries = 16, []
    for _ in range(count):
        offset = int.from_bytes(data[cursor : cursor + width], "big")
        cursor += width
        size_width = min(width, 4)
        size = int.from_bytes(data[cursor : cursor + size_width], "big")
        cursor += size_width
        end = data.index(b"\0", cursor, header_size)
        entries.append((data[cursor:end].decode("latin-1"), offset, size))
        cursor = end + 1
    return entries


def refpack(data):
    flags = int.from_bytes(data[:2], "big")
    if flags & 0x3EFF != 0x10FB:
        return None
    width = 4 if flags & 0x8000 else 3
    cursor = 2 + (width if flags & 0x0100 else 0)
    size = int.from_bytes(data[cursor : cursor + width], "big")
    cursor += width
    out = bytearray()

    def literals(count):
        nonlocal cursor
        out.extend(data[cursor : cursor + count])
        cursor += count

    def back(distance, length):
        start = len(out) - distance
        for index in range(length):
            out.append(out[start + index])

    while True:
        control = data[cursor]
        cursor += 1
        if control <= 0x7F:
            low = data[cursor]
            cursor += 1
            literals(control & 3)
            back(((control << 3) & 0x300) + low + 1, ((control >> 2) & 7) + 3)
        elif control <= 0xBF:
            second, third = data[cursor], data[cursor + 1]
            cursor += 2
            literals(second >> 6)
            back((second & 0x3F) * 256 + third + 1, (control & 0x3F) + 4)
        elif control <= 0xDF:
            second, third, fourth = data[cursor : cursor + 3]
            cursor += 3
            literals(control & 3)
            back((control & 0x10) * 4096 + second * 256 + third + 1, ((control << 6) & 0x300) + fourth + 5)
        elif control <= 0xFB:
            literals(4 * ((control & 0x1F) + 1))
        else:
            literals(control & 3)
            break
    if len(out) != size:
        raise ValueError(f"RefPack size {len(out)} != declared {size}")
    return bytes(out)


def main():
    archive, outdir, *filters = sys.argv[1:]
    data = Path(archive).read_bytes()
    entries = parse_named(data)
    written = 0
    for name, offset, size in entries:
        normalized = name.replace("\\", "/").lower()
        if filters and not any(f.replace("\\", "/").lower() in normalized for f in filters):
            continue
        raw = data[offset : offset + size]
        payload = refpack(raw) or raw
        target = Path(outdir) / name.replace("\\", "/")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(payload)
        written += 1
    print(f"{len(entries)} entries, {written} written to {outdir}")


if __name__ == "__main__":
    main()
