# Six reported dispatch gaps and quiet controller play

Work performed solo. The prior push correction
was confirmed in play: pushing now feels correct without interruptions.

## IDA source and implementation

- Motion state update828f4508: Manual1027 parent followed by
  SetScoreAugmentation slot32 and PlayAnimation33. Exact state1274 Out list
  reuses existing Manual/turning/attribute/scoring/animation callbacks.
- Motion update828f2058: folded FromGrind/FromManual thunk. Parent
  360Flip199 at828efae8 runs Trick19 then ScoringTrick23, native empty24,
  CreateAttribute25/26, native empty27. Leaf PlayAnimation28, native empty29,
  JumpInto30. GrindOutAssist828f0d80 forwards to this same parent; Nose/Tail
  828eece0 forwards to Trick19. Added all48 authored aliases of this address,
  each with its own exact behavior indices. Existing JumpInto allocation,
  first-update handling, animation marker/clock and transition path reused.
- Motion update828f2ca0: Riding277 then AttachIntent28, PlayAnimation29,
  CreateAttribute30, SetScoreAugmentation31. Both Bs1023 and Fs1024 share the
  thunk; their existing behavior occurrences remain distinct. Parent718's
  AttachIntent belongs at28 and is not executed twice.
- Motion Pre1138290dbf8 = condition193 (HasIntent), from sConditions+304.
  Sustain113 selects native default829221c8 true without condition reads.
- Motion Pre4438291d71c = c214 && (c18 || c288 || c343 || c287 || c225).
  Raw branches require WillExpire before the ordered interruption alternatives.
  Sustain443 independently selects native default829221c8.
- Action Sustain10182909c18..9ee4 = c45 && c44 &&
  ((c74 && c15 && c16) || (c36 && c38 && c39)), identical to the existing
  Pre10182904eb8 path. Extended its mask arm without duplicating condition code.

Static descriptor audit verified all51 new (state,address) entries are unique
and their callback arrays equal the complete authored ancestor expansion.
Added lazy-read regression cases for Motion443 and Action101 Sustain, plus
no-read checks for the Motion Sustain defaults. Rustfmt parse-only passed for
changed Rust files; tests have NOT been executed and no build was started.

## Quiet mode

Core diagnostics flag defaults off. The PC live entry sets it for --quiet.
Lexically scoped println/eprintln wrappers suppress routine diagnostics before
argument evaluation/formatting in both crates. The two direct stderr profiling
writers are also gated. The terminal live error report uses std::eprintln
explicitly and Rust panic reporting remains available. No file logging is
installed. Existing opt-in traces remain available by omitting --quiet.

play-quiet.bat runs the existing release executable with --live-skating --quiet,
without redirection, capture options, frame limits, or automatic compilation.
It inherits SKATE2_GAME_ROOT, using README's existing reference path otherwise,
checks the directory/executable, and pauses on nonzero exit for error copying.
Build with build-release.bat first. Runtime behavior is verified hands-on.
