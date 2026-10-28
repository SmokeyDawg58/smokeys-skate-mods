# Native pad sampling, conversion and retained history

2026-09-08. Read-only original XEX/PDB and repo audit. No Cargo
or implementation. This is the producer below the selected original input.cfg
map evaluator, not a replacement for mapping/gestures/Actor scheduling.

## Platform boundary and the exact 24 channels

cXenonPadDevice::SampleButtons8298d0d0 calls XInputGetState(device.mIndex).
Failure stores mIsActive=false and returns0 buttons; it does not produce a
fabricated24-zero sample. Success checks desired vs set vibration frequencies,
stores the submitted frequencies and invokes XInputSetState when changed,
then calls MapStateToButtons82989448 with the actual XINPUT_STATE by value.
The decompiler's MapStateToButtons parameters are badly allocated: use raw
argument stack+4 wButtons,+6 leftTrigger,+7 rightTrigger,+8/+a/+c/+e signed
LX/LY/RX/RY. Do not trust its apparent pointer/64-bit axis expressions.

The portability boundary should accept a semantic raw device packet containing
connection/sample status, u16 XInput button bits, two u8 triggers, four i16
thumb channels, and source device/remap identity. Windows/Linux/Android/Web
polling adapters translate hardware to those raw channel semantics OUTSIDE the
simulation. Do not apply OS/generic deadzones first, normalize asymmetrically,
flip Y, or quantize preprocessed floats again inside map evaluation. Device
API errors/disconnection remain distinct from a real connected neutral packet.
Platform timestamp/poll scheduling is separate from the numerical converter.

MapStateToButtons requires outputcapacity>=24 before any stores and returns24.
Digital values are literal+0/+1. Channels are pad IDs0..23, NOT map IDs74..91:

| Channels | Raw source | Meaning |
|---|---|---|
|0,1,2,3|wButtons bits0,1,2,3|Dpad up/down/left/right|
|4,5|bits4,5|Start/Back|
|6,7|bits6,7|Left/Right thumb click|
|8,9|bits8,9|Left/Right shoulder|
|10,11|u8 left/right trigger|analog trigger values|
|12,13,14,15|bits12,13,14,15|A/B/X/Y|
|16,17|positive/negative corrected LX|left horizontal halves|
|18,19|positive/negative corrected LY|left vertical halves|
|20,21|positive/negative corrected RX|right horizontal halves|
|22,23|positive/negative corrected RY|right vertical halves|

Bits10/11 are not digital outputs. Trigger integer conversion fcfid/frsp is
exact foru8, then ordinary rounded multiply by3b808081 (1/255 rounded), stores
10 at895fc,11 at89604. There is no trigger threshold/deadzone here. The
subsequent pad held-state threshold is separate from Value.

For EACH stick, sign-extend raw i16, convert to f32 then multiply38000000
(1/32768). This makes -32768 exactly-1 and32767 less than+1 before correction.
Squared magnitude is **rounded x*x then y.mul_add(y,xSquared)**, raw896e0/e8
and897e4/f8. Do not replace with two rounded squares/add or generic dot order.
Estimate896f8(left)/89808(right), two native reciprocal-square-root refinements,
product squared*inverse, exact-zero select to+0, produce length. Reuse shared
length_from_squared_and_estimate ONLY after matching this scalar FMA square
producer and the original estimate boundary. Even squared0 requests estimate.

Let length=L. Raw branch8973c/8984c BO4 BI24 tests LT-not-set: only ordered
L<3a83126f chooses scale+0. Otherwise compute rounded (L-3e800000)*3fb6db6e.
Clamp the resulting radius by branches: ordered<0 assigns+0; ordered>1 assigns1;
unordered survives both branches. Divide correctedRadius/L using fdivs, then
multiply each original normalized axis by scale. This is radial innerdeadzone
.25 and outer saturation at approximately .95, using actual literal bits;
NOT standard XInput deadzones or an independent per-axis deadzone. NaN branch
details matter at numeric capability boundaries even though raw finite packet
integers ordinarily cannot inject NaN. No extra sanitization is present.

All split channels use source fneg/fsel graphs: positiveHalf=fsel(-axis,+0,axis),
negativeHalf=fsel(axis,+0,-axis). Ordered zeros select literal+0 in either half;
unordered takes the value arm. Left source stores are18 at897cc,16 at897d8,
17 at897ec,19 at897f4; right stores20 at898a4,22 at898ac,23 at898b4,21 at898bc.
Right raw conversion begins BETWEEN left stores; preserve source completion
prefix when numeric/port calls can fail. Full digital/trigger outputs precede
left math; completed left channels precede right magnitude estimate.

After all24 channels, GuitarHero flag+0d true overwrites10,11,21,22 with+0 at
898cc/d0/d4/d8. Actual ExternalPadRemap+4 nonnull then virtual UpdateGamePad
898fc gets argument0 and mutable complete values. Do not omit a present remap
or silently claim absent from lack of an implementation. An ordinary-device
scope may require actual provenfalse guitar/provennull remap ownership.

Actual original constants, resolved signed lfs addresses and raw words:

|Use|Address|Bits|
|trigger scale|820859c8|3b808081|
|signed axis scale|821eeb40|38000000|
|small length|8200fd1c|3a83126f|
|radial scale|821eeb3c|3fb6db6e|
|radial deadzone|820103c8|3e800000|

## cInputPad retained state is a distinct producer

Constructor82986398 establishes only mNumButtons=0 (plus vtable); it does NOT
initialize all24 button records. Update829863b0 asserts incoming count<=24,
count>=0, oldcount<=24, oldcount>=0 in that order. On growth only, MemFill32
zeroes the newly exposed16-byte button records at8646c before countstore86470.
Shrinking does not reset hidden records; count0 visits no records. Reconnect
from0 to24 consequently zeroes all newly exposed records. No disconnect release
events are generated by a zero-count update.

Record at pad+8+16*i: Value+0(float),Pressed+4(byte),Released+5,Held+6,
Repeat+7,RepeatCounter+8(i32),transition gate counter+c(i32). Per record:

1. Sample supplied float and store Value864a0 FIRST. Compute ordered value>.5
   (864a4/a8), so .5 and NaN are false. Values are never clamped here.
2. If gate counter<3, clear Pressed then Released, incrementcounter, preserve
   Held regardless of new analog value. Negative retained counters also take
   this signed comparison path; do not change to unsigned.
3. Else compare candidateHeld to retainedHeld. Equal clears Pressed/Released,
   leaves gatecounter unchanged. Different stores newHeld864f0; stores
   Pressed/Released in source order (true:1,0;false:0,1), then gatecounter0
   at86510. This is a minimum inter-transition suppression gate, NOT three
   consecutive stable samples and NOT debounce on Value. A freshly grown
   button recognizes its first held transition on its fourth Update call.
4. Repeat logic reads retainedHeld after that stage. If not held, counter0
   then Repeat=false. Held with RepeatCounter0 sets Repeat=true then counter24.
   Held with nonzero counter decrements/stores (wrapping raw32 semantics); if
   now0 sets Repeat=true then counter12, otherwise Repeat=false. Initial
   recognized press repeats immediately, then after24 updates, then every12.
   Held can remain true during transition suppression while Value already0.

All records update ascending. Value/Pressed/Released/Held/Repeat are different
map evaluator services. GetButtonValue826df2a8 returns+0 for an ACTUALLY
constructed pad withcount0, otherwise performs signed rangechecks and reads
Value. Other accessors similarly consume actual retained flags; do not compute
Pressed/Released afresh from arbitrary current/previous raw arrays. This gate
and repeat count advances on pad Update calls, not elapsedseconds or game
physics dt. No generic gameplay edge detector can substitute for it.

## Poll/cache/gameplay ownership and timing

TimerCallback82729770: if actual InputManager exists, PollHardwareForInput
826e0470, then CacheInput for every input group. Lower cInputDeviceMgr poll
8298cf98 samples the INACTIVE half of its two-cache array, one actual device
at a time. Each CacheLine contains NumButtons +24 floats (0x64 bytes). It
stores count then per-deviceactive=(count!=0), and atomically publishes the
new active index only after all sampled devices. This is native concurrency
ownership, not permission to expose half-written packets to map readers.

cPadCache::CacheInput8298cc40 requires actual devicecount equals cachecount,
copies up to4 devices'0x64 records from the published manager half into its
current ring write slot, then advances writeposition modulo30. It samples
manageractiveindex separately for each copied device; a portable single-producer
schedule may model the serialized equivalent, but must state its concurrency
boundary. Do not add guessed overflow/backpressure semantics: native code
shown advances modulo30 without a new reader-distance check.

NextInput8298ce90 computes next=(read+1)%30, returnsfalse ifnext==writepos,
otherwise commits readpos and returnstrue. InputManager::LastInput (containing
826dfc04/1c calls) drains NextInput to the last available record; only if it
advanced does it invoke cInputPad::Update for each of4 group pads (calldfc70).
Thus retained flags do NOT necessarily update once for every hardware sample;
collapsed samples may be skipped. LastInput callers include AppMan.PreUpdate
8272c4d8 and GameModule.Render827321cc, for their selected input groups.
Full group scheduling/cache initialization still requires its own caller audit
before claiming exact poll-to-physics chronology or choosing a host poll rate.

Simulation::SimUpdate1 calls GameInputManager::Update826e2578 at8289f4b8 when
mEnableSkaterUpdate is true, before later animation/physics work. GameInputManager
first requires actual DebugOverlay singleton, iterates retained playerinput
begin/end, skips null pad/count0 or flags&updateMask==0, then calls
UpdateRecognizer followed by GenerateGameInputEvents for each selected player;
finally flushes its actual broadcaster queue. It does not itself poll hardware
or update all pad history records. Existing evidence identifies right thenleft
recognizer sampling and Y negations as a LATER layer; do not move those signs
into the raw pad conversion.

Actor.GetInput returns the selected InputMan.inputs[playerID].mInput (unless
actual AI branch returnsnull). This input points to its retained pad/map and
four current globals. ActionGraph listener/controller history and generated
gestures are downstream actual owners, not cPadCache or Skeleton body history.

## Existing Rust and minimum next implementation

controller_input.rs supplies exact gameplay map IDs and RawControllerInput
field/arbitration behavior. It explicitly starts at completed map expressions,
and its previous state is caller supplied; no pad converter, cache, retained
cInputPad history or live InputManager owner exists there. Contact agent owns
selected original input.cfg evaluator; reuse its pad-access trait and feed it
this real retained pad owner, not mapped snapshots or direct platform axes.

Recommended bounded next owner: actual initialized padcount plus24 dormant
records, with source growth initialization and in-place Update above; a device
converter over raw typed XInput-equivalent samples and shared numeric seeds;
and a source-backed manager/cache lifecycle controlling when updates publish.
Separate converter result from retained pad values/edges. Platform raw samples
enter only the converter; no keyboard-to-force or shortcut around mapping.
Actual joystick device selection/remap/guitar flags and inputgroup lifetime
must be explicit. Preserve source cvars/config evaluation in the map layer.

Tests: all button bit mappings and trigger0/255, signedaxis extrema, radial
near.25/.95/.001 and diagonal saturation, FMA discriminator, zero length still
estimate, literalpositivezero halves, distinct source store prefixes; guitar
and remap effects; construction count0, growth/shrink/regrowth, fourth-update
first transition, rapid release suppressed while Value changes, 24/12 repeats,
NaN Value versus heldfalse, no fabricated disconnect releases; ring wrap and
LastInput collapsing; actual mapped74/75/78 sees these retained channels. Original
input.cfg should be the integration corpus. None of this alone completes Actor
history/gestures or proves reference timing without deterministic traces.

Fresh raw hashes:
MapState82989448..89924 7099f846d24309780bc4b4c741bce1bf1188fb75e0523902d78991deaef07004
PadUpdate829863b0..86570 d5af5c71b4e0de99c3fd9ee6d4a9fd9e8fe243ea7c0b50dafad35c806a20d8c8
DevicePoll8298cf98..d088 7d6dd0d38c3be59e7fd5b85cd9a27bb232028520c1a2d82adde24c7d20008b3c
SampleButtons8298d0d0..d174 e299c2f36ac201893dda3f0f0c56da08d6c6533031526eb1293bb1fc42f8b59a
GameUpdate826e2578..2644 bf958e5ea7949bcfd4926818d1a036bd32acaf11607166c0382f957b914f2428
CacheInput8298cc40..cd88 cb7653f1c554e70fc5caec9fdc580123c44e17f282230823a7cd27dc18ebe4e6
NextInput8298ce90..cf28 b2922cd3398ace6b8066b188c91dc4c10ca6a06512ee3afdfe69615b6fa7b200
Confidence exact for listed conversion/update raw graphs, high for cache/caller
reconstruction; full cache initialization/group/host timing remains explicit.
