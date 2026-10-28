# Local online Hall of Meat input and pose bridge

The owner reports correct career/lobby behavior but restricted bouncing/control
once online Hall of Meat starts; scoring also needs attention. Retained Update25
logs confirm native S3 Wipeout300 during challenge type31, with nonzero left-stick
control and no actor pin during those samples. Do not describe this as a proven
full fallback to the S2 solver. The exact failing interval is not yet captured.

A concrete integration fault: S3 bypasses S2 Update5, but its bridge omitted
Trick.wipeout_gesture_x/y. HoM's UpdateBodyTweaks82dec470 reads those fields for
pose hold/bonuses. Forward the S3 animation_input.extra.wipeout_gesture used by
the actual bail profile to that same host scoring owner every completed frame.
This preserves authored scoring values and does not invent score multipliers.

A second fault: the source-controller focus gate required PauseMan's
CheckForPauseState826477b0 menu permission. Net::ResumeGame82791ca8 can keep
simulation/input active while another pause owner prevents opening menus.
Use actual simulation input, focus, open front ends/popups and map loading
instead. Native PlayerUI/session-marker menu eligibility remains unchanged,
as do scripted actor pins and their gesture exception. This is a targeted input
integration repair, not a change to native S3 bounce forces or global time.

Confidence: high for the omitted pose channel and distinct pause semantics;
hypothesis until an actual online round reproduces the reported control loss.
Add bounded existing-journal records for active bail profile/gesture/control,
response counters, and actual host HoM point components to make that comparison
observable. No private/public player name or chat message is needed.
