# Active Ground jump: same-owner implementation seam

Read-only audit, no implementation or new scheduling claim. Confidence high
for ownership/reuse; raw VMX decoding and full key/graph sampling proof remain
required for the active calculator. Source `Toolkit_CalcGroundJump82b315a8`
was freshly decompiled; decompiler phantom vector registers are not trusted.

The reached103 composer now handles the actual inactive StartJump branch. An
active animation event already has producers: same Skeleton event processing
and `SkeletonProcessedJumpEvents` retain strength/adjustment X/Z and StartJump;
`PhysicalPlayerAssemblyOwner.jump_attributes` already owns the retained
`SkeletonJumpVaultAttributes` with physics_jump.default, class
c2e0a8b6c13990fd/default d7edbd362d7d2152. Extend that provider's selected lazy
cached graph/attribute reads, rather than creating another global instance.

Native active calculation stages:

- 31624 samples jump normal Processed240; graph layout80 at31684 is sampled
  before projecting prepared/current velocities (2c0/190). Conditional error
  correction316a8..31918 uses actual normalize/acos/reciprocal/clamp sequence
  and lazy graph layoutd0. Reuse reckoning_numeric primitives only where the
  raw graph agrees; preserve all vector lanes and known-W propagation.
- 31960..31dfc performs ordered speed/height attribute and graph reads,
  gravity/COM-height projection, ballistic estimate/refinement and optional
  vertical-response graph. The ordinary physics.rs snapshot helpers eagerly
  accept tuning/curves, use host sqrt/acos/clamp/lerp and explicitly do not
  establish native runtime bits or sampling order. Do not directly call them
  as a purported exact calculator; do not instantiate legacy state owners.
- 31e00..32098 applies world-Y compensation, normalized body-side/forward
  adjustment and final velocity. 320bc..d0 stores actual board JumpDV (+b0)
  before final debug gate. Board force runtime currently has no JumpDV cell;
  `physical_player_board_fill_pumping::Source::jump_delta` honestly returns
  PendingJumpDeltaProducer. Retain this one actual board field with its proper
  constructor/unknown provenance and wire the existing Fill consumer.
- Caller82d8a7f4..a888 publishes completed JumpInfo, calls existing SetVelocity,
  stores jumpedThisFrame, sets drag0, obtains same Skeleton launch info,
  overwrites launch velocity/playerJumped, then same selector Launch/Update,
  and stores selector launch velocity into retained state jumpVel. Existing
  physical_player_skeleton_launch_info, onboard_launch_runtime and selector
  finish paths supply these actual consumers. No second selector is needed.

Exit10382d83ed0 sets drag0 then restores wheel material before reading retained
doJump. False returns; true forms gravity*DT+velocity, invokes the already
source-reviewed `physical_player_ground_exit::clamp_jump_velocity` leaf and
existing SetVelocity. Broaden visibility of that leaf when connecting Exit;
do not replace it with an approximate legacy vector clamp.

Smallest coherent next active packet is this lazy global/input adapter plus
the exact calculator, actual board JumpDV store/Fill connection and existing
launch consumers in103. Tests need ordered read/failure prefixes, correction
gate equality/NaN, native fused boundaries and preserved late JumpDV writes.
No new cvar is yet justified: live MaximumError already index246; audit actual
kDrawJump registration before requesting another slot.

Fresh constructor follow-up:82b389a0 loads +0; stack var_B0/AC/A8 receives
+0 at38a28/30/34 and var_A4 receives integer0 at38a38. Pointer38a7c,
load38ad8 and store38ae0 publish all four zero lanes into board+b0 (r18=b0
at38a6c). Therefore the actual constructed force-runtime field has known
zero initial bits; no per-frame zero/reset is inferred. kDrawJump is already
registered79 with try_draw_jump_debug_info; no additional cvar is required.

Root took ownership of the three existing active-jump drafts while COM owns
animation reuse. Added unregistered kernel tests and artifact registration
builder/patch (physical-ground-jump-2026-09-23). These are NOT compiled or
independently approved. Fresh root raw31b04..31c10 confirms min/high height
FMADDS, strength then gravity reads before the COM/up dot, and rounded
(1-strength)*low followed by high*strength fused addition. Fresh caller82d8a7b0
confirms active launch EXCLUDES the existing six-force tail; the artifact uses
an else branch after launch. State result is copied only after calculator
returns; late debug failure still retains board+b0 delta from the kernel.
The typed-vector migration affects only JumpInfo.velocity and103 jump_vel,
leaving other remaining-state vector types unchanged. Exit103 remains separate
unintegrated source evidence, not claimed complete by this patch.

Independent review corrected the adapter's Ground input: native Processed+1e0
mGroundPos is the existing ProcessedGroundPosition field, not prior-output
mGroundPosAllStates+1f0. The packet now exposes a narrow immutable getter and
uses that exact owner. PDB selector mLaunchInfo580 + startVel80 confirms the
post-completion +600 source for jump_vel. Kernel/adapter/registration source
review approved after this correction; registered by root for combined gate.
Confidence high for the selected native runtime projection, no full trajectory,
Exit103, hardware numeric trace, or playable completion claim.

Combined workspace-reuse-dynamics-jump gate:4879 unit +6doc PASS; original19/22(12.68s). Queued frame21 now reaches103->100 transition; camera PendingAnchorDynamics. No full active-jump original trace established.

## Reached outgoing103 continuation (root draft, not registered)

Original combined gate now reaches PendingTransition100 at frame21 after real
landing animation and first bounded reuse. Root draft ground-animation-exit
packet reuses Ground.Enter/Air.Enter via the existing transition runner, adding
only103->100/200. Previous state is103 but previous category100 (native static
GetPhysicsCategory82d83b80), unlike the earlier100/200 shortcut. Controller,
transition fields, Exit, Actor query, selected owner, Enter and success-only
wipeout request clear remain ordered; no second selector or owner is created.

Exit82d83ed0 sets drag0 at3ef8, restores material3f00, then tests doJump3f04.
False needs no gravity/time/velocity. True reads gravity964 then timestep93c,
uses retained state+20 JumpInfo velocity (not post-selector jump_vel+50), performs
full four-lane VMADD([0,g,0,0],dt,velocity)3f74 and calls existing native
Toolkit_ClampJumpVelocity82b23990 then same board SetVelocity3f98. Existing
clamp helper is exposed to its sibling; no new vector clamp. Read-only review
supplied raw/PDB provenance; final packet review and validation are pending.

Exit103 final source/code review approved and root registered the packet. Workspace/original-exit103 gate pending. No103->300 or other outgoing state admission added.
