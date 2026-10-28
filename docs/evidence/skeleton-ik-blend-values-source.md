# IK blend values: scalar graph and retained input order

Root source audit, 2026-09-08. This is an implementation-ready dependency
packet for active pre-physics IK, not an accepted implementation. IDA raw words
and scalar branches were read directly; independent review remains pending.

`SkeletonIK::UpdateBlendValues` spans `82b51d38..82b520cc`, SHA256
`9d29836f958876c9d4fcf0ccf3aee51d1e644c91d771ad2a0883301256526ec4`.
Its caller is the second stage of Update at59c78, after Status and before
CalculateInitialPartTransforms. It reads prior retained PartPos, not newly
calculated positions. Source types and the complete IK path are mapped in
`skeleton-general-ik-next-source.md`.

## Reads before any blend write

Capture deck box half width at IK+c60 and total half length at+c68, then build
full4 `[width, +0, length, +0]`. Through the SAME retained IK attribute instance,
read these keys in order, all index0 (raw r5 set at1d8c/1dd0/1e14/1e50):

| Key | Value | Null fallback |
|---|---|---|
| aa378f2ebf7aa313 | full4 extent addition | DefaultDataArea16 |
| 7ce328814f9309ae | full4 falloff range | DefaultDataArea16 |
| 39fa8c07e8356284 | on-deck blend step | DefaultDataArea4 |
| 094ace5de06851fc | external-local blend step | DefaultDataArea4 |

First extent addition occurs before the second lookup, and expanded extent
addition occurs before the third lookup. All are local; any of the four read
failures leaves every retained blend unchanged. Even four Disabled modes do
not skip these reads. Default bytes and runtime provider behavior must remain
the actual retained Vault service, not hardcoded corpus tuning.

Corrected VMX128 VA at1dcc is62, not printed94; at1dfc it is127. The first
addition is base+offset, second is extent+range, separately rounded full4.
Only resulting XYZ scalars participate below; W never affects geometry.
Scalar assignments are extent X=f28/Y=f26/Z=f27, range X=f23/Y=f24/Z=f25,
expanded X=f20/Y=f21/Z=f22, on-deck step=f30, external-local step=f19.
Positive zero and one come from literals loaded at1d7c and1eac.

## Four retained entries, ascending order

The loop advances blend pointer by4 and prior PartPos by16. On-deck blend
starts IK+1d4, external blend is+1e4, mode+1f4, prior PartPos+780,
external target blend+32c, external-set flags+324 and local-set+328.
Modes other than1/2/3 leave the entry unchanged after the required reads.

For the formulas below, `select(t,a,b)` chooses `a` when `t >= 0`, including
either zero sign, and `b` for negative or unordered. Every subtraction/addition/
division explicitly rounds to f32; there is no fused scalar arithmetic here.
Sign negation is distinct from subtraction from zero.

Define decay(value,step) as:

```text
remaining = value - step
select(-remaining, +0, remaining)
```

External mode2 asserts external-set before any current-entry write (1ec8..dc).
It captures old on-deck minus step, stores external blend from target blend
at1eec, then stores decayed on-deck at1ef8. Assertion refusal preserves current
and later entries while earlier completed entries remain.

ExternalLocal mode3 first stores external blend: one if local-set (1f18),
otherwise decay with the external-local step (1f24..30). It then separately
decays on-deck with the on-deck step and stores at1f44.

OnDeck mode1 feet0/1 read retained processed flags+8d0 at2060. Bit28 set selects
target+0; clear selects one. Hands2/3 use the prior PartPos procedure below.
No elapsed-time multiplier is applied to either attribute step.

## Hand target, including selection asymmetry

Absolute coordinates are obtained by clearing the sign bit of prior XYZ.
Raw1f84/8c/94 forms sign masks from all-ones shifted by31; corrected VA is60.
The three AND-complement sources at1f98/9c/a0 are58/57/56, the Z/X/Y splats.
The resulting scalar values are absX=f13, absY=f10, absZ=f12. Unknown or NaN W
is never consulted.

Proceed if old on-deck blend is ordered greater than zero (1fbc). Otherwise
require absX<expandedX, then absY<expandedY, then absZ<expandedZ. These reject
branches at1fc4/cc/d4 are BO4/BI24, meaning LT-not-set and including unordered.
Failure leaves the hand's old blend untouched. A positive old blend bypasses
all three box tests, including when a coordinate is NaN.

Start target at one. Each axis calculation has its own ordered-GT eligibility:

```text
if absX > extentX:
    target = 1 - ((absX - extentX) / rangeX)
if absZ > extentZ:
    z = 1 - ((absZ - extentZ) / rangeZ)
    target = select(target - z, z, target)
if absY > extentY:
    y = 1 - ((absY - extentY) / rangeY)
    target = select(y - target, target, y)
```

BO4/BI25 at1fe0/1ff4/2010 skips the calculation when GT is not set, including
unordered. Preserve X, Z, Y order. The two minimum-like selections have
different unordered behavior: a NaN Z candidate leaves target, while a NaN Y
candidate replaces it. Neither generic min nor generic clamp preserves this.

Then clamp by the exact two selections at2030/2038:

```text
lower = select(-target, +0, target)
bounded_target = select(1 - lower, lower, 1)
```

A NaN lower result therefore selects one in the second expression. Do not
sanitize earlier inputs or rewrite the selection sequence.

## Common approach and store

Both foot and eligible hand use the same separately rounded step limitation:

```text
negative_step = -on_deck_step
delta = target - old_blend
lower_delta = select(negative_step - delta, negative_step, delta)
bounded_delta = select(on_deck_step - lower_delta, lower_delta, on_deck_step)
new_blend = bounded_delta + old_blend
```

Hand stores at2054; foot at209c. Negative/nonfinite step, old blend and target
values receive no extra gate. Four iterations complete before returning.
No cvar, numeric service, collision query or pose mutation occurs here.

## Implementation and validation requirements

Extend the single retained IK owner, reusing its target/mode/external history
and actual Part-order pose authorities. The prior PartPos input must remain
retained across stages. Read the original scalar/vector attributes lazily from
its actual IK instance; all four read failures precede blend writes.

Focused tests should distinguish each attribute failure, late external-set
assertion, foot bit28, prior-position timing, strict box boundary and NaN
rejection/bypass, X/Z/Y selection asymmetry, signed-zero decay, negative or
NaN step and invalid mode. The original-data test should independently read
all four payloads and carry these blends through Initial/BlendTransforms/
Contact/DriveFrames before claiming the active IK path integrated.

Confidence high for the recovered raw graph, field/store/read order and typed
input mapping. Host exceptional-result payload/FP-status parity and the full
active IK integration remain unproven. No new Rust was written by this audit.
