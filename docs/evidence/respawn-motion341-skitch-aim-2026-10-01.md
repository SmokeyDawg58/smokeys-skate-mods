# Respawn pedestrian query, Motion341, and skitch attachment audit

Source edits and Rust syntax parsing only;
the regression checks below have not been executed.

## Respawn near pedestrians

`physical_player_living_world_respawn.rs` still rejected every nonempty pedestrian
broadphase as `UnexpectedRangeCount`. Reused the already implemented
`LivingWorldGrindServices::try_pedestrian_intersects_sphere`, also used by
UpdateCollisionStatus. No new collision owner or query implementation.

IDA `Manager::DoesAnyPedestrianIntersectSphere82703600`: raw82703698..82703928
contains a populated loop that Hex-Rays omits. It resolves up to six nearby
IDs, gets collision Part0, transforms the candidate into its local frame, and
tests Z/X/Y strictly against half dimensions plus radius. Respawn retains its
unraised candidate position, vehicle-first order, immediate rejection on a hit,
and lazy DMO capsule attribute reads after both sphere queries miss.

## Motion expression341

Independently resolved both switch tables via IDA switch metadata:

- Pre8290aa8c entry341 ->829155bc: `!condition290 && !condition291`.
  Native condition pointer offsets0x488/0x48c are indices290/291, both HasIntent.
  The first positive intent returns false without reading the second.
- Sustain8291e250 entry341 ->82920d44: `!condition291` only.

Existing condition ownership/cache is reused. Added truth-table and read-order
regression; changed the unreviewed-Sustain test sentinel from341 to u16::MAX.

## Skitch attachment

Attaching to a car launched the skater sideways. Found a concrete
translation defect in UpdateGrabSpline82d94760, not a tuning change:

- Raw82d952d0 loads r6=0x230;82d95330 loads state+r6 into v8.
- PDB `PhysState_Skitching`: AimSpace starts0x210, zAxis0x230,
  translation0x240. Curve delta is a different field at0x2a0.
-82d95334 performs translation + zAxis*0.5;82d95338 stores the aim origin.
- Rust incorrectly used curve delta*0.5. Corrected to aim[2]*0.5.
  A straight spline has zero curve delta but still needs the native aiming bias.

Extended the original-attribute constructor corpus with straight and rotated
spline cases for both authored/non-authored updates. Those checks are unrun.

Audit:22 defaults at references960..981 match native initializers82f940c0..
82f94950 and82f886e0/82f887f0. Entry limits60deg/40deg/0.5 match82f8f8b0,
82f8f910,82f8f970; authored/feed switches match82f699c8/82f8b558. Read all45
physics-state attribute keys used by the skitch kernels from the original
skater Vault; all resolve in class032cd3de521b4ecd/defaultd7edbd362d7d2152.
Graphs retain all eight authored points; no retuning or substituted curves.

Rechecked current/future/last spline frames, initial distance-rate reset,
CalcSpeedForce82d969c8, heading82d9e8d8, torque82d96d38, follow controller
82d95468, native enums, and board force/torque call order82da0960. Preserve
last-current velocity and speed-force-before-heading order. Truck damping's
raw Z-speed input is native82b23980, not an accidental missing curve.

The aim discrepancy is confirmed against source; elimination of the reported
sideways launch is NOT runtime verified. Do not claim all skitching parity.
