"""Read-only filesystem checks for skate2-core's explicit source layout.

Run from any working directory. This checks literal path/include edges, not Rust
types or cfg semantics; use cargo check for compiler validation.
"""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1] / "crates/skate2-core/src"
PATH = re.compile(r'#\[path\s*=\s*"([^"\r\n]+)"\]')
INCLUDE = re.compile(r'\binclude(?:_str|_bytes)?!\(\s*"([^"\r\n]+)"\s*\)')


def main():
    errors = []
    files = sorted(ROOT.rglob("*.rs"))
    flat = [p.name for p in ROOT.glob("*.rs") if p.name != "lib.rs"]
    if flat:
        errors.append(f"Place these files in their subsystem folders: {flat}")
    edges = 0
    for source in files:
        text = source.read_text(encoding="utf-8")
        for value in PATH.findall(text) + INCLUDE.findall(text):
            edges += 1
            target = (source.parent / value).resolve()
            if not target.is_relative_to(ROOT.resolve()) or not target.is_file():
                errors.append(f"{source.relative_to(ROOT)}: unresolved/outside source target {value}")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Layout OK: {len(files)} Rust files, {edges} literal module/include paths; only lib.rs at source root.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
