# Selected ordinary PlayerUI sample

This is the ordinary payload slice of `Sk8::PlayerUI::Update828af9f8`, admitted by
an explicit caller after simulation/focus/session-marker/network-teleport work.
It does not reconstruct those preceding stages or subsequent spawner/reset and
proxy Tick. No production PauseMan or proxy frame producer is fabricated.

Fresh IDA source: `Simulation::IsNetworkGame` at 828afc1c precedes controller
sampling; `mAllowSensor = !network` at fc3c and `GetRawInputCacheLine` at fc48 use
the actual supplied GameInfo and retained device manager. The entire current
0x64 cacheline is copied, independently of SIM/PRES group history. Unfocused
input clears exactly 24 float words at fc54..6c without changing NumButtons.
Payload focus starts true. `PauseMan::IsActive(kPAUSE_FLAG_SIM=1)` is sampled at
fc70..78 only on the unfocused branch. Its true branch queries the actual proxy
frame at fc88..98; raw `cmplwi` at fc9c proves unsigned comparison to 1. Only a
larger frame clears payload focus. Player index is sampled at fca8 and receiver
is its wrapping word plus 10 at fcc0; existing command type1 follows. The leaf
uses the supplied actual PlayerLoaderSlot identity, not player0 or a new UI owner.
Controller identity is separately supplied; out-of-range host access remains a
typed error at the cache read, with no speculative pause/proxy access.

The new sample borrows directly into existing GameInputFrameRequest, retaining
the existing codec, command manager, CRC and recognizer owners. Update mask/CRC
admission remain explicit caller inputs. The API does not acknowledge native
proxy DoCommand/Tick or claim the broader session schedule is complete.

Focused tests cover actual published device sampling after real reset/poll,
nonzero loader identity, native wrapping receiver arithmetic, all24 positive-zero
stores with retained count, pause/proxy laziness and failures, and unsigned frame
boundaries including 0xffffffff. Test collaborators are isolated test fixtures;
there is no default or unconditional-success implementation in production.

Fresh callee audit: `PauseMan::IsActive82603b30` locks its actual mutex and
returns `(mMask & mFlags & flag) == 0` at 82603b68. Therefore the collaborator is
named `try_is_simulation_active`, not a paused predicate. Constructor82738d68
sets flags0/mask-1 and then calls Push(INIT); neither its resulting state nor
proxy generation may be supplied as an unconditional host default.

Author performed rustfmt and patch apply-check only. Root owns Cargo and original
data execution. Independent source/code review is required before integration.

Registered validation: fmt and strict clippy-motion268-player-sample-1 pass;
workspace 4,639 unit plus six doc tests pass. Original-data 18/20 (5.00s),
bail frame 2 now PendingExpression269; neutral frame 2 BlendSpace Eval.
