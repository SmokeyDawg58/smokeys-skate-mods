# StoreLandingData

September 19, independent review accepted high. Lifecycle mounting underway;
runtime gates pending. No native per-tick or numeric bit parity claim.

Behaviour32 on OnBoard3, Update82ca1d00. Vtable823004e8 slots+2c/+30/+34/+38
are Create82ca1f78, BLR825187c0, Update82ca1d00, BLR825187c0. Merged Create
symbol is DisableTricks; actual vtable bytes establish this use. Create allocates
0x10 with alignment0x10, initializes float+8=0 and byte+c=false, returns NULL on
allocation failure. Neither Begin nor End resets retained history.

Update resolves cached IPhysical and ISkaterMotionGraph, checks each UCOM
identity. Filtered PhysOut+40 state2 is Air. First Air transition calls Motion
virtual+28 with zero, then stores was_air at82ca1ed4. Two independent GetPhysOut
calls read SystemReckoning (outer+24), mCentreOfMassVelocity+10 followed by
mUpVector+50. Debug UDT PhysOut_SystemReckoning confirms these offsets; they are
not board velocity nor a synthesized world-up vector. vmsum3fp82ca1f20 reduces
XYZ only. Raw40980028 at1f30 and4098001c at1f3c skip on !LT, including unordered.
Thus only current<0 AND current<previous publishes abs(current). This is not a
historical maximum: prior dot is stored at1f58 every successful call, even NaN.
Publication is not restricted to Air. Motion setter SetLastGoodLandingVelocity
825fdfb0 writes the retained last_good_landing_velocity field. Independent review
confirmed concrete ISkaterMotionGraph vtable82069688+28=825fdfb0, stfs+0xc54
relative to that interface, matching full-class PDB field+16cc.

The adapter uses actual retained physical and Motion owners and independent
component-only system reads. Host reduction uses the existing host_dot3 policy;
Xenon reduction rounding, denormals and NaN payloads remain an explicit numeric
boundary. Missing native allocations become typed host errors, not fabricated
default values. Tests exercise successive falling samples, grounded publication,
unordered previous/current and an Up-load failure after early reset/history.
