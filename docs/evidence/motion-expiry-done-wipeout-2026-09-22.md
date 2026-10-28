# Motion expiry and done-wipeout continuation

Original queued input now reaches frame3 Motion Pre expression5; bail reaches
frame3 expression270. Fresh switch bytes at8290aaa4/8290aec8 point respectively
to8290b358/82911ac0. Raw arms call condition19 (offset4c, WillExpire) and404
(offset650, IsDoneWipingOut), retaining the existing generated cache machinery.
Sustain8291e218 lists both in its actual default true return829221c8; neither
samples its Pre condition during Sustain. Existing expression tests cover both.

IsDoneWipingOut82c9ca40 validates cached/generic IPhysical then reads GetPhysOut
Skeleton pointer+14 and byte243. The existing output field is_wipeout_over is
published by the retained Wipeout Fill producer. The adapter reads that SAME
physical occurrence without substituting current state, prediction or intent.

WillExpire82c9bc78 is already reconstructed in motion_graph_conditions.rs.
The new generated adapter reuses its activation algorithm through an extracted
activate_values entry accepting the already-constructed payload. Original
condition19 has one attribute, InTime=0.1; waitForTransitions defaults true,
tag is empty. No runtime tuning constant is introduced. Nonempty tags explicitly
stop at the actual IAnimInfo lookup boundary, rather than inventing absence.

Fresh Transitioning82d720f0 checks the actual retained transition list count;
empty returns false before GetBaseAnim82592b48. Otherwise the actual base root
must occur in the time-ordered transition list or native asserts, then returns
true. The adapter validates membership using the same retained transition index.
GetAnimProp82592bf8 returns the actual four retained AddTime properties. It is
read only after unconditional Transitioning and the wait gate. Existing leaf
tests cover ordering, expiry and unordered threshold behavior. Raw bd9c..bdac
confirms the existing NOT-GT semantics (including unordered), not Rust <=.

Same PlayAnimationActor owner/interface lookup is reused; no second animation
or property cache is constructed. Confidence high for this selected slice.
Independent source/code review approves the admitted empty-tag path, including
fresh switch slots, Sustain true entries, exact NOT-GT raw40990008 and actual
transition/physical ownership. The ordered membership loop uses existing fixed
storage accessors and stops on a missing occupied entry or first matching root.
Expiry-hooks-3 gate passes fmt, strict Clippy (38.88s), 4675 unit tests and six
docs. Original19/21 (6.35s): the bail test now PASSES its three Wipeout ticks,
including real predictor harvest/consumption and post-physics presentation.
Queued input reaches later frame3 expression345. The other original failure is
the collision fixture's new hook-relocation assertion, not the expiry adapter.
Nonempty tag resolution and later animation/state branches remain pending.

Follow-up345: fresh Pre table8290aff4 points to829158ec. It samples cached
condition19 first, branches false at5948, otherwise samples14 at5968. Registered
expression is c19 && c14, reusing both existing actual conditions. Sustain
table8291e7b8 points to829221c8 (true, no reads). Independent source/code review
approves; existing lazy/failure test and Sustain no-read test extended. Its
execution gate motion345-mapped-hooks passes fmt, strict Clippy (26.13s),
4675 units and six docs. Original19/21 (6.44s): queued frames0..9 complete,
then Action expression7 on frame10. Collision positive-contact and subsequent
drive/trajectory checks now pass; the next fixture reset expectation is stale.
