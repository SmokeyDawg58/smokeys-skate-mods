# Pedestrian factory construction and model initialization

Source inspection only; no compilation, tests, or game execution performed.

`Pedestrian82e97748` reserves core1, intent4, state5, animation6,
post-adjust7, head8, spatial9, then NavRig3. Animation is loaded before the
adjust owners. Its filename is the selected OverrideAnimationData or
EntityAnimationData layout's first pointer (82e97f64..88), with the actual
test-motion cvar override at82e97f90. A hardcoded generated motion load alone
does not prove completion of this constructor.

The new retained factory function uses the existing memory owner and global
entity ID allocator, keeps all partial construction in the caller's Option,
and never publishes into EntityMan. Animation construction/loading remains a
required fallible callback accepting the genuinely borrowed physics world.
No duplicate backend is created to satisfy the existing animation Arc seam.
The native constructor suffix now performs state Reset, reverse category
lookup, authored handprop selection and positioning test height. BigBlack
construction remains an explicit error. Guard construction extends this same
base entity using its existing security data owner.

`InitModels82e78580` sets recipe and draws authored model colours before
ModelMan lookup; Collideable.Init82e605a8 creates the collision owner,
constructs actual bodies and registers their Assemblies before admission.
Raw PPC82e60624/658 reads vtable offsets20/28. PDB
`Sk8::LivingWorld::cLivingWorldCollision_vtbl` identifies these as
GetWorldCollisionBodyHiLOD_2 and GetSkaterCollisionBody_2, proving world then
skater registration. The implementation uses the existing candidate handles
and CollideableObjectMan, without another assembly registry.

The AI graph filename uses full keyca4d0ad4ea45965b. InitModels invokes a
required fallible real graph binder after collision creation; it does not
replace the graph with a default controller. Factory filename routing follows
Create82951424: last slash takes precedence, backslash only if no slash exists,
and basename comparison is case-sensitive.

Spawn82e9a510 increments the motion pool index after factory construction,
calls InitModels, destroys the entity if the collision's model is null, then
sets the actual mesh cell/target transform, admits models, sets allow-cull,
initializes contained props and invokes PostInitialize. These placement and
publication steps are not claimed implemented by the new constructor prefix.
The entity model is now nullable and starts absent until InitModels, which
assigns the actual ModelMan lookup result. No model is passed into constructor
or factory as a placeholder. Sphere and volume consumers reject an absent
model explicitly. InitModels also returns model presence so Spawn can follow
its real destruction branch.

Reverse category lookup82e7f4e0 uses the shared Vault's sorted leaf enumeration
for classadcb4d6bd3b4f85e and membersd5e1267e2d715124. It scans every entry,
retains the last matching category and leaves the field untouched if no match
exists. Raw82e7f5cc..5d4 uses ld/ld/cmpld: full64-bit keys, despite the
decompiler's misleading LODWORD comparison. Unwritten category state is Option
rather than an invented zero. RefSpec key projections avoid relocated cache
pointers. State.Reset82e981cc runs after NavRig and before this lookup.

The actual mounted Vault inspected read-only through existing vault_dump.py
(`%TEMP%/skate-ped-vault/data/db`) declares handprop list46b836ee959c0238 with
typee4aac2eabc053d0a,size32,flags25. This is NOT the similarly laid-out
tLWGroupEntry type. Typed key+8 and probability+24 projections avoid its
relocated RefSpec pointer. PDB TypeOf_HandPropSpawnProbabilityList confirms
RefSpec0 and Probability24. Ctor caps the list at32 and uses the existing
scalar census discrete-choice kernel (same native82704880), preserving its
NaN comparison and empty-list draw. The global allow cvar and creation flag
both precede Bernoulli on key3db019a08284f45c; no draw occurs if either gate
is disabled. A selected collection binds actual handprop classec8b756546e226f6
and sets AI bytecab bit2, else clears that bit. This is planned handprop state,
not a fabricated held Prop entity. Height is read from the live default-up
cvar last, matching82e98604.

PedestrianAnimConstructInfo retains the existing animation bind-pose and
hierarchy references, with nullable pre-Init state. Manager ctor82722008..10
clears these and sets pool0; Init8271f83c/844 assigns AnimMan references without
resetting the index. Increment826f55a8 computes ((old-1)&1)+2, hence successful
construction pools0,3,2,3,2. Spawn advances immediately after successful
CreateEntity and before InitModels; later model rejection does not undo it.
