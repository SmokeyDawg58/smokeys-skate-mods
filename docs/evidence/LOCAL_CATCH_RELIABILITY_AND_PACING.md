# Local catch reliability and stall pacing

Owner reports that spawning is fixed, darkcatch audio plays intermittently, and
the game sometimes feels accelerated. Sound pitch is normal. Keep these changes
local. Preserve all volume settings, original recordings, cue gains and pitches.

The PC audio bridge rejected a named catch as soon as the skateboard ceased to
be airborne. That loses a confirmation first published at touchdown. The audio
contact owner now admits that one landing edge once, retaining its existing
airborne and played history to reject stale ground descriptors. A second catch
within one jump can retain the same scoring name. Re-arm it only after at least
two off-deck audio observations followed by a physically inverted foot contact;
ordinary upright reattachment and single-observation chatter do not re-arm.
Original Skate 3 PlayFootCatch824B9268 / cue lookup824C0B90 recordings and mix
remain unchanged. Confidence: medium PC integration, pending owner feedback.

Seven focused contact tests cover late confirmation, repeated airborne catches,
retained descriptors, ordinary flips and contact chatter. These operate on the
exact contact-state implementation without rendering or speaker output.

The existing steady online/travel check of preview11 measured median59.9935 Hz,
so a global speed multiplier has not been demonstrated. The host clock did,
however, preserve unlimited wall-time debt online, including asset/window stalls.
That can run overdue local physics faster after a stall. This port transmits
skater state rather than the original lockstep pad stream (docs/online/README.md),
so use the existing four-tick admission bound online as well as offline. Preserve
online simulation while unfocused, the ordinary60 Hz timer, fixed physics dt,
authored frontend camera curves and replay timing. This targets the catch-up
burst; it is not evidence that every reported acceleration cause is resolved.

Validation includes silent copied-save clock captures, a bounded suspension of
only the diagnostic child process, original-bank/mute checks and recorded catch
input. Full gameplay coverage and public-server testing remain outside the
owner-authorized short checks. Results belong to the private release validation.

Further recorded-input evidence: preview11 emitted cue15 at audio frame199
while both feet were off deck and board-up was+0.8009. The actual inverted
returning foot appeared at204, board-up-0.6320, followed by both feet at206.
The early fallback had already marked both feet heard, suppressing that real
contact. Defer this early confirmation to the physical inverted foot edge.
Retain an inverted edge seen before its scoring label; use the one-shot landing
fallback only if no catch was heard. The seven final focused contact tests cover
this ordering, held catches, repeated catches and stale ground labels. Original
cue selection and audio gain remain unchanged. No volume boost is applied.

Final recorded-input check caught the audio bridge's Hips-axis test delaying
the second contact until frame229, after release. AudioPresentation's HipsMatY
is not the physical world-up reference needed here. Use the actual board-up Y
from the physical presentation, together with the native darkcatch confirmation
and returning-foot edge. The recorded board is inverted at205/207 and upright
again at229/231. The candidate with the delayed cue was never installed. Repeat
the final recorded-input checks before signing; original gains remain unchanged.
