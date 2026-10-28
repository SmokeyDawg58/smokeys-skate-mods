# Mutable time on retained native animation trees

Confidence: high, pending independent review and original-corpus validation.
The producer advances time and animation properties; it does not evaluate poses.

`ChannelAnimatable::AddTime82c8a558` calls channel UpdateTransitions82d74818,
RemoveExpiredChannels82d7e678, then base Animatable::AddTime82592b58. The base
calls UpdateTransitions82d7aef0, resets actual AnimProperty fields to
false/false/-1/-1, obtains the actual current channel or body root, advances it,
and only on return writes isPlaying=true. Nonempty transition lists, transitioning
channels, or channels whose removal condition is true remain explicit missing
producer errors. These paths cannot be silently ignored or approximated through
the native hash traversal's different order. Actual initialized and PlayAnim
stores pass the empty transition conditions. No null ChannelBlend base is
replaced with a bind pose.

Clip::AddTime82d6d238 preserves strict time>length expiration. It writes
timeToExpire before wrapping, and looping repeatedly subtracts length and
increments the original integer loop count. The last subtraction's excess is
reported; timeToExpire is not recomputed. A source loop whose subtraction makes
no floating-point progress returns an explicit NonProgressingLoop boundary after
retaining that iteration's writes, instead of hanging or inventing a result.

PhaseBlend::AddTime82d6bd48 reads child1/child2/root lengths, divides dt by root
length, advances child1 with the caller's AnimProperty and child2 with a separate
fresh false/false/-1/-1 temporary (raw r5 assignments at82d6be34/82d6be64).
It derives its time from child1 time, then rescales caller expiration values with
the source nonfused f32 operation order. AddSQT82588500 advances only its motion
operand, not A and B. Mirror82c92790 advances its child. Pose uses the actual
empty AddTime vtable slot; its GetTime82867190 returns zero. Phase GetTime
82d75910 and GetLength82d758b0 require both selected children before reading.

ChannelBlend::AddTime82d6d610 advances base first, computes fade factors from
the previous timer values while incrementing those timers, then uses exact FSEL
selection and unit clamps to store its coefficient. This preserves signed-zero
and NaN behavior rather than using Rust min/max. Endpoint flags determine whether
channel time advances; its own temporary AnimProperty does not overwrite the
base result. Raw insrwi at82d6d888 inserts that temporary's expired byte into
mask04000000; the decompiler's unconditional-clear rendering is wrong.

PDB ChannelBlend flags at+34 identify masks02000000 useAttributes,04000000
channelExpired,08000000 blendOut,10000000 cycle,20000000 blendOutEarly,
40000000 blendOutFromLastFrame,80000000 blendInToFirstFrame. Earlier named
constructor booleans had blendOut and blendOutEarly exchanged despite a correct
08000000 raw mirror. This batch corrects those names and verifies the original
constructor mask and EndChannel's28000000. This was caught by following actual
AddTime and RemoveExpiredChannels consumers.

The dedicated original initialized Actor fixture now invokes AddTime0.01 after
actual body Play and two real gesture channels. It checks body time advances,
actual isPlaying, retained channel timers, and first-frame zero blend coefficient.
Compilation, strict checks, execution and independent source verification are
root-owned. Fresh external packet `%TEMP%/skate2-native-animation-addtime-source.json`
contains ten functions' original bytes and decompilation. No game data is copied
into this repository. Dynamic Eval/ACS remains a separate outstanding producer.
