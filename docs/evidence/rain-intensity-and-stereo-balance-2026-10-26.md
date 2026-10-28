# Local rain tiers and channel balance

PC presentation tuning requested by the owner; local review only.
Confidence: exact for measured source PCM imbalance, high for the mixer correction.

Weather now cycles Clear, Light Rain, Medium Rain, Heavy Rain, Light Snow,
Heavy Snow. Persistence uses string keys, so existing preferences keep their
meaning even though the private in-process enum index changes. Medium Rain uses
a 0.65 rain amount, interpolated motion/film parameters and 24 drops/1000m3.
Light retains 441 drops. Medium draws 2393. Heavy density changes from 10.4 to
62.4 drops/1000m3 in the same volume: 9265 drops versus 1544 previously, about
six times as many. The existing 12000 cap, shelter/depth rejection, streak shape,
particle speed and Heavy wet-surface look remain. Snow behavior is unchanged.

The embedded stereo PCM beds measured +1.227dB right bias (Light) and +5.413dB
(Heavy). On decoding each bed once, accumulate per-channel f64 energy and
attenuate whichever channel is louder to the quieter channel's RMS. No sample
or channel is boosted. This preserves stereo detail and original PCM files;
it does not change speaker balance, master volume or any game volume setting.
Medium crossfades both existing beds. Rain Volume still scales only the rain
voice, under the existing SFX level. Pause/visibility/mute fading is retained.
This addresses the supplied rain bed's bias; no blanket claim about unrelated
positional game sounds or the owner's headphone hardware is made.

Focused checks cover menu cycling/persistence, decoded and mixed channel RMS,
no-amplification/mute behavior, actual rain counts, and silent map captures.
