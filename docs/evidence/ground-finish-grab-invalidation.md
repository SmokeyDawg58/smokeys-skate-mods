# Ground finish and environmental grab invalidation

New-only draft, not compiled/tested. Actual Ground.Update82da1028..109c follows
force-to-hook prediction. Ground.request_push_stop bytef2 gates RequestPushExit
82b3bb70 at1038: load actual IK at Skeleton3dfc, set only IK+c71=true. Do not use
the broader abort helper which also changes skateboard_dont_push. Then store
Skeleton+3dd4=true1040; load Ground.time_in_state1044 before DT93c104c; add/store
1050/1054. Only afterward test first BE byte8cc bit0 (fullword01000000).

True calls CheckForGrabbableObject82d9faa8 at106c, explicit pending after preceding
stores. This is environmental vehicle search (GSQ_VEHICLE), not airborne board
grabs. It requires live skitching box attributes and actual query producer. False
calls PlayerGrabSpline.InvalidateQuery82dcc290 at1088 then Ground.Update returns.

## Real manager removal and invalidation

Source c2a8 selects same future+220, c2ac manager+2c90; virtual callc2bc invokes
RemoveDeferredGrabSplineQuery827b0648. Under its critical section, native scans
retained query vector+30/+34. Internal record stride80, mOutput+78. It removes
EVERY matching output identity by shifting successors left; after removal it
does not advance past the shifted element. Existing typed manager Vec and Arc
output identities support stable retain, including nonempty/multiple matches.
No new manager/future or query-miss result is fabricated.

Only after removal, load actual lineBatch+2c94 atc2c0. If non-null: Syncc2dc then
Releasec2f0 before clearing pointerc2f4. Existing owner represents actual
constructor-null; non-null services remain pending at Sync after manager writes.
Afterward capture old flags2c9c atc2f8; clear query.ready2230 atc2fc, primary.ready100
atc304, alternate.ready210 atc308; store flags old&3f atc30c. Preserve all payloads,
counts/cursors, interactable readiness and lower6 flag bits. This is not Reset.

## Draft registration and narrow shared setter plan

- physical_player_grab_remove_deferred.rs child of grab_query_manager.
- physical_player_grab_invalidate.rs child invalidate of grab_spline.
- constraint_session children ground_finish and ground_finish_session.
- physical_player_ground_finish_tests.rs cfg(test) child of physics_state.

Required existing-owner setter plumbing, no new fields:
1. SkeletonIkState::request_push_exit sets only tell_anim_dont_push_or_brake=true.
2. SkeletonIk::request_push_exit delegates to that actual state.
3. SkeletonNormalAssembly::try_request_push_exit obtains existing
   current_animation.ik_runtime_mut(), errors if absent, invokes the setter.
4. SkeletonGeneralUpdate::mark_add_skateboard_error sets existing private bool;
   SkeletonNormalAssembly::mark_add_skateboard_error delegates.

Session try_finish_pair_ground_update(board,skeleton,GroundBeforePushExit)
returns GroundUpdateComplete after selected suffix only. SAME actual current
Ground/pair is verified. Reckoning/Processed locks are acquired only at late
DT/flags reads, AFTER conditional IK and mandatory Skeleton writes. PlayerGrabSpline
is acquired only on false branch, validating same manager/world and completed
constructor before source call. No outer controller/prephysics/solver completion
is implied. Four draft tests unrun: duplicate identity stable removal, actual
constructed-manager ready/payload preservation, highbyte branch/partial stores,
and DT failure preserving earlier calls and old timer. Non-null batch and active
environmental search remain explicit dependencies.
