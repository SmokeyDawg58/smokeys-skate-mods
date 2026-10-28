# Selected session terrain/input extension (2026-09-23)

The independently reviewed extension is registered as
`crates/skate2-core/src/session/selected_session_terrain_input_extension.rs` through
`selected_session_boot_owner.rs`. The reviewed registration patch is archived at
`artifacts/selected-session-terrain-input-extension-20260923.patch`.

The patch adds separate retained slots to `SelectedSession<T, C>` for one
`SelectedSessionTerrain` behind a small object-safe delegation interface, its
typed constructor error, one successful `SelectedProductionInputOwner<C>`, and
one `ProductionInputBindFailure<C>`. The terrain effect type is chosen by the host
at `try_select_terrain`, so the existing two-parameter session constructor and
the already registered input identity corpus remain source-compatible. The
interface delegates stream polling, the one resident request and poll, world
bounds, normal terrain, and the static ground pool to the exact underlying
terrain owner. Its ground-pool/terrain reads still enforce the owner's world-Arc
identity; no second world, mount, cvar, stream or arena implementation appears.

`try_select_terrain` takes an explicit catalog, spec, effects, unlock seed,
collection, DMO-parallel answer and raw arena userdata. It clones this session's
world Arc, asset MountSet Arc and cvar references into `SelectedTerrainSource`.
The one-shot flag is set before source construction. Missing boot assets/world
are precondition errors without a request. If `SelectedTerrainSource::try_new`
fails, the session retains its typed error and reports `TerrainStartFailed`.
That lower constructor consumes and drops the supplied effects and unlock seed,
and can drop a successfully constructed router if the later stream constructor
fails. It does not return a partial source or recoverable inputs. This is a
bounded constructor limitation, not full partial-owner retention; the session
does not retry. Subsequent stream/request/resident/registration failure
semantics remain in `SelectedSessionTerrain`.
This seam does not choose native collection/spatial focus or assert terrain
request timing relative to the physical pair.

`try_bind_selected_input` runs only after the player exists, moves the exact
early bootstrap into the existing `try_bind_player`, and stores either the
bound owner or the entire `ProductionInputBindFailure` (bootstrap, queue and
RNG together). Precondition errors return the untouched supplied queue/RNG to
the caller. The returned bind-error reference points into the retained failure
slot. `next_gate` recognizes the successful bound owner after the bootstrap is
moved and reports `InputBindFailed` when the failed bind is retained, instead
of suggesting InputManager initialization must repeat. No corpus remap,
controller default, offline seed or frame schedule is
introduced.

The source owner remains responsible for camera capture, local roster admission,
full queued-frame services, and app hardware cadence described in
`selected-session-terrain-input-frame-handoff-2026-09-23.md`. This patch does
not claim a runnable session. Validation: `rustfmt --edition 2024 --config
skip_children=true --check` passes for the unregistered implementation and
`git apply --check --ignore-space-change` passes for the registration patch.
After integration, `cargo check --workspace` passes in
`artifacts/check-terrain-input-extension-2026-09-23.log`. The focused
original-data input identity test now binds through this retained session slot
and passes in `artifacts/original-selected-retained-input-bind-2026-09-23.log`.
The separate selected-terrain original-data test already exercises the
underlying retained source and admission; a same-session terrain/input/frame
test is still needed.
