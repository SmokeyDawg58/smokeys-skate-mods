# Actual Update5 prerequisites for filtered Ground

Confidence high from fresh raw/decompile and current retained-owner inspection.
Read-only source packet and proposed root patch; no runtime edit or Cargo run.

The registered Physics conditioner prefix is not yet callable as a successful
Ground producer after the current corpus. PhysicalPlayerPhysOut::State resets
physics_state/category to0. The retained physical state owner has actually
selected PhysicsGround100, but this is a different destination: Update5 has not
copied its type to PhysOut. Calling the conditioner now mutates damped velocity
and empirical histories before returning InvalidPhysicsState(0). Retrying after
repair would double-advance those histories and is not valid integration.

## Real dependency chain

Physics.Update82e2f970 first calls UpdateSystemVelocity82e25a40,
UpdateEmpiricalMeasurements82e25830, UpdateFilteredPhysicsState82e2d800.
The existing registered implementation correctly uses the SAME bound output
occurrence and retains the Physics child histories. Its inputs are:

| Input | Actual producer / current status |
|---|---|
| System COM velocity | Skeleton.Fill prefix already publishes actual post_physics_records COM velocity |
| SkateboardReckoning deck position | Complete Board.Fill already publishes typed XYZ/unknown-W |
| Physics timestep | Update5 processed+93c -> output Physics+0 at82e05798; currently only constructor value |
| State current type | Actual current virtual GetType82e05400 -> State+10 at5434; not yet mounted |
| State category | Fresh GetType55f4 then GetPhysicsCategory55fc -> State+c at56b8; not yet mounted |
| Collision contact/physics type | Actual Board.Fill publications already mounted |
| Ground wallRideExit | Late current-state.Fill82e05b74 -> PhysicsGround.Fill82d8ddf0, store82d8df90; missing |

For actual type100 the conditioner chooses Air when wallRideExit is true, Ground
otherwise. It is therefore wrong to use GroundRemainder's constructor false as
the live value. Type/category require two distinct current-state reads, not a
single snapshot and not a conversion from previously processed input category.
For type100 and category100 the collision physics type is still queried for
the stairs history even though this frame's Ground result is direct.

## Proposed next owned batch

Add a new PhysOut construct child for actual guarded Update5 state publications
and a new constraint-session adapter that binds the SAME retained owner/state.
Source-store sites must remain separate: current type5434, category56b8,
HiLOD update count56f4, offboard flag5724, offboard-ground flag5764, timestep5798.
The current state handle must retain and validate its existing board/skeleton
bindings; return an explicit unsupported-state error for an unimplemented native
state rather than treating every enum as Ground. GetPhysicsCategory must be
source-decoded for admitted state types. No filtered output store belongs here.

Before mounting the existing conditioner, execute the real selected current
state.Fill callback at5b74. For the actual PhysicsGround owner, all its fields
already exist in physical_player_physics_state.rs. Reuse the existing retained
PushAttributes global physics_push.default capability (class
d4d04d2fa87a9a31, collectiond7edbd362d7d2152, global+b0), not a second tuning owner.
Fresh raw PhysicsGround.Fill82d8ddf0..e0e4 has this exact sequence:

1. Guard then read Ground+f1 is_push_accelerating -> Motion+112 atde40.
2. Require/capture global+b0 SPEED_LIMIT keya9b532ebbed16f38 atde84, then
   separately require/capture key1e76e849db99f955 atdedc. NULL alone selects
   DefaultDataArea4. Load SECOND pointerdef0, then FIRSTdef8; add second+first
   with faddsdf00. Read actual processed+948 abs body speed, ordered LT compare.
3. Guard/read Ground+f7 is_pinning -> Ground+122 atdf3c.
4. Guard -> Motion+114 captured comparison atdf64.
5. Guard/read Ground+f0 exiting_wallride -> Ground+120 atdf90.
6. Guard/copy all lanes Ground+90 -> outputGround+80 atdfc4.
7. Guard/read Ground+f3 anti_flip_nudge -> Ground+121 atdff0.
8. Guard/read Ground+bc time_to_skitch -> Ground+104 ate01c.
9. Guard/read Ground+c0 skitch_spline_height -> Ground+108 ate048.
10. Read processed+8cc low bit BEFORE guard -> Intents+7 ate07c.
11. Guard/read Ground+f9 grab_object -> OffBoard+38 ate0a8.
12. One guard then copy Ground grab_spline_type/object -> State+1c/+20
    ate0d4/e0dc (two ordered words, not two source assertions).

This is a small concrete missing state producer using existing fields, existing
Vault pointer capture/load, and existing typed output components. Its output
error must preserve the push store and prior stores on a later tuning/guard
failure. Fresh PDB UDT inspection confirms Ground+90 is mAntiFlipTorque,
rw::math::vpu::Vector3, matching existing anti_flip_torque:Option<[f32;4]>.
Read the actual retained value at its use; None remains a missing producer.

## Scheduling limits

Current Skeleton.Fill stops before angular suffix, so these would initially be
explicit selected later Update5 publications, not a claim that Update5 returned.
Between Skeleton and these sites native calls Pumping.Fill, GrindManager.Fill,
TrajectorySelector.Fill, Reckoning.Fill, StateChanger.Fill, HandPlant.Fill,
FootPlant.Fill, SkateboardController.Fill and Wipeout.Fill. Grinds remain parked
explicitly. No implementation may hide these omissions in a completion token.

After real state/timestep/State.Fill publication, an explicit selected Physics
prefix wrapper may borrow owner.conditioners and the SAME owner.phys_out once,
call try_update_physics_prefix, and return its BeforeDampedCom token. Never
reconstruct Conditioners, move out/reset histories, or retry a failed prefix.
Native broader schedule is UpdateConditioners82dff154 -> UpdateAll82e29f88:
Intents, Grinds, MissionSystem, Audio, Camera, Physics, Animation, Scoring, Stats,
PathRecording, GameCam. A selected Physics prefix does not implement those
other callbacks or full-frame publication. Parent must independently review
which earlier conditioner writes affect the selected inputs before promoting
this to an ordinary scheduled frame.

Validation proposal: source-discriminating late-failure tests for Ground.Fill,
same-owner foreign binding rejection, then actual original single successful
selected publication/conditioner call before codec-fed input replay. Expect the
actual resulting state rather than storing Ground or overwriting wallRideExit.

## Unregistered Ground.Fill draft

physical_player_phys_out_ground_state_fill.rs implements all twelve stores and
source reads above. physical_player_ground_state_fill.rs binds the SAME selected
Ground state's board/skeleton/Weak owner identity, retains its actual fields
through the callback, reads the actual processed Arc lazily and accepts the
existing optional PushAttributes capability. Missing attributes fail only after
the initial push publication; missing anti_flip_torque fails after wallRideExit.
The processed world-grab source is a BYTE at8cc: its low bit corresponds to
0x01000000 in the retained big-endian flags word, not word bit0.

Four unregistered tests discriminate late vector failure, second attribute
capture failure before either pointer load, entry guard before any owner read,
and full-W/two-word descriptor publication. Files were formatted, not compiled
or executed. Proposed registrations are construct::ground_state_fill and a
constraint-session child for physical_player_ground_state_fill.rs. Root owns
the remaining Update5 publication/conditioner scheduling integration.

## Unregistered state publication spans

physical_player_phys_out_update5_state.rs and
physical_player_update5_state_publication.rs now draft the two separate source
spans. First53f0..5434 reads actual current GetType before its guard/store.
Second55e4..5798 reads current type again, calls exact category range helper,
queries Collision.wheels only for200<=type<300, publishes Air.airborne, State
category, actual HiLOD prefix.update_count, offboard flags, and finally processed
DT after the Physics guard. The second span does not overwrite the first type
store even if a different source type is returned between spans.

Fresh GetPhysicsCategory82d83b80 requires unsigned input<=702, then descending
thresholds700,600,500,400,300,200,100; below100 gives0. Four tests cover distinct
type reads, source ranges/negative assertion, late DT failure prefix, and type
capture before a locked-output failure. The same-owner adapter currently admits
only the actual retained PhysicsGround instance and its real bindings; other
current-state implementations remain explicit. No conditioned-state writes or
conditioner invocation were added. Files formatted; no compilation or tests run.

Root independently compared complete Ground.Fill82d8ddf0..e0e4 raw against drafted output kernel: twelve guard/publication sites, second-before-first attr loads, orderedLT speed, byte8cc bit0 beforeguard and single guarded twoword descriptor match. Actual antiFlip producer and typed adapter integration remain pending.

Root fresh raw review of both selected Update5 spans53f0..5438/55e4..579c agrees with drafted reads/guards/stores, including offboardground500or504 and countHiLOD134. Category raw3b94/3b9c uses signed >-1 and<703 checks (equivalent accepted range0..702); unsigned wording should be corrected at registration.
