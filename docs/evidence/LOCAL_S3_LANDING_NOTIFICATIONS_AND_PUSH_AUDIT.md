# Original S3 landing notifier and held-X push investigation

The owner requested the original Clean and Sketchy notifier and clarified that
regular-stance mongo is X, held (the earlier mention of A was corrected). Keep
all changes local; no score, sound gain or saved volume changes are authorized.

The existing TU3 landing-quality owner (82DE61D0), deferred ScoreHolder landing
publication and original clean/sketchy score factors already grade S3 landings.
A local optional PhysicsFrame LandingGrade now carries only a successful,
completed S3 air landing. Clean comes from its retained original clean result;
Sketchy uses its threshold-qualified sketchy result. Bails, cancelled catches,
ordinary ungraded type3 and independent S2 physics produce no notifier.

The display slice is extracted from original data/fe/source/screens/hud2/
trickdisplay2 (APT SHA57370fe8901e92c7d624ecac0c3ec8dcb25741d0b94eb00d07fadb61829c9c22):
character17 Clean label10..49 and character16 Sketchy label10..54. Preserve
all original per-frame glyph transforms, colour transforms and the original
Futura Std Medium font pixels/metrics. Crop only the ten required atlas glyphs;
no substitute font or generated lettering. Score tinting is outside this slice.
The external build input lives beside the existing player pack as
skate3-landing-notifier.bin and is embedded by the app build script. Original
assets stay external to the repository. The normal APT movie clock advances it.
The badge attaches to the existing score clip so projection/HUD hiding follow S2.
No new online HUD bits are emitted: existing clients reject unknown flag bits.
This optional local notification is not reconstructed from remote/replay HUD
records; networking, challenge scoring and the existing recording format remain
unchanged. Current original S3 animation grades are high-confidence; UI extraction
is exact for the selected lettering layers, with visual integration pending.

Three silent short comparisons on preview14 covered A and X holds/taps, then X
held across an onboard marker return. Both buttons accelerated normally in
those runs; the intermittent weak X push is not yet reproduced or fixed. Do not
claim a speed/strength correction based solely on these healthy runs. The native
push-force path (82D948B8), authored regular/mongo clip coefficients (82BAD260),
input requests and native PushCycle remain unchanged. Scoped in-game journal
observations now retain actual target/signed/absolute speeds, push limits, force
branch and the native push-cycle state during pushes. These extend the existing
journal, not a separate PC watcher or persistent task. Original PPC pages used
for the comparison, baseline logs and asset provenance remain in the local audit.
