# Held manual score protection (local preview)

Owner reports points falling while holding one manual. Confidence: high for
the reproduced meter-expiry path, medium for the reported live integration.
This is an explicit requested PC Skate 3 policy, not a retail-parity claim.

ScoreModule::Update82dfcddc..cee0 limits how long a Ground collector can keep
the line timer above zero. An announced manual can outlast that grace period.
The expiry resets the multiplier, reducing the live HUD preview. The source
runtime also zeros its preview on expiry. A full400-point line hid the problem
in a30-second manual test; a62-point line reproduces it at tick671, including
114.1538 ->0 at1.5x and228.3077 ->0 at3x.

Protect an active, announced manual from that ground timeout in both the host
ScoreModule and the Skate3 scoring runtime. Do not consume the finite idle-ground
grace counter during manual protection, so release can settle and bank the score
before ordinary expiry resumes. Ordinary Skate2 scoring, idle ground, bails,
runout, teleports and held-darkcatch rejection retain their independent paths.

The focused probe compiles the actual scoring_runtime.rs against its existing
dependencies and loads the original embedded ScoringData. At1x/1.5x/2x/3x, a
30-second low-meter manual never decreases and banks exactly its peak preview:
165.2887/247.9331/330.5774/495.8661. Subsequent idle frames expire normally and
restore1x. It is a deterministic accounting test, not a physical balance test.
The first attempted protection exposed a release-settlement gap; no such build
was installed. The final counter rule preserves the held reward through banking.

Private validation also records the actual game's two manual holds, with copied
saves and file-only audio. The owner's shared updater reinstalled public update9
during this work, so that capture's final EXE hash is the public9 hash; it must
not be represented as validation of local preview11 or the new candidate.
