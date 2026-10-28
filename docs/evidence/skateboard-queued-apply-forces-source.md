# Queued Skateboard ApplyForces consumer

2026-09-08, bounded read-only debug-XEX source audit.
This packet prepares the actual deck consumer for the recovered front/rear
force producers. No implementation, Cargo execution, original force corpus or
native per-tick trace accompanies this packet. Broader DMO work stays parked.

## Source identity and callers

Healthy IDA instance: `sk82_na_m.xex`, image base `82000000`, original debug
XEX/PDB. SHA256 values cover the half-open original-byte ranges.

| Symbol | Range | SHA256 |
|---|---|---|
| `?ApplyForces@Skateboard@Physics@Sk8@@AAAXXZ` | `82b35380..82b355a0` | `15499ec513913eaf53ffdb17b1e006e4e93b87d25eeddfa1547f44cfec616608` |
| `?UpdatePrePhysics@Skateboard@Physics@Sk8@@QAAXXZ` | `82b37cb8..82b37d20` | `e5e718462b7fb35b43542fc0ccd7b5285bfdbe3d010a55170494bf889ae02886` |
| `?UpdatePostPhysics@Skateboard@Physics@Sk8@@QAA?AW4PostPhysicsResult@123@XZ` | `82b37d20..82b37e90` | `b3c9a8114922683a336f64bb58aafa3a61b727dd9be163cc32efb5ae37042e49` |
| `?AddSkateboardForce@Skateboard@Physics@Sk8@@QAAXW4eSkateboardForceType@23@ABUSkateboardForce@123@@Z` | `82b38ce0..82b38d88` | `0256b541ee65fae2373758d7c83802bd0542f6bf21dd0b46cba2be3f0c15299a` |

The direct code call is `UpdatePrePhysics` at `82b37d04`. A separate xref at
`82403f98` is outside a code function and must not be called a second runtime
caller. The decorated ApplyForces signature takes only `this`; the extra
decompiler parameters are inference artifacts around attribute calls.

UpdatePrePhysics first reads the actual retained Reckoning up vector `+1b0`,
obtains the actual SkateboardBody, and starts its wheel line test (`37cfc`),
then calls ApplyForces. This packet does not replace that preceding wheel
query stage. UpdatePostPhysics resets/rebinds the applied-force queue at
`37e78/7c/80` after the normal/recovery branch. ApplyForces never clears it.

## Layout and retained authority

| Owner field | Offset | Source meaning |
|---|---|---|
| Skateboard.mDeckRigidBody | `+10` | Actual admitted deck body, reread per entry |
| Skateboard.mSkateBoardForcesAdded | `+120` | Retained fixed-vector owner |
| queue begin/end | `+120/+124` | Ascending current entry range |
| Skateboard.mDebugAttributes | `+534` | Own `Attrib::Gen::physicsdeck` instance |
| AppliedSkateboardForce.mForceType | entry `+0` | Stored by Add; not read by Apply |
| AppliedSkateboardForce.mForce.mWorldSpaceForce | entry `+10` | Full four-lane world force |
| AppliedSkateboardForce.mForce.mLocalSpacePos | entry `+20` | Full four-lane local arm |
| AppliedSkateboardForce size/stride | `30` | No force-kind dispatch in this consumer |

PDB `rw::physics::RigidBody` has size `b0`. Cached orientation vectors are
`Ri+40`, `Up+50`, `At+60`; cached inverse inertia is `Ifull+70` and
`Isplt+80`; cached inverse mass is scalar `+7c`. Force XYZ is `+90/+94/+98`,
torque XYZ is `+a0/+a4/+a8`, cooldown is `+ac`. Adjacent words are unrelated:
orientation W lanes contain stasis/inertia/tag fields, Ifull.W is inverse
mass, Isplt.W is body state, force.W is kinematic state, torque.W is cooldown.
Only the listed scalar force/torque/cooldown stores occur.

The existing body pool stores all needed semantic XYZ fields and checked
occurrence authority. Resolve the board's actual Part 6 body rather than
constructing a second rigid body, using serialized metadata or applying a
generic impulse to a replacement accumulator.

## Lazy own-attribute boundary

Apply first compares begin/end (`35394..a0`). An initially empty queue returns
without attribute reads or body access. A nonempty queue then performs:

1. Own `physicsdeck` key **de53599e9c53fa4a**, index zero, through
   `Attrib::Instance::GetAttributePointer` (`82976308`, call `353c4`). A null
   pointer calls `DefaultDataArea(4)` at `353d4`.
2. Load its f32 value at `353dc`. Reload queue begin/end at `353e4/e8`, then
   construct `[+0, value, +0, +0]`. The post-lookup queue range is captured;
   initial nonemptiness is not an eager permanent range snapshot.
3. Iterate that captured range. Even if it became empty during the attribute
   boundary, continue to the final lookup below.
4. AFTER every force/torque/cooldown mutation, own key
   **93d84377c4da2d5b**, index zero, at `35584`. If absent, call
   `DefaultDataArea(1)` at `35594`. There is **no subsequent byte/value read**
   in this compiled function. Do not invent a debug draw or eager bool getter.

`physicsdeck` class key is `9ed291a274453fc0`, matching the existing class
mapping. These calls use the actual Skateboard-owned instance, bound to
collection **550740b00aa8d4ac**, not global `default=d7edbd362d7d2152`.
The constructor recovery below establishes this distinction; sharing the
class or coincidentally equal payloads does not make the bindings equivalent.
No scalar literal replaces the runtime key read. `DefaultDataArea` `828b5340` returns `gDefaultDataArea`;
its first 16 original bytes were independently checked to be zero.

## Correct raw operand decoding

IDA prints misleading high-VA aliases in VMX128 instructions. The raw words
below were decoded using the published
[Xenia VMX128 encoding fields](https://raw.githubusercontent.com/xenia-project/xenia/master/src/xenia/cpu/ppc/ppc_instr.h),
solely as instruction-format documentation. All behavior and bytes come from
the original game; no interpreter or emulator runtime is proposed.

| Site / raw word | Correct operation |
|---|---|
| `35448 / 17dfe835` | v62 = v63 + v61: queued arm plus local Y offset |
| `3545c / 159ce2f1` | v12 = v60 OR v60: retain the original world force |
| `354ac / 153ac8b1` | v9 = v58 * v57: body Ri times arm.x |
| `354b8 / 16dcb8b5` | v54 = v60 * v55: original force times inverse mass |
| `35520 / 143fa0b1` | v1 = v63 * v52: Ifull times torque.x |
| `35524 / 15bff021` | permute v63/v62 using mask in v0 |
| `3552c / 141ff021` | permute v63/v62 using the newly loaded base mask |

Ordinary `vmaddfp D,A,C,B` computes `A*B+C`; `vnmsubfp D,A,C,B`
computes `C-A*B`. The three `vpermwi` immediates `63` at `35454`, `354ec`,
`35510` select **[Y,Z,X,W]**. These facts fix the cross-product direction and
FMA operand order, which should not be guessed from a conventional API name.

## Exact per-entry numerical graph

Below `fma(a,b,c)` is a single f32 fused multiply-add. Other additions and
products round separately to f32. Let `F` be the queued WORLD force, `P` its
LOCAL application arm and `h` the own-attribute value fetched once above.

```text
L = P + [+0,h,+0,+0]                     // full-lane add, 35444/48
A0 = body.Ri * splat(L.x)                // 354ac
A1 = fma(body.Up, splat(L.y), A0)         // 354bc
A  = fma(body.At, splat(L.z), A1)         // 354c8
D  = F * splat(body.inverse_mass)        // 354b8, independent of A

// A is a world-space ARM, not an absolute point. No COM/P subtraction,
// root translation, effective-board substitution or normalization occurs.

positive = A * [F.y,F.z,F.x,F.w]          // 354d8
negative = [A.y,A.z,A.x,A.w]              // 354ec
cross_zxy = positive - negative*F        // fused negative-product 3550c
T = [cross_zxy.y,cross_zxy.z,cross_zxy.x,cross_zxy.w]

T.x = fma(-A.z, F.y, A.y*F.z)
T.y = fma(-A.x, F.z, A.z*F.x)
T.z = fma(-A.y, F.x, A.x*F.y)             // A CROSS F, original force!
```

Torque does **not** cross with `D=F*inverse_mass`; it uses the original force.
No timestep scales this consumer. Force and torque accumulators already have
the existing solver's acceleration semantics.

The two original permutation masks start at `8225c4d0`:

```text
base: 08090a0b 18191a1b 10111213 0c0d0e0f  -> [full.z,split.z,split.x,full.w]
+10: 04050607 14151617 18191a1b 0c0d0e0f  -> [full.y,split.y,split.z,full.w]
```

With `Ifull=[Ixx,Ixy,Ixz]` and `Isplt=[Izz,Iyy,Iyz]`, this yields the
cached WORLD inverse-inertia multiplication:

```text
C0 = [Ixx,Ixy,Ixz]
C1 = [Ixy,Iyy,Iyz]
C2 = [Ixz,Iyz,Izz]
Q0 = C0*T.x                              // 35520
Q1 = fma(C1,T.y,Q0)                      // 35530
Q  = fma(C2,T.z,Q1)                      // 35534
```

This is not three independent diagonal multiplications. No body quaternion
rotation or tensor refresh occurs: Apply consumes the existing cached tensor.
Full-lane operations are present, but the YZXW permutations do not route W
into XYZ, and the tensor masks route only semantic tensor XYZ into output
XYZ. Thus unknown adjacent pointer/state lanes must not be invented, but their
absence does not prevent computing the source-consumed force/torque XYZ.
Do not accidentally turn scalar stores into a four-lane write over kinematic
state or cooldown.

## Read, arithmetic and commit ordering

For each entry, source reloads `mDeckRigidBody` at `35438`, loads its local
arm and world force (`35444/4c`), reads inverse mass (`35460`), and reads
orientation Ri/Up/At (`35474/7c/84`). Old force XYZ is loaded at
`3548c/94/9c`. The local iterator advances one `30`-byte record at `35480`;
there is no queue mutation and the captured end controls the next iteration.

Force additions are evaluated Z then Y then X (`354d4/dc/e0`). Their exact
operand orders are `oldForce.z + D.z`, `oldForce.y + D.y`, and
`D.x + oldForce.x`. Stores commit **X, Y, Z** at `354e4/e8/f0`.

Only AFTER those stores does source load cached Isplt (`354f8`), Ifull
(`354fc`) and old torque Y/Z/X (`35500/04/08`). The cross/inertia graph above
finishes. Torque sums evaluate Z then Y (`35548/4c`), commit **Y** at
`35550`, evaluate X at `35554`, then commit **X, Z** at `35558/5c`.
Each sum uses `oldTorque.axis + Q.axis`. Cooldown commits zero at `35560`.
The next entry starts only after this complete prefix. Type/kind, body state,
kinematic value, stasis, velocity, omega, COM and orientation are not written.

There are no native calls inside the arithmetic loop. A checked Rust body
adapter must not preflight a late inertia/torque failure before the earlier
force stores or stage all entries atomically. A failure at the initial own
scalar lookup leaves bodies untouched; a later entry failure keeps earlier
entries; a final own lookup failure preserves all body effects and the queue.
The queue remains populated on success and failure. Repeated explicit calls
apply it repeatedly until the source postphysics owner clears it. No implicit
retry, replay or rollback is warranted.

## Minimal implementation contract and validation

The needed consumer can be a small retained full-lane queue owner plus a
checked actual-deck-body adapter. Use the actual `physicsdeck` instance and
separate initial scalar/final lookup-only operations. The source foot producer
packet supplies the full world force and local position; the old XYZ-only
`physics::SkateboardForceQueue` is not sufficient evidence of that handoff.

The existing skeleton displacement consumer is a different graph and cannot
replace force-at-local-arm application. Existing body pool semantic fields
are sufficient for a focused mutation operation; keep shared implementation
ownership with root. Native wheel queries before Apply and postphysics queue
clear remain separately scheduled dependencies.

Recommended discriminators before integration:

- Empty queue makes no owner/attribute reads; final lookup is still performed
  after the post-first-attribute captured range becomes empty if aliases are
  supported. Otherwise state the immutable queue/attribute callback boundary.
- Non-axis orientation, nonzero local-Y offset and off-diagonal inverse tensor
  distinguish arm rotation, cross direction and full tensor multiplication.
- Nonunit inverse mass distinguishes original-force torque from incorrectly
  scaled-force torque; nonzero old accumulators distinguish ordered additions.
- Two entries apply in insertion order and reread the same actual body; a
  failure on the second retains the first, and final-lookup refusal retains all.
- Unknown/full sentinel W lanes leave semantic XYZ correct without modifying
  adjacent kinematic/state/pointer words; cooldown changes only at its store.
- Repeated Apply before postphysics repeats effects; cap21 applies at Add,
  with later inserts ignored, not at Apply. Preserve the actual queue owner.
- Original Vault tests must validate both own keys/schema/missing fallback,
  and original actual-body corpus tests must compare force, torque, cooldown,
  unchanged unrelated slots and solver-visible effects. Deterministic native
  per-tick traces remain the final numeric/timing parity gate.

Confidence **exact** for raw call/branch/load/store sites, layouts, masks,
key identities and decoded arithmetic graph; **high** for the proposed typed
actual-deck mapping. No implementation or native exceptional-value/denormal
parity claim is made. Separate independent review is still required.

Related packet: [front/rear foot forces](skateboard-ground-foot-forces-source.md).

## Own instance and queue construction/lifetime addendum

Fresh read-only constructor recovery, 2026-09-08. The actual outer
`Skateboard::Skateboard` is `82b38918..82b38c30`, SHA256
`549f427fc9d4992c37323a8f70b6d3bbadd88502f7434bd8ac3ebd6d98d8792f`.
Its decorated symbol is
`??0Skateboard@Physics@Sk8@@QAA@PAUIPhysical@12@PAVEntity@2@PBUProcessedPhysIn@12@PAVReckoning@12@PAVPumping@12@ABVMatrix44Affine@vpu@math@rw@@@Z`.
`PhysicalPlayerHiLOD` constructs it at `82e0413c`. The `82404038` metadata
xref is not a second code caller. Inputs are actual physical/Entity/processed/
Reckoning/Pumping collaborators and the passed placement matrix.

At `82b38b5c`, the raw argument producers are:

| Source construction | Result |
|---|---|
| r3 low `74453fc0` at `38ae4`, high `9ed291a2` at `38aec`, `insrdi` at `38afc` | class **9ed291a274453fc0** |
| r4 low `0aa8d4ac` at `38acc`, high `550740b0` at `38adc`, `insrdi` at `38af4` | collection **550740b00aa8d4ac** |

The call is `Attrib::FindCollection` `82976528`. Its raw code preserves r4
as the collection key, passes original r3 as class key to Database::GetClass
at `976548`, then passes the saved collection key to Class::GetCollection at
`976558`. IDA's displayed 64-bit argument expressions are wrong at both
caller and helper and must not override these raw registers. No lookup of
global default or fallback-to-default occurs here. This packet does not guess
the human-readable name of collection `550740b00aa8d4ac`.

At `38b60` the returned collection pointer becomes r4; r3 is own `this+534`
at `38b64`; r5 is message-port zero at `38b68`; `Attrib::Instance::Instance`
`82975bb0` is called at `38b6c`. That constructor stores the exact collection
pointer and msgPort, clears cached layout and flags, and for nonnull collection
copies its layout and increments its reference count. A null collection stays
a null instance, allowing the already recovered missing-attribute default
behavior at later Apply calls. There is no claim that binding should reject
absence or silently select another collection.

Queue initialization precedes this own-attribute binding:

1. Inline storage begins at `this+140`; size `3f0` holds 21 records of `30`
   bytes. The one-past-capacity address is `this+530`.
2. Stores `38b40/44/48/4c` set allocator poolBegin (`+130`), end (`+124`),
   begin (`+120`) and capacity (`+128`) to those inline addresses. Thus the
   actual queue is already constructed empty before FindCollection/Instance.
3. Own instance is bound at `38b5c/6c`, before SpeedWobble allocation/
   construction (`38b8c/98`) and actual SkateboardBody construction (`38bd4`).
4. After retaining the body and marking `mRemainOnReset`, source resolves its
   actual assembly Part 6 body (`partList+28c`, six `60`-byte Parts plus body
   offset `4c`) and stores `mDeckRigidBody` at `38c10`.
5. Source resets end, begin and capacity again at `38c1c/20/24`. This is the
   same inline queue; neither a new force collection nor copied body authority
   is introduced. A late construction failure must not erase the earlier
   queue/own-instance prefix.

AddSkateboardForce's cap21 check happens before record payload copies.
Available inline insertion increments end before copying kind and the two
full vectors. The generic capacity-exhaustion branch calls EASTL DoInsertValue
when count is below21 but capacity is exhausted; under the ordinary inline
constructor invariants capacity is21 and the guard prevents overflowing it.
Do not infer arbitrary overflow allocation is exercised in that normal path.

Destructor `82b352c8..82b3537c`, SHA256
`5aa6213a6fa587cb416111e2a81aea0cdce17b06965f8b134f85ca3fa755e685`, destroys
SpeedWobble first, then the actual SkateboardBody, then own debug attributes
(`3532c`). It frees queue begin only if nonnull and different from allocator
poolBegin (`35330..48`), then removes the UCOM interface. It never applies
remaining queued forces. Native PostPhysics clear remains the established
`37e78/7c/80` operation on the same owner.

Current `SkateboardBodyConstruction` and `SkateboardAttributes::bind` produce
the admitted physical board and its global-default construction attributes.
`PhysicalPlayerAssemblyConstruction` supplies paired assembly ownership, but
does not thereby implement the complete outer `Skateboard` constructor,
Reckoning/Pumping lifetime, own `+534` instance or applied-force queue.
A bounded new queue/attribute projection must be retained by the actual board
owner at its explicit integration stage. It must bind the exact collection
above; reusing `SkateboardAttributes` as the Apply attribute owner would be a
behavioral mismatch. The immutable runtime Vault retains the selected binding
and lacks native database/editor reload and alias-driven queue mutation; those
limits must remain explicit rather than treated as a live constructor claim.
## Original own collection values (2026-09-08 follow-up)

Read-only existing `target/debug/examples/inspect_vault.exe` inspected original
extracted `skater.vlt/bin` in the temporary `skate2-foot-vault` directory.
The own collection `550740b00aa8d4ac` in class `9ed291a274453fc0` has
**parent zero**, with only three explicit entries: `59d5044a7283feb7`,
`93d84377c4da2d5b`, `bee65fce9c7183ed`. Its collection header is at VLT
`5b2f0`. Therefore initial key `de53599e9c53fa4a` is absent and the original
own-instance read resolves to **positive zero**, via `DefaultDataArea(4)`.
The global default's same key is at `5b1c0`, inline word `3d4ccccd` at
`5b1c8`; that approximately 0.05 value is not inherited by the own collection.

The final lookup key is an explicit inline own node at `5b350`, word
`00000000` at `5b358`, type index1/node flags40/entry flags00. Its definition
is scalar size1, maximum_count1, flags4, alignment0, type id
`15c52cb4069556a3` (decimal1568709196924671651). The consumer does not read
that byte. Raw-byte scanning independently confirmed the header and node
offsets. No parent chain exists for this original own collection; do not
substitute the global default's nonzero offset in the mounted force oracle.
