# Independent input pattern/parser/matcher review

2026-09-08. Reviewer /root/ik_world_medium; read-only production review, no Cargo. Result: PASS for the explicitly selected immutable coord parser, retained history/matcher and constructor-bound immutable recognizer attribute projection. No live PlayerInput/Actor/event scheduling claim.

Reviewed input_pattern.rs, input_pattern_parser.rs, input_pattern_tests.rs, input_pattern_matcher.rs and input_pattern_matcher_tests.rs against fresh original IDA raw and input-pattern-runtime-plan.md. The author and reviewer are separate agents.

Confirmed:

- Parser preserves duplicate pattern occurrences and authored order; coord push_front826e30bc produces reverse point order. min_time826e30dc is read/discarded. Finite decimal atof-double then f32 rounding is explicit; unsupported clock/CRT syntax/name/capacity inputs refuse. This does not claim native partial reload side effects.
- Actual history201 storage uses one empty sentinel: raw826e19cc..1a24 moves begin backward, evicts end only on collision, otherwise increments size. At most200 valid samples; no constructor zero-history fabrication. Unique history occurrence guards retain correct owner.
- Generated recognizer constructor826def84..fa0 forms class7f086875d621c88c; fallback layout size30 atdefc0/c8. Provider retains actual immutable instance and reads cached layout byte21 only at the source waiting stage. Layout checked-snapshot boundary remains explicit; generic test providers do not establish global mutable singleton parity.
- Candidate and finished node gates, rounded target-current XY square/add graphs at826e0ca8..cd8 and0dec..dfc, first-point admission, waiting suppression,10-bit total time and6-bit waiting wraps agree. Actual branch words0cf0=4099007c and0d10=40990028 have BO4/BI25, so unordered distance takes target/stay branches. Waiting scalar read0d44 follows counter store0d34; failure preserves counter and earlier-node effects.
- Winner captures min target then max target before iterating nonempty nodes (dfb2c/30), reads values per finished node(df b4c/60), preserves scalar FSEL operand order and separately rounded n?/n?/n4/denominator. First finished candidate wins even NaN; later replacement is ordered greater only (dfbb0=4099000c, BO4/BI25).
- FindMatch cooldown1434/1440 clears candidates and success bit without clearing caller output; history<=1 also leaves it. UpdatePatterns precedes output reset. Winner pattern/time/error publication precedes fastest capture/value; ratio branch10f0 maps unordered to speed1. Slowest capture/value110c/10 and repeated value1124 precede reciprocal/product/negate/FMA1140. Code preserves this callback order and failure prefixes.
- Held compares current minus final authored endpoint, subtracts current pattern tolerance, releases only ordered positive (dfa70). NaN remains held; false does not publish a new name. Actual pattern Arc is preserved.

Tests discriminate duplicate identity/first tie, reverse points, held release/NaN, cooldown, history capacity/foreign occurrence, waiting-prefix and wraps, unordered target admission, failed cvar publication prefix, late slowest reread and actual cached attribute/default binding. Original-file count helper is intended for root's mounted corpus; this review did not execute it.

Two evidence-only address corrections sent author: FindMatchPattern call is826e14d8, while14dc tests the return; waiting byte read is826e0d44, while0d4c is its branch. Neither affects implementation acceptance.

Remaining scope is explicit: actual command-produced gameplay axis sampling, deadzone, enclosing held-first/match-second event delivery, matcher construction within the same PlayerInput, mutable cvar ownership, and Actor/generated graph scheduling still require their real producers. This review does not infer them from the semantic leaf.
