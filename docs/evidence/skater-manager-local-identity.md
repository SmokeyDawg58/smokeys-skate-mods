# Selected local identity and Update5 outer continuation

Source audit 2026-09-13; exact reference control flow, unregistered drafts.
No Cargo run. Root integration and independent review remain required.

## Native identity producer

SkaterManager ctor825fb300 retains Simulation atb3c0, initializes empty roster,
ghost=null atb454, local=null atb458, localIndex=-1 atb45c. Mapping ctorb468,
SkaterAnimSharedb470, recorder/playback nullb474/b478, Rumbleb480 and command
receiver registrationb4b4 remain complete-manager constructor dependencies.
Workspace.InitWorkspace828a7f04 constructs then publishes manager7f14.

GameModule.ResetGame8273a680 initializes requested player0 ata740. Only
isNetworkGame && !playbackGame replaces this by human/local-peer search.
Offline short-circuits without reading playback. Ghost-enabled/nonnegative
branch constructs GhostProxy(simController,index) ata85c and passes8; ordinary
branch passes the requested index ata908. Actual virtual SetLocalIndex ata91c.
SetLocalIndex82600380 rereads ghost flag; its ordinary false branch stores
index at826003f8 then always UpdateLocalPlayer82600400.
UpdateLocalPlayer825ffef8 captures old local, FindByIndex atfff1c, stores local
fff20, notifies observersfff44 and deletes recorder only if pointer changed.
FindByIndex825f19c0 scans the actual roster, returning null for its empty range.
The draft permits this transition only before first admission, with its actual
constructor-empty owned roster and null local. It does not admit an Actor or
claim complete manager construction. Root must schedule selected construction
and ResetGame before first SpawnPlayer, not infer identity from receiver0.

Input manager Reset826e2c20 already constructs receivers0..8 in
gameplay_input_update_manager.rs. ActorConstruction obtains actual receiver
index and publishes Actor+1d8. actor_input_generation.rs reads this field in
PhysicalStateActorQuery::player_id. DoStartSinglePlayerGame8277b168 separately
creates backend player0 PT_Human; the input receiver loop alone is not local
player authority. SpawnPlayer82600630 publishes roster, sorts by index827? (call
82600780), then InitPlayer8260078c. InitPlayer ends UpdateLocalPlayer82600374.
Roster admission and observer delivery remain explicit future integration.

GetLocalIndex folded824859b0 reads+174. GetLocalSimIndex825eddb8 returns this
unless index8 and actual ghost nonnull, when it reads ghost.mSimPlayerID.
Draft cannot publish8 or construct a ghost, so both getters read actual index.
Actor.IsLocalPlayer825f0c60 reads manager index before Actor.GetPlayerID;
Actor.IsLocalSimPlayer825f0d00 reads Actor ID before manager sim index. Draft
query methods preserve this difference through a callback to the actual Actor.

## Backend defaults and cvar integration

Static sGhostSystemEnabled83040430 image byte is0. Initializer82f653d8 invokes
cVarBoolRef ctor82f65404: section Debug, label/title kbEnableGhostByDefault,
default false, null callback. Debug menu ctor827412e8 copies the LIVE value to
backend global; this must happen at that transition, not every identity query.
BackToSP8273dad4 explicitly clears ghost and network then ResetGame8273dd4c.
Online setup8275dba4 sets ghost true. Debug menu edits82736218/82736728 also
write it. Draft represents static initialization and debug-menu sample write;
network/ghost ResetGame branches return explicit pending before index write.

Root cvar proposals, reserved after frame_review159..190:

* 191: Debug / kbEnableGhostByDefault / kbEnableGhostByDefault / Boolean(false),
  source initializer82f653d8, ctor82f65404.
* 192: Debug_Scoring_DebugDraw / kDebugRenderLocalSimPlayer /
  kDebugRenderLocalSimPlayer / Boolean(true), initializer82f9e968, ctor82f9e994.

Use existing InputCvarRuntime declaration/registration machinery; add named
accessors calling try_current_boolean(191/192). Native initialization ordering
is ghost-default initializer before debug-render-local initializer. Backend
static initialization precedes later debug-menu copy. Retain same
PhysicalGameInfo Arc already supplied to physical World services. The draft
does not allocate a private replacement GameInfo or change network state.

## Update5 output span and adapters

Register physical_player_phys_out_outer_locality.rs as construct::outer_locality.
It begins immediately after actual Wipeout.Fill82e04f84 and returns after53ec,
before existing current-type span53f0..5434. Root owns5438..55e0 continuation.
Every source read/error must use the actual retained same-owner producer:

* Fresh Processed8c4 bits29,30,25,27,26,28 -> six State brake/push flags.
* Live cvar192 then actual local query -> State debug flag5140.
* HiLOD physics_state().state_count -> State5170.
* HiLOD tail().collide_with_world_only (byte120 bit0) -> State51a4.
* Processed8cc bit26,8c8 bit12 -> pinning/teleport51dc/5214.
* Fresh8c4 bits21/22 -> Air preparing/starting524c/5284.
* Processed.animation_attribute_fields().turn -> Ground52b8.
* Capture actual Processed Arc52e0; ground_position_fields_mut().ground_position
  is PDB mGroundPos1e0, copied to Ground.mLastGroundPos70 at52f4.
* Capture a FRESH Processed pointer52f8 before guard5300, then read that
  owner's8d0 bit15 at5308 -> hippy82e0532c. Do not reuse ground capture52e0.
* Fresh Processed.jump_events().strength() is mJumpStrength950 -> Ground5360.
* Fresh8c4 bit18 -> State wiping53a4.
* Fresh Processed.physics_state_fields().balance9a0 !=0 -> State manual53ec;
  NaN compares unequal and negative zero compares equal.

Independent raw review corrected misleading decompiler expression addresses.
All six brake/push reads precede guard sampling (including foot4fd0/4fd4,
right5078/507c and general brake50b0/50b4). Teleport51e8/51ec,
preparing5220/5224 and starting5258/525c also read before guard sampling.
Pinning51b0/51b4, hippy5300/5308 and wiping536c/5370 sample before flag reads;
assertions follow those reads. Collide and balance read before sample/assert.
Count, turn, ground vector and jump strength read after the output assertion.
No unknown source field defaults to zero; missing backend must return Source
error at the actual local query after six flag publications. Component source
callbacks are now bound in physical_player_update5_outer_locality.rs: actual
same-owner Processed/Reckoning, HiLOD count/tail, cvars and Actor player_id.

## Registration and required caller changes

In lib.rs: `mod skater_manager_local_identity;`.
In PhysOut construct module:
`#[path = "physical_player_phys_out_outer_locality.rs"]`
`pub(crate) mod outer_locality;`.

Retain LocalBackendOwner and SkaterManagerLocalIdentity in actual shared
startup/session ownership. Construct backend using actual shared GameInfo;
sample cvar191 only at native debug-menu initialization if that path is run;
execute try_reset_game_before_first_spawn at selected ResetGame phase. Mount
locality source using actual owner.actor_query identity and manager from the
same Simulation. Do not call reset lazily from Update5 or inject current Actor
ID into the manager. Validate foreign Actor/pair/GameInfo before execution.
Manager backend().shares_game_info(...) is available for this check.

Then consume update5_wipeout::Update5AfterWipeoutFill, call
try_publish_update5_outer_locality with actual source, preserve owner and
grind_policy into next token, then publish existing current type and root tail.
The new concrete session method validates pair, Actor allocation/query and
same World/GameInfo owner before entering the output kernel. Missing manager
is deliberately deferred to the real query after six flags. It does not
construct or reset manager lazily. This draft is not a completed live frame.

Focused authored tests: native -1->offline0, repeated reset refusal, enabled
ghost pending preserves-1; complete outer callback order/captured owner reuse,
NaN manual/negative-zero turn, late missing-backend prefix preservation, first
source read before locked-output failure. Rustfmt completed; tests unrun.

## Concrete session integration additions

Register in constraint_session:

```rust
#[path = "physical_player_update5_outer_locality.rs"]
pub(crate) mod update5_outer_locality;
```

Add a retained field to SimulationAssemblyConstraintSession, initialized None:

```rust
local_skater_identity: Option<Arc<crate::skater_manager_local_identity::SkaterManagerLocalIdentity>>,
```

Add to PhysicalWorldServices (read-only identity access, no new owner):

```rust
pub(crate) fn game_info(&self) -> &Arc<PhysicalGameInfo> { &self.game_info }
```

Add InputCvarReferences named getters after registering191/192:

```rust
pub(crate) fn try_enable_ghost_by_default(&self) -> Result<bool, InputCvarError> {
    self.try_current_boolean(191)
}
pub(crate) fn try_debug_render_local_sim_player(&self) -> Result<bool, InputCvarError> {
    self.try_current_boolean(192)
}
```

Root startup should retain backend and manager before creating first physical
Actor. At selected native static/SkaterManager constructor phases:

```rust
let backend = LocalBackendOwner::construct_static_fields(Arc::clone(&actual_game_info));
let manager = SkaterManagerLocalIdentity::construct_local_fields(backend);
```

If executing native debug-menu initialization, call
`manager.backend().try_apply_debug_menu_ghost_sample(cvars.try_enable_ghost_by_default()?)?`
at its source transition. At selected native ResetGame phase, before SpawnPlayer:
`manager.try_reset_game_before_first_spawn()?`; retain manager in session field.
Preserve the manager if any subsequent startup stage fails. This schedules an
actual selected native producer; do not fetch an Actor ID or receiver ID to
construct local identity. The actual live gameInfo owner must be the same Arc
later passed to physical World initialization, not an equivalent value copy.

Concrete continuation signature:

```rust
try_continue_update5_outer_locality(
    &mut self,
    board: &SimulationSkaterAssemblyHandle,
    skeleton: &SimulationSkaterAssemblyHandle,
    before: Update5AfterWipeoutFill,
    actor: &mut ActorConstruction,
) -> Result<Update5BeforeCurrentType, Update5OuterSessionError>
```

Next root continuation consumes returned owner/grind_policy to execute
existing type publication followed by new decisions5438..55e0. Native Actor
ID is fetched through existing PhysicalStateActorQuery implementation, so its
allocation/query binding checks precede reads and local/local-sim source
ordering stays in manager helper methods. No concrete source is still missing
for this span once the listed registration/startup bindings are supplied.
