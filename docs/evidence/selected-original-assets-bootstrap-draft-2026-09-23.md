# Selected original-assets bootstrap draft, 2026-09-23

Unregistered implementation packet for independent review. Files:
`crates/skate2-core/src/selected_original_asset_classes_draft_20260923.rs`
and `crates/skate2-core/src/selected_original_assets_bootstrap_draft_20260923.rs`.
No registered source or Cargo file was edited. Standalone rustfmt checks pass;
there is no type-check or execution claim until root registers the modules.

## Owned inputs and current mismatch

`physical_player_assembly_corpus::with_original_inputs` obtains a mounted
Vault through `attribute_runtime_corpus::mounted_physics_attributes_with_classes`.
That helper prepends three base classes, then the caller supplies 72 additional
classes inline. The corpus obtains live cvars from
`game_input_recognizer_corpus::original_input_cvars`, which links
`InputCvarManager` to `InputCvarReferences` and reads external
`data/startup.ini`. The repeated-frame corpus later parses
`data\\content\\world\\stream\\BAM\\BAM_Sim.loc` and uses the first
`A_RobsPark` transform as a host-selected first-test spawn. These are already
real readers but assertion-based, test-only compositions. The registered
`selected_session_world_startup` and `selected_session_player_startup`
currently receive Vault/cvars/spawn externally.

The draft `SelectedOriginalAssets::try_new(Arc<MountSet>, &Path)` calls the
existing `AttributeRuntimeVault::try_load_mounted` with one exact selected
class vector; constructs/links the existing cvar manager, reads the same
startup path and deserializes its bytes; then reads the location through the
same supplied mount and retains the authored Rob's Park transform. The
returned struct retains the mount, Vault, cvar manager, references and spawn.
It does not reopen any BIG, introduce a new Vault parser, change ini
conversion, or construct world/Actor/presentation owners.

The class module records the three base keys and returns the exact 72-entry
corpus additional selection, in the same order and with no deduplication.
An entry-by-entry text comparison of noncomment expressions reported
`corpus 72 draft 72 mismatch None`. This is the class selection needed by
this bounded corpus path; it is not evidence of a complete native Vault
registration list or unrelated game content.

## Native evidence and confidence

Direct IDA `LoadAttributeSystemVault82ab99f0` first hashes/looks up the
requested Vault path and calls `FileLoader::SyncLoad` on a miss, then
constructs and publishes the Vault. The existing Rust
`attribute_runtime_mounted.rs` explicitly stops at an immutable selected
post-load graph; the draft calls that reader and does not claim native map
publication or constructor sequencing. Confidence: high for reuse, no
native parity claim.

Direct IDA `Sk8AppMan::InitBootDebug8272e490` passes
`data\\startup.ini` to `cVarSerializer::Deserialize`.
`Deserialize82a8ad50` reads through `cIniFile` and visits the root section
only on successful load. The existing Rust cvar manager already implements
selected links/deserialization. The draft makes unreadable startup bytes a
typed host boot error instead of silently proceeding with initial targets;
that is an explicit strict first-test admission policy, not a claim about the
native missing-file branch. Confidence: exact for call and path; high for
reusing the existing selected Rust implementation.

The `A_RobsPark` location choice is established by the mounted corpus and
`BAM_Sim.loc`, not by a recovered native SpawnManager branch. Confidence:
exact for the authored serialized lookup, hypothesis for whether the native
free-roam spawn manager would select that transform. The draft names and
documents it as first-test host policy.

## Typed failure sequence and identity limit

Errors preserve the reached stage: mounted Vault/dependency/read failure,
startup file IO, cvar link/deserialize, location mount read, location parse,
then missing `A_RobsPark`. No missing class, file, or location receives a
fabricated value. The root path is retained for audit, and the mount Arc is
the same caller-supplied owner used for both Vault and location reads.
`MountSet` does not expose a canonical source root, so the constructor
cannot prove that the supplied root's startup.ini came from the same external
directory as the mount. The app mount owner must supply the matching pair and
report both paths when first booting. A future mount-source identity value can
make that relationship checked; it should not be asserted from this type.

## Atomic registration move for the root

Register the two draft modules in `skate2-core/src/lib.rs` together. In
`attribute_runtime_corpus.rs`, replace the helper's private three-value
`selected_classes` prefix with
`selected_original_asset_classes_draft_20260923::BASE_PHYSICS_CLASSES.to_vec()`,
then extend with caller additions exactly as before. In
`physical_player_assembly_corpus.rs`, replace the entire inline
`mounted_physics_attributes_with_classes(&[...72 entries...])` expression
with
`mounted_physics_attributes_with_classes(&crate::selected_original_asset_classes_draft_20260923::selected_additional_classes())`.
Remove now-unused imports for the moved class constants from that corpus.
These replacements must happen in the same integration so the class list
has one registered source of truth. Rename the draft modules after review if
desired, updating all paths in that one move. The bootstrap itself calls
`selected_classes()`, which concatenates the same shared base and
additional lists.

Before integration, independent review should check each selected class
symbol/value and the linked cvar/initialization ownership. Root can then
type-check, run the mounted selected 23/23 original-data baseline and the
scripted 120-tick Kickflip, and add a focused mounted bootstrap probe that
compares Vault class availability, live cvar reads and all 16 Rob's Park
transform bits against the prior corpus path. The first-test boot must still
choose and supply the same source-matched mount/root pair and complete the
separate player/physical/presentation construction. This draft alone cannot
launch a window or prove native PROXY activation.
