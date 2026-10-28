# Controlled first-skating spawn and pose audit (2026-09-23)

Read-only audit of the matched debug XEX/PDB in the live IDA database, the
registered Rust path, and the bounded Windows capture. No native per-tick pose
trace was available. Source branch/layout facts below are `exact`; the proposed
visual cause is a `hypothesis` until the discriminating measurements are made.

## Native spawn branch and current gap

`Sk8::SkaterManager::OnCommand` (`0x82600920`) decodes command 3 at
`0x82600984`, calls `AdjustSpawnPosition` at `0x82600998`, then destroys the
previous Actor, calls `SpawnPlayer` at `0x826009B4`, and updates local identity.
`AdjustSpawnPosition` (`0x825F1A20..0x825F1BB8`) first copies all four matrix
columns. PDB `Sk8::SkaterManager+0x50` is its `Sk8::Simulation*`; the
`Sk8::INetworkGameQuery` interface starts at Simulation+0, and its vtable slot
zero is `IsNetworkGame`. Raw `0x825F1AD8..0x825F1AFC` returns the unchanged
matrix when `IsNetworkGame` is false **and** the live
`Physics_Debug.kbEnableAdjustSpawnPosition` value is false. This is a gated
branch, not unconditional terrain snapping. The cvar initializer at
`0x82F569F8..0x82F56A30` passes a default argument of `1`; no name match for
that cvar was found in the mounted `data/startup.ini`. Its actual linked value
still needs a runtime read before claiming the branch taken in a native boot.

When enabled, raw `0x825F1AA0..0x825F1ACC` builds vertical offsets +0.75,
-500 and +0.15000001; `Physics::World::LineCollisionTest` is called at
`0x825F1B70`. A hit replaces translation with hit position plus the final
offset at `0x825F1B8C..0x825F1B98`; a miss retains the input transform. Axes
are preserved. The authored A_RobsPark translation is
`[588.8617,-54.231647,996.5048]`; the existing mounted real-terrain probe
hits `[588.8617,-54.431,996.5048]` at that X/Z. If the native gate is enabled
and this hit is selected, the expected Y is about -54.281, roughly 0.05 below
the currently passed authored Y. That small delta may cross a wheel/contact
threshold; it does not by itself explain the character's visible rotation.

At the time of this read-only audit, the controlled facade read the authored transform in
`selected_session_public_facade.rs::try_begin_controlled`, calls
`SelectedSession::try_continue_controlled_player` and constructs the physical
pair before `try_select_terrain`. Resident ground is requested and admitted
later in `try_poll_controlled_boot`. Thus the native pre-Actor adjustment cannot
use this session's ground at the present call site. The same owner already has
`SelectedSession::try_select_terrain` (which needs assets and world, not the
player), `SelectedSessionTerrain::ground_pool_for`, and
`PhysicalAssemblyWorld::try_world_line_test` / `SimulationWorldLineQueries::try_line_test`
(`World::LineCollisionTest`, `0x827AB338`, `BF_Default=7`). A faithful
continuation should start and admit that terrain on the retained world, query
the live cvar and line, then pass the adjusted matrix to the existing Actor and
physical constructors. This requires deferring those constructors until
residency completes or polling terrain synchronously before them. A second
world, visual-map raycast, late Actor teleport, or hardcoded 0.05 shift would
not preserve the source route. At that point the selected 342-cvar Rust table had
no `kbEnableAdjustSpawnPosition` entry, so the live cvar needs a narrow retained
binding before this can be called source-equivalent. The later integration
and corrected line arithmetic are recorded in
`controlled-spawn-adjustment-refactor-audit-2026-09-23.md`.

## What the 30-frame run establishes

`artifacts/controlled-skating-state30-2026-09-23.log` records completed state
Ground at frame 1, Air at frames 2-22, GroundAnimation at frame 23, and Ground
at frames 24-30. No WipeoutGround was observed. The same run environment's
Windows pad test found XInput slots 0-3 disconnected (status 1167), so no
intentional controller bail input is evidenced. The frame-1 and frame-30
screenshots differ; the low seated/fallen appearance is not a static bind-pose
image. These facts do not establish a native-equivalent landing pose or a
presentation-matrix defect.

`physical_player_system_logic_session.rs::select_ground` follows native
`CalcSuggestedState` (`0x82DDC558`). Its ordinary Ground-to-Air route in
`finish_ground_selection` requires no processed wheel/contact flag, an
off-ground board wheel-line height/time condition, and zero teleport countdown.
The transition itself is source-ordered through
`physical_player_ground_to_air.rs::try_update_pair_system_logic_with_actor`.
Thus neutral frame-2 Air can follow first-frame wheel clearance/contact data;
the state trace alone does not prove a false transition.

The current Actor frame path runs original input intents, Action, Motion,
animation phases/sync and physical setup before the physics step
(`actor_selected_frame_session.rs`). `actor_animation_body_dispatch.rs` uses
the authored `rig_Tpose` bind fallback only when both the root channel and base
animation are null. The selected cross-owner and public-facade original-data
tests check command delivery, completed physics/camera updates, and palette
publication; they do **not** compare first-frame neutral bone matrices, board
placement, or an upright pose against a native trace. Whether the mounted Motion
graph selects the correct first-frame clip and whether the eventual Ground pose
has the correct animation remains unknown.

## Smallest discriminating measurements

The existing `SelectedSessionFrame::try_step_queued` observer receives the
same `PhysicalAssemblyWorld` at BeforeActor, BeforePreSolve, BeforeSolver and
AfterSolver; the public facade currently passes `None` at its call around line
660. At first boot, after frame 1, before frame-2 state selection, and at frame
30, log on the same pair: physical current state; Processed
`num_wheels_in_contact`, contact flags `8c4`, `frames_since_jump`; board
`wheel_lines.height` and `post_fields.time_since_wheels_touching`; Vault
`CLEAR_HEIGHT`, `RECENT_HEIGHT`, `CONTACT_GRACE`; board Part 6 transform; PhysOut
root matrix and wipeout bit; submitted local hips/skateboard matrices; selected
Motion base animation name. Compare the physical root/board transforms with the
published palette root to locate physics versus presentation divergence. Keep
the probe read-only and remove temporary diagnostics after capture.

## Read-only contact measurement after this audit

A temporary `SKATE2_TRACE_CONTACT` read at the completed public snapshot ran
three release frames and was removed afterward. Log:
`artifacts/live-contact-trace-release-2026-09-23.log`. At frame 1, the
physical state was Ground but Processed and Body both reported zero contacting
wheels. The four retained wheel distances and height were each `0.14285605`;
Body time since wheel contact was `0.016666668`, teleport countdown zero,
and the live Vault thresholds were CLEAR_HEIGHT `0.12`, RECENT_HEIGHT `0.06`,
CONTACT_GRACE `0.2`. Consequently the registered selector's `height >
CLEAR_HEIGHT` condition is true before frame 2, consistent with its observed
Ground-to-Air transition. At frames 2 and 3 the state was Air, wheel count
remained zero, and height was `0.2` with four zero distances. This identifies
the Rust branch input; it does not prove that native adjusted spawn would
produce contact or that the later reclined pose is entirely a physics issue.
The native gate, line result and exact initial board/wheel transforms remain
to be compared under the same retained terrain query.
If the earlier candidate spawn adjustment lowered all initial board wheel
points by the same roughly `0.05`, the measured `0.14285605` clearance would
be about `0.093`, below CLEAR_HEIGHT; RECENT_HEIGHT would still be exceeded,
but `0.016666668` is below CONTACT_GRACE. That would change this branch.
This is a conditional geometric inference, not a measured adjusted-frame
result or proof of native behavior.
