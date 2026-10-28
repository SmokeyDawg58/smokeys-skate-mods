# Windows raw controller boundary

Draft, not registered or hardware tested. Native device, mapping and cache
semantics remain in core; WindowsPadPlatform only maps OS result records.
No normalizing, trigger thresholding, packet-number deduplication or alternate
button mapping is performed here. Failed capabilities expose unknown fields,
not fabricated zero output; the native device retains that uncertainty.

Safe wrapper selection: thindx-xinput pinned0.0.0-2024-02-04 exposes raw
integer State/Gamepad, explicit capability flags, native status via Error.kind,
and two-u16 vibration. It permits the actual flag1 (Gamepad) call. This keeps
workspace unsafe_code=forbid while isolating Windows dependencies in the app.
The published crate archive SHA256 matches crates.io checksum
ad118686f15658b4f2ccb0d96c629c2f9dac4e854701b5a9393e8e5b0d58f05d.
API/source inspected September22; pinned dependency and adapter now registered.
Strict Clippy native-pad-4 and workspace-native-pad-2 pass. Actual Windows API
probe windows-pad-api-1 passes: all four slots report status1167 disconnected.
Connected hardware and selected boot polling remain untested.

Primary sources: [crate API](https://docs.rs/thindx-xinput/latest/xinput/),
[capability source](https://github.com/MaulingMonkey/thindx-xinput/blob/master/src/functions/get_capabilities_.rs),
[raw gamepad](https://github.com/MaulingMonkey/thindx-xinput/blob/master/src/structures/gamepad.rs),
[status error](https://github.com/MaulingMonkey/thindx-xinput/blob/master/src/error.rs).
Published version source was separately checked from its checksum-verified crate.

Alternative rusty-xinput1.3.0 has a public raw state, but its capability method
hardcodes flags0. That is unsuitable for the native flag1 call without an
additional wrapper change. No dependency was added for it. See
[wrapper source](https://github.com/Lokathor/rusty-xinput/blob/master/src/lib.rs).

Windows XInput is a host hardware service, not a claim of matching every Xbox
Xam device failure, subtype, or driver timing. No failed-call memory is exposed
by these safe wrappers; Option unknown is retained. Actual selected boot must
supply the same initialized core device manager and live cvar owner. No second
default input manager or cvar manager is created inside this adapter.
