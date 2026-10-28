# Local preview follow-up: marker response, facing and stock frontend timing

The owner rejected preview8 as still slow and reported that off-board marker
returns face backwards. Changes remain local; public update5 is held. Only
short, copied-save gameplay diagnostics are authorized.

## Frontend clock — high confidence

The normal official v0.2.7 EXE (SHA256 recorded in the private provenance file)
and preview8 ran the same 600-tick main-menu route with copied profiles. Stock
took18.282 seconds including boot; preview8 took21.734. More decisively, stock
maintained about60 frontend ticks/second, while preview8 fell to40 despite
about120 rendered frames/second and sub3ms tick cost. No host-clock debt clamp
was involved. Preview6 introduced an unconditional accumulator/previous reset
on every Frontend outcome. At120 FPS this discarded each fractional timer slot.

Remove that reset within a frontend mode. Retain actual pause/focus boundary
resets and the bounded offline-stall policy. Restore stock fixed1/60 replay
updates as well, removing the later wall-time workaround which masked the
discarded slots. Authored menu camera curves remain unchanged.

## Off-board checkpoint facing — high confidence diagnosis

The actual marker-place/recall recording showed root yaw+0.3740047 at placement
and-2.7675876 after return: exactly180 degrees. The saved Actor transform still
had+0.3740047 yaw. RestoreFootForward changed the incoming biped mirror bit
(Processed2476 bit2), which locomotion then applied to the reset root. The board
switch compensation alone did not account for this off-board mirror.

Before the existing board-switch compensation in ResetPlayer82DB8998, cancel
the current biped mirror for an off-board reset only. Use the incoming reset
packet's actual stance rather than the former walking stance. On-board reset
and position/clearance policy are unchanged. This is an explicit PC bridge
repair; final executable must preserve heading in the reported sequence.

## Animated-root turning — medium confidence PC integration

Preview8 integrated normalized KickTurnSteering Spin as radians/second. The
authored control response it omitted is physics_manual/SpinVsSpeed,300 degrees
at rest, ManualSpinBlendSpeed0.09 and SpinNoLiftScalar0.5. Original and embedded
data agree byte-for-byte. Toolkit_CalcHeadingTorque82D92CF8 converts this curve
with native word3C8EFA35, blends rate history, and applies the wrong-wheel scalar.

Use those authored response values for the PC kickturn root binding. Retain
the clip's native signed Spin and kickturn flag. Other GroundAnimation takeoffs
keep their retained heading. This is not a full recovery of generic Reckoning
and is not claimed as retail parity. Validation must measure meaningful angular
response in both directions and physical-board agreement, not just nonzero yaw.

## Tailwalk values — high confidence data match, live result pending

The installed original Skate3 data and embedded pack both define tailgrab_airwalk
(id172, class2, score type4): base140, completion delay0.2s, flags1, rate scalar2.45.
Tailgrab is base80 with rate scalar2. Hold bonuses use the original cumulative
grab-time curve, followed by repetition, air metrics, stance, landing quality
and combo rules. These values do not establish a flat final score for tailwalk.
Do not inflate its point table merely from uncertainty; verify recognition and
the actual banked result through the final EXE against the recovered scorer.

Original executable/data remain external. Opt-in probes are finite, inactive
during normal play and use copied saves. No hosting or monitoring is started.

## Reset-stage follow-up

The first final-EXE off-board check still returned180 degrees backwards. The
compensation read Processed2476 before Skeleton::ProcessData populated its new
mirror bit. Use the current Actor animation packet10370 (mirrored) at that reset
stage instead. The packet is already published after GivenStance; the processed
mirror field is only written later. This is why an apparently correct mirror
test at the wrong lifecycle stage did not change the observed result.

The on-board response check did pass with solved board deltas-1.2004/+1.7991
radians, compared with preview8's-0.3114/+0.3687 over the same windows. Both
observations remained in GroundAnimation103. These results are not reused as
validation of the next version's executable.

The 600-tick menu check of build 200fc50e measured 59.943 Hz versus stock 59.996 Hz (preview8: 39.861 Hz). Replay sampling must rewind by parameter, rather than a fixed trigger count, to avoid mistaking the end of the clip for slow playback. This diagnostic correction awaits the v0.2.8 private build.
