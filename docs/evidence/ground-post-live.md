# Ground post-physics on the actual Actor pair

`PhysState_PhysicsGround::UpdatePostPhysics`82d8e998 first invokes
`Wipeout::CheckForGroundWipeout`82de09c0 at82d8e9b4. It then freshly captures
Processed and reads its AI word6b0 at82d8e9bc. Only mask80000000 invokes
`Skateboard::UpdateAIPathFollowing`82d8ea00, after requiring GlobalAttributeCollection
and physics_ai_default. The ordinary clear branch returns. This AI read still
runs when the detector returns after decrementing constructor invincibility.

Root registered the independently reviewed detector and adapter under the
constraint session. Runtime attribute calls use the actual board construction's
retained SkateboardAttributes through try_with_ground_force_board; each lookup
and subsequent float load remain distinct and lazy. No new attribute binding,
contact snapshot or replacement collision producer is created. Pair identity
is validated before native writes. The late AI read preserves detector writes
if its bit is unknown or the still-pending positive AI branch is reached.

The original solved Actor corpus calls this after the real complete Skeleton
post-physics and before Update5. Its first call exercises the actual constructor
0.04 timer decrement with the actual Processed timestep. It does not artificially
expire the timer or claim full detector collision coverage from that first call.
The detector's focused tests cover its active branches and request ordering;
future recurring frame integration must exercise them on evolving real contacts.

Input cvars193/194 use the exact initializer and native table order from
ground-wipeout-detectors.md. Root rechecked table82fd4538..4570:193 follows131,
then129, then194, then141. The current branch defaults are live false values;
enabled diagnostic paths remain explicit pending errors.

Independent runtime_path review accepted kernel, concrete adapter, final lazy
attribute adjustment, and late AI read order. Confidence high for these source
semantics; existing numerical service boundaries remain. Validation results
are recorded in the rolling checkpoint after current gates finish. The remaining
Skeleton post-wipeout IK/pose continuation prevents a full Update4 claim.
