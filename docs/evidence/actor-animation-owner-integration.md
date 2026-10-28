# Actor animation owner integration, 2026-09-11

Work in progress, not a playable checkpoint. The active resumption preserves
the previous dirty workspace and extends the real Actor constructor instead
of treating independently constructed physics fixtures as a complete Actor.

`actor_animation_mount.rs` is a child of the existing Actor owner. It validates
the actual shared `Arc<Mutex<PhysicalAssemblyWorld>>` manager identity before
source effects, retains an external-reset allocation with unknown contents,
and samples local-player, network, ghost, marquee and CAC selection in source
order. The same animation allocation registers its three interface occurrences
in the existing Actor registry. Publication of Actor+1fc is rejected until the
animation constructor actually returns. Animation Initialize, body playback,
hands, AnimOutPhysIn and physical creation are subsequent unfinished stages.

Source: Actor constructor `825fe850`, selected continuation `825feccc..825feefc`;
`Actor::IsLocalPlayer` `825f0c60`. Root inspected raw instructions, and a separate
An animation agent approved the selected caller ordering. Actual
command/playback/database managers are now captured in that order before the
constructor's interface effects (`825fee04..825feec0`). Separate failure tests
retain the allocated, unattempted animation without publishing interfaces or
Actor+1fc. The independent reviewer approved those failure tests as well.

Original-data corpus now retains that Actor and the same shared world around
the existing physical factory. It reads `characters_marquee` class
`91bca6693efc9aa7`, selected collection `2275b90050166969`, cached text offset0
through the real relocation graph and expects `tutorial_male`. Backend answers
and scheduling remain explicitly controlled diagnostic inputs. The physical
factory has not yet become an Actor constructor stage.

Confidence: high for reviewed selected source ordering; incomplete for whole
Actor startup. Workspace gate55119 passed 3,345 core tests and245 other unit
tests, plus six compile-fail docs. Original-data gate56582 passed all14 tests;
strict Clippy gate99527 and format/diff checks passed. These precede the next
Actor Initialize/job and generated Action-load batch. The cleanup fixture
explicitly begins its next frame after velocity writes invalidate the old one.

See `actor-face-playback.md` for the actual archive, RAW extraction, loaded
manager and fixed-frame ownership route. The selected corpus now runs actual
ordered Simulation body database and fixed hand setup before Actor face
construction, using the same managers; face occupies slot4 after those owners.
Thread identity and backend scheduling remain controlled corpus inputs.
