# HoM replay exit: missing clothing matrices

Date: 2026-10-14. PC checkout only; solo.
Status: source fixed, syntax parsed. **No build.**

## Report and cause

After a Hall of Meat replay, a playtest hit `Stage::Visual` with
`material ropa state: selected character palette part 0 has no current matrices`.

The replay-return clothing repair from `replay-cloth-return-2026-10-14.md`
passes the retained game palette through `step_restored_cloth` and
`publish_cloth`. Both operations read `CharacterPalette::cloth_matrices`.
`pack_for_upload` has already moved each part's local matrices out of
`PalettePart::matrices` with `take()` and into `PalettePart::packed`.
The accessor only read the emptied field, so this valid, completed game pose
failed as if no pose existed. The matrices were still retained; part 0 is the
reported part index, not evidence of a missing character asset.

Source chain:

- Career's paused and running replay-exit branches call `restore_game_cloth`.
- Replay Editor exit calls the same helper.
- The helper visits the published local/party/AI runtime owners, excluding
  borrowed NIS characters, with the existing cloth gates.
- `SelectedPresentationRuntime::restore_game_cloth` steps and publishes the
  retained game pose before the paused world can draw again.
- `SkaterRopaRuntime::skin_drivers` and `publish` both need its local matrices.

## Reference evidence

Live IDA was unavailable. IDA 9.3 batch extraction used the existing
external reference database `world_probe_temp/waypoint_replay_20261014.i64`.
Input SHA256: `3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.

New decompiler/disassembly packet: `.scratch/hom-cloth-matrices-20261014/native.json`.
Also re-read the prior packet `.scratch/replay-cloth-return-20261014/native.json`.

| Native function | Address | Relevant contract |
| --- | --- | --- |
| `Sk8::UpdateBoneTransforms` | `0x827efb78` | Gathers remapped local matrices from the current bone buffer into each model instance. |
| `SkaterPresEntity::InitSkinParams` | `0x827d2938` | Calls that gather; reads model `m_matrices` at +0x14 and binds it to the CPU skin parameters at `0x827d2f30..2f4c`. |
| `SkaterPresEntity::StartJobs` | `0x827dbb90` | Gathers transforms before `BeginSkinJob`, with existing cloth-ready/disabled gates. |
| `SkaterPresEntity::BeginSkinJob` | `0x827d7c48` | Starts the skin job and its cloth dependency. |
| `SkaterPresEntity::EndJobs` | `0x827dbd50` | Completes skin jobs, then packs model matrices for upload. |
| `cModelInstance::PackAndMultiplyMatricesForUpload` | `0x828221e0` | Converts local 4x4 inputs into world-space 3x4 rows, in place in the native buffer. |
| `SkaterPresEntity::Update` | `0x827e8568` | Runs StartJobs/EndJobs for a local entity not restored this frame, without requiring another physics tick. |

The native upload overwrites its matrix buffer with packed rows; it gathers
local matrices again before the next skin job. The port keeps those exact
pre-upload matrices separately in `packed`. This fix uses that retained local
pose, **not** the world-space GPU rows, when restoring the same game pose.
It does not assert that native packed rows are suitable for CPU skinning.

## Change

Only `CharacterPalette::cloth_matrices` changed:

- A successfully published palette reads its retained local matrices from `packed`.
- An unpublished palette continues to read its current `matrices` allocation.
- Missing parts/matrices still error. An unfinished/failed publication cannot
  fall back to the older published pose because `published` is false.

The accessor borrows the existing allocation. No extra pose copy, animation
update, simulation tick, arithmetic, cloth tuning, restart rule, or time value
was introduced. GPU generation and clothing publication order remain as in the
previous replay-return repair. The fix is shared by HoM, film/photo replay and
Replay Editor return paths.

## Review and limits

Confidence: **high** for the reported error's source cause and the native
local-matrix contract. The failed runtime report itself has not been reproduced.

Reviewed the constructor, gather, successful and partial upload, repack, both
cloth consumers, and all three replay-exit call sites. A successful upload moves
the same vector to `packed`; a subsequent gather clears `published` before
writing new matrices; partial uploads therefore cannot select retained old data.

`rustfmt --edition 2024 --config skip_children=true --emit stdout` parsed the
changed source successfully without rewriting it. No tests added or run, no game
launch, and no build started. The existing release executable does not include this fix.

Source/EXE/doc backups and the scoped diff/hash record are in
`.scratch/hom-cloth-matrices-20261014/`. Android and real saves were not changed.
