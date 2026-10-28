# Same-instance solved-frame Skeleton.Fill corpus extension

2026-09-12. NEW-only proposal and unregistered helper; no Cargo or execution.
The existing controlled collision fixture and its assertions remain unchanged.

`physical_player_skeleton_full_fill_solved_corpus.rs` is proposed as a child of
`physical_player_collision_corpus.rs`. Its `check(world,board,skeleton,owner,frame)`
belongs after dedicated `postphysics_live::check_live_postphysics` and before
teardown in check_mode(live_postphysics=true). It must not be enabled as a passing
test until the real prerequisites below are connected. It does not absorb an
expected missing producer into a successful result; each production stage fails
with its name. The helper only receives the actual existing owners/frame.

## What the fixture already has

The factory's actual Board construction creates complete PhysOut on the same
owner. The later `phys_out_corpus::construct` function merely asserts this;
calling it is not necessary to manufacture a constructor. HiLOD construction
also initializes line records with invalid result flags and source Sim allocation
zero starts. Update5 can consume those actual initial values without a fake
miss callback. This does not claim a line test was scheduled on this occurrence.

The original terrain fixture loads the same original Vault, body parameters,
hierarchy and RAW pose, with an explicitly controlled uniform pose translation.
It creates a separate owner from the original Actor's physical owner. It solves
actual original terrain contacts and retains the actual solved-frame identity.
PostRecordsCorpus.prepare uses controlled current-animation and grounded
Reckoning inputs on that new owner; this is already an explicit fixture limit.
Live postphysics produces history, collision/specific-contact bytes and retained
post-records on the same owner. Audio can consume those real fields once reached.

## Definite missing prerequisite

The new terrain owner has no actual Actor animation input publication:
SkeletonConstructorBindings.animation_input starts None at constructor234.
The only production assignment is physical_player_skeleton_process_data.rs129,
the actual ProcessData ece8 source store. Neither terrain_inputs, the assembly
factory, nor PostRecordsCorpus/RootCorpus's direct current-animation helper runs
that ProcessData caller or owns an Actor AnimOut allocation for this pair.
Therefore Skeleton.Fill prefix's board_flipped read will encounter MissingInput
even if every preceding Board.Fill stage succeeds.

Do not attach original Actor's AnimOut to this different newly constructed
terrain owner; do not create an empty AnimOut, copy the original flags, manually
write bindings.animation_input, or invent Update5 tuple tokens. Need an actual
Actor/SkaterAnim owner bound to this pair, its real phase2/phase3/Dispatch/Sync
and GetPhysUpdateData, and the same input through actual ProcessData before the
solve. Existing real-Actor startup corpus already has this producer sequence
on its own world/pair. A valid alternative is a dedicated solved-frame invocation
on that same Actor world after its real ProcessData, using actual resident ground
and input/placement, rather than borrowing its input into this standalone pair.
Retain the original translated-contact fixture separately in either approach.

The first failure in the full attempted pipeline is not claimed without running
it. Board.Fill also reads fields normally updated by Board.PostPhysics/Update1.
The current fixture runs surface-selection voting on actual solved contacts;
that does not establish the entire Board contact/normal/velocity producer chain.
Constructor Reset establishes many contact fields legitimately, but a successful
read of those initial fields is not evidence they represent this solved frame.
The dedicated full-frame extension needs actual Board.PostPhysics predecessors
before Skeleton.UpdatePostPhysics, including the source wheel/contact updates.
Do not run a later source phase early just to fill an Option. Root owns that
scheduling integration; this helper does not substitute synthetic values.

## Concrete producer-to-consumer calls

The new helper obtains every token from existing production functions:

1. try_begin_player_update5 resets the same output and fills actual hips-line data.
2. Board Fill prefix, Pumping, contacts, deck, surface, wobble, forward,
   wheel positions, ground, final; all use the same pair and returned tokens.
3. Skeleton prefix captures the actual585b0 COM velocity. It is compared with
   the independently read pre-Fill post-record value, including W knowledge.
4. Skeleton parts forwards that capture; suffix receives that same value and
   executes mandatory IK/audio through the live adapter.
5. Successful return is BeforePumping, with unchanged owner/output/Processed and
   history/collision-frame identities. Output COM velocity matches the capture.

No direct output unlock/locking, token literals, replacement pose/owner or
canonical-W conversion appears in the helper. It never manufactures valid
collision/audio data. Full Skeleton.Fill success would validate the consumer
connection; it does not imply remaining Update5 manager callbacks or final
conditioning/locking occurred. Every stage's numeric policy remains its existing
explicit host contract, not target hardware bitwise parity.

Original test mounting should preserve both old modes and cleanup assertions.
Add the helper only to an independently admitted full-input solved occurrence,
or use it temporarily to report a concrete failing stage with that failure
remaining a failure. Do not relabel a MissingInput/error result as corpus pass.
