# Board deck-line output and acceleration

Registered, pending gates, September 12, 2026. Source confidence: high, with raw
82b2fc24..82b2fdd4 inspected. No Cargo or original-data result is claimed.

The caller consumes the same-owner `Update5BeforeBoardDeckLine` token, including
the contact-owner Arc captured by the preceding wheel/contact block. The line
pointer is obtained at fc30 from the actual retained board constructor's
`SkateboardRetainedWheelLines` through `try_board_wheel_line_state`. This obtains
the same record; it runs neither Start nor End and synthesizes no query result.
There is no contact-spy frame prerequisite. Source fields remain individual
late reads under their actual retained-owner locks.

| Native operation | Output and ordering |
|---|---|
| fc34 guard, fc58 valid read, fc60 store | record Body+1f10/+34 to Ground+123 |
| fc64 guard, fc8c start read, fc90 store | full record+0 to Ground+90 |
| fc94 valid reread | independently controls the following three stores |
| fca0 guard, fcc8 read, fccc store | full record+10 position to Ground+a0 |
| fcd0 guard, fcf8 read, fcfc store | full record+20 normal to Ground+b0 |
| fd00 guard, fd24 read, fd28 store | record+30 surface bits to Ground+f0 |
| fd38 guard, fd64 read, fd6c compare | read the already published deck position and compare splatted X with itself |
| selected fd80 guard, fda4 store | only NaN X selects full positive-zero replacement |
| fdac latch, fdb4 read, fdb8 assertion, fdd4 store | same captured collision.acceleration[6], Body+210, to Motion+70 |

The first published validity does not control the conditional copies: the source
explicitly rereads record validity. Tests change it between reads to distinguish
this behavior. Invalid records still require and publish the start vector but
do not read stale position/normal/surface payloads. All copies preserve full W
knowledge. Infinities, signed zero, NaN in Y/Z, and unknown W do not trigger the
deck-position correction. Raw fd6c18642065 uses actual VA36/VB36, the splatted X;
the decompiler's extended vector aliases must not be treated as different data.

Focused draft tests cover both validity transitions, lazy invalid payloads,
full-vector W handling, exact NaN-X behavior, locked-output capture-before-read,
and a late acceleration failure retaining the line stores and NaN correction.

Root independently reviewed the raw source and caller before registration of
`constraint_session::board_fill_deck` and `PhysOut.construct::board_fill_deck`,
alongside the wheel/contact registrations. The existing continuation
types supply the same owners; no shared payload or getter needs modification.
The returned `Update5BeforeBoardSurfaceMotion` stops before fdd8. Existing
surface-motion math must be reused in the subsequent actual-owner adapter.
