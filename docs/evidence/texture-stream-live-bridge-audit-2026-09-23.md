# Live texture-stream bridge audit, 2026-09-23

Read-only proposal; no implementation or cvar index reserved.

## Source sampling and existing consumer

Fresh initializer82f4e1e0 calls cVarBoolRef at82f4e1e4 with section `Stream`, label
`kEnableTextureStream`, title `Enable Texture Stream`, defaulttrue, callbacknull.
`UpdateGameStreamFileLoaded824e1f48` state5 checks actual simulation XML state==2
at21c4 before dereferencing the live cvar target at21d4. True starts texture at
21f8 then writesstate6; false writesstate7 at2224..223c. Pending XML only calls
Update2250. Bounds calculation occurs on a later state7 poll22b4, then state0 and
returntrue22dc. No null-target default is present at the live read.

Current `game.rs` outer state machine already matches that ordering. Its gate,
`LoadedStreamPublisher` gate in world_stream_adapter.rs, and `TextureStreamGate`
in world_stream_runtime.rs are all infallible. This is the only missing error
transport required to bind the existing live target; no parser/streamer rewrite
is justified.

## Minimal error path

1. Change backend/publisher gate methods to `Result<bool, Self::Error>`.
2. Give the leaf gate a typed fallible result. The shortest non-generic option is
   reusing the actual InputCvarError (make the enum public+Clone if exposed by the
   public trait), wrapped in WorldStreamRuntimeError::TextureGate. This avoids
   parameterizing every existing publication token and runtime latch with a new
   generic error. If public cvar-error exposure is undesirable, an explicit public
   gate-error projection can preserve each actual error kind instead; never map
   error totrue/false or a hardcoded default.
3. Adapter maps the publisher error into its existing shared error representation
   with a dedicated texture-gate phase/variant; this call has no single owned
   file parameter, so do not pretend a file-local latch was updated.
4. Outer state5 maps that error to a new WorldStreamOperation::TextureStreamGate.
   Existing outer poll latches this exact shared cause permanently. Preserve
   state5 and all completed presentation/simulation publications; start no
   texture file and publish no bounds after gate failure.

The production leaf retains the SAME InputCvarReferences clone and calls the
new index's `try_current_boolean` only at that transition. Tests should change the
live target after runtime construction/before state5, testfalse skips texture,
and prove an unlinked/error gate is sampled once and outer-latched without later
backend effects. Existing app debug probe and synthetic backend implementations
merely wrap their deliberate booleans in Ok.

## Retained production composition

Extract the real ownership sequence from physical_player_camera_stream_corpus:
WorldStreamCatalog → MountedStreamBackend(supplied Arc<MountSet>, same catalog)
→ WorldStreamRuntime(same catalog, live gate, supplied effects)
→ WorldStreamAdapter → WorldStreamLoads → supplied WorldStreamSpec handle.
Retain catalog, adapter, loads, handle and SAME WorldBoundsPublication. Expose a
single fallible poll and read-only catalog/bounds capabilities. Do not include
corpus timeouts, yield loops, assertions, environment lookup, fixed BAM naming,
or a second mount/catalog. The existing `WorldStreamLoads::debug_reference`
constructor only sets the source seed1 and HashMap capacity16; factor/rename its
native constructor if desired while retaining the diagnostic alias.

On CompletedThisPoll, the consumer still must read the actual bounds handle;
WorldStreamRuntime's legacy void finalization records a typed bounds error rather
than inventing bounds. A retained owner should propagate that failure before
advertising usable camera bounds. Source state completion and bounds availability
remain distinct where a checked host calculation failed.

Current callers to adapt are camera_stream_corpus::load from ground_startup_corpus
(camera bootstrap) and grind_corpus (same catalog used by resident startup).
Both should pass already-retained actual cvar refs. The corpus keeps its deadline
and assertions outside the new poll owner. selected_resident_startup already
accepts supplied MountSet+WorldStreamCatalog and must consume this same catalog.

## Genuine remaining effect boundary

All current WorldStreamEffects implementations are observations/test probes
(main.rs ProbeWorldStreamEffects, camera StreamObservations, runtime TestEffects).
The runtime already computes and retains real bounds itself, but presentation
add_high_collection still invokes the supplied external effect seam. A production
owner must require a genuine supplied effects owner; it cannot substitute a no-op
or call observations complete native registration. This proposal connects live
cvar, mounted stream publication and bounds without claiming that unrecovered
high-collection gameplay/presentation effects or a full boot scheduler are done.

## Authorized implementation packet

Root reserved index329. New unregistered selected_world_stream.rs/_tests.rs plus
artifacts/live-texture-stream-2026-09-23.patch implement the above connection.
The actual InputCvarError becomes public+Clone and is re-exported as
world_stream_runtime::TextureStreamGateError; no cvar owner is exposed. The
publisher/runtime/adapter errors preserve the exact variant. Outer state5 uses
WorldStreamOperation::TextureStreamGate and its existing permanent shared-cause
latch; there is no new retry or default path.

Fresh raw initializer/table correction: function ENTRY is82f4e1b8, while82f4e1e0
is the `li r7,1` instruction. Callbacknull is `li r8,0` at1dc, constructorcall1e4.
Reference8303dbec and initializer table82fd0368 were verified from raw words.
This precedes all previously registered selected entries (first was82fd1008).
Section/title metadata is Stream / kEnableTextureStream / Enable Texture Stream;
its title-order test row follows the existing State section rows.

SelectedWorldStream requires supplied MountSet Arc, shared catalog, live cvar
references, explicit WorldStreamSpec and WorldStreamEffects. It retains the
original adapter/controller/files and actual bounds publication, performs one
poll, and checks bounds on completed native state. Existing corpus loops retain
120-second deadlines, yields, original BAM selection and finite-bounds assertions
outside this owner. Both actual corpus entry points pass their retained refs.
The app diagnostic probe keeps its explicit atomic bool fixture through Ok.

Tests cover actual unlinked error identity through the public alias; construction
before linkage; late live target modification and manager relink; and an outer
state5 gate failure which preserves state, starts no next-stage work, and returns
the SAME Arc cause without another backend invocation even after its mock failure
is cleared. Existing false-gate stream tests continue to exercise skipping texture.
No author Cargo run. Independent review and root original acceptance are pending.

The required effects remain supplied. Fresh root source audit located actual
AddHighCollection824dc678 proxy-map behavior; this packet neither constructs a
fake empty proxy map nor claims corpus observations fulfill that owner.
