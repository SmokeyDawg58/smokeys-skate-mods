# Career research tools

Python readers used for docs/career. Extract `data/db/db.big` into a folder (biglist.py
`entries`/`read`) and point `SKATE2_CAREER_DB` at it (`skater`, `ChallengeCore`,
`progressioncore` .vlt/.bin). `SKATE2_CAREER_NAMES` may name a text file of extra candidate
identifiers (one per line) for resolving hashed class/attribute names.

- `vault.py` — VLT/BIN reader and `Attrib::StringHash64`.
- `vault_dump.py CLASS [N]` — decoded records of a class.
- `luadump.py FILE.bin` — Lua 5.1 chunk statistics (float numbers, little-endian).
- `luacalls.py FILE.bin` — per-handler native call traces (linear, not a decompiler).
- `progression_graph.py "Start State Machine"` — a progression state machine.
- `challenge_graph.py m_1a_01_goto_slappys` — a challenge state graph.

Port-era helpers (2026-10-02, see docs/career/HANDOFF.md):

- `attrib_hash.py NAME...` / `--find 0xKEY NAME...` — `Attrib::StringHash64` and name search.
- `dump_signpost_save.py SKATER.P` — savable order and the SignPost tables of a save.
- `unbound_natives.py [MODULE...]` — natives in `binding_table.rs` the port does not register.
- `../ida/career_ida.py` — IDA py_eval helpers (`dec`, `disasm`, `keys64`, `find`, `show_type`).
