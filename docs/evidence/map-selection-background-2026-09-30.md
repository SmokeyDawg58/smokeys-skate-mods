# Challenge Map selection and wide-window background

Fixes the missing animated selection arrow and adds background-only
wide-window stretching (a PC adaptation). Compilation and playtesting remain pending.

## Source evidence

IDA `FrontEndState_ChallengeMap::SetHighlightRegion` 8265ef80 sends state60
then71 on activation (61 then70 on deactivation). Original ChallengeMap APT
constructor hides the menu cursor, but `PlaySelectHighlightRegion(Stuff)`
5cb8..5cc2 calls `SetCursorInvisible(false)` and then Update at5ccd.
The host's active list incorrectly retained the constructor's true flag.
The active category/destination picker now passes false. Info Teleport already
uses false. No replacement arrow, timer or animation was added.

Original menu_picker Update33348..33c9 admits the indicator for a present row,
moves its Y to that row, and hides it otherwise. Existing update_picker does
this; its authored X=-41 remains intact. Indicator sprite53 contains sprite52's
animated artwork; sprite53 stopping at frame14 does not stop descendant ticks.
All original timeline frames continue through the existing Clip owner.

Read-only tools/inspect_apt.py audits used the original files under
%TEMP%/skate-map-audit/data/fe/source, including adjacent .const files.

## PC aspect adaptation

Flat APT normally preserves the 1280x720 stage aspect. At wider aspects it
pillarboxes everything, including full-screen background layers. Only shape19
of ChallengeMap (1145.5x550 backdrop, authored placement66.5,88), shape3 of
LetterBox (1280x92 bars), and shape2 of dimmer (1280x720) now cancel that
horizontal pillarboxing. Authored backdrop margins remain proportional;
full-stage dimmer and bars reach the window edges. Shapes keep original UVs,
alpha, and animated matrices. Foreground map/text/icons remain aspect-correct.
At16:9 or narrower this introduces no change. This is a deliberate PC
widescreen adaptation, not a recovered original ultrawide layout policy.

No compilation, tests, or runtime launch performed. To verify: arrow
animation/navigation, intro/outro, Teleport/back, and resizing the open map.

October1 continuation: active goal authorizes agent compilation/runtime checks.
Added original_map_selection_animates_and_only_background_expands in app UI
tests. It opens the actual map asset, checks indicator draw geometry changes
over time, changes selection, and compares 1280x720/2560x720 foreground and
background geometry/UVs. Test execution pending behind the running core camera
regression build. This note supersedes the historical manual-only verification
workflow above; it is not a claim of a passing test or screenshot parity.

Verification: release original-asset test PASS (1 passed,0 failed), recorded in
artifacts/map-selection-aspect-test.log. Authored arrow geometry changes over
30 frames after intro, selection moves its indicator, and1280x720 versus2560x720
comparisons preserve foreground aspect and background extents/UVs. This verifies
the requested animation and aspect behavior; no pixel-identical screenshot claim.
