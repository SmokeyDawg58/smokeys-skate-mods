# Action expression7: shared handplant condition

Source/code confidence: high; independently approved September22.
Pre table828ff9e8 points to828ffe18; Sustain829060a0 to829063bc.
Both dispatch condition37 (offset0x94, cache word1 bit0x20) through the
existing parent-tagged condition cache and IsHandPlanting.ActivationFunc82c9abf0.

The same existing Motion physical implementation is reused. Each graph validates
its own context first. Actual physical interface identity is checked; Air flags
are read before State, Air compares state600, Ground tests flag0x80000000,
and active FS/BS uses bit0x20000000. No new physical state or guessed result.
Tests cover both masks, both boolean results, callback identity, and failure.
Original queued input previously stopped at frame10 expression7; execution of
this registered change passes action7-recipe-2: fmt, strict Clippy53.26s,
4685 workspace units and six docs. Original19/21 in6.83s passes7 then reaches
Action42 on frame10; mounted recipe completion also succeeds.
