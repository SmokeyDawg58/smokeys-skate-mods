# Retained trajectory start-velocity adjustment

Proposed unregistered batch; no Cargo execution. Source confidence high, numerical
bit parity limited by the existing documented host ReckoningNumeric primitives.
This implements AdjustStartingVelOnVerts only, not private Launch or prediction.

Fresh debug IDB source: TrajectorySelector::AdjustStartingVelOnVerts82dbfbc0,
called by public Launch82dc4118 after the launch record copy and ready/grind-bonus
clears. Existing physical_player_onboard_launch_prefix.rs already implements
that preceding copy/reset prefix and must be reused, not replaced.

New files: physical_player_trajectory_adjust_start.rs and its _tests.rs sibling.
Register as a child of physical_player_trajectory_owners.rs:

```rust
#[path = "physical_player_trajectory_adjust_start.rs"]
pub(crate) mod adjust_start;
```

The prior typed migration of launch.start_velocity is required. The API is
OnboardTrajectoryOwner::try_adjust_starting_velocity(owner, attributes,
trajectory, overlay, numeric). It verifies the retained selector binding against
the same physical owner and actual processed occurrence; it creates no owner or
live tuning snapshot. Supply the already retained global Reckoning attribute
reader, global trajectory attribute reader, debug-overlay slot and numeric
service. Do not hold processed/Reckoning locks while calling it. A successful
return authorizes private Launch as the next operation, but does not run it.

## Ordered data and tuning reads

Source clears mOnVert at82dbfbf4, then reads processed ground-normal Y+1d4.
Three source FETCH calls read global physics_reckoning.default in this order:

| Behavioral role | Key | Lookup / load |
| --- | --- | --- |
| redirect blend | 0b42dfbb7f7756a0 | fc3c / fc54 |
| maximum ground-normal Y | 94923d136ea2da37 | fc94 / fcac |
| minimum normalized component Y | 58bfc8d41216c7ee | fcec / fd0c |

Each explicit null-pointer arm calls DefaultDataArea4. The actual existing
ReckoningAttributeRead Global lookup/float adapter preserves that behavior and
reports real malformed/unsupported data errors.

The source then normalizes start velocity (fd2c), rereads processed ground
normal+1d0, computes a component, normalizes it (fda0), computes component length
squared, reads transition input+95c, runs reciprocal-square-root refinement and
two scalar FSELs. Transition factor is source clamp(transition-.25,-1,+1), with
NaN selecting +1 through the second FSEL rather than preserving NaN.

Fourth FETCH 723ff102068622cf occurs atfe78 after that work. Another ground-normal
read follows. NormalizeSafe of the desired direction occurs atff0c even when the
later ground-normal threshold will reject adjustment. No flat-ground fast exit
may skip these reads or numerical calls.

## Raw VMX decoding and semantic math

IDA's displayed high VA operands are unreliable here. The VMX128 field layout
was cross-checked against the public [Xenia instruction-format declaration](https://github.com/xenia-project/xenia/blob/master/src/xenia/cpu/ppc/ppc_instr.h).
It was used solely to interpret source bytes; no decoder, emulator, guest
register model or instruction-shaped implementation is included in the port.

Selected raw words and corrected operands:

| Address | Word | Interpretation |
| --- | --- | --- |
| fd80 | 17ddfdb5 | dot3 start(v125),normal(v63) |
| fd88 | 179ff0b5 | normal(v63) times dot(v62) |
| fd98 | 17ddd83d | add vertical(v61), horizontal projection(v59) |
| fef8 | 15f480b5 | normal(v52) times transition factor(v48) |
| ff00 | 15cf88b5 | previous(v47) times fourth scalar(v49) |
| ff04 | 15bf7075 | world-up(v63) minus previous(v46) |
| ffb8 | 158950f1 | desired-unit(v41) times component-speed(v42) plus remainder(v12) |
| ffd4 | 14ac61b5 | original(v44) dot original(v44) |
| ffd8 | 150820b1 | initial normalized original(v40) times (1-blend)(v36) |

The projection's Y is overwritten with +0 atfd90. Its X/Z/W are retained,
including unknown W. The vertical vector is [0,startY,0,0]. Thus the component
is NOT original plus projection and NOT the ordinary tangent rejection.

On the vert branch, desired direction is NormalizeSafe(up -
((normal*transitionFactor)*fourthScalar), zero). Its component-speed-scaled value
is fused-added to (original-component). That redirected velocity is blended
against the original UNIT direction, not the original velocity, before a final
NormalizeSafe and rescaling to original speed. Separate multiplies and fused
operations follow the source. The source's final full-vector store is0058.

Shared numeric helpers preserve unknown W, XYZ-only dot products, NormalizeSafe's
threshold/fallback, two reciprocal-square-root refinements, and zero length
selection. Their established host estimate/reduction/denormal/NaN-payload limits
remain explicit; no console bit parity is claimed.

## Branches and failure prefixes

The late raw BGE atff2c exits if LT is clear, including unordered ground Y.
The following BLE atff38 takes the non-vert arm if GT is clear, including an
unordered normalized Y. Vert branch stores mOnVert=true atff4c before any
remaining fallible numeric work. A failure there retains that flag and the old
start velocity; successful calculation publishes only the final velocity.

Non-vert arm checks playerJumped first. If false, it reads processed state+8e4
and returns unchanged for200 (PhysicsAir). Otherwise it rereads start/normal,
projects onto normal, captures global physics_trajectory generated layout+1e4,
and replaces normal projection by its scaled value while retaining the tangent.
The generated global trajectory constructor828b1068 installs DefaultDataArea24c
for null layout; the existing wrapper's None->zero policy is that constructor
contract, not a fabricated consumer fallback. Source then unconditionally gets
DebugOverlay at00fc AFTER the velocity store00f8. The bridge uses actual
DebugOverlaySlot::require and preserves velocity on overlay failure.

Nine controlled tests cover unconditional flat prefix; full/partial blends;
the unit-direction discriminator; unknown W; PhysicsAir and playerJumped lazy
tails; velocity publication before overlay failure; onVert publication before
late numerical failure; early attribute failure; FSEL endpoints/NaN. These are
source-boundary tests awaiting root compilation/execution. Original-corpus
GetLaunchInfo->public prefix->this routine integration is still required.
