# Actual solved-board contacts to next-frame selected surface

2026-09-08. Read-only source/current-code audit; no Cargo
or implementation. Builds on [processed surface proof](skater-processed-surface-source.md)
and [actual contact-list evidence](contact-spy-feedback-source.md).

## Main finding

The actual solved-contact publisher already exists. Reuse it. The missing
piece is a retained **board** contact winner/vote and published collision
surface projection, not another ContactSpy exporter, skeleton collision
normal calculation, world line query, or fabricated surface fixture.

Current `physical_player_contact_list.rs` implements original
`Island::CreateContactSpyList827ba710..827bab14` and `GetCollisions827ba6b0`.
The world retains actual Body-vector entries, and each PhysicalContactInfo
holds the exact sealed SimulationContactSpyFrame Arc/index, actual component
handles, part tag, packed surface side, normal/position/relative velocity,
and self/side flags. Its constructor is private. Normal is source spy-right
times +1 for A/-1 for B, with no normalization; surface is spy tag low16 for
A/high16 for B. A and B append in source order with per-Body cap16 before
velocity reads. This publisher already retains same-owner contacts; the board
consumer must apply its own later source admission rule.

`skateboard_ground_frame.rs::try_step_authored_ground` calls existing
contact feedback (static-force reset and freezing) then
`try_create_physical_contact_lists` at its source827bae74 position. Original
CreateContactSpyList has one direct caller, Island::Step_Solver3 at827bae74.
`physical_player_contact_list_corpus.rs` already uses actual original physical
board construction, original terrain, real compiler/Horus/integration/export,
and proves nonempty/current/stale behavior. Its explicit wheel placement is
controlled diagnostic stimulus; no second producer is required for new tests.

## Existing authority and reusable access

| Existing code | Reuse |
|---|---|
| `physical_player_contact_list.rs::try_get_physical_collisions(board)` | Actual retained Board vector, current nonempty frame validation, source map-entry creation |
| `SimulationPhysicalContactLists::try_validate_completed_frame(frame)` | Require completed exact frame even when empty; avoids treating preframe empty getter as completed postphysics |
| `PhysicalContactInfo::{normal,position,relative_velocity,part,surface,this_body,other_body,spy_frame}` | Actual admitted source record data and retained origin |
| `physical_player_island.rs::physical_body_owner_for_contacts(handle)` | Narrow actual Body.mpOwner relation, checked manager/occurrence/membership, returns meaningful PhysicalPlayerAssemblyOwner Arc |
| `physical_player_history.rs` world wrappers | Existing pattern: validate source frame, obtain actual list, then borrow retained list and mutable owner separately under exclusive world access |
| `physical_player_board_forces.rs::try_with_board_force_runtime` | Existing actual board-owned runtime access; surface tokens must share its retained slide/collision provider identity |
| `skateboard_processed_surface.rs` | Already assigned retained constructor/Dispatch selection; consumes published enum and publishes actual selected collection/material |

Do not use `physical_contact_list_prefix` as a current frame; it intentionally
exposes old/partial diagnostics. The successful getter permits an empty vector
before first frame, so a new completed board-postphysics wrapper needs the same
feedback/current-sealed-frame/list-completion gates as the existing skeleton
history wrapper. Actual zero-contact completed frames then legitimately vote
smooth. Raw handle IDs/assets are not Entity identity: compare returned owner
Arcs with Arc::ptr_eq, as the existing publisher does.

## Exact missing source graph

`SkateboardBody::UpdatePostPhysics82b36a58` is called by outer
Skateboard::UpdatePostPhysics at82b37e5c. It reads the body's actual island
GetCollisions at36ad0, clears seven inContact bytes, and visits its actual
ContactInfo vector. This proposed projection is only the selected-surface
dependency, not all body postphysics dynamics/debug writes.

For every source contact, retain its part and packed surface ID. If
other_body is Some, sample the other Body.mpOwner then this Body.mpOwner:
raw loads36c9c/36ca0, equality branch36cac to36eb4. Equal owner skips the
per-part winner and inContact write, including collisions with the player's
own skeleton. Null other body continues. Do not shortcut using only the
stored self_colliding bit: that publisher bit also includes same-Body and
the explicit later owner reads have distinct validation/order provenance.

Among admitted contacts, the first contact for each part is selected. Later
contacts replace it only on **ordered currentNormal.Y > selectedNormal.Y**
(`36db4..36dcc`). Equal Y and NaN keep the earlier winner. The existing
normal already has source endpoint orientation; do not normalize/reflip it.
Source winner stores normal36dd4, position36e00, relativeVelocity36e08,
pattern36e10, physics36e14, audio36e18, then inContact36e68. For a narrowly
scoped surface projection only normal.Y, selected packed type and inContact
are needed, but their state must be explicitly a projection; do not pretend
unimplemented sibling stores are ready. Keep prior type/normal fields on
parts without accepted current contacts, while current inContact is false.

ChoosePhysicsSurfaceType `82b24878..24a00`, called at **82b3709c**,
counts only inContact wheel parts0..3 in ascending order. Twelve
local counters; source indexes the raw physics type directly. A host checked
index failure should be explicit for out-of-range values, preserving previous
per-part work; do not normalize to smooth before counting. It scans types1..9
with strict count > previous best; ties retain the earlier/lower numeric type.
Initial winner1/best0 yields smooth for zero contacts. Types0/10/11 can be
counted but are never winner candidates. Truck/deck do not vote.

After all wheel reads/counts, read live kForcedWorldSurface ref8308d858
target+d0 at249e4. Default0 independently confirmed initializer82f7b7a0;
any nonzero value replaces winner, including values later normalized by
Dispatch. Failure at this late cvar read must retain already selected per-part
state but preserve the previous overall type. Final source write is body+348
at249f0 or249f8. This projection must preserve the raw overall enum.

Next, original Skateboard::FillPhysOut82b2f3e8 is called only by
PhysicalPlayerHiLOD::Update5_ProcessOutput at82e04ef8. The board overall
type is copied to PhysOut_Collision+10 at82b2fc20 after the write-lock check.
The next Update1_ProcessInput calls SurfacePhysics::Update at82e02268 and
reads that **published** enum. Keep separate current-body overall and
published PhysOut values. Do not dispatch directly from freshly solved
contacts within the same input phase. Publication failures must preserve
the previous published value while retaining completed body state.

## Minimal implementation sequence

1. Add one retained board contact-surface projection to the actual admitted
   board occurrence, using existing physical owner lifecycle/removal. Store
   seven optional winner fields plus inContact, raw overall/published values
   with explicit readiness, and a per-current-frame one-shot progress marker.
   Constructor/reset defaults require source proof before claiming ready;
   Option/unready is acceptable until first completed real producer.
2. Add an exclusive world entry after completed actual feedback/list creation.
   Validate frame and actual Board ownership once, then preserve source
   per-contact reads/order and same-Entity exclusion. Use the actual retained
   PhysicalContactInfo list; no caller-built list, numeric owner substitute,
   second spy export, or generic contact-normal producer.
3. Implement the four-wheel vote and late forced-surface read on that same
   state. Failures retain current clear/selection prefix and refuse replay
   until next real frame, following existing world construction/history
   patterns. Keep full board-postphysics omitted siblings explicit.
4. Provide a separately scheduled narrow FillPhysOut surface publication on
   the same physical player owner, then let next-frame processed selection
   consume the published value through the existing retained source collection
   owner. Reuse the same slide/collision attribute provider; different provider
   occurrences intentionally reject foreign selected tokens.
5. Extend the existing actual-board contact corpus: independently compute
   admitted winner/vote from actual sealed records and owner Arcs, compare
   the retained result, publish, then verify next-frame selected Vault layout.
   Include legitimate zero-spy completed frame, known ground contact, same
   Entity exclusion, first/tie behavior, late cvar failure, stale/replayed frame
   refusal, and publication boundary preservation. Reuse the real stimulus
   and actual contact parser; keep special edge branches in focused tests.

## Avoid adjacent duplicate work

`physical_player_history.rs::try_update_skeleton_normal_post_physics` already
runs the separate skeleton history/collision/records consumer. It does not
produce the board's wheel vote, and must not be rerun to obtain board normals.
`physics.rs` has skeleton surface scoring and `grind.rs` has grind-specific
surface resolution; neither is this majority vote or retained publication.
`simulation_assembly_contact_feedback.rs` handles solver feedback/reset/freezing,
not the board postphysics winner. Preserve these existing responsibilities.

Confidence high for implementation gap/reuse audit and exact for previously
reviewed winner/vote/Dispatch raw sites. Existing actual ContactInfo producer
has independent source/corpus evidence; no new source parity or live Actor
scheduling claim follows from this plan. Root remains sole integrator.

## Bounded implementation follow-up

Root assigned new `skateboard_contact_surface.rs/tests`. The implemented
projection retains only per-part inContact, optional winner normal.Y and
physics type, plus optional raw overall result; unimplemented sibling
collision fields are not marked ready. `CompletedSkateboardContacts::try_new`
checks the actual sealed frame against manager/pool, completed exact-frame
list, and each retained record's Body/frame; token lifetime ties those source
borrows through use. Root must separately validate actual Board owner and
schedule the world wrapper. Same-frame reentry refuses before clear/writes.

Four focused tests exercise shared production kernels for ordered owner
reads/null admission, first/tie/unordered selection, current contact-flag
reset, wheel-only vote/candidate range and late cvar failure prefix. They do
not forge PhysicalContactInfo or claim a public completed-frame test.
Root's actual original corpus must exercise that integrated public entry and
next-frame publication. Standalone rustfmt passed; no author Cargo run.

## Implemented actual contact route (validation in progress)

The retained outer-board runtime owns its bounded surface state through one
Arc/Mutex. This is host borrow synchronization under exclusive world access,
not a separate native buffer or fabricated entity. The session wrapper validates
actual board authority, validates the current completed list even for zero
spies, invokes the original-equivalent GetCollisions getter (including lazy
entry creation), then locks the same state and consumes a validated token.
Owner reads remain staged other then this through the actual physical registry
and assembly pool; no eager substitute owner snapshot is used.

Independent leaf/source and corrected wrapper review pass. Root
original-data run93529/20edac passes all eleven checks including actual RobPark
solved winners/vote, wrong-body rejection, no-frame refusal, no rigid-body
mutation and same-frame refusal. A further original completed zero-spy export
check has been added before the first map entry exists; its execution is pending.
This specifically checks the GetCollisions side effect identified in review.

Surface state is not automatically relabeled PhysOut. Raw FillPhysOut stores
into the caller's output-buffer collision+10 only after checking PhysOut+4c;
actual buffer occurrence, writable gate and next-frame selection remain required.
The motion/output ownership source audit is active. No complete Actor tick or
full PostPhysics position/velocity/grind collision projection is claimed.

Validation update: accepted18370/1eaaa4, workspace3241 units/sixdocs,
original73431/58c467 eleven PASS, strict Clippy/fmt/diff PASS. Independent
A review passes the integrated bounded owners/corpus. The empty
contact test uses a separate fresh construction, preserving the preexisting
positive-contact fixture. Actual Actor/PhysOut scheduling remains open.
