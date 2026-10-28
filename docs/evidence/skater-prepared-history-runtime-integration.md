# Prepared jump history, spatial inputs and command-owned input

2026-09-08. Root integration with separate source reviews.
Confidence **exact** for checked stores, branches, constants and wire format;
**high** for the selected native Rust paths. Full debug per-tick numerical
equivalence and playable Actor scheduling remain unproven.

## Prepared history

`physical_player_prepare_velocity.rs` implements `PhysicalPlayerHiLOD::Update1`
`82e0379c..82e038c4`. A single non-Clone history owner moves through the actual
physical-player lifetime. Its constructor stores full zero velocity at +c0,
last Spin at +158 and count at +13c before the selected Reckoning constructor.
No per-call replacement or Reset clearing of retained history is introduced.

The source captures processed input three times. PREPARE copies the prior
actual velocity. Otherwise the actual surface velocity goes through the shared
NormalizeSafe graph, signed Y, positive gravity `411ccccd` from `82359ae8`, and
fixed step `3c888889` from `8236e734`. Separate products and full-vector FMA
preserve unknown W. Spin uses the actual processed type-0 Spin producer, whose
payload is negated. Its product increments the wrapping count only for ordered
positive values; zero, negative and unordered reset the count. Count publishes
before the final prepared-vector store, with partial failure preserved.

`skeleton_processed_prepare.rs` extends the SAME processed owner. Reset +2c0
is full zero (`82b39348`), count +924 zero (`39528`). The same-pair session
validates actual backend, bodies, attachments and owner identity. The original
moving-board corpus exercises repeated accumulation, actual Spin, and PREPARE
copying of the published board velocity. These are explicit diagnostic calls,
not a claim that native Update1 or animation events run autonomously.

## COM and gravity

See [raw spatial proof](skater-jump-spatial-inputs-runtime-plan.md).
`physical_player_phys_out_com.rs` preserves the asymmetric Fill sequence:
sample latch, read early COM velocity from actual history, check latch, read
late COM and store System+30, then a second latch guards early velocity's
System+10 store. Full four-lane history values survive without recomputation.
`physical_player_com_session.rs` borrows the SAME actual Skeleton history and
requires its completed frame Arc to match the current validated world frame.
The original collision corpus compares the publications with that retained
history and rejects reuse after another actual solve.

The existing System output owner now initializes/resets both slots to full
zero with constructor/Reset provenance. Existing prior-output copying adds
separate guarded processed +250/+260 stores before Reckoning up. Processed
Reset separately writes those slots and gravity +964 (`c11ccccd`, actual
`820100b8`). Allocation leaves gravity unknown. This negative jump-gravity
constant is distinct from the positive prepared-history decay constant.

## Input

The actual command-updated gameplay pad is separate from SIM/PRES caches.
`game_input_player.rs` retains that pad and original map, source bounce gates,
four globals and receiver identity. `game_input_command.rs` implements sparse
Pack/Unpack including signed byte quantization and the empty-focus Button13
sentinel. `game_input_manager.rs` registers and synchronously dispatches to
the same retained player, computing enabled CRC before receiver lookup.
All 256 original CRC table words were independently compared with the source
polynomial. See [command proof](game-input-command-runtime-plan.md).

`game_input_original_corpus.rs` takes the actually mounted Xbox input.cfg
through that codec, registry and mapped gameplay owner, checking frozen wire
bytes, all selected map channels, bumper arbitration and focus/bounce state.
Its starting channel values are controlled diagnostics, not hardware polling
or the missing PlayerUI/proxy/offline frame scheduler.

The original seven .pat files feed the ordered parser (285 patterns).
`input_pattern_matcher.rs` retains history occurrence, incremental candidate
counters, original call-count cooldown, held endpoint, staged runtime Vault
waiting byte and cvar reads. Independent review:
[pattern proof](input-pattern-independent-review.md). The actual enclosing
PlayerInput update, broadcaster/listener and generated Actor intents are next;
pattern component tests do not prove that those runtime producers are wired.

## Validation

Workspace `86201/f5e6f0`: **3,434 unit tests** (3,191 core, 202 formats,
38 app, three symbols) and six docs pass. Strict Clippy **26065/77e633** and
all twelve original-data checks **37184/f83404** pass. The COM corpus exposed
a fixture ordering error: selected PhysOut construction originally occurred
after its first COM write. Construction is now an explicit earlier diagnostic
stage, preserving the initial NotConstructed assertion and all access guards.

Next spatial work is the actual four-wheel CalculateGroundPos and its retained
category/jump-count gates. Active jump force/trajectory routing, input delivery,
full Actor state/animation/physics scheduling and portable presentation remain.
The offline playable goal stays active; broader DMO work stays parked.
