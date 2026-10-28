# Update3 prediction and push reset prefix

Confidence: high source/control-flow reconstruction; host fused arithmetic is not a claim of bitwise Xbox VMX parity. Validation is pending the parent integration gate.

`PhysicalPlayerHiLOD::Update3` 82dfe708 increments actual +130 stateCount at e730 before dispatching actual selected state vtable+10 at e740. The selected Ground and Air vtables (8234a338, 8234a2a8) both target PhysicsGround.PredictFutureOfDeck82d869c0, which delegates to PhysicsState.PredictFutureOfDeck82d83ae8. Sleeping823715ec and WipeoutGround8234a6ec directly use the base. Runtime checks the actual state bindings against the same physical owner and board/skeleton handles; no category is assumed.

The matrix return ABI is hidden output r3, state r4. Base3b0c calls raw Skateboard.GetTransform82b237f8, which calls Body.GetPosition(6). This is NOT GetEffectiveTransform82b23778: no board-flip flag read or VXOR belongs here. The parent review caught and corrected an initial draft using the effective helper before acceptance. Existing `try_live_part_transform_host` supplies the same real Part6 matrix. Source then reads actual processed DT+93c at3b1c, fresh GetDeckVelocity82b25ff8 at3b20, and performs full-vector velocity*DT+translation fused at3b5c. Retained XYZ uses f32.mul_add; W propagates only when both source W lanes are known, including when DT is zero. No unknown W becomes zero.

Update3 e774 stores the resulting translation into actual Skeleton+3ce0, then e778 sets its +3dec prediction-valid byte. Fresh GetSkeleton precedes ResetPushAbort82b3bb80 at e78c: clear only the actual constructed SkeletonIK skateboardDontPush byte. No IK reinitialization occurs. Fresh GetSkateboard precedes native +530/40000000 clear at e7ac. The new board-entry projection starts unknown at generic construction and becomes Some(false) only here; it is separate from collision flags and PhysOut.Motions.abort_push.

The actual nullable PlayerGrabSpline owner is tested at e7b4; when present its Sync82dd0110 runs at e7bc. Sync first clears old-valid flag40, then checks retained line batch, then actual query future readiness. Constructor-completed idle owner has no batch and ready=false, so returns without manufacturing targets or results. A real batch stops at its missing synchronization consumer; ready query stops before its missing 32-to-5 copy/validation launch. Earlier flag clear is preserved on these errors. These environmental grab queries are distinct from airborne board-grab input.

Successful `PhysicalAssemblyWorld::try_update_pair_prediction_before_state` stops BEFORE state.Update at e7d0. It does not execute SkateboardController.Update, Skateboard.UpdatePrePhysics, StoreForceDisplacement or the later stateTimer update. Parent mounts it only after actual Update2. No query runs early to supply condition inputs.

Independent wheel-bootstrap finding: Body constructor82b382dc calls CollisionInfo.Reset82b23630 before timer386d8/Init387e8. Reset loads +0 at23650 and stores wheelHeightToGround+318 at23710; loop2373c..5c writes gJ normals and +0 distances for four wheels. Parent implemented that actual constructor projection, preserving unknown generic allocation. This supplies first-frame Update2 wheel height without reordering later wheel Start/End queries.

Focused tests cover full-known prediction W, unknown W with zero DT, untouched matrix basis, actual future readiness and partial old-valid clear, and board abort unknown-to-cleared transition. Parent owns Cargo, source review and original corpus gates; no local Cargo was run.

Parent correction September12: source Part.GetTransform is the correct call,
but try_live_part_transform_host explicitly supplies host affine W rather than
original packed native W. The Update3 adapter now carries unknown W for all
returned columns. Prediction already propagates W by Option::zip, including
DT zero; only XYZ is numerically known. Prior wording that this getter produced
full-known original lanes was incorrect. Workspace31 passed before this fix;
original28 then exposed the analogous required heading W at Ground/Reckoning.
