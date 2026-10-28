# Post-Skeleton output continuation

2026-09-13 source/caller audit and bounded unregistered implementation.
Confidence high; independent reviewer and Cargo/corpus gates pending.

Fresh IDA disassembly of `?FillPhysOut@Pumping@Physics@Sk8@@QBAXPAUPhysOut@23@@Z`
82dd4348..447c confirms the existing pumping output draft exactly: guarded +0
Skeleton+200; Pumping+38,+3c,+40,+44 to Skeleton+204,+208,+210,+214, then Pumping+30
to Ground+fc. Five source floats are copied without arithmetic or normalization.
Lock sampling precedes each source read except deck absorption441c, which
precedes its sample4420. Guard checking follows source loading for those five.
The existing owner child captures the same Pumping Arc once, loads fields lazily,
and retains an initial-zero prefix if the actual Pumping owner is unconstructed.

Fresh `?Update5_ProcessOutput@PhysicalPlayerHiLOD@Physics@Sk8@@UAAXXZ` decompile
82e04ea8 confirms complete Skeleton.Fill call4f18 immediately precedes Pumping
call4f24. New `physical_player_update5_pumping_session.rs` consumes the actual
Update5BeforePumping token, validates its owner against the supplied pair in the
actual world, executes the existing owner callback, and returns
Update5BeforeGrindManagerFill. There is no reset, replacement Pumping, or output
conditioner call. Foreign pair rejection precedes output mutation.

Root registration instructions:

1. In physical_player_phys_out_construct.rs add path
   physical_player_phys_out_pumping_fill.rs as pub(crate) mod pumping_fill.
2. In physical_player_pumping.rs add path physical_player_pumping_fill.rs as
   pub(crate) mod fill.
3. In simulation_assembly_constraint_session.rs add path
   physical_player_update5_pumping_session.rs as pub(crate) mod update5_pumping.
4. At the solved same-Actor corpus's successful
   try_continue_update5_skeleton_suffix return, pass that actual token to
   world.try_continue_update5_pumping(board, skeleton, before_pumping).
   Preserve/read its returned owner and explicitly report the next parked
   GrindManager.Fill boundary. Do not promote the token to Update5 completion.

Existing three focused tests preserve late-error prefixes and locked entry.
New uniquely named physical_player_phys_out_pumping_session_tests.rs adds full
source order and bit-preserving publication of negative zero and a quiet NaN.
No Cargo or original-data test was run by this implementation author.

## Manager audit beyond assigned implementation

Fresh IDA decompiles establish these subsequent call sites and producer writes:

| Update5 call | Function | Output |
| --- | --- | --- |
|82e04f30|GrindManager.Fill82dd3970|GrindJumper.Fill then hang-primitive flag; parked, not a no-op|
|82e04f3c|TrajectorySelector.Fill82db64d8|Air targeting grind/wallride flags|
|82e04f48|Reckoning.Fill82dd4150|System up/facing, signed dynamic lean, Air spin/flip flags, Animation vector, Ground transform|
|82e04f54|StateChanger.Fill82dd3bc8|revert exited normally; request-teleport true store only when requested|
|82e04f60|HandPlant.Fill82db8580|animation handplant vector/time/individual flag bits, found/grabbing/surface|
|82e04f6c|FootPlant.Fill82db6890|only when actual results-valid, timings/plant flags/surface/left-right foot flags|
|82e04f78|Controller.Fill82dc8370|holding, yaw via actual Spine3 world transform/effective transform, pitch+0, hiding/dropping/retrieving flags|
|82e04f84|Wipeout.Fill82dd46b0|only request6; landing speed and special state using prior FootPlant outputs|

The Controller yaw math is unconditional even for the ordinary attached state;
it cannot be skipped by labeling all outputs offboard. Its VMX128 decompile
registers require raw decoding before implementation. Wipeout explicitly reads
FootPlant publications, proving manager callbacks cannot be arbitrarily reordered.

## First concrete codec-to-state gap

game_input_loaded_action_corpus.rs retains real codec/matcher/listener input.
generated_action_runtime.rs:266 evaluates filtered state through
actor_action_motion_intent_bridge.rs:173, reading the same PhysOut owner.
There is no production caller of Conditioners::try_update_physics_prefix;
current uses occur only in its unit tests. Registered selected Update5 state
publications copy type/category/dt, but do not change Filtered state.

The bounded next state producer is existing unregistered
physical_player_ground_state_fill.rs::try_fill_current_ground_state plus
physical_player_phys_out_ground_state_fill.rs. Its late actual Ground+f0
wallride-exit publication is required before selected Physics.Update, even when
constructor output happens to be false. Reuse registered current-type and
category/dt spans and existing PushAttributes, then invoke the retained SAME
Conditioners owner once through try_update_physics_prefix. This produces a
selected PhysicsBeforeDampedCom token; it is not complete UpdateAll or a frame.
Do not attempt then retry on missing state: velocity and empirical histories
have already advanced before InvalidPhysicsState is returned.

Full scheduling additionally needs the intervening manager callbacks, remaining
Update5 writes/state.Fill/suffix, and earlier UpdateAll conditioner effects.
Native UpdateAll order is Intents, Grinds, Mission, Audio, Camera, Physics,
Animation, Scoring, Stats, PathRecording, GameCam. The filtered Ground producer
can support a clearly identified selected corpus closure first; it cannot
justify silently omitting those stages from the eventual first-test schedule.
