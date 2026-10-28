# PhysicsGround collision, brake and push continuation

2026-09-12 source audit and **unregistered draft**, not a completed Ground frame.
No Cargo run. Existing registered modules and state owners were not changed.
Parent owns registration, independent review and validation.

## Exact route and current boundary

`PhysState_PhysicsGround::UpdateSkateboard82d9a018` reaches collision only after
the actual WallRide result, live `kApplyFakeTorque` branch and actual Ground
`doJump` branch. The earlier packet
`ground-force-queue-schedule-continuation.md` maps both nonordinary branches.
Neither branch may be replaced by an assumed false value.

The ordinary branch initializes the force result at a524/a528 and passes NULL
optional new velocity to `Toolkit_CalcCollisionForce82b27530` at a52c. The torque
scratch has no demonstrated initialization before this call. An early false
callee return does not publish a torque result. The existing complete collision
leaf can be reused, but a caller must not claim its scratch initializer was a
native retained output.

On true, copy force world vector to Ground+30 at a560, local vector to +40 at
a568, and torque to +70 at a56c. Only afterward lookup collision key
`d613692c606fe326` through global+10c; load/store timer+ac at a5c8/a5d0;
`TargetAngularDisplacement` at a5d8; Board+530 bit80000000 at a5ec; queue Add
kind15 at a5f4; NormalizeSafe(force, source fallback), fresh Processed190,
dot3, late `kDotToStopPush` target/value, then request_push_stop+f2. BO4/BI24
at a648 means NOT LT, so only ordered LT sets the flag. This route exits the
Ground control call; it never executes braking or push acceleration.

On false, global feet+bc key `21b212a085d31d3b` lookup a69c/load a6b0 precedes
braking a6bc and push a6c8. The threshold remains live in f30 for later foot
queue decisions; it cannot be discarded in a convenience wrapper. The next
unimplemented leaf is AbsorptionFootForceMag82b28c60 at a6d0.

Do not invoke `skateboard_ground_force_tail`: its GroundAnimation82d8a8b8
schedule calls collision last and supplies different foot inputs. Reuse only
`SkateboardSlideCollisionServices::try_collision` and
`SkateboardBrakeDragServices::try_braking` at the above sites.

## Full push force recovery

`Skateboard::Toolkit_CalcPushAcceleration82b272f8..27528`, called at a6c8,
uses hidden output r3 and actual board r4. Draft `skateboard_push_acceleration.rs`
contains the complete leaf, retaining output mutations when late reads fail.

1. Local position+10 gets full +0 at27348 and again2734c; world force+0 gets
   full +0 at27350. Both precede the first processed flag read.
2. Processed8c4 bit02000000 absent returns those zeros. Present samples8c8
   bit00000800; set causes real Board+530 bit40000000 at27380 and returns.
   This is the same abort-push field reset by Update3 and consumed by board Fill.
3. Load animation push speed940 then prior Z speed944; scalar subtraction.
   Only ordered GT zero retains the difference; NaN and both zeros become +0.
4. Global+b0 key `76feee54a3ad2af5` lookup27400, null-only default4 fallback,
   value27414. Capture upper comparison, negate bound, capture lower comparison
   against the ORIGINAL difference. Upper and lower stores are independent;
   negative bounds can satisfy both and lower wins. Do not use a clamp or else-if.
5. Fresh processed absolute body speed948 at27468 precedes second global lookup
   key `a9b532ebbed16f38` at274a4. Addition274b8 precedes pointer value274bc.
   Raw branch274c4=`4099000c`, BO4/BI25 NOT GT: NaN bypasses correction.
   Ordered overflow computes limit-speed then fsel(remaining,remaining,+0).
6. Mass970 at274e0, separate rounded multiply274e4, DT93c at274e8, full vector
   direction160 at274ec, scalar division274f0, vector multiply2750c and final
   world-force store27510. Unknown W stays unknown; known W gets the same scale.

No new cvar is needed by this leaf. `PushAttributes` retains a once-bound
capability for global+b0: constructor828b2510..2538 builds class
`d4d04d2fa87a9a31` with the global default collection `d7edbd362d7d2152`, then
stores+b0 at2544. It is not a generated layout-backed instance. Only missing
attribute data selects DefaultDataArea(4); decoding errors remain errors.

Repo search finds no earlier compatible retained push or global trajectory
instance. Existing global slices bind different members. A read capability
stored once on the existing board ForceRuntime matches its existing feet/brake
projection; this is not certification of complete GlobalAttributeCollection Init.
The actual HandPlant trajectory class is different and must not be reused.

## Same-owner adapters and proposed shared amendments

New `physical_player_push_source.rs` borrows actual processed Arc, existing
`SkateboardEntryState` and retained PushAttributes. It samples exact fields:
animation_attributes.push_speed940, prior_output.z_speed944,
toolkit.abs_body_speed948, physical_mass970, physics_state.time_delta93c,
toolkit.body_z_direction160. It never constructs owners or reads tuning early.

New `skeleton_processed_ground_force_input.rs`, proposed Contact child, feeds
the existing brake/collision traits with fresh same-Arc reads. Captured selected
surface is the existing retained SurfacePhysics instance, not a new binding or
diagnostic collection. It reads collision8c8/40000 and process_data_output
hook_to_hips2e0 from actual ProcessData outputs; actual board Reckoning normal
comes from the separately verified same owner, not a stale processed snapshot.
No broad readiness predicate masks an earlier valid source field.

Required shared changes, **not applied**:

- Register force_runtime::push and Contact::ground_force_input, plus the new
  physical caller child; retain PushAttributes once beside current feet/brake.
- Add `SkateboardEntryState::request_abort_push` setting its existing
  `abort_push=Some(true)`. No second board flag state.
- Migrate actual Ground collision_force[2] and collision_torque to typed vectors
  before positive collision publication; preserve known constructor/reset lanes.
- Recover/integrate actual TargetAngularDisplacement before claiming the positive
  collision route completes. Preserve Ground force/torque/timer prefix writes if
  this service is still pending at a5d8.
- Add actual live `kApplyFakeTorque` and `kDotToStopPush` references after their
  initializer/table proof, not constant false services. No slots reserved here.
- The later LinearDrag consumer requires distinct Processed `mPush+9a4`.
  No retained producer was found in current modules; its adapter currently reports
  explicit Missing at that getter. It must not substitute animation push speed940.

The new files are deliberately unregistered. The push runtime references the
proposed entry setter; it is not yet independently compilable/mounted. Four
draft tests cover gate laziness and abort, exact read order, typed W scaling,
negative-bound two-comparison behavior, NaN branches and retained zero writes on
late missing direction. Tests have not been executed.

## Reproducible raw spans

Original external IDA bytes, SHA-256, exclusive end:

| Span | SHA-256 |
|---|---|
|82b272f8..82b2752c|2638f7df5cd9b6dc0bc026c997c966081c6b23ae964e1ee4fb75153c05de2951|
|828b24f0..828b2548|ef5d7460a7f56a33438aee6fea586c2b9aae52eeb42855de420565bf5dfa41ae|
|82d9a4d8..82d9a6d4|077ce2b3fae8c2f26a9f068ae6b6217f031d1f93de92bfb4a5e1dba8a1e0004b|

Arithmetic retains source operation order and explicit host floating-point
policy; no bitwise Xbox execution claim is made.
