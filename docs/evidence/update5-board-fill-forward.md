# BoardFill forward-speed continuation

Unregistered adapter and output leaf cover82b300e4..301a0. They reuse the
existing typed effective board transform, published Motion.DeckVelocity, and
Motion.ZSpeed scalar. No additional numerical leaf, scalar owner, or W default
is introduced. The endpoint precedes four wheel-body position copies.

Native Body.GetPosition(6)300e8 precedes the late Processed8c4 flip read300ec..f8;
the selected sign XOR affects full X/Z columns, preserving unknown packed W in
the typed helper. First output assertion30134..50 follows that computation.
Native Motion pointer30154 is followed by a second sampled latch30160, then
Z3016c and retained velocity30170 loads and dot3 at30174. The sampled latch is
asserted at3017c..190 before Motion+a8 publication301a0. The draft directly
reads the retained velocity field to avoid the public accessor's additional
assertion before dot3. Only XYZ reaches dot3; unknown W remains unknown.

Tests cover the fresh transform read before an initial locked-output refusal,
absence of a dot call after that refusal, late numeric failure preserving the
old scalar, and successful XYZ projection with both source Ws unknown. These
tests have not run; registration is held for the parent gate.

## Following wheel-position descriptor

Native entry f430 reads assembly pointer from captured Body+18, and f438 saves
it at stack var270. The next loop reloads that SAME assembly at3019c, then reads
its current Parts pointer+8 separately at301a4,301dc,30214,3024c. The selected
body pointers are current Parts+4c,+ac,+10c,+16c. Each body+10 position is read
late relative to its output latch and stored into Reckoning+d0,+e0,+f0,+100.

Proposed retained counterpart is a cloned actual SimulationSkaterAssemblyHandle
captured at the initial Fill boundary, threaded alongside existing continuation
identity and validated with same_as. It identifies the same retained session;
wheel-body pointers and position values must still be resolved per source read.
Do not snapshot four bodies at entry, or impose a ContactSpyFrame requirement on
these direct current-body loads. This descriptor propagation is not yet edited.
