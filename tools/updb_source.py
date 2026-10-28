"""Extract preprocessed HLSL and reflection from original Xbox 360 UPDB files.

Research aid: the final render.big shaders ship `.updb` debug databases whose
`<file id="0">` element holds the complete preprocessed effect source with
original `#line` markers, plus the selected entry point and constant table.

  python updb_source.py SHADER_DIR OUTDIR
Writes OUTDIR/<effect>.hlsl (one per distinct source) and OUTDIR/index.txt.
"""

import hashlib
import re
import sys
from pathlib import Path
from xml.etree import ElementTree


def load(path):
    root = ElementTree.fromstring(path.read_bytes())
    source = next(f.text or "" for f in root.iter("file") if f.get("id") == "0")
    shaders = [(s.get("entrypoint"), s.get("target")) for s in root.iter("shader")]
    constants = []
    for table in root.iter("constanttable"):
        for c in table.iter("constant"):
            constants.append(
                (c.get("name"), c.get("register"), c.get("count"), c.get("regset") or c.get("type"))
            )
    return source, shaders, constants


def main():
    shader_dir, outdir = map(Path, sys.argv[1:3])
    outdir.mkdir(parents=True, exist_ok=True)
    by_hash, index = {}, []
    for path in sorted(shader_dir.glob("*.updb")):
        source, shaders, constants = load(path)
        digest = hashlib.sha256(source.encode()).hexdigest()[:12]
        effect = path.stem.rsplit("_", 1)[0]
        if digest not in by_hash:
            by_hash[digest] = effect
            (outdir / f"{effect}.{digest}.hlsl").write_text(source, encoding="utf-8")
        index.append(f"{path.name}\tsource={by_hash[digest]}.{digest}\tentry={shaders}\tconstants={constants}")
    (outdir / "index.txt").write_text("\n".join(index), encoding="utf-8")
    print(f"{len(index)} updb files, {len(by_hash)} distinct sources")


if __name__ == "__main__":
    main()
