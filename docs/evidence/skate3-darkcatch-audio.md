# Friend Test 4 catch sound restored for Skate 3 physics

## User reference and verified source

The user rejected the replacement sound and provided
Skate2-Skate3-Physics-Friend-Test-4.zip as the known working sound reference.
Its game EXE SHA-256 is
`ebc891f3c5a013c253620f80cb08a9600f564b55ac5f70493f5ea312a0a1f401`.
The package includes Source/Modified-Source.zip, allowing source comparison
rather than inferring the desired sound from a newer console asset.

Test 4 loads `SK8_Foot_Catch.bnk` from the original game's
`data/audio/splicerskateboard.big` into bank 16. Its `PlayFootCatch`
(0x8251C6C8) selects samples through aud_splicer/footcatch collection
F1647BFB782BE97F, field 19FAE35258317C7C. The supplied corpus has these ranges:

| Contact | Soft | Medium | Hard |
| --- | --- | --- | --- |
| First foot (kind 2) | 0..2 | 3..8 | 9..12 |
| Second foot (kind 0) | 13..15 | 16..21 | 22..25 |
| Both feet (kind 1) | 26 | 27..28 | 29..30 |

Values come from the runtime Vault, not hardcoded runtime cue ranges. The
medium/hard thresholds are 2 and 5 in this corpus. The original minimum
off-deck interval is 290 ms, which can exclude shorter inverted catches.
The user's bank has 31 splice records, one container and 46 samples; SHA-256
`91c86bf64bb3a5222378765ad1b59e18cdedec17836795343a5ab1c59c609b3d`.
Original assets remain external to source control and are supplied by the game.

## Focused PC mod bridge

Restore Test 4's bank and first/second/both-foot selector. A confirmed airborne,
non-bail, named darkcatch with the board inverted against the hips can admit
a short or signed returning-foot contact. Use speed magnitude for this
confirmed branch, then the original runtime Vault strength thresholds and cue
selector. A per-foot latch tolerates late scoring confirmation, with one cue
per returning foot until release or landing. Ordinary catches retain the
original timing, positive-speed gate and mixer policy.

Confirmed mod catches use the authored splice/sample gain and pitch, controlled
by the existing SFX slider directly. This is an explicit PC audio integration
policy, not a claim that the entire Test 4 mix or a console AudioSystem is
reconstructed exactly. The helper admits nonzero initial gain without changing
native callers' Play/Update ordering. Test 4's own authored sample delay remains.

The separate original Skate 3 alternate bank tried earlier was rejected by the
user. Its slot, embedded bank input and loading path are removed from this build.
It is not a substitute for the user's provided Test 4 sound reference.

## Validation and limitations

`test4_catch_bank_decodes_and_honors_sfx_mute` loads the original bank and
checks first/second/both-foot cues at all three strengths through the game's
exact stereo fold, with finite nonzero output and exact silence at zero SFX.
Some Test 4 cues start after the opening 16 ms by design; do not impose the
discarded bank's transient timing on them.

A full AudioSystem reproduction with original game banks at 30% SFX selected
bank 16 cues 1 and 17 and produced stereo peak 0.4341724. The earlier replacement
produced a different sound and remains a failed user validation, regardless of
its nonzero samples or passing decoder test.

Shared build checks require bank 16 cue requests at actual confirmed inverted
contacts, Test 4 bank decode/mute validation, ordinary-flip policy, actual marker
walking, the original-data map reset regression and existing physics features.
User validation of the restored Test 4 build is pending.

SKATE2_AUDIO_DUMP records instead of opening a speaker stream. Audible gameplay
checks omit it. The user's current game output is a virtual eight-channel
96 kHz device; test playback must use the game's CPAL path or user-controlled
playback, rather than assuming a different application's audio route is audible.
