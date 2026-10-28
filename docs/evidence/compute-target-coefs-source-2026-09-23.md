# ComputeTargetCoefsFromSpeedAndStrength source and retained push attributes

The raw-A selected Motion child reaches behavior occurrence 2443 after
`SetPushCoefs` Create/Begin. The generated binding is factory `0x82c911c8`
(behavior allocation `0x20`), constructor `0x82c8d860`, Create `0x82bb3da8`,
Begin/End `0x825187c0`, and Update `0x82ca8e18`. Create returns a null
instance; Begin/End are single `blr` callbacks. The constructor reads authored
`regularAttributes` with true default into behavior byte `+0x1c`; occurrence
2443 has false. Update tail branches to `SetCoefs` `0x82ca0520` on every tick.

`SetCoefs` validates cached `ISkaterMotionGraph` (`Actor+0x1f8`), `ISkaterAnim`
(`Actor+0x1fc`), and `IPhysical` (`Actor+0x200`) against the entity interface
list. It obtains the retained PushState through the Motion graph virtual
`+0x38`, then reads `PhysOut.mSkateboardMotion.mSkateboardZSpeed` through
`IPhysical` virtual `+0x28` and clamps it to nonnegative with `fsel`. The
animation virtual `+0x18` is `ISkaterAnim::GetIsSwitch` (PDB vtable), despite
the decompiler's misleading `HasChannel` name from an overlapping interface
type. The method reads `mRelativeStance == 1` at `0x82cb7580`. It selects the
regular shared push block iff `regularAttributes != isSwitch`; otherwise it
selects mongo. The manager virtual `+0x34` returns its retained
`SkaterAnimShared`; there is no per-Update animation tree lookup.

`SkaterAnimShared` is `0x1e0` bytes. Its regular `PushAttributes` is at `+0`,
mongo at `+0x40`; each is `0x40` bytes. Each block has four triples of
`(length, beginVelocity, endVelocity)` at offsets `+0x00` (lsp_hst), `+0x0c`
(hsp_hst), `+0x18` (lsp_lst), `+0x24` (hsp_lst). The final four floats at
`+0x30..0x3c` are min/max begin velocity and min/max end-minus-begin delta.
Constructor `0x825f6ba0` zeros both blocks. `SkaterAnimShared::Init`
`0x82cd48b0` populates regular then mongo through `InitPushAttributes`
`0x82cbf508`, before its wipeout recovery graph. Each block's four named
animations are read in that field order, with the regular names ending
`_N_0_CYC1` and the mongo names ending `_MONGO_0_CYC1`. They are the
`R_PUSHLSP_HSTR`, `R_PUSHHSP_HSTR`, `R_PUSHLSP_LSTR`, and `R_PUSHHSP_LSTR`
variants. `GetPushAttributesForAnim` `0x82cb6d98` constructs each tree from
the original `Skater` playback, reads its length, then scans every 64-byte
`GetAttributes(...,15)` record in order. `HStr_Vel_B` or `LStr_Vel_B` sets
begin velocity; `Vel_E` sets end velocity. Last matching scalar wins; missing
names retain the zeroed field. A matched non-scalar type reaches a native
assert. `InitPushAttributes` derives the bounds with staged `fsubs`/`fsel`
operations and writes begin min, dv min, dv max, begin max in that order.

For finite ordinary inputs, `SetCoefs` first clamps board speed to the
block's begin-velocity bounds and `PushState.mCurrPushDv` (index 1) to the dv
bounds. Within each HSTR/LSTR row it clamps **board speed** between the row's
low- and high-speed begin velocities. Let `L0,V0,E0` and `L1,V1,E1` be the
row's low- and high-speed length, begin velocity, and end velocity. It solves
`t = L0*(V-V0) / (L1*(V1-V) + L0*(V-V0))`, applies the source `fneg`/two
`fsel` unit clamp, then computes blended length
`L = L1*t + L0*(1-t)` and end velocity
`E = (L1*t*E1 + L0*(1-t)*E0)/L`. Source code uses `fmadds`/`fnmsubs` and
`fdivs` in a particular order; the Rust kernel preserves those staged
operations and the repeated unit clamp before the weighted row calculation.
It writes HSTR and LSTR `t` to target PushState indices 5 and 6. Next it
clamps `dv + speed` between LSTR and HSTR blended end velocities and solves
the same length-weighted ratio across the two rows. After two native unit
clamps it writes that strength ratio to target PushState index 7. The writes
are `+0x14`, `+0x18`, `+0x1c` in that order. The callback reads no `dt`.

The selected session now builds these blocks once from the same original
Skater playback and shared animation attribute pool already used by recovery
graph construction. It retains one `Arc` in the selected session and installs
that same `Arc` into the Actor bridge before Motion continuation; a distinct
second install is rejected. The Rust path reuses `AnimationTreePool::length`,
`AnimationTreePool::get_attributes`, `FacePlaybackData`, the mounted
`SkaterAnim` relative stance, retained Motion PushState, and physical z-speed.
Focused synthetic tests cover missing/duplicate scalar attributes, derived
bounds, distinct speed/dv inputs in the solver, and unordered unit-clamp
behavior. These tests do not establish a per-tick Xbox trace. Native repeated
`SkaterAnimShared::Init` can retain earlier values when attributes are absent;
the selected session only supports the one-time initialization path.

Confidence: high for owner/field layout, lifecycle, authored flag, formulas,
and ordered writes; exceptional PPC versus host floating payload identity is
not established.
