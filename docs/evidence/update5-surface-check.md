# Update5 physics surface requests

Source reviewed 2026-09-13: DoPhysicsSurfaceCheck82dfef60..f14c,
ConsiderSurface82dfeeb0..ef5c; actual Update5 call82e061ec, after push/intents
and before ProcessIntents82e061f4. Raw instructions and PDB enum/layout checked.
Drafts are unregistered; root owns integration and Cargo gates.

## Source behavior and owner reuse

Entry captures Processed pointeref70 and reads mPhysicsState8e4 atef74.
The same captured owner is reused for hips valid414 and surface ID410 in state602.
PDB mCurrentPhysicsState8e8 is a DIFFERENT field. Concrete adapter uses
physics_state_fields().physics: existing copy_prior_state writes that field at
82e02460->8e4, then .current at2468->8e8. No state category/current output
substitution is made. Missing actual input state/hips fields is an explicit error.

Ground100..104 calls guarded GetCollision atefa4, then reads current output
Collision+10 physics surface type atefb0 and calls ConsiderSurface atefb4.
This is the existing same PhysOut collision_type owner, populated through actual
board output, not another collision query or surface reclassification.
Existing board ChoosePhysicsSurfaceType82b24878 (contact surfaces plus live
forced-surface cvar) and its collision publication remain the producer.
There is NO table access in DoPhysicsSurfaceCheck or ConsiderSurface; do not
repeat the upstream table/attribute lookup, vote or force-override selection.

ConsiderSurface uses PDB pegasus::tPhysicsSurfaceType: Unrideable=6, Wipeout=9.
Value6 reads guarded State.physics_requested_teleport and only if false calls
guarded GetState again before storingtrue atef44. Value9 similarly reads
physics_requests_wipeout and conditionally stores at eef8. Every other integer
is a source no-op; no clamp or invalid-enum substitute is introduced. Existing
true requests survive and no request for the other action is cleared.

All other branches also use existing fields, so the draft covers them:

* 300: guarded Collision.skeleton.unrideable_contact; iftrue, two-stage guarded
  teleport request. This does not run any GrindManager or change host grind policy.
* 500,501,504: guard OffBoard.left_foot_down; iftrue, fresh guarded left ID;
  consider `(id >> 7) & 31`. Repeat right foot AFTER left consideration.
* 601: guard Air.is_foot_planting, then iftrue fresh guarded footplant ID and
  the same packed surface extraction.
* 602: read valid from entry-captured Processed; only iftrue read its hips ID
  and consider extracted type. Neither input access checks the output latch.
* Every other signed state returns without accessing any output component.

No additional field producer is needed. Inactive feet/plant/hips do not read
their IDs. All mandatory component accesses use existing concrete construction
status; null components error at the reached source access. Prior completed
requests survive subsequent failure. The actual output owner remains locked by
the host mutex for the helper, while native write-latch guards remain explicit.

## Files and root integration

* physical_player_update5_surface_check_output.rs: complete helper/output kernel.
* physical_player_update5_surface_check.rs: same-owner runtime adapter, actual
  Reckoning Processed capture, native8e4/hips reads.
* physical_player_update5_surface_check_tests.rs: five focused tests.

In lib.rs register `mod physical_player_update5_surface_check;`.
In PhysOut construct module register:

```rust
#[path = "physical_player_update5_surface_check_output.rs"]
pub(crate) mod update5_surface_check;
```

After actual `try_publish_push_intents(owner)` and before actual ProcessIntents:

```rust
crate::physical_player_update5_surface_check::try_do_update5_surface_check(owner)?;
```

The function takes the same `&Arc<PhysicalPlayerAssemblyOwner>` already validated
by the outer session. It adds no alternative pair or startup owner. The caller
keeps its existing grind exclusion token and does not claim full Update5 solely
from this helper. ProcessIntents, AnimOut.Fill, Reset and count remain root work.

## Validation

Five authored tests cover every Ground state and surface6/9/ordinary/unknown,
request preservation, both offboard feet, collision300 and footplant601,
same-captured hips/lazy invalid ID, signed unhandled states touching no output,
and a genuinely null Collision allocation refusing at Ground access.
Rustfmt completed; no Cargo or registered/shared edits by author.

Separate preexisting audit observation sent root: some other adapters label
`.current` as8e4 despite copy_prior_state's distinct stores. This draft uses
the verified8e4 owner and does not alter those other adapters.
