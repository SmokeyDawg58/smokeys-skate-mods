# CanEnterSlide runtime, September 22

Actual original-motion442-1 reaches Motion Pre220 on frame2 after two fresh selected frames;17/18 original tests pass. No completed full-frame claim.

Independent raw source audit: Pre220 table8290ae00 targets829106b4, reads condition318 (pointer4f8/cacheword9 mask40000000). Pre224 targets82910834, reads condition324 (pointer510/cacheword10 mask10). Both Sustain targets829221c8 return true without reads. Authored LeftSlide220 and RightSlide224 are adjacent selection branches.

Both use the existing Motion CanEnterSlide payload, not Shared. Condition318 attribute2162 sets right=false. Condition324 has no attributes, retaining constructor right=true. Focused constructor tests assert both actual generated declarations.

Activation82c9c740 resolves/validates ISkaterAnim first, ISkaterMotionGraph second. Motion virtual3c atc8b4 returns data before animation virtual0c atc8cc reads riding-fakie. Concrete Motion vtable82069688 slot3c targets825fdfc8, returning self+C84. Flag words data+4/+c correspond to existing ActorSkaterGraphState.slide_bits[0]/[1]. Side is right XOR fakie; only that side's highbit80000000 is read after fakie. Existing constructors establish clear high bits; missing known bits remain typed errors.

Root implementation reuses actual animation identity checks and the same graph state, validates graph interface2 against its occurrence, borrows data before reading the retained animation flag20000000, then checks only selected slide highbit. No generated controller lock reentry or alternate owner. Tests cover both authored constructor variants, selected unknown flags versus unread opposite side, fakie swapping, direct leaf results/error propagation and Sustain no-reads. Source confidence high; code review and validation gate pending at writing.

Independent source/code review approved the registered implementation and tests.
First Clippy pass found only the separate mounted-source fixture callback lint;
root documented its fallible callback signature. Full new batch gate pending.
