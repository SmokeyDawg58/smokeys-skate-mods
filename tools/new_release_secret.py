"""Writes release-secret.txt at the repository root: the release key build-release.bat and
build-release.sh read (apps/build_secret.rs), so the Windows game and the Linux server of one
version get the same build secret. Keep it private, and copy this same file to every machine that
builds a release. An existing key is kept unless --force is given: a new key changes the secret of
every later build.

  python tools/new_release_secret.py [--force]
"""

import secrets
import sys
from pathlib import Path

path = Path(__file__).resolve().parent.parent / "release-secret.txt"
if path.exists() and "--force" not in sys.argv[1:]:
    sys.exit(f"{path} exists; pass --force to replace it")
path.write_text(secrets.token_hex(32) + "\n", encoding="ascii")
print(f"wrote {path}")
