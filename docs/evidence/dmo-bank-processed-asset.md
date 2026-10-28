# Dedicated DMO-bank processed arenas

## Scope

`dmo_bank_processed_asset.rs` is the narrow builder-session boundary for the
main DMO model bank. It admits only the supplied reference corpus fingerprint
under `data\content\world\dmo\DMO\DMO_Pres`, retains the catalog's owning XST
snapshot and physical order, and builds every XSF block through the existing
decoded-arena, Simulation-collision, and normal-arena-plan owners.
Construction has no manager-registration effects.

This does not relax the ordinary Simulation-only collision session. It also
does not reproduce the hull loader's complete schedule, publish the completed
collection globally, or dispatch the full normal/Location/DMO-pass messages.

## Native anchors

| Address | Behavior |
|---:|---|
| `0x82e0e628` | `cDMOBankManager::LoadMainDMOBank` requests `DMO` with register-on-load flag 8 |
| `0x82eeedc8` | `cHullLoader::LoadHull`; adds userdata bit 1 at `0x82eeee68` |
| `0x82eeef40` | `cPathedHullLoader::LoadHull` path-owning wrapper |
| `0x82eed1b0` | starts the physical stream-collection load |
| `0x82eed410` | collection-load completion boundary |
| `0x82eecd50` | initiates collection registration |
| `0x82d5c8c0` | `NotifyCollectionReg`, Simulation notification before Presentation |
| `0x82d5c468` | `NotifyCollectionSimReg` forwards the retained collection |
| `0x8277e988` | Simulation collection receiver: complete normal pass, then separate DMO pass |
| `0x82d5fee0` | `cProcessArenaAsset::ProcessArena` builds the authored-order processed list |
| `0x828a01e0` | `Simulation::RegArena` forward normal-arena walk |

The load chain forwards raw userdata `0x0a`: the caller supplies bit 3 and the
hull loader adds bit 1. That exact callback path justifies a dedicated
Presentation-owned route into normal Simulation processing. It does not imply
that arbitrary Presentation streams belong on the route.

## Admission and ownership

Session creation first proves that the caller's complete collection snapshot
equals the current catalog snapshot. It then requires a registered
Presentation owner with the expected path and active parsed XSM/XST state. The
parsed manifest must match all of these shipped-corpus facts:

- XSM version 3, stamp `0000000094d14dc9`, stream format 2, dead name-table
  sentinel, and exactly one collection;
- collection `e87878efd7f50339`, parent `e171e1abfedb69ed`, source index zero,
  `cPres_Global.xsf`, payload `0x80 + 0x2f3180`, and 125 assets;
- XST version 3 with the same stamp, zero name-table offset, 125 unique
  `ab329a6a` assets, first ID `91d724075ae44972`, last ID
  `d3096f13cf1c0aab`, physical-ID signature `9a861845e82ee292`, and full ordered
  descriptor signature `20d518b82728d8d5`;
- serialized-size total 7,117,012, maximum 1,382,292, and resource totals
  `[945452, 0, 6171560, 0, 0]`.

The descriptor signature is host FNV-1a over each parsed XST entry in order:
big-endian full ID, type, serialized size, all five `(size, alignment)` pairs,
name offset, and unused word. Header fields and collection fields are checked
separately. An external corpus oracle recorded in the research log retains the
raw mounted XSM/XST/XSF SHA-256 values; the application validates parsed
structure and fingerprints rather than claiming to calculate those hashes.

Every local XST asset must also have an active positive-reference global
catalog entry with identical metadata. This intentionally preserves the
catalog's recovered first-owner behavior: an identical entry registered first
by another owner remains usable, while a same-ID metadata collision is
rejected. The builder retains the DMO file snapshot's existing `Arc<AssetToc>`
rather than copying or later consulting mutable global order.

## Build behavior and confidence

Every `build` call checks the next full asset ID and complete retained XST
record before decoding. It then runs `DecodedArenaAssetBuilder`,
`SimulationCollisionAsset::from_decoded`, and
`SimulationNormalArenaPlan::from_collision_asset`. The complete Arc-backed
result is constructed before the cursor advances. Errors leave the same
physical asset pending, and an extra asset after index 124 is rejected.

Physical XSF order, full IDs, metadata, decoded resource sizes, RW4 structure,
and the resulting 125-plan/93-kind-11 shipped corpus are **exact**. Reusing the
already validated readable constructors and treating every valid shipped asset
as non-null is **high-confidence host composition**. Native allocator failure,
malformed-input consequences, and first-null termination remain unknown.

## Validation and remaining boundary

Focused tests prove snapshot comparison precedes owner policy, arbitrary
Presentation rejection, metadata checks before decode, no cursor movement on a
build failure, one-step movement after a complete owned result, result identity,
and overrun rejection.

The release `--inventory-bam-sim` gate now loads the exact XSM/XST through
`ManifestOnlyStream`, observes one collection and 125 catalog assets, and runs
the catalog-backed mounted resident driver with this builder over the real XSF.
It yields 125 assets in physical XST order, 93 kind-11 operations and no other
represented normal operations, the frozen stored-byte total/max, and zero
queued or retained chunks. A second real load now runs through
`CollectionResidentStreamer`: all 125 assets exist before wrapper creation,
the wrapper publishes before Simulation-then-Presentation request dispatch,
and the coordinator stalls at raw state 8 after two exact Up mutations carried
by occurrence-bound host-safety holds. The same
collection is rejected by the ordinary Simulation session, each request-ID
lease ends with its driver, and explicit manifest unload returns the catalog to
zero. The older strict direct app gate independently retains parsed corpus
signatures and prepares the 125 plans into one aggregate pointer bank before
the 93 runtime register/deregister pairs; the external oracle retains the raw
file hashes.

The resident probe now prepares all 125 arenas for the bank-scoped mixed
Presentation adapter. The public collection constructor refuses any census
other than the exact main-bank identity: 125 ordered outer XST IDs with FNV
signature `9a861845e82ee292`, 124 leading zero-action arenas, then 94 actions in
the final arena `d3096f13cf1c0aab`. Its independently versioned typed action
signature is `a73f2da81dae97b8`.

Action zero is raw material `0x00eb0005` at TOC index 68, GUID
`8f51812cef1b09a4`, ObjectId 1, dictionary 1/resource 0 span `544..30451`.
`ProcessMaterial` (`0x82d5ebd8`) emits exactly one Presentation kind-2 asset
whose data is the complete `tRMaterialData*`. The raw `0x00eb000d` container at
TOC index 162 then expands the 93 kind-11 records. The outer XST IDs are the
refcounter keys; all 125 differ from their inner RW4 IDs.

Raw `0x0a` produces 125 first-use counts, one material-add, and 93 forward
LivingWorld-add/DMO-register pairs. The same non-cloneable occurrence is
retained through hull unload, where one material-remove and 93 forward
LivingWorld-remove/DMO-deregister pairs complete before the Presentation hold
acknowledgement; all counts and owners then return to zero. Versioned host
FNV-1a signatures are `93ec8ff6b3ec727b` for 187 registration events and
`b1428a651a6d70a1` for the complete 374-event lifecycle. The earlier DMO-only
substream signatures `ebfdce16a6a52bef`/`03d8b9ef5e479295` remain checked.
These hashes are host regression policy, not native algorithms.

This closes the processed kind-2/kind-11 action census for this one bank, but
still cannot authorize a production Presentation acknowledgement. The gate
dispatcher records the exact call boundary and does not implement
`TextureMan`'s texture lookup or in-place channel stream mutation, LivingWorld,
or DMO managers. Simulation, Location, the separate kind-31 DMO pass,
first-null behavior, native registry wrapper processing/unprocessing, and
message timing remain required.

## Opaque Simulation/model preparation binding

Core commit `fd1aa3a` adds the effect-free preparation owner needed before the
processed main bank can enter the reconstructed outer Simulation receiver. It
takes one snapshot through `SimulationDmoBankModelRegistrationBatch::try_new`,
retains every exact processed-asset `Arc`, prevalidates all model sources, and
then derives the complete `PreparedSimulationCollection` plan sequence only
from those already-retained assets. There is no second resident traversal and
the two prepared halves cannot be extracted or paired with another binding.

The production binding accepts only the pinned census:

- 125 retained arena assets;
- exactly 93 kind-11 model rows;
- unique authored-order outer asset IDs;
- first ID `0x91d724075ae44972` and last ID
  `0xd3096f13cf1c0aab`;
- big-endian FNV ID signature `0x9a861845e82ee292`.

The derived collection uses `0xe87878efd7f50339`, the sole collection ID in
the supplied `DMO_Pres.xsm`. `OwnedResidentCollection` itself stores no
semantic collection ID, so this assignment is corpus/upstream-gate policy and
not independent owner-ID evidence. Likewise, 125 and 93 are pinned corpus
counts rather than native table capacities or hardcoded XEX constants.

The one-snapshot `Arc` identity is safe-host provenance policy. The native
receiver's reuse of one resolved collection vector does not establish Rust
ownership, atomic snapshotting, host/guest pointer equality, or persistent
residency. A frozen binding keeps its bytes available after tombstoning, but a
later model-source issue still performs the existing fresh residency check and
refuses with `ResidentExpired`.

Six tests bring the core count to 1,624 and cover exact positional owner
pairing; asset-count, kind-11-count, duplicate-ID, ordered-signature, and
combined-poison precedence; no-second-snapshot derivation; exact cursor/source
pairing; resident expiry; and batch-first source-error precedence. The full
workspace gate, formatting, diff checks, and warning-denying Clippy pass.

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_collection_binding.rs` | `6B3CB5B9A61600B5DAA58614528B175A639BD82E4389A1EC668337977880C94C` |
| `simulation_dmo_bank_model_registration_batch.rs` | `7807F2259A62BEAF0D40FBB12555759991BA24ED2097BE855039FAB37500630D` |
| `simulation_dmo_assembly_part_session.rs` | `626636FC6B2DFA90DAD4A9FFC21829712BCC2F8195BAA80CBE245968242EE1B0` |

This preparation does not invoke `RegArena`, Location, or `RegDMOArena`, create
or finish a runtime controller, select active/deferred delivery, prove a live
native registry occurrence, acknowledge the hull, execute `SpawnStatic`, or
construct a DMO Body.

## Sealed outer collection coordinator

Core commit `b180f44` consumes the opaque binding without exposing either
prepared half. `MainDmoBankCollectionCoordinator` owns that inseparable
binding, the outer collection adapter, and the exact model-registration
runtime. Callers cannot supply another batch, raw collection ID, userdata, or
controller.

The active and deferred registration routes each create one model-register
controller for the full 125-arena normal/Location prefix. The controller sees
the 93 authored kind-11 rows at their exact positions and must finish before
the restarted DMO pass. Deregistration first completes all 125 DMO-arena calls,
then creates one model-deregister controller for the normal/Location pass and
must finish it before the outer occurrence is removed. The coordinator uses
main-bank userdata `0x0a`, whose provenance is the established load-chain flags
rather than a receiver constant. Duplicate model keys remain separate authored
source occurrences and retain the manager's already proven first-winner/no-
refcount behavior.

The binding fields and coordinator remain private under the DMO assembly-part
session module. Domain-qualified non-cloneable continuations, fresh live-row
resolution, controller/completeness checks, typed errors, and sticky no-
rollback failure retention are safe-host policy. They protect the human-
readable implementation but are not claims about native controller objects,
exceptions, or allocation. The native receiver evidence establishes the call
order and equal userdata values; it does not establish Rust's ownership or
fault model.

Twelve binding/coordinator tests cover the pinned 125/93 corpus, exact active,
deferred, and DReg streams, resident expiry at both model directions, early
deferred completion, foreign capability recovery, repeated outer occurrences,
duplicate keys backed by distinct records, prefix retention, and terminal
fault replay. Together with 17 collection-adapter tests, the strict workspace
gate passes 1,631 core tests plus all other workspace and documentation gates.

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_collection_binding.rs` | `17C6206042FAB05B73476F925D7F6B1A28BFA59F118CCF14D5542E62FDB3C4FA` |
| `simulation_dmo_bank_collection_coordinator.rs` | `89A7C227C2BDFD693CB5A40FF7F01E348B3D323424308630B52905C3DA26E5F4` |
| `simulation_collection_adapter.rs` | `63DA541B96525E30822A569A5E5D699167A9DC5BEBCAFBC95EF0F653327D5A22` |

Live notification/admission selection, FIFO delivery and callback timing,
hull acknowledgement, receiver-time native registry lookup, locator-table
registration, `SpawnStatic`, and DMO Body construction remain outside this
checkpoint.

## Caller-asserted notification session

Core commit `7194982` retains the sealed coordinator inside one private
main-bank notification session. Separate entry points represent a
caller-selected immediate route or a caller-selected deferred route; neither
chooses the live defer controls. Immediate Collection completion moves to a
distinct registration-callback boundary. Deferred setup has no arena effect,
then exactly 125 caller-asserted Arena fronts precede the activation-zero
Collection front and the later callback boundary. Teardown similarly stages
the opaque callback, runs the coordinator's DMO-first `DRegCollection`, and
only then permits that callback to be returned.

The session owns no native or reconstructed FIFO. It does not dequeue or invoke
callbacks, and callback-capability return is not hull acknowledgement. It also
does not expose the prepared binding, model batch, controllers, collection
registration, or deferred continuations. On a pre-effect deferred-begin
refusal, an opaque owning error retains both callback and coordinator for an
exact same-route retry. Reached faults preserve their prefix and withhold the
callback. These stages, failures, retry, and ownership constraints are
safe-host policy around native `void` boundaries.

Five tests bring the complete core count to 1,636. Their immediate, deferred,
and DReg traces retain the pinned 125-arena/93-model ordering; failure tests
freeze the first Add and complete-DMO-plus-first-Remove prefixes without
rollback. Dropping the session performs no implicit dispatch, callback
invocation, hull acknowledgement, rollback, or native teardown; ordinary Rust
destruction of its owned callback and coordinator fields still occurs. Strict
full-workspace validation passes.

| Source | SHA-256 |
|---|---|
| `simulation_dmo_bank_notification_receiver.rs` | `28A1C1769FBADD3434D60AB692130AC12909C25F186689243BEB3585F22B2D69` |
| `simulation_dmo_bank_collection_binding.rs` | `3FEC94646511B2CAFE587149FA7DE17B896355331AB0A5432B5D5D4F410247E5` |

The exact binding still is not admitted by the production notification router,
and no FIFO, callback invocation, hull latch, locator manager, spawn, or Body
effect is added here.
