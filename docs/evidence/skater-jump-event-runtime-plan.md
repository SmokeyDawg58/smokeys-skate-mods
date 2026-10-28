# Actual jump event and prepared-velocity producer

Read-only source audit, 2026-09-08. Debug XEX/PDB, fresh IDA raw instructions and
decompilation. This packet proposes the next bounded producer; it does not claim
Actor scheduling, gesture recognition, or playable jump integration. Broader DMO
work remains parked. No Cargo or production edits were performed for this audit.

## Actual record owner and publication

`Sk8::Actor::SetUpPhysics` is `825ff6c0`. The actual sequence is:

1. Capture the actual motion-graph and animation collaborators; call the animation
   collaborator at `f710` (decompiler virtual method names are not relied upon).
2. Reset the retained `mAnimOutPhysIn` at `f740`, then animation and graph physics
   exports at `f75c` and `f778`; copy graph intents at `f79c`.
3. Clear only the attribute vector end to its begin at `f7ac`.
4. Obtain the graph's MgAttributes at `f7bc`, append them at `f7cc`.
5. Obtain animation attributes at `f7e0`, append them at `f7f0`.
6. Perform the remaining actual Actor physics-input exports, then call the actual
   Physical `SetPhysUpdateData` at `f978`.

`PhysicalPlayerHiLOD::SetPhysUpdateData` (`82dfe688`) stores that exact
`AnimOutPhysIn*`; it does not reinterpret the events. Skeleton's retained raw owner
is `+2ca4`, and its attribute vector is `AnimOutPhysIn+a70`. The native record
stride is `0x40`, not a host scalar-command list. Graph records precede clip
records, and repeated records retain their source order.

`AnimOutPhysInAtts::AddAttrs(MgAttributes const&)` (`825fdc98`) constructs the
native event form: scalar payload at `+00`, begin/end `-1`, packed five-word name
at `+20..+33`, status byte `6` at `+34`, type byte `0` at `+35`, sequence ID `-1`.
Its source is the actual ordered MgAttributes collection. The Andale overload
(`825fdbf0`) copies the complete existing runtime Attribute, retaining type,
status, name, payload and order. Both use the actual fixed-vector append path;
the in-capacity path publishes the advanced end before its placement copy.
Capacity/error semantics should be recovered separately if the next bounded
projection attempts to replace that container rather than borrow its records.

## Type-0 dispatch and its irreversible prefix

`Skeleton::ProcessAnimAttributes` (`82b59de8`) loads the signed type byte first
at `a418`, then copies the five packed event-name words at `a420..a43c`.
Type-3's `status & 0x0c` gate is separate. The type-0 gate at `a6f4..a6f8`
does **not** test active status and does **not** require a nonzero scalar payload.
Do not transfer the existing contact handler's payload-name interpretation:
contact bone name lives at `+00..+13`, whereas event name lives at `+20..+33`.

The exact packed `PrepareJump` name is the `Sk8::PrepareJumpAtt` object at
`830a5544`; its initializer `82f822d0` assigns the literal `PrepareJump`.
The existing `kJumpName` initializer `82f7ca70` assigns `jump`. Compare packed
FastString words, without a semantic-name fallback.

For PrepareJump (`82b5ae08..aeb8`):

- Compare the event name, load flags at processed `+8c4`, OR `0x00200000`,
  **store flags at `ae2c` before querying Actor input**.
- Read actual Skeleton `mIActorQuery+3e70`, call GetInput at `ae40`. Null input
  preserves the previous Skeleton jump adjustments and proceeds to the next
  attribute; the prepare bit remains set.
- Read the returned input's actual map/pad and g0..g3 fields; sample map ID
  `74` at `ae6c`, then ID `75` at `ae98`. Each call reads the current input
  fields in the raw source order; do not prefetch a combined axis snapshot.
- Lower-bound X between those calls. After the second returns, lower-bound Y,
  upper-bound X, store Skeleton `mJumpAdjustX+3e34` at `aeac`, upper-bound Y,
  store `mJumpAdjustZ+3e38` at `aeb8`. Failure of the second call leaves both
  previous adjustments intact, while retaining the already-published flag.

The bounds are the actual `fsubs`/`fsel` graph, not Rust `clamp`: NaN reaches
`+1` after the two selections; signed zero is retained. Repeated PrepareJump
records call the input service repeatedly. A `jump` match at `aec8` separately
ORs `0x00400000` into the current flags at `aed8..aee0` and reads no payload.
The real ProcessedPhysIn Reset stores flags `0x00002000` at `82b39428`.
A bounded jump projection must share that word and preserve other producers;
it must not silently replace the reset with zero or own an unrelated bool pair.

## Post-loop strength and adjustment are distinct source stages

After the full record traversal, `b79c..b8e4` conditionally resolves jump
strength at processed `+950`. It reads global `anim_motion_jumping` at singleton
`+124`, retaining separate instance captures and lookup/value timing:

| Key | Read | Effect |
| --- | --- | --- |
| `b52e6ee4f322a9f4` | byte, missing DefaultDataArea(1) | Enable endpoints |
| `982f499ec97735a9` | float, missing DefaultDataArea(4) | Lower endpoint |
| `a1f65bb8c4d1ba8b` | float, missing DefaultDataArea(4) | Upper endpoint |

The lower comparison at `b868/b86c` takes its upper path only for ordered GT;
otherwise it stores the last MinJump scalar. The upper comparison at
`b8d0/b8d4` takes the authored-height path only for ordered LT; otherwise it
stores `1`. Thus unordered values do not match the ordinary `<=`/`>=` pair in
the existing semantic `physics.rs::apply_jump_strength`. Hippy flag `+8d0 &
0x8000` can subsequently overwrite strength with authored height at `b8f4`.
The local height/minimum event producers and their entry initialization must
remain part of the eventual strength implementation, not guessed defaults.

Adjustment publication follows, even outside the start-jump branch. The global
instance at singleton `+d0` supplies key `34bb37c315bc14b0` (byte, missing
DefaultDataArea(1), lookup `b938`). True copies retained Skeleton X then Z to
processed `+954/+958` at `b95c..b968`. False calls current Actor GetInput again;
nonnull samples IDs **74 and 78**, at `b9ac/b9d8`, and stores clamped X then Z
at `b9ec/b9f8`. Null preserves the current processed values. This is not the
PrepareJump sampling pair (74/75), and cannot be folded into that earlier call.

## Prepared velocity is a HiLOD lifetime field

PDB: `PhysicalPlayerHiLOD+mPrepareJumpVel` is full Vector3 storage at `+c0`.
Constructor `82e03c50` explicitly creates full zero, then stores it at
`82e03dac`: r25 is zero at `3c7c`, stack var_B0 is built from zero, loaded at
`3d48`, and stored with index `0xc0`. This is actual constructor execution,
not allocation readiness or a fresh-per-frame default. The inspected direct
`ResetSystems` (`82dfff20`) and `TeleportPlayerNow` (`82e00618`) paths contain
no write to this field; do not invent a reset there.

`HiLOD::Update1_ProcessInput` calls actual `Skeleton::ProcessData` at
`82e03718`. The later prepared-history stage is:

- `379c..37ac`: load processed flags and test `0x00200000`.
- True: copy all four current processed velocity lanes (`+190`) into retained
  HiLOD `+c0` at `37b8`.
- False: NormalizeSafe the actual processed surface velocity `+1a0` at `37d8`.
  Use the source fixed timestep at `8236e734` (bits `3c888889`), not caller dt;
  use source 9.8 (bits `411ccccd`). The source multiplies negative normalized
  Y by 9.8 with its scalar round trip, multiplies the full normalized direction
  by that scalar, then full-lane FMA with fixed dt and the old retained history
  at `387c`, storing history at `3880`. Reuse the numerical port, preserving
  NormalizeSafe estimate behavior and unknown W; do not insert guessed W.
- Spin-history reads/writes intervene at `3884..38b4`.
- Only then load full retained history at `38c0` and publish processed
  prepared-jump velocity `+2c0` at `38c4`.

The source input dependencies are therefore current deck velocity `+190` and
current surface velocity `+1a0`; the latter requires the actual Reckoning-driven
surface-motion publisher being audited separately. Constructor-origin values
must remain labelled as such until that producer executes.

## Minimum integration and remaining upstream boundary

Reuse `skate2-formats::andale`'s runtime attribute sampling and ordered phase
blend evaluation (`AndaleRuntimeAttribute`, `runtime_attributes`,
`sample_phase_blend_attributes`). Its typed runtime form retains name/status
and value, but does not directly expose the complete raw native record or
packed name. A source-backed conversion may recover the event type from the
typed value, but must preserve the packed spelling and order; do not pretend
the current decoded string alone establishes actual Actor ownership.

Reuse the existing contact raw handler on the same ordered stream. Add a narrow
type-0 jump/prepare dispatcher against the **same mutable processed flag word**,
with retained Skeleton adjustments and staged actual Actor-input lookup. Follow
it with the separately retained HiLOD history stage at its actual scheduling
point. Keep source stages independently fallible: a flag/input failure retains
the successful prefix, and must not run later history/publication consumers.
Tests should discriminate zero-valued jump records, type/status differences,
two PrepareJump records, null input, second-axis failure, packed-name payload
confusion, NaN bounds, and full-vector history with unknown W.

Existing `physics.rs` scalar intent and handoff helpers are semantic probes,
not the actual Actor/Mg/Andale handoff; they also contain the endpoint NaN
difference above. `play_animation_lifecycle.rs` has attribute collaborator
contracts, but the repository search found no core consumer of
`AndaleRuntimeAttribute` connecting that format sampler to this actual raw
physics owner. Do not create a fake live input by setting startjump=false,
injecting a jump event, or treating a diagnostic Mg list as a player gesture.

The missing upstream closure remains actual motion-graph/animation event
generation driven by retained input history, held/released flick, combinations,
and Actor phase scheduling. The event appenders prove how those real outputs
enter physics; they do not prove their gesture producer. The jump consumer also
still needs the source strength/adjustments, prepared/current velocities,
ground state/COM and existing launch/trajectory ownership described in
`skateboard-ground-jump-runtime-plan.md`. This audit does not duplicate the
already recovered CalcGroundJump math or the parallel Reckoning owner work.

## Reproducible raw byte ranges

End-exclusive SHA-256 over debug image bytes:

| Range | SHA-256 |
| --- | --- |
| `825ff6c0..825ff984` | `a01b9ccbbd88e9e0140f8397580988f1bf8bc34aa2932166f336f2f47caca293` |
| `825fdbf0..825fdc98` | `d2e7e6a428680ce9cfa53cd28d455377065037ea44167ac199df7f26591a8e4a` |
| `825fdc98..825fdd98` | `88cadbf7f48447a2d776235ceaaeaf53dacaa694decbb6698c9353f94e0a671c` |
| `82b59de8..82b5ba00` | `837f9bcd557f8db6eed33ae407cb1af9d6bfbdd855dd899a3e320100b99ae1b9` |
| `82e0379c..82e038c8` | `9da9cf94d56d196d4337d0b55a3e40ac3dae87fc367df27894c870d23b806593` |
| `82e03c50..82e03db4` | `75096dd689c1fca63b6acf2638435542978c9a4d0e5cf1d7cdb65cb24afa6f28` |

## Same-owner HippyJumping producer added

The earlier missing-hippy boundary is closed for this selected event path.
Raw82b5b4c4 packed comparison against exact literal HippyJumping (822f96a4)
sets processed+8d0 bit8000 atb4dc. Reset39478 writes zero from r30 initialized
391ac. No other write in the traversed routine clears this bit; begin-pass
preserves it. The retained jump state now owns Option<bool> for this bit:
unknown allocation, Reset false, matching type0 event true regardless of
payload/status. The consumer reads it only atb8e4 after strength stores and
refuses if unknown. The former external boolean callback was removed.
Independent raw-source review accepted the producer/consumer and
failure-prefix tests. Other bits of+8d0 remain explicitly unrepresented.

The actual paired jump/contact bridge now borrows the same Skeleton adjustment
owner, same processed Arc and retained global Vault providers. Raw records and
AI/human selection remain explicit caller boundaries; omitted attribute semantics
are not claimed complete. See skater-owner-surface-jump-integration.md.
