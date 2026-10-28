# ComputeRepushDeadline End source

The raw-A continuation enters PushContact/Right at step 70 and reaches Motion
behavior occurrence 2352, `ComputeRepushDeadline`. Its factory `0x82c910a8`
allocates a `0x20` behavior and constructor `0x82c8d8d0` reads
`regularAttributes` (default true) into byte `+0x1c`. Occurrence 2352 has
`regularAttributes=false`; the Left occurrence 2355 has true. Create
`0x82bb3da8` returns a null instance. Begin and Update are the shared
single-`blr` `0x825187c0`. **End** `0x82ca0d60` performs the calculation.

End validates cached `ISkaterAnim` (`Actor+0x1fc`), `ISkaterMotionGraph`
(`Actor+0x1f8`), and `IPhysical` (`Actor+0x200`), then gets the retained
PushState via graph virtual `+0x38`. It calls `ISkaterAnim::GetIsSwitch`
virtual `+0x18` (the decompiler's `HasChannel` label uses the wrong union
interface type) and selects regular `SkaterAnimShared::PushAttributes` at `+0`
iff `regularAttributes != isSwitch`; otherwise mongo at `+0x40`. These same
retained blocks were constructed before the selected recovery graph. End
reads `PushState.mCurrPushDv` at `+4`, then `PhysOut.mSkateboardMotion` z-speed
through `IPhysical` virtual `+0x28` and clamps speed nonnegative with `fsel`.

Let `Bmin=block[12]`, `Bmax=block[13]`, `Dmin=block[14]`, `Dmax=block[15]`,
`dv=PushState[1]`, and `speed=max(boardZSpeed,0)`. For ordinary finite inputs,
End computes

```
u = clamp01((clamp(dv,Dmin,Dmax)-Dmin)/(Dmax-Dmin))
v = clamp01((clamp(speed,Bmin+Dmin,Bmax)-(Bmin+Dmin))/(Bmax-(Bmin+Dmin)))
mix = (1-weight)*u + weight*v
PushState[0] = min(cap,max(0,mix))
```

This is a description of the finite case, not an instruction to sanitize
degenerate inputs. Native code uses `fdivs`, `fneg`/two-`fsel` unit clamps,
`fmadds` for the weighted mix, and `fsel` for the final lower/cap bounds.
The Rust kernel retains those operations and allows zero denominators to
follow the native unordered selection path. No `dt` is read.

The two fractions are computed before either Vault lookup. The cap is a fresh scalar lookup through the existing global
`anim_motion_pushing` `+0x13c` collection, key
`0x792c_66a0_ddc0_2df7` at `0x82ca1118`; the weight is a second fresh lookup,
key `0x1761_c0ae_4c9e_79c2` at `0x82ca1170`. Missing pointers use
`DefaultDataArea(4)` (zero). The cap value is read before the weight lookup;
the final store to `PushState.mOutFactor` (+0) occurs only after both reads,
at `0x82ca11a4`. The callback does not clamp cap or weight themselves.

Rust reuses the existing Actor, Motion PushState, physical z-speed, mounted
SkaterAnim stance, retained `SkaterPushSharedAttributes`, and
`PushingMotionAttributes` Vault owner. The generated Motion lifecycle now
admits null Create and empty Begin/Update and dispatches End on exit, while
retaining the normal behavior identity and active-set checks. Two focused
tests pass for mixed/capped values and unordered selection. No Xbox per-tick
trace yet establishes bit-identical exceptional PPC/host float behavior.
Confidence: high for lifecycle, owner/field layout, scalar keys, finite
formula, and write timing; exceptional floating payload identity remains
unproven.

The first 120-frame raw-A replay after End was added passed that callback but
stopped on the selected `PushContact/Right1280` update thunk `0x828f4648`.
IDA shows it calls Push1028 and then SetPushCoefs, ComputeTargetCoefs,
ComputeRepushDeadline's empty Update, and PlayAnimation. The loaded parent
PushContact1180 owns behavior **2180**, while the leaf Right1280 owns
2351..2353; Left1281 owns 2354..2356. An initial local expectation used
2350 for the parent, but that belongs to a different PushStart leaf. The
retained active-set trace exposed the mismatch; correcting it advanced the
same raw-A replay to frame 74, where `PushCycle::CreateInstance` 0x82caf9b0
is the next explicit pending boundary. See
`artifacts/raw-a-push-after-parent2180-2026-09-23.log`. This is source-backed
state dispatch, not a second implementation of the repush formula.
