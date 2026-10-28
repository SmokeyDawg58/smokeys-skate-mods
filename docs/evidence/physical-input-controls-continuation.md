# Physical input controls continuation

Source `PhysicalPlayerHiLOD::Update1_ProcessInput82e02174..82e0242c`,
reviewed raw and decompiled 2026-09-12. Implementation is registered under
constraint session as `physical_input_controls`; validation is pending root
Cargo/corpus. No completed Update1 or playable frame is claimed.

`try_continue_player_update1_controls` consumes the graph-prefix continuation
carrying actual owner, processed, AnimOut and previously captured category.
It checks both supplied handles against that owner before any native write.
Its returned continuation stops immediately before prior-copy82e02430.

Source sequence:

- 2174..2184 copies HiLOD120 bit20000000 into processed8c8 bit1000.
- 2198 invokes actual IActorQuery.GetInput+4. Actor825f11a8 returns null when
  its actual AI controller exists, otherwise obtains its selected gameplay
  input. The host query child upgrades the retained Actor weak binding,
  checks that AI field and evaluates the same registered GameInputPlayer map
  and pad. Unsupported map global expressions remain explicit failures;
  no fabricated InputSK8 global context is supplied. Null skips the write.
- 21b0..21ec reads control75 and performs two FSEL operations, preserving
  negative zero and yielding positive one for NaN.
- 21fc..2260 uses the earlier captured category100. Ground increments the
  signed count with wrapping arithmetic, adds processed dt to ground time,
  then writes prefix high bit from count>6. Nonground clears count/time only;
  it does not clear the prefix high bit. Publish processed ground time.
- 2268 invokes existing same-pair SurfacePhysics dispatch against previous
  published PhysOut collision and retained surface instances. All preceding
  writes survive a failure here. No line-query surface substitution occurs.
- 2288 writes mirrored flag8cc bit8. Active AI sets8c8 bit40000000 then
  copies ten vectors and seven flags through bit02000000, preserving other
  processed AI flag bits in the existing physics_state.ai_flags owner.
  Incoming04000000 retains the whole AI input in existing HiLOD prefix.ai.
  Otherwise restore five trajectory vectors from prefix history, replace
  only flag08000000 and set04000000; desired deck data remains incoming.
- 235c writes under-AI8c8 bit20000000. 2370 writes prefix high bit into8c4
  bit2000; 2384 copies existing8c4 bit8 into bit40000.
- 2390..23cc updates framesSinceTeleport, resets on prefix20000000 or wraps
  increment otherwise, publishes it, then clears that prefix flag.
- 23cc..23e4 copies state count/timer from the existing physics-state owner
  and update count from existing prefix. These sources are not duplicated.
- 23fc..242c writes weight-forward8c8 bit4000, prevent-manual-respawn8c4
  bit2 and ignore-reset-button8c4 bit1, before returning for prior copy.

Root independent raw review additionally confirmed AI constructor stores
82e00f18..54: all ten vectors zero except trajectory-time index8 at -1.
The actual processed constructor now initializes those vectors before Reset.
Reset preserves the vectors and only changes AI flags. Actual cleared Sim
allocation establishes all AI flag bits as known; generic construction/reset
does not invent knowledge of the lower25 bits. The existing shared
`AnimationAiInput::assign_source825f0f60` handles HiLOD history assignment.

New processed payload child retains only missing scalar destinations and ten
AI vectors. Its flags reuse existing processed words. Reset scalar stores
are verified at82b394d4/39508/3950c/39510/39530. Unknown unassigned AI vectors
are represented explicitly and are not supplied as zeros to inactive paths.

Focused tests added: transition FSEL endpoints/NaN/signed zero; AI assignment
preserving low flag bits; trajectory-only restoration preserving incoming
desired-deck data and unrelated AI flags. Root owns compilation, strict lint,
original corpus and independent source review.
