"""Export the 20-entry `Sk8::BigFileMan::mBigFileInfo` table."""

from __future__ import annotations

import json
from pathlib import Path

import ida_bytes


ADDRESS = 0x82FE5218
COUNT = 20
RECORD_SIZE = 264
OUTPUT = Path(__file__).resolve().parents[2] / "artifacts" / "bigfile-table.json"


def main() -> None:
    data = ida_bytes.get_bytes(ADDRESS, COUNT * RECORD_SIZE)
    if data is None or len(data) != COUNT * RECORD_SIZE:
        raise RuntimeError("could not read BigFileMan::mBigFileInfo")
    records = []
    for index in range(COUNT):
        record = data[index * RECORD_SIZE : (index + 1) * RECORD_SIZE]
        records.append(
            {
                "index": index,
                "mount_group": int.from_bytes(record[0:4], "big"),
                "use_big_header": bool(record[4]),
                "filename": record[5:261].split(b"\0", 1)[0].decode("ascii"),
            }
        )
    OUTPUT.write_text(json.dumps(records, indent=2), encoding="utf-8")
    print(f"Skate 2 BIG-file table export complete: {OUTPUT}")


if __name__ == "__main__":
    main()

