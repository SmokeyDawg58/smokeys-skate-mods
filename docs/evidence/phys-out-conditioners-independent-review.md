# Independent conditioner constructor review

2026-09-12. Reviewer ground_path. Implementation author
input_physout. Read-only implementation review; no author files edited.
Root owns mounting and all Cargo/original-data gates.

Reviewed the new Conditioners module and its eleven implementation children
(simple, grinds, animation, sequence, scorable, score_holder, collector_base,
collectors, air_collector, wipeout_score, score_module), with fresh connected
IDA decompilation and critical raw/PDB checks. No blocking mismatch found in
the reviewed constructor/reset graph and field-preservation cases below.

- Outer82e316f0: eleven allocation sizes/categories/order match, with each
  original pointer published after its child constructor. Null children stay
  null; source dependent calls do not receive stand-in successful owners.
- ScoreModule82dfb508: Metrics, Holder/Sequence, Air, Grind, Ground,
  Handplant, Offboard, Other, Line, Momentum, Wipeout/three entity children.
  Same output occurrence and shared Metrics/Holder retained throughout.
- Scoring82e2b3a8 explicitly performs a second ScoreModule.Reset after
  its constructor. Null module still reaches the Reset dereference. Host
  publication/error prefix matches this ordering.
- ScoreModule.Reset82df26f0: Holder first; metrics UP/context counts/index;
  six collectors then Line/Momentum; currentOther pointer; virtual Enter;
  scalar/flag suffix; finally guarded PhysOut.Scoring2.hasBeenReset. Raw
  Other vtable8236c3ac slot+c points82deb5e8 Reset, so the implementation's
  second reset_analysis is correct rather than an omitted Enter behavior.
- Air.Reset82df4720 reads actual deck position, shared metrics normal,
  ground record and two output matrices in sequence. Source guard failures
  retain earlier analysis/position/metrics/gap effects. Later resets retain
  the documented Scorable settings residuals.
- Animation82e2fdc0 reads copied PID controls at indices33/32/31, performs
  two distinct one-byte PhysicsDebug allocations, then resets. Preserved
  control tuning is not replaced by live reset-time reads.
- Sequence.Reset82dedbe0 clears matches, active count, counters and high3
  flags while preserving pattern items/active settings/scorable payloads.
  ScoreHolder.Reset82dee248 resets Sequence before accumulator arrays and
  local score fields, preserving session total and quicklist contents.
- Ground82df9a78 graph pairs0/1,3/2,7/6 match. Non-Sim total score,
  tracker last positions and low29 flag residues remain explicit unknown.
  Offboard matrices and total score are likewise not invented as zero.
- TrickSettings82de97f0 does not write switch variant or connector arrays.
  Raw82de9800 sets full r11=0,82de9850 stores64-bit r11 to attrib key.
  Broken decompiler pointer-high reconstruction at that store is not used.
- WipeoutScorer82df5618 writes its known damage/rotation/bone/score fields
  before allocating its three entity scorers. Pitch/Yaw/Roll values2/0/1,
  25-bone count and Sim-only matrix zero provenance agree. Null children
  do not remove subsequent allocations; DMO tuning uses real lookup.
- Audio.Reset827c4b28 resets its histories and transform while preserving
  constructor-sampled isWipingOut; wheel contact defaults true after Reset.

This is source review of constructor closure, not validation of continuous
conditioning, scoring gameplay, real-time animation or audio playback.
