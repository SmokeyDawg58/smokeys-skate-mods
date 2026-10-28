# Local rain continuity preview32,2026-10-26

Owner reports falling rain repeatedly stopping/restarting. Prior rain dt came
from native WorldAnimationClock samples. That recovered native clock deliberately
holds time on repeated restored frames and suppresses large jumps/rewinds;
PC rain should not inherit those pauses. This is a source-supported failure path,
not a captured trace of every owner stall.

PC-authored cosmetic PresentationClock uses monotonic elapsed render time for
rain ribbons, gusts and their existing rain-surface/cloud/lightning presentation.
No native world, physics, scoring, replay or game-audio clock changes. Suspended
rendering resumes with a short local step; it does not catch up a long interval.
AfterRain bird mixer, audio gains, native controller settings and the accepted
water material are unchanged. Larger contact-effect and grazing-angle revisions
are deferred because the owner requested this continuity preview first.

Focused regression replays repeated/frozen/rewound native samples through the
production ribbon generator and requires motion on every120Hz present. Also
checks bounded resume behavior, existing frame-rate/cover/roof tests, production
GPU1x/4x and a short silent actual-map diagnostic. This is an authored PC fix,
not recovered retail weather parity. Full workspace suite is outside the owner's
short-diagnostic scope. No public publication.
