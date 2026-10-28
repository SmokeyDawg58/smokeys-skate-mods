# SettingBodyTilt retained callback draft

Unregistered: actor_motion_setting_body_tilt.rs, motion_setting_body_tilt.rs,
motion_setting_body_tilt_tests.rs. No Cargo run, no controller/state mounting.

Fresh original Create82caf1e0 allocates0x14, name unknown, alignment0x10.
Allocation failure returns null. Success establishes reference ownership and
angle+8=+0, delta+C=+0, applying byte+10=false. Begin/End825187c0 are BLR.
The generated runtime must retain an instance identity independently of these
three payload fields and pass None on allocation failure, not invent a payload.

Update82c9e288 resolves IPhysical, IAnimatable, ISkaterAnim,
ISkaterMotionGraph in that order. Cached physical/skater interfaces assert at
their respective acquisition; the final required-interface assertion follows
all four. Motion virtual44 is GetApplyingBodyTilt825fdfd0: graph+171c high bit.
Only then is instance dereferenced. False-to-true resets both floats, and the
new byte is stored before any active-path physical reads. False stops without
resetting float history or publishing a settable attribute.

Active order: capture PhysOut.Animation pointer (outer+38) at e56c;
ISkaterAnim virtual1c at e574; read Animation.anim_tilt X at e58c/e5a0;
mirror true keeps X, false XORs the sign bit. Fresh PhysOut.Air pointer+8,
body_spin_speed+bc is read at e5d8 and fabs applied. This is NOT System speed.
Global pointer is captured before that scalar read, validated after it; host
capability validates at the later graph call, with no mutable shared pointer.
Global.anim_motion_body_tilt is pointer+148, cached layout+4, graph+560;
Evaluate4 receives X at570/Y580. Multiply graph result by signed tilt X.

Fresh PhysOut.Filtered pointer+40, first word==2 selects air clamps. Separate
Global.Get calls obtain acceleration then velocity clamps. Accessors82c8abf8,
82c8ac40,82c8ac88,82c8acd0 use attribute IDs550960064,574906395,
2471991737,690307421 respectively and actual DefaultDataArea4 on absence.
The draft tuning capability is intentionally NOT a replacement global owner;
actual anim_motion_body_tilt construction and live cvar targets are still needed.

Raw e69c..e6ec first limits(target-angle) by velocity, then limits(result-delta)
by acceleration. Each limit uses two ordered fsubs/fsel instructions, preserving
unordered and negative-bound behavior (not Rust clamp). Store delta+C first,
then angle+8. No DT is sampled. At e6f0 read kUseTilt; false publishes+0 and
skips kSetTilt/kTiltX. True reads kSetTilt; only true reads kTiltX. State writes
survive later missing targets or publication failure. e750 calls IAnimatable
AddSettableAtt(name,value,false,-1) using the previously captured allocation.

The adapter captures actual physical/animation/Motion owners at resolution and
releases locks between callbacks. Read methods use Animation/Air/Filtered, all
confirmed from PDB outer pointer layout. Only X is consumed; unknown W is not
promoted. The tuning trait must eventually bind the shared Global collection
and late live cvars; no eager values or fake defaults have been inserted.

Focused draft tests cover inactive retained history/null instance boundary,
rising-edge stores preceding failed output acquisition, state retained across
late cvar failure, continued integration with use=false, and fsel NaN/negative
bound/signed-zero behavior. They await root integration and execution.

Proposed registration: child setting_body_tilt under actor_motion_runtime_session;
its kernel is public for runtime instance storage. Add SettingBodyTilt instance
variant carrying identity plus TiltInstance; allocation request0x14/alignment10;
Begin/End no-op, Update delegates with actual payload.tilt_x and a genuine late
tuning capability. Do not register a fake tuning backend or change graph state.

Next live cvar binding source: initializers82f85a70/82f85ac0/82f85b10,
table slots82fd4ea0/4ea4/4ea8, all section Animation_Skater_BodyTilt.
kUseTilt title Use Body Tilt defaults true; kSetTilt title Hard Code Tilt
defaults false; kTiltX title TiltX defaults0 with GUI range-1..1.
The float initializer's decompiler call argument reconstruction is suspect;
verify its raw callback register before registering it. No slots are reserved
or shared registry edits made by this draft.
