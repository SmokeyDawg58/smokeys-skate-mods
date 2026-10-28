# Core source organization — September 29

This applies the physical folder organization previously proposed, with
compiler verification. This turn was SOLO. No gameplay/physics/AI logic was
changed and no game or controller test was launched.

## Result

- Moved 1,977 Rust files from `crates/skate2-core/src` into subsystem folders.
  All 1,978 original Rust files remain; only `lib.rs` is at the source root.
- Organized actors, animation, assets, camera, input, LivingWorld, physics,
  rendering, session, streaming, world, math and diagnostics. Larger areas have
  subfolders; the largest direct file group contains 92 Rust files.
- Kept existing filenames, logical Rust module identities, visibility, cfg
  attributes, macro order, public exports and `crate::`/`super::` relationships.
  Only literal `#[path]` and `include!` paths changed within existing Rust source.
  New top-level path attributes in `lib.rs` preserve its original declarations.
- Tests/corpus helpers remain in their owning subsystem areas. Existing files
  outside the currently reachable module graph were retained, not deleted.
- Updated fully qualified source paths in 47 Markdown files. Historical patch
  and one-time code-generation scripts are retained as evidence, not rewritten
  into new generators. The relocation index resolves their original filenames.
- Updated active `tools/graph_query.py` to use repository-relative locations for
  both graphs. Added `tools/check_core_layout.py`, `docs/CORE_LAYOUT.md`, the
  source-directory README and a complete `docs/core-file-moves.tsv` index.

## Verification

Before moving files, `cargo check --workspace --all-targets --offline -j 1`
compiled the normal libraries and PC application, then exposed two pre-existing
test compilation failures:

1. `physics/grind_data_runtime_tests.rs:111`: `Arc::downgrade(&source.owner)`
   expects an Arc, but the current owner is `GrindSplineOwner` (E0308).
2. `session/selected_session_public_facade_corpus.rs:137`: the boot-progress
   match does not handle `SelectedBootProgress::DmoBankPending` (E0004).

After the move:

- `cargo check --workspace --offline -j 1` **PASS**, with existing warnings.
- `cargo check --workspace --all-targets --offline -j 1` reports **exactly those
  same two existing test errors**. No new compiler errors. Test fixtures were
  not changed to conceal or mix those fixes into this structural task.
- Byte/hash audit of all 1,978 Rust files passed: after normalizing only path
  attributes and the single include target, contents match the pre-move bytes.
- Every literal module/include edge preserves the same target and multiplicity.
  All 1,972 resulting literal path edges resolve within the source directory.
- No missing source files, duplicate destinations, overwrites or forwarding
  copies. Original source count and filename set preserved.
- Active Motion state20 and Action state37 graph queries succeed after relocation.
- 94 documented full core Rust path references resolve.

No release binary was rebuilt, no tests executed and no runtime parity claim is
made. Compiler checks do validate the current normal workspace after the earlier
handplant and vehicle fixes, but not their gameplay behavior. No whole-workspace
formatting was applied: existing Rust contents were preserved apart from paths.

## Handoff

Use [CORE_LAYOUT.md](../CORE_LAYOUT.md) and the [source map](../../crates/skate2-core/src/README.md).
Future additions belong in a subsystem folder, with explicit module ownership;
search recursively before implementing anything. Do not create old-path copies.

Reversible migration records are outside the repo:
`original-core.zip`, the before/after SHA256 `manifest.json`, original changed
documentation bytes, and compiler logs. All original untracked work was included.
No Git reset, checkout, clean, staging or commit was performed.
