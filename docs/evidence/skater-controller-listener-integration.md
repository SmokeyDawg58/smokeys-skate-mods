# Controller and queued listener integration, 2026-09-08

Accepted checkpoint 28736/2aea31: strict all-target Clippy PASS; workspace
56018/e600cc 3459 units (3216 core, 202 formats, 38 app, 3 symbols), six docs;
original corpus 0f818d twelve PASS; format/diff 6e7ae0 PASS. Independent
source/code review explicitly approves all components below.

The physical pair retains ONE SkateboardController. Its selected constructor
82dd02a0, AddDrives and InitHandDrives use runtime physics_skatecontroller.default,
two separate live UseHandDrives captures, and the actual drive pool. The original
collision fixture leaves two free slots after its 49 other drives. Actual
Skeleton Parts3/7 and Deck6 are checked against the same retained frames/dynamics
and admitted handles. Controller starts off, Controlled, hand2; native Stop for
state100 is therefore valid. False late removal cvar preserves both handles and
blocks physical body cleanup. True removal releases both in source order.
This is an explicit late constructor test, not native HiLOD startup chronology
or active retrieval/LetGo/Start. See physical-player-state-owner-runtime-plan.md.

GameInputEventRoute owns the recognizer's SAME queue6 broadcaster with the actual
GameInputBroadcaster hash. Three ordered listener registrations preserve their
cleanup capability even on late allocation failure. Failed delivery retains the
message and completed handler prefix; explicit expired-handler cleanup resumes
without repeating earlier delivery. These checked-host policies do not claim
native EventSys allocation or reentrant callback scheduling.

The listener retains the SAME constructed gameplay player and borrows an existing
logical Actor authority; unconstructed gameplay storage is rejected. Pattern,
held-candidate and ordered intent-suffix mutations follow the recovered source.
Original Ollie recognition remains tested by the preceding mounted corpus;
native Actor ownership and full Fill/generated graph scheduling are still open.
See game-input-actor-listener-runtime-plan.md and actor-owner-input-runtime-plan.md.

Next unaccepted batch: actual board SetStandard and live collision-volume flags,
then remaining PhysicsGround.Enter effects; native Actor constructor prefix is
being recovered independently. No input-to-jump shortcut or generic physics is
substituted. Goal remains active and broader DMO stays parked.
