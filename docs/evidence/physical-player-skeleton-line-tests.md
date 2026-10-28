# PhysicalPlayer skeleton line tests

Confidence: **high** for source ordering, retained ownership and the selected
zero-gravity, duration-one request graph. Numerical parity remains **medium**:
host estimates/reductions and existing terrain geometry are explicit policies,
not a bitwise Xbox claim. Full Update1 and Update4 scheduling is still pending.

## Source and lifetime

`PhysicalPlayerHiLOD::Update1_ProcessInput` 82e01f98 is vtable82371730 slot3.
It calls Location.Tick1fb8, Jump.Tick1fc0, Landing.Tick1fc8, then
`StartSkeletonLineTests`1fd0 ->82dfe850. The prior PhysOut category is read2008
before Processed.Reset200c. ProcessAnimOut2134 precedes input retrieval2198;
GroundPos follows the prior category at3524/3560. PrepareToolkit36cc precedes
`FillPhysInFromSkeletonLineTests`36d4 ->82dfeca8 and conditional surface-ID
stores36dc..370c, immediately before Skeleton.ProcessData3718. Selector.Update
3a88 precedes the final PhysOut write lock3ae8. These are executable calls;
standalone prior-output/prepare corpus probes do not constitute Update1.

Start launches **new** batches. Fill copies **old retained results**. The only
executable caller of `EndSkeletonLineTests`82dfeb70 is Update4_PostEngineAdjust
at82e01c94. End performs Sync, GetResult0, copy/clear, Release, null per batch,
hips then left then right. Successful copies use groundNormal, not normal.
Failed query results clear position, normal, surface and validity. Fill copies
left/right/hips and updates each foot surface ID only when its result is valid.

Processed.Reset82b39098 clears all three line surfaces. In particular, r7 is
changed to stack var_100 at3932c before393a0/ac loads/stores the LEFT normal.
3927c loads zero;39284/8c/94 store XYZ,39298 stores W through var_F4. The earlier
gJ load39308 does not initialize the left normal. Unit regression covers this.

## Actual inputs and owners

Start reads Skeleton parts23,15,19 before acquiring any batch. This calls
`Skeleton::GetPartTransform`82b45e18, not Part.GetTransform826c3480. Raw5e48
uses the plain Animation Instance+4 and key1409323ffccfd7ea, index0. Missing
attribute -> DefaultDataArea1 byte0. Nonzero byte selects retained physics
record+1f50; zero composes anim-to-world+2d80 with adjusted volume+3040.
The original plain Instance is transferred once from the actual constructor
runtime to SkeletonNormalAssembly, alongside its independently owned physics
animation binding. No second Vault binding is made.

Hips starts at its query translation with velocity(0,-100,0,0). Each toe starts
at query translation plus(0,0.33,0,0), exact 3ea8f5c3, and velocity is
(0,-1.5,0,0) minus that offset. Every request has gravity0, duration1 and both
error parameters0. Each acquisition/Begin/start store precedes its own live
`kFeetLineTestRadius` read. Registration82f9ea08/ref830e0d98/table82fd62bc:
Physics_Systems_OffboardIK/kFeetLineTestRadius/LineTestRadius, default3cf5c28f,
min0 max1, null callback. The getter reads the retained live reference.

The physical pair owns three actual trajectory leases and previous results in
its HiLOD tail. Existing SimulationWorldLineQueries owns the same ground, DMO
and vehicle collections used for trajectory requests; no parallel fake world
exists. World/Island AcquireTrajectory827ab958/827b8998 selects children using
the existing island flags. Constructor-empty DMO/vehicle jobs require both
their actual empty slots and actual zero membership; unpublished membership
returns an error. Geometry/job errors cannot synchronize into successful misses.

## Selected native trajectory graph

Add827bdcc0 checks XYZ for NaN before capacity; full capacity returnsfalse before
state assertion. Local827c2270 calls Walk827c1e58. The source error threshold is
((errorStart*radius)*errorStart)*radius, raw1f78/1fb8/1fdc, not radius squared.
With source error0, a positive chord is tested even when shorter than radius.
The selected g0,T1 path produces one segment, retaining velocity and gravity
FMAs. Existing StaticGroundPool sphere sweep and bounded64 ground-normal cache
are reused over actual resident triangles. XYZ geometry APIs leave W unknown.

CalculateTimeOfCollision827bce08 uses dot3, reciprocal-square-root estimates,
two Newton refinements with negative-rounded-FMA residuals, then distance/speed.
The epsilon gates atcffc..d038 are present in raw code even when decompilation
omits them. Frame=floor(raw ratio), while d080 multiplies the **unrounded ratio**
by fixedDT3c888889. The generic trajectory helper's floored-frame time and
radius-squared threshold are not used here. Numeric tests distinguish both.
Native exceptional vector float-to-signed conversion remains a trace boundary;
ordinary finite source requests exercise the selected graph.

## Validation status

Focused tests cover time/frame distinction, threshold, new-job/old-result lag,
groundNormal selection, failed job retention, cleared reset normals, actual
constructor collection leases and unknown W. The existing original terrain
collision corpus now calls a new child after the same pair's solved history and
adjusted animation targets are produced. It exercises Start -> old Fill -> End
-> next Fill without replacing its pose/history or manufacturing contacts.
An additional explicitly controlled hips-shaped request at the existing park
terrain anchor plus1Y requires a genuine positive terrain hit using these same
world query owners. This independently exercises nonempty geometry even if the
adjusted-pose fixture's three actual requests happen to miss; it does not inject
that controlled request into the retained physical pair's result records.
The corpus controls original pose placement and schedule; it is not a live
Actor frame. Root owns the pending compilation, strict gates, original run and
independent source review. Root's original11 run observed all three actual
skeleton requests hitting terrain surface133 and the separate positive request
hitting terrain. The later IK stage exposed duplicate query-world construction.
The corpus now constructs one shared owner before the first skeleton request:
trajectory capacity3x1, line capacity1x5; later IK and wheel requests reuse it.
The complete original corpus and strict gate require a rerun after this fix.
