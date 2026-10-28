# Retained pad cache and group scheduling

Scope: original executable/PDB inspected in IDA on 2026-09-08; read-only production audit. This extends input-pad-runtime-plan.md and connects the implemented input_pad.rs owner to its native consumer schedule. No Cargo was run. Confidence exact for raw addresses and ring/group operations below; high for decompiled constructor/lifetime composition. Timer creation and elapsed-time policy remain outside this packet.

## Construction and lifetime

InputManager::InputManager 826ddb98 initializes device-manager pointer null, then clears cache pointers (8 bytes), map pointers (4), pad pointers (0x20), input pointers (0x20), menu pointers (0x20), and vibration pointers (first 0x10). InitBoot constructs at82740da4, calls Init826e2788 at82740db8, and only then publishes AppMan+90 at82740dbc. A newly allocated storage owner is not an initialized input service.

InputManager::Init constructs one cInputDeviceMgr (0x344 bytes), initializes it, loads the actual config/input.cfg map, then constructs groups in numeric order0,1. Each group gets its own cPadCache (0x2ef4) and Init before cache-pointer publication, then four cInputPad owners (0x188), each followed by InputSK8 and menu-wrapper construction. Each InputSK8 retains that exact group's pad and the shared loaded map; its four global arguments initialize +0 and DPD-updated flag initializes1. Vibration owners are constructed afterward for actual manager device count. Do not share a single retained pad history between the two groups.

cInputDeviceMgr constructor8298cf28 clears the two-half input cache, atomically sets active index0, then sets count0. Init8298df20 calls cPadDevice::Construct8298d310 with capacity4, stores returned count, asserts count>0, clears eight active bytes and0x320 cache bytes, then atomically sets active index0. Construct selects min(maxpads,4), allocates an actual Xenon device for each index, and returns this slot count, not number of currently connected controllers. In this Init route all four slots exist even while disconnected; their samples may have NumButtons0. Do not initialize manager count from currently connected host devices. Partial allocation failure is not proven to admit a valid null device.

InputManager destructor826dfbd0 delegates Purge826de990. Purge destroys/clears vibration owners first, then each group's menu/input/pad owners and cache, then the shared map, then calls device-manager Purge and destroys/clears manager. The decompiler shows cache purge/delete inside the four-pad loop with the pointer nulled after its first iteration; do not infer four distinct caches. AppMan PurgeBoot calls destructor82733864 and clears+90 at82733874; PurgeApp also calls destructor82737d00 and clears+90 at82737d0c. These are application-lifetime input owners, not per-board or per-physics-frame objects.

## Ring lifecycle and observable collapse

cPadCache constructor8298cb10 clears all30x4 records (each0x64: count then24 floats), atomically initializes read0 then write0, retains the actual manager pointer, and initializes device count0. Init8298cba8 separately writes write1 (8298cbd0), then read0 (8298cbf0), asserts manager present, reads manager count+328, and stores cache count+2eec (8298cc24). Init does not clear ring bytes. The usable empty state is read0/write1; ctor0/0 must not be exposed as an initialized empty queue.

Cache read index is+2ee4, write+2ee8, device count+2eec, manager+2ef0. CacheInput8298cc40 asserts equal retained manager/cache device counts; copies min(count,4) records into the current write slot, then advances write modulo30. The writer independently samples ring write index and manager active half for each device copy. It does not check reader distance, reject overflow, or keep an additional occupancy counter.

NextInput8298ce90 computes next=(read+1)%30, reads write, returns false without a store if next==write, otherwise stores next as read and returns true. Flush8298cd88 reads write and sets read=(write+29)%30; it then rereads write and read and asserts (write-read+29)%30==0. Flush does not clear values or reset retained pad history.

With Init0/1 and no concurrent producer, one published record advances write to2 and NextInput selects slot1. Two records advance write to3 and LastInput drains through slots1,2 before updating pads once from slot2. Thirty unconsumed publications wrap write back to1 and appear empty to NextInput: this is native modulo ambiguity, not permission to introduce a host queue overflow policy. Ring bytes persist across consumption/flush.

LastInput starts826dfbd8 (earlier packet listed its internal calls). Null group cache returns false. First NextInput call826dfc04; on success set local advanced=true and repeatedly call826dfc1c until false. Only if advanced, iterate device indices0..3: NumButtons826dfc50, then separate ButtonValues826dfc60, then actual retained pad Update826dfc70. Both cache getters independently sample read index after signed device-range checks. Count is sampled before values; do not preflight all four pads or update a pad on an empty drain. A later collaborator failure must retain earlier drain/pad effects. All four pads update, including disconnected count0 records.

Therefore debounce/repeat counters in input_pad.rs advance once per successful LastInput group consumption, not once per hardware sample and not unconditionally once per simulation tick. Skipped press/release samples are genuinely collapsed. Simulation and presentation can have distinct histories over the same hardware publications.

## Producer and caller order

TimerCallback82729770 checks actual AppMan+90; PollHardwareForInput at82729798 comes first, then CacheInput group0 at the loop827297a0..b4, then group1. Group enum is kINPUT_SIM=0, kINPUT_PRES=1, count2 (raw compare827297b0). Device Poll8298cf98 writes inactive half, samples each actual device into24 channels, stores count/active flag, then publishes active index after all devices. This is distinct from updating retained pads.

AppMan::PreUpdate8272c478 first performs network Idle when app started, then calls LastInput with literal group0 at8272c4d0/d8. Missing manager or no advanced input returns false before texture/assets/Flow/vibration work. This is a scheduling gate, not merely a convenient polling point.

GameModule::Render827320f0, when not exiting, performs earlier asset/message work and samples debug-menu status, then calls LastInput with literal group1 at827321c4/cc. It drains even if debug menu is active; only the later presentation Input call is suppressed by debug menu or no advancement. Do not condition the PRES drain itself on menu inactivity.

AppMan::Flush827296c0 flushes groups0 then1 if manager exists. FlushInput82729730 forwards its selected group. GameInputManager::Update826e2578 is a separate recognizer/event stage; SimUpdate1 invokes it at8289f4b8 when skater update enabled. It does not replace LastInput. Current audit does not recover base timer creation, timer frequency, thread wakeup policy, or the outer loop's exact handling of PreUpdate false. Choosing a generic host poll rate would exceed this evidence. A portable deterministic scheduler should initially accept explicit poll/consume events whose provenance is stated.

## Minimum implementation proposal

Retain one actual device manager with four constructor-backed device slots and two published sample halves; retain two distinct initialized caches bound to its unique occurrence; retain four existing InputPad objects per group. Preserve constructor versus Init state. Expose explicit PollHardware, CacheInput(group), NextInput/Flush and LastInput(group) stages. LastInput calls existing InputPad::try_update in source device order; input_map.rs already implements InputMapPadValues for that actual retained pad. Reuse it without copying mapped results or maintaining a competing button history.

A serialized portable implementation can reproduce a specified event trace, but must explicitly narrow concurrency: native writer samples manager half separately per device, and count/value reads are separate. Do not claim a mutex-atomic four-device snapshot reproduces every concurrent interleaving. No input-manager/cache production owner currently exists in the Rust tree; input_pad.rs and input_map.rs cover converter/history and selected map execution only.

Required discriminators: uninitialized0/0 refusal versus initialized0/1 empty; one/two/30 publication ring cases; flush preserving pad history; SIM/PRES independent drains; collapsed brief press never reaching Pad.Update; no-advance leaves transition/repeat counters untouched; disconnected0 count still updates selected pad; fresh reconnection triggers existing growth clear; late device failure retains earlier pad writes; foreign manager occurrence refusal. Native timer/Actor scheduling is a later integration requirement, not a hardcoded host interval in this owner.

Fresh executable SHA-256 anchors (end exclusive):

- Cache ctor8298cb10..8298cba8: e8f2f4dc3e856b3ca293de7e3a7b585aee6ba3a58b32e0af305576200369548a
- Cache Init8298cba8..8298cc3c: 60632a4f1a8b23298af9a2aa58a9e30bced45ba09b804022b6bcdca090121d2e
- Flush8298cd88..8298ce90: d8fceef547c5202933184d7394d0e1b71c5d30ad2ca6f3e0370cf39cbf18b5ba
- Device ctor8298cf28..8298cf94: f69bc32b500bae4262e83a98b062843e80e94350d66500214b04afcafadecf29
- Device Init8298df20..8298dfb0: d254abb1e165ae5b415e3fe97e037ad4bf611923a738a91ca279c2d1116abe5f
- LastInput826dfbd8..826dfc90: dfff1088a23b66cdb8f5f1dae0a092d9eac9af49adfceb980434fdbfea8a5219
- Manager Init826e2788..826e2abc: 4ace5a7608edf06e4416a3b220aa8e803a37a04eb1664075f7ab8347e81dbaec
