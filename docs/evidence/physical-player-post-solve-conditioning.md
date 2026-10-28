# Selected post-solve production orchestration

2026-09-20. Confidence high for extraction of the existing selected call chain;
this is not an additional whole-frame parity claim. New unregistered file
`physical_player_post_solve_conditioning.rs`, intended as a child of
`simulation_assembly_backend::constraint_session`. Root owns registration,
independent review and Cargo/original-asset gates. Exact paths were checked absent.

## Reuse and source boundary

The implementation extracts production calls from
`physical_player_actor_solved_fill_corpus.rs` and its
`physical_player_skeleton_full_fill_solved_corpus.rs` child. No numeric algorithm,
output owner, token constructor, controller state, attribute provider or physics
history is recreated. It begins after a successful actual authored-ground solve
and completed contact feedback on the same pair. The caller must have executed
the original input/Update1/force/line-start prerequisites for this tick.

Source schedule and ownership proofs are retained in
`actor-solved-skeleton-fill-insertion.md`,
`skeleton-postphysics-live-caller.md`, `post-wipeout-pose-live.md`,
`update5-entry-line-output.md`, the existing `update5-board-fill-*` and
`update5-skeleton-fill-prefix.md` packets, `update5-three-manager-fill-live.md`,
`selected-ground-output-conditioning.md`, `conditioner-animation-four-updates.md`
and `conditioner-animation-turn.md`. Relevant caller identities include
HiLOD.Update4 82e01d00, IsDeckInHand82dc8358, Ground.Post82d8e998,
Update5 outer stores82e04f88..82e053ec and Animation.Update82e31970.

Exact extracted order:

1. EndSkeletonLineTests; complete ordinary board post-physics (including its
   existing EndBoardWheelLines); validate native board return0.
2. Read actual controller state+1bc. Only state1 calls GetSkateboardBody and
   reads Body+360 `mCollisionInfo.numPartsInContact`; pass its nonzero result
   to Skeleton post-physics. Other states skip the body read and pass false.
   The retained board post-physics contact count supplies this byte.
3. Skeleton post-physics; Ground post-physics; complete post-wipeout caller
   including its already-integrated mandatory IK, head and pose publication.
4. Update5 entry; Board Fill prefix, pumping, contacts, deck, surface, wobble,
   forward, wheel positions, ground, final; Skeleton prefix, parts, suffix.
5. Pumping, managers, plants, controller, wipeout and outer-locality publication.
6. Borrow actual retained board-force runtime/push attributes; selected Ground
   output and Physics prefix; finish Physics landing/stairs; five Animation
   conditioner callbacks. Return their existing selected-completion token.

No additional pose publication, input processing or Motion update is appended.
The corpus's appended Motion callback is next-cycle evidence, not part of this
post-solve function. The caller owns the following Actor cycle and must avoid
duplicating the ProcessBoundInput already performed by normal Update1.

## Capabilities and failure semantics

PostSolveRequest borrows the same board/skeleton handles, physical owner Arc,
resident StaticGroundPool, retained ground reckoning phase and mutable Actor.
GrindManagerFillPolicy is explicit and survives in the returned existing token;
parked grind exclusion is not synthesized as a query miss. Actual source stage
methods retain their same-owner/Actor checks. Initial physical-owner checks are
host identity validation with no source branch sampling or publications.

PostSolveNumerics borrows all five existing numeric capabilities: collision line,
reckoning vector, sin/cos, truck drive and skeleton collision. This extraction
does not instantiate a host numeric policy. Existing deeper methods retain their
current selected host helpers (notably selected-ground Animation field
publication); this does not upgrade those methods to portable exact numerics.
World debug overlay and owner cvars are sampled from their retained owners.

Every fallible stage has a distinct typed error variant, boxing existing errors
without erasing their type. Earlier writes and token consumption persist on
failure. The entire operation is not safely restartable as a rollback; callers
must treat failure as a partial execution and preserve its reported stage.
Unexpected board return1/2 is explicitly refused before Skeleton continuation;
current board reset branches already return their own pending-service errors.

No gravity setting, fixed solver budget, ControlledNoFreezing, fixture-only
contact-count or output assertions, synthetic state, owner replacement or
new debug reports are introduced. The caller must retain line/query/grind
residency between ticks. Remaining UpdateAll callbacks and scheduler policies
are outside this selected token and remain outside a full native-frame claim.

## Validation

Standalone rustfmt parsed the new Rust file. No Cargo was run by the author.
The root should register and replace the corpus production chain with this
function while retaining useful observations at its boundaries, then run the
existing original-asset same-owner corpus and normal repository gates. This
wrapper adds no math or independent state requiring duplicated synthetic tests.
