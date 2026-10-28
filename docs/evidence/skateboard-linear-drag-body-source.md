# Actual board linear-drag publication

2026-09-08 root, original IDA debug XEX raw source. Independent review pending.

`Skateboard::SetLinearDrag` 82b23a98..82b23aa4 SHA256
c865d143cb06c7422980b45c58e5d558524147d993e5edcaa589d83b4c5d26e6:
lwz r3,+0c (actual SkateboardBody); li r5,-1; tail branch to
`Body::SetLinearDrag` 82de7de0..82de7f38 SHA256
900ccf4b68c98689631e9e91d41c196e4b480bde7c8beada594432b51e67deac.
Decompiler invents one extra argument; raw r5 is the signed Part index.

GroundAnimation non-jump calls at 82d8a9cc only AFTER normal queue writes
front4/rear5/slide1/brake2. Collision-true queues type15 and omits this call.
Jump calls at 82d8a840 with zero. Other calls from GroundAnimation Exit,
PhysicsGround Exit/Update and SlideGround Enter/Update do not change this
setter's meaning. This packet implements only index -1 used by Skateboard.

Body gets assembly+18 and count+1c; signed index>=count asserts line316 then
DebugBreak. Index -1 chooses all Parts. No finite/positive drag validation.
Constant at 8225fd98 is **426fffff**, approximately 59.999996, not 60.
For count>=4, `fmuls f0,f31,f13` at 82de7e58 computes drag*constant once.
For each four-Part group, body pointers load ascending at 7e6c/74/78/7c,
then all four inertia pointers body+5c load ascending 7e84/88/8c/90,
then four scalar stores to inertia+20 at 7e94/98/9c/a0. The next group
begins after those stores. The tail computes the same product anew at 7ebc,
then each remaining Part body/inertia read precedes its scalar store 7ee0.
For a seven-Part board, four grouped stores precede three tail stores.

This changes the actual retained inertia's linear_drag field only. It does
not change cached inverse mass/tensor, motion, force/torque, cooldown or RF.
The existing simulation_dynamic_update consumer samples current retained
inertia and computes positive_or_zero(frequency-linear_drag), so an unused
separate Skateboard scalar would not implement the setter.

Current board binding copies each authored component inertia into one immutable
runtime Part payload. Actual runtime drag requires that single payload to be
mutable from creation, with the SAME owner admitted to the body pool and read
by the solver. Do not replace an already admitted inertia or update only a
construction snapshot. The existing SimulationMutableInertia safe-host pool
authority and locking model is reusable; group pointer acquisition must precede
the first store, and later failures preserve earlier committed groups/stores.
Immutable diagnostic templates can retain explicit refusal rather than fake
successful drag application. Unknown fourth tensor lanes remain unknown.

Planned discriminators: original seven Part payloads and all body inertia
snapshots see exact scaled drag; cached physics fields remain unchanged;
signed zero/NaN preserved; stale/foreign/immutable errors; missing fourth
group pointer prevents writes to the earlier group members; a tail failure
retains prior four writes. Existing source solver must consume the changed
retained inertia on a subsequent real step. No whole GroundAnimation or native
pre/postphysics schedule is claimed by this setter alone.
