# Actual Motion ChannelBlend dispatch

Original-presentation-startup-1 completed frames0/1, then frame2 Motion reached
PlayAnimation Dispatch UnsupportedDispatch(ChannelBlendAnim). Existing transition
stores, body tree allocation, time/evaluation and channel blend command consumers
already existed. Root extends that producer instead of implementing another one.

TransitionChannelBlendAnim82d7aeb0 is a wrapper for TransitionAnim82d7aa68:
raw aebc/aec8/aed8 exchange length/playback floating arguments, preserving time;
currentFrame r8 goes r9, placement r9 goes r10, useFromWeights r10 becomes stack
byte57, r6 receives transition type2. Ordinary wrapper82d7ae70 uses type1 and
false weights. IDA's inferred argument names confuse type/currentFrame and
must not be used as the call ABI.

Both blend kinds reuse actual retained pools, pruning, new tree, name/speed/time
stores and youngest/root placement. After successful Init, flags31(type==2)
and30(useFromWeights) replace only the top two bits at82d7ad78/ade4, before
parent or root publicationad7c/adf4. Failed Init preserves its prior prefix and
never performs those later writes. The original regular wrapper remains usable.

Independent review also found an inherited adjacent Init bug: raw82d6b87c/
b884/b888 preserve lower29 flag bits while replacing currentFrame bit29 and
clearing top2. Root corrected that mask and retained lower known-zero evidence.
Focused tests cover both modes/currentFrame choices and reuse with nonzero low
flags. Accepted channel-transition gate: fmt2, strictClippy2, workspace4,549
unit+6doc pass; original17/19 advances frame2 to BlendSpace type6.

The body evaluation already handles channel bit31 when useFromWeights is false.
True useFromWeights remains an explicit consumer boundary until reconstructed;
no unconditional success or substitution with ordinary blend is introduced.
