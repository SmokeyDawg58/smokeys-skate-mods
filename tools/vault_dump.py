"""Read-only AttribSys (VLT/BIN) dumper for research.

Mirrors crates/skate2-formats/src/attrib.rs (chunk, export, fixup layouts) and
the app's `resolve_attribute_location` precedence (explicit collection entries
through the parent chain, then static (flag 0x10) or layout (flag 0x2) class
defaults). Names are recovered by matching `Attrib::StringHash64` against the
BIN string table plus an optional extra name list.

  python vault_dump.py VLT BIN classes
  python vault_dump.py VLT BIN class NAME [COLLECTION_NAME_OR_HEX] [--names FILE]
"""

import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from match_attribute_hashes import string_hash64  # noqa: E402

CLASS_TYPE = 0x2A7895AC4A876152
COLLECTION_TYPE = 0xAD303B8F42B3307E


def u16(b, o): return struct.unpack_from(">H", b, o)[0]
def u32(b, o): return struct.unpack_from(">I", b, o)[0]
def u64(b, o): return struct.unpack_from(">Q", b, o)[0]


class Vault:
    def __init__(self, vlt, binary):
        self.buffers = [vlt, binary]
        chunks, offset = {}, 0
        while offset < len(vlt):
            tag, size = vlt[offset:offset + 4], u32(vlt, offset + 4)
            chunks[tag] = (offset, size)
            offset += size
        exp_off, _ = chunks[b"ExpN"]
        count = u64(vlt, exp_off + 8)
        self.exports = []
        for i in range(count):
            o = exp_off + 16 + i * 24
            self.exports.append((u64(vlt, o), u64(vlt, o + 8), u32(vlt, o + 16), u32(vlt, o + 20)))
        ptr_off, ptr_size = chunks[b"PtrN"]
        self.pointers, target = {}, None
        for i in range((ptr_size - 8 - 24) // 16):
            o = ptr_off + 8 + i * 16
            delta, encoded, argument = u32(vlt, o), u32(vlt, o + 4), u64(vlt, o + 8)
            kind, dependency = encoded >> 16, encoded & 0xFFFF
            if kind == 2:
                target = dependency
            elif kind == 3:
                self.pointers[(target, delta)] = (dependency, argument)
        self.classes, self.collections = {}, {}
        for object_id, type_id, size, offset in self.exports:
            if type_id == CLASS_TYPE:
                self.classes[u64(vlt, offset)] = offset
            elif type_id == COLLECTION_TYPE:
                self.collections[(u64(vlt, offset + 8), u64(vlt, offset))] = offset
        self.names = {}
        table_end = u32(binary, 4)
        for raw in binary[8:table_end].split(b"\0"):
            if raw:
                self.add_name(raw.decode("latin-1"))

    def add_name(self, name):
        self.names.setdefault(string_hash64(name), name)

    def name(self, key):
        return self.names.get(key, f"{key:016x}")

    def definitions(self, class_offset):
        vlt = self.buffers[0]
        dependency, offset = self.pointers[(0, class_offset + 16)]
        data = self.buffers[dependency]
        result = []
        for i in range(u32(vlt, class_offset + 12)):
            o = offset + i * 24
            result.append(dict(key=u64(data, o), type=u64(data, o + 8), offset=u16(data, o + 16),
                               size=u16(data, o + 18), max=u16(data, o + 20), flags=data[o + 22]))
        return result

    def collection(self, offset):
        vlt = self.buffers[0]
        types_length, entry_count = u16(vlt, offset + 38), u32(vlt, offset + 32)
        entries_start = offset + 48 + types_length * 8
        entries = {}
        for i in range(entry_count):
            o = entries_start + i * 16
            entries[u64(vlt, o)] = (o + 8, vlt[o + 14], vlt[o + 15])
        return dict(id=u64(vlt, offset), cls=u64(vlt, offset + 8), parent=u64(vlt, offset + 16),
                    entries=entries, layout_slot=offset + 40)

    def locate(self, class_offset, collection_offset, definition):
        current = collection_offset
        while True:
            c = self.collection(current)
            if definition["key"] in c["entries"]:
                slot, node_flags, _entry_flags = c["entries"][definition["key"]]
                # Collection ctor 0x8297be90/0x8297bef0 reads serialized +14.
                # Runtime Node moves these flags to +15; the serialized trailing
                # byte is not that runtime field. Match attrib.rs's reader.
                if node_flags & 0x40:
                    return ("inline", self.buffers[0][slot:slot + 4])
                return ("entry",) + self.pointers.get((0, slot), (None, None))
            if c["parent"] == 0:
                break
            current = self.collections[(c["cls"], c["parent"])]
        if definition["flags"] & 0x10:
            dependency, offset = self.pointers[(0, class_offset + 24)]
            return ("static", dependency, offset + definition["offset"])
        if definition["flags"] & 0x2:
            dependency, offset = self.pointers[(0, collection_offset + 40)]
            return ("layout", dependency, offset + definition["offset"])
        return ("absent",)


def floats(raw):
    return [round(v, 6) for v in struct.unpack(f">{len(raw) // 4}f", raw[: len(raw) // 4 * 4])]


def main():
    vlt_path, bin_path, command, *args = sys.argv[1:]
    names_file = None
    if "--names" in args:
        names_file = args[args.index("--names") + 1]
        args = args[: args.index("--names")]
    vault = Vault(Path(vlt_path).read_bytes(), Path(bin_path).read_bytes())
    if names_file:
        for line in Path(names_file).read_text(encoding="utf-8", errors="ignore").split("\n"):
            if line:
                vault.add_name(line)
    if command == "classes":
        counts = {}
        for (cls, _), _ in vault.collections.items():
            counts[cls] = counts.get(cls, 0) + 1
        for cls in sorted(vault.classes, key=vault.name):
            print(f"{vault.name(cls)}  collections={counts.get(cls, 0)}")
        return
    class_id = string_hash64(args[0])
    class_offset = vault.classes[class_id]
    definitions = vault.definitions(class_offset)
    print(f"class {args[0]} {class_id:016x}: {len(definitions)} definitions")
    for d in definitions:
        print(f"  def {vault.name(d['key'])}: type={vault.name(d['type'])} off={d['offset']} size={d['size']} max={d['max']} flags={d['flags']:#x}")
    collections = sorted((cid for (cls, cid) in vault.collections if cls == class_id), key=vault.name)
    print("collections:", [vault.name(c) for c in collections])
    wanted = args[1:] or []
    for cid in collections:
        label = vault.name(cid)
        if wanted and label not in wanted and f"{cid:016x}" not in wanted and f"{cid & 0xffffffff:08x}" not in wanted:
            continue
        coffset = vault.collections[(class_id, cid)]
        parent = vault.collection(coffset)["parent"]
        print(f"== collection {label} ({cid:016x}) parent={vault.name(parent) if parent else '-'}")
        for d in definitions:
            where = vault.locate(class_offset, coffset, d)
            if where[0] == "absent":
                continue
            if where[0] == "inline":
                raw = where[1]
            else:
                dependency, offset = where[1], where[2]
                if dependency is None:
                    print(f"  {vault.name(d['key'])}: <no pointer>")
                    continue
                raw = vault.buffers[dependency][offset: offset + max(d["size"], 4) * max(d["max"], 1) + 16]
            print(f"  {vault.name(d['key'])} [{where[0]}] hex={raw[:64].hex()} f32={floats(raw[:64])}")


if __name__ == "__main__":
    main()
