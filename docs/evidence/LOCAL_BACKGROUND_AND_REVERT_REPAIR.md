# Local background clock and online revert crash

Owner confirmed preview12's gameplay repairs, then reported a freeze while
Alt-Tabbed or clicking away. It resumes on returning. The host clock explicitly
discarded every unfocused offline interval. Focus now controls input alone;
offline and online clocks share the existing four-tick stale-debt bound. Remove
the diagnostic-only clock exemption so tests exercise the same policy as normal
play. Actual pause menus, controller focus, fixed physics dt and volume settings
are unchanged. Existing minimized/background window timers keep the client
running when no swapchain image can be presented. Confidence: high diagnosis,
pending final inactive-window integration check.

Two actual owner reports stopped with Score(Invalid("revert missing animation
type")); latest run1792933829471432600-30928 used preview12. A rejected held
darkcatch at source tick10697 was followed by repeated score resets. Physical
state102 (Revert) remained active at10705, while cancellation suppressed its
animation scoring flags. GroundCollector::update_revert (ground.rs) required a
direction that its reset had legitimately cleared, terminating the game.

For the source Skate3 scorer only, an active physical revert with no observed
animation type waits instead of starting a scorable. A later known nose/tail
type follows the existing start/end/bank path. No side is guessed, no untyped
revert points are awarded, and the independent original Skate2 invariant remains
strict. Existing darkcatch rejection still clears unbanked points. Confidence:
high for the captured failure path, medium for broader online coverage.

Validation uses the captured failure ordering, the exact revert update method
with narrow mocked scoring collaborators, silent recorded-input online checks,
and an inactive offscreen window with ordinary focused-input policy. Full public
match coverage is not claimed. Keep the replacement local pending owner review.
