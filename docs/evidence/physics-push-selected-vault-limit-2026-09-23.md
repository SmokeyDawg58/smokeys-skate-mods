# Selected physics push limits from original Vault

The 120-frame selected raw-A replay reaches Motion push, with a nonzero
`PushState.mCurrPushDv`, active `push_contact`, processed pushing flag
`8c4/02000000` on four contact frames, and animation `push_speed+940` about
9.87–11.4. Board displacement remains about 0.0022 and z-speed about 0.003.
The push-force diagnostic reports `change=0`, `SPEED_LIMIT=0`, board mass about
11.56 and `dt=1/60`; the retained direction is valid. A zero `CHANGE_LIMIT`
would also clamp the same positive animation-speed difference to zero before
the speed-limit comparison. These are selected-host observations, not an Xbox
per-tick trace; the existing replay is `original_selected_live_facade_grounded_right_push_route`.

Fresh read-only IDA disassembly confirms the original binding and read sites:

- `GlobalAttributeCollection` at `0x828b2510..0x828b2544` forms class
  `0xd4d04d2fa87a9a31`, passes the retained default collection key in `r28`
  to `Attrib::FindCollection` at `0x828b2528`, constructs an `Attrib::Instance`
  at `0x828b2538`, and stores it at global `+0xb0` at `0x828b2544`.
- `Toolkit_CalcPushAcceleration` reads `CHANGE_LIMIT`
  `0x76feee54a3ad2af5` at `0x82b27400` and `SPEED_LIMIT`
  `0xa9b532ebbed16f38` at `0x82b274a4`, both from global `+0xb0` at index
  zero. Only a null attribute pointer calls `DefaultDataArea(4)`
  (`0x82b2740c..0x82b27410`, `0x82b274b0..0x82b274b4`).

The prebuilt read-only `inspect_vault.exe` was run against the extracted
original `skater.vlt` and `skater.bin` (851,568 and 812,520 bytes). Class
`physics_push` is `0xd4d04d2fa87a9a31`; its `default` collection is
`0xd7edbd362d7d2152`, has no parent or cached layout, and has both explicit
inline scalar nodes (`node=0x40`):

| Key | Raw big-endian word | Float |
| --- | --- | ---: |
| `CHANGE_LIMIT` `76feee54a3ad2af5` | `3fc00000` | 1.5 |
| `SPEED_LIMIT` `a9b532ebbed16f38` | `41080000` | 8.5 |

The Rust `PushAttributes::bind` used the correct class, collection and keys,
but `selected_original_asset_classes::selected_additional_classes` did not
admit this class into the selected immutable Vault. `AttributeRuntimeVault::instance`
therefore produced a null instance; both `capture` calls returned `None`, and
the source fallback supplied zero. The same `PushAttributes` owner also feeds
Ground.Fill's pushable-speed check. The root integrator added one selected-class
admission for `physics_push`; no numeric tuning was hardcoded. A new mounted
replay and direct loaded-value assertions are required before claiming restored
board acceleration or original numeric parity.

Confidence: **high** for original asset payload, binding, lookup control flow,
and cause of the zero limit. The post-admission 120-frame mounted replay passed
(`artifacts/post-reboot-push-vault-restored-2026-09-23.log`): the contact frame
read `CHANGE_LIMIT=1.5` and `SPEED_LIMIT=8.5`, clamped the calculated change to
1.5, and queued approximately `[1040.34, 0, 11.46]` force. The selected board
position moved from `[588.86163, -54.343555, 996.5043]` at frame 31 to
`[589.70715, -54.34355, 996.5005]` at frame 119, with final deck speed
approximately 0.95. The corpus now asserts a meaningful positive displacement
and final speed; that assertion passed in
`artifacts/post-reboot-push-movement-asserted-2026-09-23.log`. This confirms selected-host acceleration, but direct native
per-frame force and trajectory parity remains unmeasured.
