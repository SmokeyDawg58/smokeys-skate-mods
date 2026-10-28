# Pedestrian perception owner and event producer

Source-only implementation; no compilation, tests, or runtime execution.

The sole embedded owner is Pedestrian+2720 (`mPerception`), not ActorTracker.
PDB PedestrianPerception has owner+16, last audible+32, last spectate event+64,
eye position+112, eye direction+128, and parameters+144. Constructor82e6b2b0
initializes both events inactive with zero payload. Eye vectors remain absent
until Init; they are never substituted with bind-pose values.

Init82e73cd0 is called by Pedestrian ctor82e97df0, before animation construction.
GetTargetableCategoryKey82e6ad90 reads core+224 mEntityKey. The original category
lookup uses class60d703c77cab1631 and RefSpec a27862c1107f64ec. Missing parent
collection leaves parameters null; a present parent with missing referenced
perception collection asserts. Factory now follows this same initialization.

RecordAudible82e6b368 and RecordSpectateEvent82e6b398 copy the actual payload.
ClearLastAudible82e6b388 and ClearLastSpectateEvent82e6b3d0 clear only the active
flag. Update82e6b3e0 copies SimLODBoneGlobal[0] translation and second basis
vector into eye position/direction, using the actual retained pose.

LWPedestrianMan.UpdateAudibles82e838b8 iterates SkaterManager index order.
Wheel-landed rising edges compare current bytes68..71 with previous88..91;
the maximum positive impact is taken from Audio52..64. The loud flag is
wheel_slide>0.3, grinding, or State.is_wiping_out. Once set it persists across
actors, skipping later audible branches; scoring still runs for every actor.
Audible delivery scans the sole pedestrian hash roster, uses actual Actor
GetPosition (Skeleton root translation), and requires distance squared<=5625.
Quiet landing strength must exceed (distance*0.013333334)*0.4+0.1. Payload
position is actual SystemReckoning centre of mass, not that root translation.

The spectate suffix reads PhysOut_Scoring2 banked_point_change_this_frame+128,
just_wiped_out+17968, and wipeout_interrupted_trick+17978. A positive banked
change or either wipeout flag broadcasts to every pedestrian without a distance
or visibility gate. Later qualifying actors overwrite the previous event in
native order. No qualifying actor leaves the existing event unchanged.

`LivingWorldGrindServices::update_pedestrian_audibles(actors,numeric)` implements
that producer using original actor physics outputs and the sole typed roster.
Root owns scheduler placement. Perception CanDetect/MoodEvent geometry and
plugin Spectate leaf/controller integration remain incomplete; neither this
owner nor its stored events establishes playable NPC behavior alone.
