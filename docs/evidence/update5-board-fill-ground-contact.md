# BoardFill ground/contact suffix 30288..30658

Unregistered adapter/output/tests use only existing retained owners. Body post
fields contain timeSinceWheelsTouching+1e00 (constructor386d8) and numWheels+361
(CollisionInfo.Reset236bc). Outer up_state contains dynamicUp+70 and contact
acceleration+90 (constructor38aa0/38ac0). Collision state contains last-valid
normal Body+50, average normal+40, seven contact bytes, normals and accelerations.
Their constructor/Reset availability does not imply later UpdatePostPhysics ran.

The source schedule preserved in the draft is:

* Fresh body call30294, latch30298, time read3029c, assertion then Air+b0 store.
* Assertion302c4 precedes outer+70 full-vector read302ec and Ground+40 store.
* Fresh body calls30304 and30344 each independently read Body+50 after their
  latch and before assertion, writing Ground+50 and+60 respectively.
* Assertion30374 precedes outer+90 read30398. A new latch303a0 precedes the
  read of **already published Ground+50** and dot3; Ground+100 store303d8
  follows assertion. The second last-valid-normal read cannot replace the first.
* Fresh body303e8, latch then Body+40, assertion, Collision+20 average normal.
  Fresh body30428, latch then unsigned byte+361, assertion, Collision+0 count.
* Body30468 contact5 is tested first; only false makes another body call30488
  and reads contact4. Assertion follows this short circuit; result goes+d70.
* Fresh body304d8, latch then contact6, assertion, Collision+d73 store.
  Existing output count>0, then truck, then deck short-circuit, with separate
  assertions only for reached output reads. Final assertion precedes+d74 store.
* Fresh body305d8 and fresh contact6 decides force computation, independent of
  the previously published deck flag. True reads normal6 at30600 and acceleration6
  at30604, then dot3, sign-bit clear, multiply3aa3d70a, and two source FSELs.
  False retains earlier f31 positive zero (loaded82b2fe78). Assertion30630 follows
  calculation; Collision+18 store30658 is the endpoint.

Raw30608=15ae79b5 decodes VA46 acceleration and VB47 normal;30610=158d8275
clears sign via the mask. Native multiply is separate; lower selection is
`fsel(-scaled,+0,scaled)`, upper is `fsel(1-positive,positive,1)`. NaN therefore
finishes at1, both signed zeros at+0. The draft uses explicit ordered predicates,
not generic clamp. Only dot3 XYZ is consumed; retained vector W stays unknown.

Tests distinguish both normal reads, use the first published normal for force,
back-truck short circuit, fresh deck validity reversing between publication
and force, late failure preserving earlier stores, locked entry source reads,
and NaN/infinity/signed-zero force selection. They have not been run.

The adapter makes fresh current body/owner accesses at each indicated capture;
no ContactSpyFrame or completed solver token is fabricated. Missing actual
constructor fields remain errors at their selected reads.

Next actual getters, not implemented here: GetPartSlideSpeed82b242f8 called30670;
GetDeckToWheelBase82b2e718 called306b4. Their results precede output guards and
storesCollision+14 at306a0 / Reckoning+c0 at306e0. Processed wheel hardness+9c8
is read after assertion30704 and storedMotion+c8 at30710, then conditional
outer WantsRespawn+101 output/clear remains. These are explicit later boundaries.

Root independently reviewed raw301a4..30284 for the preceding wheel-position
draft and raw30288..30658 for this draft against both output and same-owner
adapters. The asymmetric wheel body-pointer/latch ordering, two fresh normal
reads, ground-force consumption of the already published GroundUp, ordered
contact short circuits and the final two FSELs agree. Approved for the next
integration window; both drafts remain unregistered and untested at this point.
