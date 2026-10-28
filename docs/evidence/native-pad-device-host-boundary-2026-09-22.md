# Native pad device host boundary, September 22

Read-only source audit; no new host sampler implemented. Existing input_pad.rs already owns raw Xbox button/trigger/stick conversion from MapStateToButtons82989448. Reuse it; do not add a second deadzone or mapping implementation. Actual application device sampling remains absent.

cXenonPadDevice is28bytes: ExternalPadRemap+4, index+8, active+c, guitar+d, two Desired/Set actuator pairs+e, vibration+16. Construct8298d310 allocates ascending min(maxPads,4), remap0/index/active0, Desired0/Set1 and vibration0. No explicit guitar-byte constructor store was observed.

SampleButtons8298d0d0 calls XInputGetState each sample. Failure writes active=false and returns0 without mapping/remap. Success publishes changed vibration Desired values to both Set fields before XInputSetState, then returns the24 mapped channels. Packet number does not deduplicate input. No active=true store was observed here.

SetActuator8298d178 bounds actuator<2 and queries XInputGetCapabilities(index,1) every invocation. Capability success plus Type1 gates vibration flag2. SubType7 controls guitar mapping. The decompiled failure path still reads SubType from capability output; its initialization/host failure semantics require further raw proof. Do not invent a false guitar default. Low/high scale/offset targets are lazy; zero frequency stores Desired0, otherwise source conversion/clamping produces a u16. ResetActuators82989308 calls SetActuator for each actual actuator.

InputVibration.Reset826e1590 clears actual event/channel owners and resets actuators. InputManager.Init826e2788 stores actual pad device at826e2a84 and calls reset at826e2a8c. InputVibration.Update826e1650 calls both actuator setters even without active events (or Reset when disabled), refreshing capabilities. XInputGetCapabilities82c54f28 has the native SetActuator code callsite8298d1cc.

The host adapter must preserve actual device/remap/vibration ownership and SIM/PRES cache separation. Existing mapping uses the retained guitar flag to zero channels10,11,21,22, then invokes a freshly read optional ExternalPadRemap.UpdateGamePad(player0,values). Device connection/capability failure is an explicit remaining boundary. Confidence: high for inspected ordering; unknown failure-output initialization. No runtime test or platform parity claim.
