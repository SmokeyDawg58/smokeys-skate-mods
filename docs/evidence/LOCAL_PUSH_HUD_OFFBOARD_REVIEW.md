# Owner review after public Update 18

The owner rejected the stationary mongo boost as too fast and requested the
normal push speed. The local target gain is reduced from 1.5 to 1.2, retaining
the original animation clocks and native force solver. This is a mod calibration
against the same scene's normal push, not a retail Skate 3 parity claim. Rolling
strokes, normal pushes and the original 8.5 m/s cap retain their existing path.

Clean and Sketchy retain their original lettering, colours and timelines.
Their original timeline placements differ by about 125 stage units. The new
layout removes each timeline's fixed placement, anchors both to `multiplier_mc`
at its upper right above the points strip, and bounds the projected lettering
inside an eight-pixel viewport margin. Scoring and grading are unchanged.

The owner's friend sees on-foot spawn bouncing on both clients. A silent
560-tick on-foot marker replay reproduces a 0.25 m root-height excursion and
3.19 m/s upward body velocity just after reset. S2 AdjustSpawnPosition825F1B8C
adds 0.15 m clearance and S3 reset82DB8A10 adds another 0.1 m. Grounded offboard
targets now use a short actual collision query and compensate that board-only
reset clearance before all bodies are placed. The same target path covers
constructor entries and later host teleports. A second check found a separate
0.21 m body rise after feet were grounded: the intermediate board-reset pose
seeded a crouched COM into the standing biped. Explicit offboard reset entry
now seeds its actual standing-pose COM and resets the physical bones once at
Biped Sync before enabling the existing drives. Ordinary dismounts do not
request that reset. Missing, distant or steep support
keeps ordinary placement; boarded reset heading and height stay unchanged.
This is an explicit PC bridge correction, not a change to the native kernels.

The supplied `4-skate3-rb-prop-menu-no-flicker.patch` is adapted to the current
player owner without removing its focus policy: zero-fraction prop contact is
accepted and successful interactables are retained for at most 30 missed ticks
while RB remains held. Release, a new successful query and host teleport reset
the cache. The original S2 solver path is unaffected.

Validation uses focused component/layout checks and finite silent executable
comparisons with copied saves. Results and executable hashes live in the local
review audit. No full workspace regression, public lobby play, exact retail
parity or automatic publication is claimed by this source note.

The first one-shot physical-pose reset did not eliminate the rise. Its log
recorded local hips at 0.6835 m: the graph was still playing the original
`BR_SPAWN_DISMOUNT_INTO_STAND_0` clip, whose `dismount.xml` SpawnDismount node
uses a 0.1-second blend and 0.5 playback speed. An explicit offboard reset now
plays that clip's original terminal standing pose immediately. Native expiry
then exits the node normally; ordinary dismounts and other clips are untouched.

The owner also reported turning locked while holding a manual. The actual
780-tick pad trace confirms GroundAnimation103 freezes rider heading at zero
change through both left and right manual inputs, though the deck reacts.
The existing animated-root rate binding was gated only on KickTurn flag28.
It now also admits nonzero manual balance, matching the native manual heading
predicate and using the same authored rate, damping and wrong-wheel scalar.
The previously working non-manual kickturn path is unchanged.

## Final local-preview follow-up

The owner requested background checks and a separate playable copy. The usual EXE
and its live saves are no longer installation targets. Every scripted game check
uses an inactive offscreen window, a windowed copied profile and an audio dump.
Revision4cc993f verified 121 on-foot return ticks: zero root/floor excursion,
body height range0.013163m versus0.19627m before, facing dot0.99999998.
Manual holds now turn both ways; the same input had zero root yaw before.
Mongo is4.904675m/s versus normal4.894490m/s, with identical normal control.
The owner then clarified the grade must be level with, directly right of the
multiplier. A temporary reserved slot separates it from the points strip.

### Air-gap scoring publication

Confidence: high for the missing publication, medium for the reported online
score symptom pending owner review. TU3 StartSkeletonLineTests82DB63C0 and
EndSkeletonLineTests82DB6580 already run a 100m hips trajectory; deck probe
82C07788 also runs through the source world. PhysicalPlayerPhysOut's source
observer published neither result. S2 Update5/FillBoardDeck is bypassed by
source stepping, leaving the host AirGroundData inputs invalid/zero. Thus grass,
stairs and height-relative gap/context rewards could not observe these facts.
Retain the submitted hips origin with its completed result and export both
actual source probes. Do not fabricate a floor or issue another query. The
collector continues using original embedded S3 curves, factors and shared
sequence/challenge/online facts. No independent display-only score is introduced.
Original TU3 Air trajectory82DA8FF8..91B8 and grab analysis82DA93D8..98C0 were
checked against the installed original code pages. Their curve offsets and
base/hold arithmetic agree with the shared collector; its ordinary takeoff,
repetition and Spot Battle multiplier rules are therefore retained. Original
tailwalk data remains140 base,0.2s delay and2.45 hold-rate scalar. This is not
a certification of complete retail Skate3 scoring or a guessed score boost.

### Supplied park-editor patch

The owner's patch5 adds the editor, local props, physical board copies and replay
recording. It is merged into the current source, preserving the existing RB
inside-prop/retention repair and current focus/teleport handling. Its additional
grind-segment test accessor is retained. Props remain local; this patch does not
add remote prop synchronization. Online rounds disallow/close the editor.
Compilation and focused checks are recorded with final executable hashes in
the local review folder. Public update18 stays held.
