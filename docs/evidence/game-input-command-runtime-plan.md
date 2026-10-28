# Gameplay command routing and codec runtime plan

Read-only source/PDB audit, 2026-09-08. No implementation/Cargo. Exact confidence for raw callsites/schema below; high for decompiled lifecycle and offline orchestration. Existing game_input_player.rs intentionally begins AFTER successful typed Unpack and does not implement this pipeline.

## Actual owner and registration

Simulation::Workspace::InitWorkspace828a7d30 constructs CommandManager at828a7e64, publishes workspace+1c at828a7e74, then constructs GameInputManager828a7e88 and publishes workspace+20 at828a7e98. These correspond to Simulation's retained command/input services (GameInputSK8 reads command manager through Simulation+c0). The command manager is workspace-lifetime, preceding gameplay receiver construction, not a board/pair-local event bus. ShutWorkspace828a7008 calls its destructor at828a71b8. Root must retain one actual selected manager and bind receivers to that same occurrence.

CommandManager ctor825e58d0 constructs an empty ordered receiver map (native fixed-map inline capacity59 with overflow allocation enabled), then SyncLog(SL_High,SS_World,GetCRC). Reset825e5490 only clears SyncLog; it does NOT unregister receivers. RegisterReceiver825e5a28 calls receiver.GetReceiverID, finds that key and asserts absent, then calls GetReceiverID AGAIN for insertion and stores the actual receiver pointer. Duplicate ID cannot silently replace an old receiver. GameInputSK8 GetReceiverID826df330 returns stored index+10 with native32-bit addition. Registration call in its constructor is826df294.

UnregisterReceiver825e5710 calls GetReceiverID, asserts key exists, then erases by key. Native does not separately compare supplied pointer to retained pointer; stronger host occurrence checks must be labelled authority invariants, not claimed additional source behavior. GameInputSK8 destructor826df3d0 unregisters its receiver through actual manager (follow teardown before manager destruction). No automatic host Drop delivery/registration repair is justified.

DoCommand825e5610 optionally logs CRC BEFORE lookup: when addToCRC and actual syncLevel>=SL_Med, feed receiverID, type, dataSize, then CRC(data,dataSize) in order. Find receiverID; missing key does NOTHING (source-backed miss, not an assertion). Found null receiver asserts; found actual receiver invokes its OnCommand synchronously. No queue is owned by this manager. A selected implementation should retain exact receiver occurrence and ordered keyed dispatch, with explicit optional sync-log collaborator; do not fabricate checksum values.

Repository search found no production CommandManager, SimController command queue, or offline transport runtime. New GameInputReceiverRegistration is a port awaiting this owner. Generic collection queues and animation command services are different domains and cannot substitute.

## Offline proxy and batch schedule

Actual normal proxy implementation is SimController::DoCommand828a6470. It appends a copied Command to retained mCommands, preserving call order; inline capacity10 allows overflow allocation. It does not immediately dispatch a gameplay receiver. SimController ctor828a4bd8 retains Simulation, null network, NT_None, nextGenFrame1; constructs empty ordinary/ghost/execution-ghost command vectors, validGhostfalse, actual stopwatches, null streamer, pauseTick0, and registers its teleport handler on Simulation broadcaster. A configured offline network is a later prerequisite, not constructor-default success.

OfflineNetwork ctor82793618 constructs empty pending commands and validfalse. SendCommands827a0ef0 refuses when already valid, otherwise copies the supplied batch in order (self-alias avoids copy), sets validtrue and returns true. Its frame argument is unused. GetCommands827a0f70 refuses when !valid; otherwise copies pending batch into supplied destination, clears valid and returns true. Requested frame and visualUpdate are unused in this offline leaf. It does not clear retained batch bytes on retrieval. OnReset8278e0f0 clears valid then vector end. This is one pending batch, not an arbitrary FIFO or a host timestamp queue.

GenerateSyncCommand828a89a0 asserts actual network, refuses when validGhost already true or network SendCommands(nextGenFrame,mCommands) refuses. On success it increments nextGenFrame, clears ordinary commands, copies ghost commands to execution-ghost commands, clears ghost commands, then sets validGhosttrue. Even a constructed empty ghost collection participates in this source lifecycle; omit neither the flag nor the empty batch semantics merely because the session is offline.

SimController::Update1828a72c8 starts timers, asserts network, clears current update mask, samples actual pause state, dispatches Simulation messages, updates network, stores latency, calls NIS update, and in NT_SP resamples pause state. Only active path continues through install/streamer effects and command execution. A valid ghost batch executes in order through manager DoCommand(addToCRC=false), clears execution batch and valid flag, and contributes Visual update flag. Then network.GetCommands(sim.tick+1,localBatch,visualFlag) may contribute Simulation flag. For EACH ordinary command it calls actual Simulation.GetRandom before manager DoCommand(addToCRC=true); do not omit RNG advancement as irrelevant to input. Only a nonzero resulting mask sets current mask and calls Simulation::SimUpdate1. This is the delivery-before-recognizer ordering: native GameInputManager.Update is inside later SimUpdate1.

GameModule::DoGameUpdate82730418 is the generation caller (GenerateSyncCommand at8273061c/58). It checks game state, actual controller/network and input-update flags, calls network Input with SIM-group input, performs UI/session-marker/front-end/pause effects, calls GameUI.Update with actual canSimulate/focus, then conditionally submits sync commands. In the selected input-update branch canSimulate depends on not-paused AND previous mSentSyncCommand. Do not replace this with an unconditional poll->deliver per host frame.

Minimal honest next manager closure: constructor-backed same-workspace receiver map plus typed registration and direct DoCommand dispatch can complete the new receiver port. A selected offline batch owner can then preserve append->send/refuse->get/consume ordering, but native full Update1 requires actual pause/NIS/streamer/RNG/sync collaborators. No no-op callbacks or generic clock defaults should claim the full scheduler.

## PlayerInputData command schema

Native in-memory payload: cache-line count at0,24 float channels at4..63, allowSensor byte64, hasFocus byte65. Neither cache count nor allowSensor is encoded by this Pack/Unpack. Command::SetData<PlayerInputData>828aa690 writes type1 first, resets ByteStream, attaches command data with capacity255, calls Pack, stores stream position to dataSize, asserts no write error, then resets stream. GetData826df340 asserts type1, resets/attaches to actual data/dataSize, calls Unpack, then resets. It does not verify final position equals size or add a trailing-byte rejection.

Pack82a8c070 first tests hasFocus. False writes ZERO bytes. True builds eight compressed fields, then a sparse byte stream. IMPORTANT: eight compressed fields does not mean fixed8-byte wire payload; focused encoding has1..9 bytes.

Compressed field order: LX,LY,RX,RY,LT,RT,B1,B2. B1 bits0..7 correspond channels12,13,14,15,6,7,8,9. B2 bits0..5 correspond channels0..5; remaining bits0. Each bit uses ordered channel>0; raw bgt paths at82a8c0ac/c4/e4 etc ensure NaN is false (decompiler <= branches incorrectly suggest otherwise). Signedzero is false. No0.5 threshold here.

Axis graph: rounded fsubs (16-17), (18-19), (20-21), (22-23), then rounded multiplies by127 (bits42fe0000). Triggers10/11 are rounded multiplications by255 (437f0000). Actual arithmetic schedule: four axis differencesc260/c270/c280/c28c; LT productc2a4, LXc2a8, RTc2ac, LYc2b0,RXc2b4,RYc2b8. Conversion schedule LT fctidz c2bc, LX fctiwz c2c4, RT fctidz c2d0, LY fctiwz c2d4,RXc2e4,RYc2f0. Extract low byte of each integer result; store compressed LX/LY/RX/RY/LT/RT atc304..318; WriteInputc31c. There is no saturating byte clamp. Valid source device ranges give signed axes[-127,127], unsigned triggers[0,255]; source remap can alter this, so arbitrary nonfinite/out-of-integer-range conversion needs an explicit PPC numeric policy or typed refusal. Rust saturating float-as-integer casts are not automatically exact. Do not hide this limitation with clamp.

GetWriteMask82a8b808 sets bit i iff compressed byte i !=0. WriteInput82a8b8b8 writes mask first, then only flagged fields in the above order. Thus focused all-zero input encodes [00], unfocused encodes [] and they are observably DIFFERENT. Each source write is separate; failure does not roll back earlier bytes. ReadInput82a8ba08 zeros eight compressed fields and mask, reads mask, then flagged fields in same order; absent fields remain zero.

Unpack82a8c330 writes hasFocus=(stream.size!=0) FIRST atc35c. Empty payload clears all24 channels ascendingc380..38c then writes Button13=1 atc398. It does not write cache count/allowSensor. This is not a generic disconnected-zero state. In GameInputSK8's local payload, count/allowSensor were initialized0 before GetData, so they remain0; receiver later calls Pad.Update with literal24.

Nonempty Unpack reads compressed bytes before any channel stores, then writes digital B1 channels12..15,6..9 followed by B2 channels0..5 as exact0/1. Triggers10 then11 convert unsigned bytes to float and multiply by bits3b808081. Each signed axis byte is sign-extended; positive half is byte*bits3c010204 only when byte>0, else+0; negative half is byte*bitsbc010204 only when byte<0, else+0. Stores proceed16,17,18,19,20,21,22,23. A wire -128 therefore decodes magnitude128/127, without clamp, even though normal producer emits no -128. Preserve that valid byte-pattern decode. Original signs are those reconstructed halves; recognizer Y negation remains later.

ByteStream Reset82a6e0c0 sets data=null,size0,pos0,errorfalse. Attach82a6e4a0 asserts nonnull pointer then retains data,size,pos0,errorfalse. ReadBytes82a6e0d8 asserts data and nonzero size; insufficient remaining length asserts, sets errortrue, returns0 without copying or advancing. WriteBytes82a6e1a0 same initial validity assertion; insufficient room sets errortrue/returns0 without copying/advancing (no capacity assertion in that branch). Typed fail-stop host errors should preserve completed source prefixes; do not call malformed stream behavior a recoverable zero-fill parser. Native callers may continue after DebugBreak, whereas checked host implementation can explicitly stop at that assertion. Empty payload is handled before ReadInput, so it is legitimate despite ReadBytes requiring nonzero size.

## Implementation recommendation and tests

First implement codec as staged semantic operations over actual fields and shared explicit integer-conversion policy; retain destination writes on decoding failures. Then exercise Pack->Unpack->registered SAME GameInputPlayer, not a direct originalfloat?typeddecoded shortcut. Quantization is essential to gameplay feel. Add schema cases [] versus[00], all eight mask bits/order, all digital bits/NaN predicate, one omitted field, -128 decode, signedzero axis halves,127/255 rounding/truncation boundaries, malformed-short read prefix, ignored count/allowSensor, and focusgain bounce interaction. A no-focus [] after prior bounce unlock can produce Button13=1 at the receiver; do not sanitize it away.

Receiver manager tests should distinguish duplicate registration, Reset preserving registered owner, source missing-ID miss, manager/receiver occurrence provenance, and sync-log-before-delivery failure prefix. Offline batch tests: valid empty batch, refused second send retains first, get consumes validity once, generation increments/clears ONLY after successful send, command order and per-command RNG before later GameInputManager.Update. These are prerequisites to native Actor scheduling, not evidence that omitted services are already implemented.

Fresh SHA-256 anchors (end exclusive):

- Manager ctor825e58d0..825e5918: 4f4c9948ff8ff7421a76c700af0a81ff4438154c1d3511a3da868bf2c168772c
- Register825e5a28..825e5ae8: a8d0027a29e7f89332694062af5210eed678d67ae0b5d31c1951642ef88ce6b3
- DoCommand825e5610..825e570c: 480e9463262daeee8f96b28c9e9ce32c6f5ebdd41bec476d429f922169f50624
- OfflineSend827a0ef0..827a0f70: 47726ccfcf1d9252f2b713aa610f6658d0db6180bec6e69a81d56b45e88994c8
- OfflineGet827a0f70..827a0ff0: e36c51384731123de549074386648fbd262e6ac9b36bbc225db63d8a12c7d2a6
- GenerateSync828a89a0..828a8a70: 6bcd230ed1b40754a796a2781213a3e61df62555329dcea41367d89952caf163
- Mask82a8b808..82a8b8b4: 5de11f60cd8da13f170433183c67eca51d47349ce9e6bb49e17f2a26d1881c19
- WriteInput82a8b8b8..82a8ba08: 83014621c88cf6ae8bdd56bd7653cd92ccfb97e7fd7a265b20dd9891c6698ced
- ReadInput82a8ba08..82a8bb54: 4a581361f78734e5b0642f85cd7750bcd1b1130e78079f7589dbcdf76181f014
- Pack82a8c070..82a8c330: 8dea3ff7838ab7cb398bd22e88385a472e51b7d38f3c1c07aa4a861945d61670
- Unpack82a8c330..82a8c734: d9fe42ae9033bb6cc2cbdc9c3a6145a65b7502a8dcd25b8ea4fbcfc7b6b31f01
