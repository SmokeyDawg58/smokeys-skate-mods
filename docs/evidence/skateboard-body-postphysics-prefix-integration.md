# Actual board activation and compression prefix

2026-09-08, root integration and independent source/code
review. This continues the accepted wheel checkpoint 49472/bb2165.

The actual physical board now exposes the selected `PostPhysics` entry:
ordered dt > 0 assertion, actual ascending Assembly Part activation, then
`CalculateAverageWheelCompressions`. It stops before the collision-list getter;
it does not certify the remaining contact, normal, truck or Actor stages.

## Evidence and ownership

- `Assembly::Activate` 82ae09b8 reads each actual body state, activates only
  Frozen bodies, rereads the Part body and unconditionally clears cooldown.
  `Simulation::ActivateRigidBody` 82ae3228 reuses the existing pool rings,
  appends to Active, preserves state bit8, updates counts and initially stores
  wrapping configured cooldown minus one. No replacement body is allocated.
- Attached `Part::GetBodyTransform` 827ab5c0 reads the actual body pose; unlike
  `GetTransform`, it does not compose the inverse Part mapping. The narrow
  getter refuses detached bodies and preserves unknown native W lanes.
- `CalculateAverageWheelCompressions` 82b2b538 uses a general cofactor inverse,
  reciprocal estimate and two refinements. It reads body transforms6,0,1,2,3,
  performs the X projections, then reads the SAME construction owner's retained
  wheel distance before Y/Z projections. Front and back stores remain distinct
  and ordered. The distance originates in the original Vault constructor read;
  this stage neither hardcodes it nor fetches a replacement instance.
- Shared inverse-basis extraction preserves the existing two-bone caller's
  cofactor and translation ordering. Numeric estimates retain their explicit
  host boundary; Xenon bit parity is not claimed.

The source details, raw ordering and hashes are in
[the runtime plan](skateboard-body-postphysics-runtime-plan.md) and
[body-transform evidence](skateboard-body-transform-source.md).

## Validation and limits

Workspace run 48385/ec0b04 passes 3,304 unit tests (3,061 core, 202 formats,
38 app, three symbols) and six doc tests. All eleven original-data checks pass.
The actual wheel corpus now runs this prefix after the retained batch's End
and real ground solve. A separate f64 Cramer's-rule oracle projects the actual
wheel body positions into the actual deck body frame and agrees within 0.0005
with both computed compressions. This tolerance covers the host f32 projection;
it is not a native trace tolerance or gameplay equivalence claim.

Focused tests additionally cover non-rigid scaled/sheared frames, read order,
late failures retaining prior compression values, frozen ring ordering, spy-bit
preservation, cooldown underflow and partial Assembly effects. Unsupported raw
pool state bits continue to fail integrity validation explicitly.

Compression readiness means the last successful calculation, not completion of
current-frame PostPhysics. The later normal publication must bind its own
completed-frame provenance. Native constructor compression zero stores are
documented in the plan; this selected calculation's readiness does not stand in
for constructor/reset completion.

The same checkpoint includes two independently reviewed ground-jump tangent
helper corrections: unordered error comparisons take the prepared fallback,
and the correction bound includes prepared tangent length. These helpers still
do not constitute the live GroundAnimation jump orchestrator.

Strict all-target Clippy with warnings denied, fmt and diff checks pass at
54650/7c519e. Logs are `%TEMP%/skate2-body-prefix-workspace.txt`,
`skate2-body-prefix-original.txt` and `skate2-body-prefix-clippy3.txt`.
Full playable skating remains unfinished.
