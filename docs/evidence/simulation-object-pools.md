# Simulation body, Joint, and Drive pools

Date: 2026-09-05 (Europe/Helsinki)

This checkpoint reconstructs the fixed-capacity object pools owned by
`rw::physics::Simulation`. It covers pool construction, body creation/reset/
removal, Joint creation/removal, Drive creation/removal, and the recovered
Drive-build admission gate. The recovered Assembly transaction is now wired to
these pools through a concrete safe owner. A separately bounded foundation now
freezes the Drive-Jacobian value layout, exact RQD helper, coefficient graph,
and HARD magnitude clamp. It does not claim the complete Jacobian builder,
complete constraint solver, contact generation, island scheduling, or
simulated motion.

## Provenance

The reference inputs remain:

- XEX SHA-256
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- matching PDB SHA-256
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`;
- matching MAP SHA-256
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.

Reproducible extraction packets in `world_probe_temp`:

| Slice | Evidence SHA-256 | Raw/extractor/oracle SHA-256 |
| --- | --- | --- |
| `Simulation::Initialize` | `71B67B606CD3DDDF12033F1B54A7709B4E163CC9DE149EB331DB0B8BB3A4C0E3` | raw `8D17A696733EA988C5F92C9446362271040FA5ECA817BFEE9DBF5824D3A9AD35`; extractor `27014F8485AB03398C50D84E473FC55C0F6683B77FB13982C9F2C0E126BBAB17`; layout oracle `4553E68A49910DDA43A1562F2B0112360E95A38A58005E204DC1D11A74386672` |
| `Simulation::AddRigidBody` | `9341172B429C3D7866C1075E9C9A732A8CA22C932A6D5B2348DD17F6540A8EC8` | raw `4F46E8D55F2B2F4985A8833CC831624A946FA2CFBC15765B8F2D9B9DE2974D69`; extractor `54BB8FAF56A65033A4D11E35EA24622755E007DB2548AE7AD14F71CF686EE0E4` |
| `RigidBody::Reset` | `31735307DAA1D7E24511EED07A4C7EB7F0D33008578B25A36A454C88C2ABB492` | discriminator source `52BA02B39E3523B7E91002CD519653D5DBCF3FCC6609A62C7FCB1CB393FAE548`; output `7B145C5F696353308360628011BE42085C4F9195715C3CF30B2E7374E2486827` |
| `Simulation::RemoveRigidBody` | `2FF351D84384A93BD69725DBA8815A1FF331C90A80D1D9E302A2A1DDC2139661` | raw `DDD9FCD1440586C69129244B4916D9F6CAD05107FDC61A5184DB8722301607EF`; extractor `E1277503A37F57431041C7FA9EE1A9F6A9EF6F28BDED5DC41A0DA565B39275F1` |
| Simulation tick RF storage/traversal | `72FA66B457974B96DF8871CAF9DB167117917575154E75DCE3E06F9E7ACF7F32` | raw `638BE77E79E3C441CA3EEA69F4070915B8E47C28C6B91DFBB5326EB24AD08554`; extractor `D229371C02D49EDB05B27325F7767332FF60395B1324BFE83552267C5A3DEE38`; RF scan `C68E439D61037AF454DE7AE4A9A9C63536C7B434E7CF56B1E8DE62821240DD99` |
| Joint pool | `F01D1B6EC25988B5A3C7C6877B7211FA62ED400D7BD1EA016C0579BF95352387` | raw `E27AD9FFCB48D0B63A34C1E3CFAE65AB368B2B3CB9D7524C28877CB86CAF45DC`; extractor `52664EA3C292B4EA2D846446FE49695B77D3D2057E346068E0C4667BFE7EC7BD`; oracle source `380BC1E4613EDC1F244EEC8B7ACFC60DAC3C432A0424688096E6BD7400D6C111`; output `420F5B9FA94AE6AC41C295E05B26845E060E8217A0A27EF512BE81C9F8976C03` |
| Drive pool | `AE4AA3787723D74C1CD46871029DFF56E206045972A70BEF36FBE686999744E5` | raw `7FF8BAD2E0FF7D9A5FFFB65FC0B0BC18D63F57C36DAC1FB6F854A6BDA64EED82`; extractor `D101BC262A3E1B75CE651EA12B9ED2B505B99A868C6E8653CDE4D2C850B10A1D`; FIFO oracle `61CD450AD16FF7C7338928CABCC45FF9820C0ED4388C43F7B62E28F0B51F9CBA` |
| `DriveJacobian::Build` foundation | `D9A67A8A1B42B94760DADB592FFEF4591468638C9212EE3AAEEA395B7F9A80A8` | raw `34BDB7600B52ED587D20D9B14066B1B7FA1C4D884F5BA40BA596372464025638`; oracle source `71F0D348CCD6D68B3D40361A30E010BBF812F069CFCA2D58D1FDF2032048D889`; output `AF38D45224B2AEA3842B23565E67AF654B088C29D35FB68670404884BE60A693` |
| `DriveJacobian::Build` geometry continuation | `7FD9C7CA85226DDADB98E73FDF5CD32554667C17512F46A4172A88436F46F8B2` | raw foundation above; geometry oracle source `8548463AC3D5E0C2ADEF6439F77B9E0FFFE35F7539ACD0711D08EB45EA5618DB`; output `EA55588C68981F228EDD9684D7F70ABA488A4A36BB90572AD9989B6B29549626`; fused-search source `C968E58C15BCB26AF8C4AA3545D14D07D77BFA79F891DF5E2E67D4AD65ED4868` |
| `DriveJacobian::Build` angular continuation | `CB149DDC76661334A452AE31CEAD2A9069425BA870842F717D2FB1832167E193` | blueprint `0195B5F7E5081F995BB609DDD298BCAE93C4B380E6000479F273F56E1CE7E1F1`; oracle source `46952FF475FD17F8C789DDDB8B5F682202E463C4C8ACC47BE97C173B5411CAB6`; output `5C3999FD69C1756FABF77F18ACF3861F8163633C4C9F372DFEBE6B2018CEDD22` |
| `DriveJacobian::Build` final-output continuation | `0D32D1D5DD767D4F9E85B6C046C46B1897EFE89795F7E26909FDFC96E6514D25` | independent packet `346E01705BDF99281CF8FC29397860E2A724B33512CE49AD9D4469BEE016A0FF`; raw `452F5B3F0B8A76232B553F45CB2EE3C8BAE71DDFA4A86A201E12811BDCB9EF94`; extractor `E8490DDF3FCDF9BA1A29D43299C5776688B531E48EF7CCCC4AA6A7B2A74A5BD5`; oracle source `C062B983F4EF8BF207E08824F7BC0A2437CB464E24CF86404745680F8E9564A9`; output `6AEB83671E9A04DF0874E94FCB57DE930917A3334E614BF2D18C1F3E780F6FC2` |
| complete pure Drive Build composer | `6EB127BD511479121175E470A1723E16688073810C0BF56020F25CE9B33FE9CD` | oracle source `7A5466F9B53E58D988C921CC695591D15184FC261D4E85E661F81F02FC61528B`; output `C6301F53A09A06EDFC955B45CEB4EE657DEE249184C79A7D75885B4D7D4343B3` |
| `Simulation::DriveBatchBuild` | `09D683032CF7D4FB83B4B94000A11A086C7659F52C8FDAB06CB05FA5084DC8BC` | raw `93DBD24BCB9A047F1160A6E483E1AB169987F6EBD1FCE9472FED085B6972540F`; extractor `AFF29817F6E90C245A2AA3FCD7C819EE3AE3A016E5F6DCD2EFB8CE75BB286545`; oracle source `F9CBEC645FD7985F96D0BCDF3CB6C58E455AE1D9AF93F0064C75867321643EC5`; output `C9EEDA3A369FDE1D405DCDA36F8B20DE3C59BF03F2D298BF419FBBC3132B9121` |
| direct Horus Drive consumer | `7AAA1105D7E974CE3411F707FEB5D98432D3941FEF5610CC9A5DC0726E8FA15D` | raw `2544B6BB5EF9DC874A27B3B8B096CEDAB665C4DDD28FA23A4D247DC8562F1961`; decoder `2D5E8B4C7A9EBFF8C74BE01EDF938933F63E4C4FC49361088659536C80618CC4`; decoded `33AA5FBC7069B2D2C0EDA5AEB79DAAB35AA4DF388876A0039A076E922C597305`; corrected oracle source `9F84EBFEC378E134731A7DF5BA668594749DA622504852D67DE55933885BCE2A`; output `1325F62D15E0C9FA47C1883ABE2A03EA6F3122CFDBB03F21112055315FC64775` |

These files are descriptive provenance, not runtime dependencies.

## Native storage topology

`Simulation::Initialize` is `0x82ae7b10..0x82ae7edc`. One caller-owned arena
contains the `0x300`-byte Simulation, `N` 64-byte RF rows, four `0xb0` body
sentinels, `N` usable bodies, two `0x20` Joint sentinels plus `J` slots, and two
`0x20` Drive sentinels plus `D` slots. Destruction frees the arena once; there
are no per-slot allocations or destructors.

Body lists are Free, Static, Frozen, and Active. Joint and Drive lists are Free
and Active. Every free list begins in ascending slot-address order. Acquisition
always takes the Free head; destination insertion and removal-to-Free both
append at the tail. Removal followed by acquisition therefore produces FIFO
reuse.

The PDB names body `+0x1c` as `mId`, but the executable stores the address of
RF row `i` there. It is a runtime pointer to four 16-byte solver-delta vectors,
not a logical body ID. The host keeps a separate 0x40-byte, 64-byte-aligned RF
row per slot and uses an explicitly unrelated typed slot identity; it never
truncates a 64-bit pointer into the native 32-bit field or exposes RF mutation
as public body/removal authority.

Initialize aligns the RF base to 0x40 at `0x82ae7b30..0x82ae7b3c` and clears
the complete RF table once at `0x82ae7b80..0x82ae7bb0`.
Reset, Add, Remove, and FIFO slot reuse preserve each row's bits; native
`RigidBody::DynamicUpdate` consumes and clears all four vectors later at
`0x82ae48d0..0x82ae48dc`. The current crate-private, occurrence-qualified
access and Active-ring visitor preserve that lifetime and traversal boundary.
The bounded ordinary-finite Active integration seam now consumes and clears
them; it does not implement Jacobian generation, solving, or a complete tick.

Initialize has a verified source quirk: its `N` calls to `RigidBody::Reset`
start at the first body sentinel. For `N >= 4` they reset all four sentinels,
then the first `N - 4` usable records, and miss the final four usable records.
For `N < 4` they reset `N` sentinels and miss every usable record. Every
acquired body is reset before use. The safe host initializes all semantic
payload up front and does not expose allocator-dependent bytes; it records this
as a boundary rather than pretending the native loop began at slot zero.

Initialize installs gravity bits `[00000000,c4160000,00000000]` and integer
cooldown `30`. The host constructor installs those exact values. Explicit
setters model later Simulation configuration, and Add samples current fields at
call time.

## Rigid bodies

`Simulation::AddRigidBody` is `0x82ae6e00..0x82ae79e0`. A zero Free count
returns null before inspecting state, matrix, inertia, or list links. Otherwise
it pops the Free head, decrements the wrapping `u32` count, calls Reset, moves
the body to a destination tail, applies state-specific values, orthonormalizes
the matrix, derives body axes/COM/quaternion/inertia, increments the destination
count, and returns the same slot.

The host Add now delegates through a crate-private reservation. Empty Free
still returns before ring, occurrence, state, matrix, or Inertia inspection.
For a nonempty pool, preparation validates every ring, selects the exact Free
head, freezes exact-state destination dispatch, and captures the next
non-reused host occurrence without mutation. Dropping the reservation is
inert. Its consuming commit performs the pre-existing Add write graph and has
no recoverable branch, allocation, or callback after the first mutation. Ring
and occurrence validation before Free-count mutation is an explicit host
atomicity strengthening; native decrements Free before dispatch and assumes
valid storage.

This split is not a second Add implementation. The public pool method uses the
same prepare/commit path, so every existing Static/Frozen/Active, null-Inertia,
FIFO, reset, numeric, exhaustion, and corruption test exercises it. The seam
exists so a later Definition-aware owner can bind the selected Inertia source
after proving a usable body occurrence exists but before committing any body
effect, then install a preallocated occurrence sidecar infallibly afterward.

State dispatch compares exact raw words: `1` is Static, `2` is Frozen, and
every other value becomes canonical Active state `4`. Static discards the
inertia pointer, uses current integer cooldown, and retains reset-zero force and
kinematic value. Frozen retains inertia, uses current cooldown, installs
`f32::MAX`, and copies current gravity. Active retains inertia, leaves cooldown
zero, installs `f32::MAX`, and copies gravity. All paths begin with zero linear
velocity, angular velocity, and torque.

`RigidBody::Reset` at `0x827c8f88..0x827c9084` explicitly writes identity pose,
positive-zero dynamics/derived fields, tag zero, state zero, and integer
cooldown zero. It does not write ID, right/left links, owning Simulation, or the
inertia pointer. Host code uses explicit assignments rather than clearing the
record.

The exact finite host operation graphs committed earlier are reused after
orthonormalization: matrix-to-quaternion selection/normalization and rotated
inverse inertia. `vrsqrtefp`, vector reductions, exceptional values, denormals,
signed-zero details, and NaN payload behavior remain Xenon/host numeric
boundaries.

`Simulation::RemoveRigidBody` at `0x82ae3190..0x82ae3224` has no calls or
validation. It unlinks the body, appends it to the Free tail, chooses the
decremented counter from `state & 7`, clears the whole state word, and increments
Free. The mask admits the proven spy combinations `9`, `10`, and `12`. Every
non-link/non-state field, including stale inertia, remains bit-preserved until
the next Add overwrites it.

The public host pool rejects foreign, stale, free, or structurally corrupt
handles before mutation. Process-unique authority plus non-reused occurrences
prevent ABA. Separate inert diagnostic identity and occurrence-bound read-only
observation types cannot be converted back into removal handles.

## Joints

`Simulation::AddJoint` at `0x82ae4358..0x82ae43ec` returns null with zero
effects when Free is empty. Otherwise it moves the Free head to the Active tail,
writes `spy=0`, retains the frame and limit pointers, stores API parent in
native `bodyB` and API child in `bodyA`, leaves tag untouched, then decrements
Free and increments Active with wrapping arithmetic.

`Simulation::RemoveJoint` at `0x82ae3308..0x82ae335c` moves an Active slot to
the Free tail and changes only the two counts and six list-link stores. Frames,
limits, both bodies, tag, and spy remain stale while Free. Reuse overwrites all
of those except tag and resets spy to zero.

Normal Assembly teardown removes bodies before Joints. Joint removal therefore
must not dereference or validate stored body capabilities. Public Joint
snapshots and bindings expose only pool-qualified inert body identities, never
occurrence-bearing removal capabilities. `Arc` retention is an explicit host
lifetime extension for native nonowning pointers. Initial absent payloads and
zeroed tag/spy are deterministic safe-host initialization, not a claim about
native allocator bytes.

## Drives

`Simulation::AddDrive` at `0x82ae43f0..0x82ae4484` has the same Free-head to
Active-tail transition. Its exact payload order is `spy=0`, frames pointer,
dynamics pointer, `bodyB=parent`, `bodyA=child`; it never writes tag. Counts are
updated last. `Simulation::RemoveDrive` at `0x82ae3360..0x82ae33b4` appends to
the Free tail and clears no payload, producing FIFO reuse.

`DriveBatchBuild` walks Active order. It reads parent/bodyB state before
child/bodyA state and emits a Drive Jacobian exactly when either raw state has
bit `4`. The current pool slice returns admitted Drive identities only.

The separate `SimulationDriveJacobian` foundation freezes the PDB-proven
`0x180` size and every field offset. The packed `m_ia/+0x0c`, `m_ib/+0x1c`,
and Drive `+0xcc` words are private host tokens: they are neither native
pointers nor public identities or mutation capabilities. The all-zero
constructor reproduces Build's three initial 128-byte clears, including +0
impulses and pads.

`Jacobian_RQD::Create` (`0x82ae3750..0x82ae3930`) is complete for ordinary
finite `f32` values with its exact multiply/add/subtract graph. The recovered
Drive coefficient helper preserves distinct SOFT and non-SOFT paths and the
two fused Newton refinements. HARD alone (`raw type == 2`) applies the strict
maximum-magnitude clamp: equality and below preserve the error, while only
`length_squared > (h*spring)^2` selects the scaled result. Unknown raw types
use non-SOFT coefficients without entering the HARD limiter.

The next pure geometry continuation covers `0x82ae4c98..0x82ae4f14` without
mutating a Jacobian or pool. It composes body A/B quaternions with their frame
quaternions in the proven body-times-frame order, passes those values through
the existing RQD helper, expands qB into the three source-rounded linear axes,
and forms both attachment offsets with the native two-FMA chain. Tests reject
reversed quaternion operands, a textbook basis rewrite, defused attachment
arithmetic, and any dependence on frame-position W. Inputs and outputs remain
plain values; body observations, RF rows, Drive identities, and locks do not
cross this helper.

The next value-only continuation covers Build's independently gated effective-
mass graph through its six pre-reciprocal denominators. It reconstructs both
packed symmetric inverse-inertia tensors, exact +0 inactive payloads, the
source-shaped fused cross/tensor graph, linear-only inverse-mass terms, and
A-then-B accumulation. Noncommuting, A-only, B-only, both-inactive, and swap
fixtures reject diagonal-tensor shortcuts and combined state gating. Xenon
`vrefp`/`vrsqrtefp` seed bits, `vmsum3fp128` reduction rounding, and exceptional/
denormal/signed-zero behavior remain explicit numeric boundaries; zero
denominators are retained rather than assigned invented host behavior.
Point prediction and global linear bias are now separate value-only helpers.
They preserve Build's N-then-M fused point cross, child-A/parent-B deltas,
direct force/torque accumulator use, sequential timestep multiplications, and
two ordered bias FMAs. The bias seam requires a post-HARD position error and
returns Right/Up/At projections without exposing a body or pool capability.
The angular continuation is also value-only. It reconstructs the single
global fallback decision, full four-lane T axes, angular error, direct relative
omega/angular-acceleration prediction, six ordered projections, coefficient
dispatch, HARD gate, and final angular bias at `0x82ae5138..0x82ae54a4`.
Estimate seeds and the dot reduction are explicit inputs. HARD alone adds the
seventh error dot; its strict comparison occurs between the two rsqrt Newton
rounds, and its post-comparison arithmetic executes unconditionally. Returned
`angular_error` is pre-HARD, while `bias_projection` demonstrably consumes the
gated copy. Unknown raw types stay non-SOFT and non-HARD. This is exact for the
ordinary-finite dependency graph; Xenon estimate/reduction bits, exceptional
VMX selection, denormals, and FPSCR timing remain open.

The final value-only continuation covers `0x82ae574c..0x82ae5904`. It accepts
explicit linear/angular `vrefp` seed values, performs both unconditional fused
Newton rounds, and returns the exact ordinary-finite offset, limit, transposed
LT/TT, impulse-zero, and dynamics values. It also retains all packed L/T W
lanes, reconstructs `pad_0=TT_ri.x`, and packs inverse mass into W for every
active A/B inverse-inertia column. Two independent raw reviews resolved the W
words from the literal `vperm` control bytes rather than decompiler aliases;
inactive tensor groups remain exact +0. The result is still capability-free:
private pointer/token binding, complete record assembly, Drive-batch
publication, Xenon estimate bits, exception/FPSCR parity, and live Horus
execution remain open. Body `DynamicUpdate` is reconstructed separately, but
meaningful gameplay constraint deltas still require batch binding and solver
execution.

The module-private complete composer now threads geometry, point prediction,
linear coefficients/HARD gating, linear bias, angular construction, effective
mass, and finalization into every word of one `0x180` record. It preserves the
source-ordered `(linear_strength*h)*h`, skips linear HARD dot/seed work unless
the raw type is exactly 2, and keeps A=child/B=parent through every stage. Its
body/build/seed/binding aggregates are copied values and private to the module;
the three pointer-shaped words are inert host bits, not capabilities. A
standalone source-shaped oracle asserts all 96 words with distinct A/B RF,
Drive, and spy patterns. This closes pure ordinary-finite Build composition,
not live pool observation or publication.

The separately frozen `DriveBatchBuild` loop resets DS then DR count, treats
active count only as a zero guard, walks the right ring to its sentinel, reads
body B then A state without short-circuiting, and emits the dense admitted
subsequence for `(stateB|stateA)&4`. Native stores `n+1` before calling void
Build into slot `n*0x180` and has no local capacity/failure path. The host will
instead prepare copied occurrence-bound records into scratch and publish only
after complete validation; that failure atomicity is an explicit safe-host
policy. The existing order query remains a preflight only.

The PDB-named angular-axis pad words at `+fc/+10c/+11c` are live Build output,
not permanent zero padding. Raw register-liveness adjudication proves the full
normalized T axes survive into the stores at `0x82ae57d0/57d4/57dc`, including
their W residues. Only the initial dcbz-cleared record guarantees +0 there.
The linear basis likewise carries live computational W: Right.W is overwritten
by the private Drive token at `+cc`, while Up.W/At.W survive at `+dc/+ec` and
also feed full-vector angular fallback. The geometry value therefore retains
packed four-lane axes in addition to its semantic XYZ projections.

The direct-array Horus Drive record body at `0x82ae2568..0x82ae2814` is now a
separate crate-private ordinary-finite value helper. It freshly stages A/B RF
rows, preserves every packed fourth-lane neighbor, computes point response and
right/up/at impulse accumulation through the raw fused M/N graph, clamps in
native min-then-max order, retains both dynamics W words, and returns A-add/
B-subtract updates for RF `d0/d2`. Its complex golden covers every output lane;
separate tests freeze the malformed-negative-limit order and response signs.
RF `d1/d3` are untouched. A later executor must prevalidate every binding,
then commit exactly `A[d0], B[d0], A[d2], B[d2]` for each dense record before
advancing; this is required for self-pair B-wins behavior and shared-body
sequential visibility. Native pointer-derived W bits remain an explicit host
token boundary. No live Drive or pool is coupled to this helper yet.

The gameplay island form is now independently decoded too. Its indirect Drive
consumer at `0x82ae2eec..0x82ae317c` has the same ordinary-finite arithmetic
dependency graph and the same RF `A0,B0,A2,B2` stores as the direct helper, but
it follows a `DriveJacobian **` list and stores the Position accumulator before
the Angular accumulator. Direct Horus stores Angular before Position. Closed
final values agree; intermediate store chronology does not. The evidence and
corrected decode hashes are
`0A2BC1B7AA27E2BA4061441BDECE630BF31D1EA3F0B9CFED4F20E99274ECF77B`
and
`B40E86A66CBC43366AC387EFD980E053D8EB0411F03B877A36F76C4AD3C16C53`.

`CreateSimulationIslandTables` is now frozen through its Drive producer and
required lifecycle. `StartFrame` clears all ten island body/contact/joint/Drive
counts every frame before construction. The producer scans the Active Drive
right ring, applies the same bit-4 admission, looks up body B's signed island
before A's, and selects A only when B is exactly `-1`. It appends each admitted
Drive and its corresponding dense Jacobian ordinal to exactly one island, so
every island list is a stable subsequence of admitted ring order. Differing
valid endpoint islands select B; Drives do not merge islands. Native has no
bounds or capacity error in this void function. Physical labels are `0..9`;
malformed selected bytes can write out of bounds and become a pre-effect typed
host refusal. Every island table owns full global Drive capacity. Its
evidence/oracle hashes are
`BDB78FD72CAF7CC5D82CBE7B2CBF79AAE2172DE586A291BA3277091F32E1F3C1`
and
`E290CD50E7F7644373D9BAFB53A6CCE6DAC350E664E4A3C27C168D4D4CA3DA33`.

The combined private Drive preparation now publishes the dense Jacobian records
and all ten stable island subsequences as one non-cloneable value. A body label
entry is occurrence-qualified, and the map and batch carry the body pool's
private frame epoch. Preparation checks the Drive/body binding, map authority,
frame, all record tokens and dense ordinals, then every B-before-A signed label
before reserving staging, dense records, and occupied island lists. The first
numeric callback occurs only after every destination capacity is real; any
failure returns no batch. These error, allocation, occurrence, and frame rules
are safe-host hardening around native unchecked pointer tables.

One selected island can then execute through the exclusive `DynamicUpdate`
phase. Preparation reads body-B state, body-A state, body-A numeric values,
body-B numeric values, and the frozen timestep through that phase and stores its
pool-local nonreused identity in the batch. Execution compares both frame and
phase identity before table, record, or RF effects, so neither old-frame nor
same-frame replacement replay can pass. All table sidecars, Drive
occurrences/endpoints, private tokens, item selectors, and A/B body occurrences
are then preflighted before effects. Stored order, duplicates, self/shared
bodies, then-live RF history, and A0/B0/A2/B2 commits are retained; records in
other islands remain untouched. Island accumulator assignment is Position then
Angular, with an executor-level observer freezing the first-store panic
boundary. Tests additionally prove failed-preparation retry, inert batch drop,
same-phase repeated sweeps, and rejection after a consumed `DynamicUpdate`.
The identity is safe-host replay protection, not native storage or scheduler
evidence. This proves one ordinary-finite one-island Drive sweep, not
Contact/Joint production, the outer Horus iteration, or gameplay scheduling.
Full Contact -> Joint -> Drive Horus ownership remains open.
The design/mutation packet hash is
`462E5FB19B0FB3BB72C21D3C78F44370AD54CBD80EE6BE66391EAA0D8D547046`.

Native Drives retain live pointers into their owning `DrivenPair`, so later
frame/dynamics changes must be visible. The host retains shared `RwLock` owners:
this preserves live mutation and lifetime without unsafe aliases. Locking,
poisoning, and allocator behavior are explicit host synchronization boundaries;
Add and Remove themselves acquire no payload lock.

The private Drive-batch snapshot boundary now walks the validated Active right
ring, performs the exact parent/body-B then child/body-A state admission reads,
and copies only admitted inputs. Each copied body contains precisely Build's
state, quaternion, COM, velocity, omega, basis, force, torque, split inverse
tensor, and inverse-mass values plus its occurrence-bound observation. The
Drive snapshot adds current frames, dynamics, timestep, spy, and Drive
occurrence identity. It owns no `Arc`, lock guard, removal handle, RF-row
reference, or mutable capability; self-pairs and shared endpoints remain two
ordered roles. Scratch reservation and all-or-error return make this host seam
failure-atomic. Native instead resets DS/DR counts and publishes each increment
before its void Build call, so publication chronology remains deliberately
unclaimed. The preparation evidence packet hash is
`FE03AF467215A95B64F8D2F529157D759C67F9E2B89E7AC84BA9171FA492C2D7`.

An independent opcode inventory also proves Build generates its estimates
directly with 10 static `vrefp`, 3 `vrsqrtefp`, and 25 `vmsum3fp` sites; there
is no software table or seed-producing callee to recover from the XEX. Exact
Xenon LUT, exceptional-value, denormal, and FPSCR behavior therefore remains a
hardware boundary. A crate-private fallible backend now assigns a distinct typed
site to every reciprocal estimate, reciprocal-square-root estimate, and three-
lane reduction and receives only requests reached by the exact raw-type and
body-state branches. Its host implementation uses `recip()`, `sqrt().recip()`,
and nested FMA solely as the documented ordinary-finite policy. The complete
HARD/SOFT both-active trace freezes all 32 operations, their order, and every
operand bit against a standalone independent 96-word oracle; complementary
SOFT/HARD, unknown-type, inactive-body, exact-limit, packed-zero, and interior-
failure tests discriminate the remaining routes. Its evidence packet hash is
`1A025BC67E433D55919C45644B584B6D24EAADD67A7B7DF57C675D530F24E195`.
The exact site map and current-fixture trace hashes are
`D9805298ED2E5E7CFAB697533AD545E27C6273DFE8A31663FB1B31CC2047968B`
and `6F2C9B7B3DBA1D6E7884EA9535455C5EC46858F34921B5E0EC37208C0E7A93E7`;
the independent oracle source/output hashes are
`DDBF3D988FC889135F0BFB500685FE01523AF4483A496B0523BCC7C0C305B22C`
and `D4F6B84A403EB7326F2718016F422A3F09A0E5D2A01BB605C315599393D0C05D`.
Checked RF revalidation and publication remain separate from record preparation.

The pool-backed preparation half is now implemented without crossing into RF
mutation or publication. A private coordinator consumes the complete detached
Active snapshot, preflights checked nonzero `slot + 1` tokens for child/body A,
parent/body B, and the admitted Drive, reserves and stages every input, then
composes records in dense snapshot order through one lazy backend. Each record
retains occurrence-qualified A/B/Drive sidecars, and the enclosing non-cloneable
batch retains both source-pool authorities, including for an empty batch. Tokens
are deterministic host lookup words, not native pointer bits or ABA authority.

The safe-host path returns no batch on pool, staging-allocation, record-allocation,
token, or numeric failure and never reads or mutates RF. Tests freeze all 96
words of two independently generated records with different complete numeric
inputs in deliberately reversed snapshot order; live Drive slot order `1,0`
after FIFO reuse; self/shared endpoint roles;
lock release before callbacks; later-record numeric failure; and unchanged pool
and RF state. Native instead publishes each `DR_Count` increment before its void
Build call, so whole-batch atomic return is explicitly not native fault timing.
Evidence packet hashes are
`7153D1C9CF35111EF1B3EA2C15FF12AA72FBB47C897D594DC020AF804337375E`,
`C44DA330E1E4DF561B713C8D84D91F3D32AC4FCEA4873D6EDF7AB3DC260BC5F5`,
and `2E742A5BA63B2D9D6E897FA492AD6EEFAD4FD38AC68F9EEEF6884D8BC6995CA8`.
The second standalone oracle source/output hashes are
`21CE64E8494991BD842074931E84B8570EE8A7E185E14DC0E8BD923149A13AE4`
and `90F4B865E4BC3475CC940C7A71F3036325E7D20B58F348DADAACB49742CE3127`.
Batch-wide revalidation against live Drive/body occurrences and a pool-owned,
self-alias-safe RF commit transaction remain the next boundary.

The older direct/global Drive sweep is now retained only under `cfg(test)` as
an arithmetic and RF evidence harness. Its tests continue to freeze occurrence
and endpoint validation, self-pair B-wins behavior, sequential shared-body RF
visibility, and exact `A[0], B[0], A[2], B[2]` commits. The raw store anchors are
`0x82ae27b4`, `0x82ae27b8`, corrected `0x82ae2804`, and `0x82ae2810`;
`0x82ae280c` is not a store. Because observed gameplay uses per-island indirect
record order, this direct harness is not compiled as a production executor and
cannot bypass the phase-sealed island path.

The full native outer loop runs Contact -> Joint -> Drive for the runtime
maximum-iteration count. Therefore neither repeated Drive-only execution,
complete Horus parity, nor gameplay scheduling is claimed. The ordinary-finite
math boundary still excludes exceptional Xenon VMX behavior and native
pointer-derived W bits. The historical raw chronology packet is
`B589DB7F7E9651676583A01ED9A9D19EE51B1FC9EA274E451FE0606C36EAF439`
and the corrected representable oracle output is
`C817071BD6C1F45B45B0998C61F84E4D0DE534389EE70C150AFD538629E0B32B`.

The public safe Drive seam binds a Drive pool to one body-pool authority and
prevalidates live, same-Simulation parent then child before a non-full Add.
This is host safety around a raw native primitive that trusts pointers. A full
pool still returns `None` before endpoint validation, matching the native early
exit. Removal remains body-independent because Assembly removes bodies first.
Build uses occurrence-bound body observations, so a removed/reused body slot
cannot silently alias. Public read access uses occurrence-bound inert Drive
identities; only explicit owners retain removal handles. Unknown native tag
bits remain `None`, and no public tag writer is invented before its real writer
is recovered.

## Shared constraint quaternion and Joint/Contact island evidence

Joint Build's raw register flow at `0x82ae596c..0x82ae59b0` proves the same
body-then-frame quaternion composition already used by Drive Build at
`0x82ae4c98..0x82ae4d58`. The shared private helper preserves the exact
ordinary-finite graph: body multiplied by YZX(frame), fused subtraction of
YZX(body) times frame, YZX rotation, then frame/body-W and body/frame-W fused
adds. The caller supplies the three-lane reduction result, so the helper does
not claim Xenon `vmsum3fp128` association. Corrected orientation and three
separate fusion fixtures are frozen by oracle source/output hashes
`0006EA8CF6B35D2A3827CBBA3618FB55237F762A859BF0182A279C2C6612C4C4`
and
`D88FE577F17F338441FF66C00A0790E1A76A018D549E68A212CC93A726E565F1`.
The older discriminator's semantic argument labels were reversed and are not
used as goldens.

The PDB-exact `JointJacobian` value layout is now represented as an aligned
`0x180` host record with exhaustive offset tests. Native `m_ia`, `m_ib`, and
`m_joint` words are private tokens, not exposed pointers, indices, or mutation
capabilities. Its constructor reproduces only the three late output clears at
`0x82ae6308/0x82ae6314/0x82ae631c`; it does not claim Joint Build's temporary
calculation chronology or final selective stores.

Exact public value layouts also freeze `JointFrames` at `0x50` and
`JointLimits` at `0x40`, both 16-byte aligned. Their 20 and 16 words round-trip
bit-for-bit, including packed position W lanes and arbitrary high-bit raw
Swing/Twist values. The pool remains generic; only the exact typed
`SimulationJointPool<SimulationJointFrames, SimulationJointLimits>`
instantiation will be Build-capable. No downcast or byte reinterpretation is
introduced.

The earlier phase-owning prepared-Joint design was frozen under hash
`9E1E6556E1404F678B9653E82119EF0A41B1FE669DE42000CFA4C89D8ED8DC51`.
It is now superseded by a non-Clone batch specialized to that exact pool
instantiation. The batch uses Joint/body occurrences for ABA checks and retains
the immutable Joint-pool borrow, while preparation only borrows the exclusive
DynamicUpdate phase. Frame identity remains a StartFrame identity, not a
guessed mutation epoch; the batch additionally stores the phase's private
nonreused identity. It publishes no record or batch until the complete numeric
Build and checked token installation succeed.

The Joint island producer/consumer packet has SHA-256
`EBB7F06F41CCA96149EAD6BD66DB402A2A8E036E7E8ED69375D7E25AC0900B77`.
It proves forward Active-ring order, B-then-A state/label reads, bit-4
admission, exact B `-1` fallback, dense `0x180` correspondence, stable indirect
subsequences, retained duplicates, Angular-then-Position accumulator stores,
and A0/B0/A2/B2 RF stores. Its complete ordinary-finite Joint consumer graph,
inert Joint Build record, occurrence-bound record preparation, checked
body/Joint token installation, and ten stable island subsequences are now
available. One private checked selected-island executor now connects those
tables to live RF effects; the complete owning Horus loop remains open.

The pure per-record Joint consumer is now implemented behind a crate-private
value boundary. Raw correction proves its TT/angular source is `B2-A2` at
`0x82ae2cc8`, while only LT/position uses the attachment-point difference at
`0x82ae2d60`. It retains Joint's independent Min/Max dead-zone inputs, full W
lanes, Angular-then-Position record writes, and staged A0/B0/A2/B2 results. A
24-vector literal golden checks the entire record and both RF blocks; a
self-pair freezes the later B-wins commit values. The implementation packet,
standalone oracle source, and output hashes are
`AF53F3A0DF458D4988F243845C322A988301F36ECAA853CE8AF2DB08780BC8D7`,
`5AF27B4F995D8A3B5387C2F4B8F68413F752C49241F044B5133E40D3A3F1EDB1`,
and
`D01C2A4B899CFF2EDE2D02FC8C1E965D011727662942969D8CE013CDC23FA57E`.
This pure helper alone is not an RF transaction or table executor; the checked
batch/executor integration described below supplies that separate capability
boundary.

Later raw Build chronology supersedes the preparation packet's provisional
spy-to-record interpretation. The scalar spy store to `+0x4c` at
`0x82ae6704` is dead: `stvx128` at `0x82ae676c` overwrites all of
`+0x40..+0x4f` with packed `m_LT_ri`, so final PDB `m_nb` is that vector's W
word. The private composer sources its bits from the final vector, not
`Joint::m_spy`.

The complete 927-instruction Joint Build is now implementation-ready under
packet SHA-256
`C0CFBABF59D8ECCD9EDDF9C9AEA227CEF05A22AC617BF5B52BED4785B3758D3A`.
Its raw and normalized instruction hashes are
`8065692E1F38DC076ECABA6A8840D8B637DB5DD09A929544BE30243DF15D225C`
and `500AB9A67E50B9CDFE9582105E47AC15F62B8BD192FFB94F2089FF0A7C7D3401`;
the independent full-range oracle source/output hashes are
`64009622DA7966E53FD2AF7EBCAEF78DF2F6541F5B51BF21F7AEF3B3D73DD521`
and `EA440FF0F360E7C3F6DF1136368EA44C23F65269EEBE4BC4969D20B0E717A730`.
The packet freezes every input, raw Swing/Twist route, twelve sum sites, six
lazy estimate sites, independent body-state gates, final store, and eight full
ordinary-finite records. It does not turn host reciprocal/reduction results
into a Xenon bit-parity claim. Raw RQD re-review corrected `Jwk.y` and
regenerated the 0/0, 2/0, and 3/0 route goldens before integration. The private
pre-Swing numeric/value slice across `0x82ae596c..0x82ae5dcc` is translated with all
nine reduction sites, full checkpoint values, operand traces, callback-failure
prefix, and packed prism W lanes. Its corrected Stage 1 packet/draft/emitter/
checkpoint hashes are
`FAEEED6FF3BC6D73834093AAAF7EFE162A671DC42D2B8B8B3D00F4C88295EC86`,
`FC68DC0EA15F87845D38AA55A407CE144AC85E6E5CDD7556467A3E2EFA55E278`,
`61D7EE3B239028836A34FC9E4F39BEF173983F5AFA7B095CF6270CCB8C12E754`,
and `486E78BD101979000932C87A9796F65FC4FE00ACCB426E27B47EB9D15D44DD5F`.
The checked Swing/Twist continuations and their common angular prediction are
now privately composed with Stage 1 through `0x82ae6304`. Their evidence
hashes are
`226AD436AA9EB9BFE718842A9A72A69DE6CC617702DC1CE39D1C8955A5AB2AE8`
and `295E2ABC7D4F32F9DED422DCFA1056A19ECE5D771155EB413D0C69EED72AC162`.
Literal HINGE/ARC and LOCKED/LOCKED outputs plus a maximal CONE/ARC 15-prefix
failure test cover the shared backend. The independently proven
effective-mass prefix/middle/suffix hashes are
`B2384B79EDE797A4ED97F3303018041249597651FD86418B2AC3DDD80F3BE42A`,
`997CCABE9B21671EB3BD687A69F76F3A52D80BCD4363EF55F2FB22C75C5D7F10`,
and `DD023CDB7D06E6C715FD64CD97E342DB119D5E2529777EA40BC8A2C5F02E21B4`;
the suffix corrects native `rwp_One_Half_47=[0.5;4]`, which the earlier oracle
fixture had omitted. Those three stages are now integrated with the earlier
prediction graph and terminal record assembly. The independent wiring audit
hash is
`1035E60D8B31F6142D338F936B7EB7ABC29E8C2F8FEF09B12D0C6EF071E8FABA`;
the independent final-assembly audit and typed-draft hashes are
`492B57DFBD3025DD5D3AA1D1D8857FE0A9FF787AAB19C2CC5536C666A33C997A`
and
`3D6B832BF2B9FA456008CA3CC0E19FE46986E5B406B90E3419F2AF86FE8BF59D`.
Stage 1 retains rather than rematerializes the four native attachment
permutations. The complete composed CONE/ARC path has 17 callbacks; all 17
failure prefixes return no value or record. CONE/ARC and LOCKED/LOCKED produce
the corrected 96-word FNVs `d1a5dd735b04e060` and `f01b59e5f4a4bc19`.
The standalone record remains an inert private word image. The checked
pool-backed preparation boundary now installs one-based body-RF and Joint tokens
into `SimulationJointJacobian` while retaining occurrence-qualified Joint/body
sidecars. One checked selected-island sweep now consumes that batch through a
separately borrowed phase; owning runtime publication and the complete Horus
interleave remain open.
PDB `mSwinga` (`+0x28`) and `mTwista` (`+0x2c`) remain in the exact Limits
layout but are not inputs to this Build function; the other six scalar/raw
words at `+0x20/+0x24/+0x30/+0x34/+0x38/+0x3c` are consumed.

The private, unpublished batch combines two distinct native results. The
`JointBatchBuild` subset (`0x82ae7a78..0x82ae7b0c`) supplies validated Active
right-link order, B-then-A state reads, `(B|A)&4` admission, and dense records
without rejected holes. The Joint block of `CreateSimulationIslandTables`
(`0x82ae3eec..0x82ae3fec`) supplies signed B-then-A label reads, exact B `-1`
fallback, and stable per-island subsequences of those dense records. The host
transaction preflights every allocation and checked nonzero one-based binding
token before numeric callbacks, retains Joint/body occurrences plus source,
frame, and private phase-identity seals, and publishes only after every record
succeeds. Combining the two native passes in one private batch is safe-host
ownership hardening, not a
claim that they are one native operation. The explicit 24-row installer changes
only rows `0[3]`, `1[3]`, and `12[3]`. A manually staged nonzero-token canonical
CONE/ARC image has FNV `3e59e7450bd50ab2`; its literal callback transcript
freezes the host zero-W adapter and is not a native reduction-operand claim.

The independent combined implementation audit packet hashes to
`1D6D8BDDA9CE336C37DC8E1AB429355407BEF1A6CC4B95DE21837165BC1938DC`;
the audited `simulation_joint_preparation.rs` hashes to
`F6952E6869486898ECFDC2E91F9F34955C843F932117CFF2D61C27C8F07949CC`.
Focused tests cover null-map island zero, every selected signed-label route,
missing and stale B/A labels with exact access order, map authority/frame
binding, nonmonotonic Active mutation with stable cross-island subsequences,
selected-only range validation, all allocation boundaries, and a later occupied
island reservation failure before every numeric callback.

The private checked executor runs one selected Joint island by separately
borrowing the exact DynamicUpdate phase used for preparation. Before the first
record or RF effect it validates
island range, frame and source authorities, exact dense table-to-record
sidecars, each Joint occurrence and parent/body-B then child/body-A endpoint,
all three one-based binding tokens, record selectors, and all body occurrences.
It then consumes table order verbatim, including duplicates: every entry reads
then-live RF, invokes the recovered ordinary-finite Joint helper, writes the
record's full Angular accumulator before Position, and commits RF A0, B0, A2,
B2 while preserving rows 1/3. Returned errors are pre-effect; panics retain the
already-applied prefix. Integrated fixtures cover literal `[2,0,2]` selection
with an intervening record and repeated live-RF history, a pure-helper result
comparison, empty-island inertness, detached source/table/token rejection,
cross-paired Joint endpoints, and a later invalid token refusing before the
first entry. The same phase supports repeated sweeps, but a reacquired phase is
rejected before effects even in the same pool and frame. Its pool-local,
nonreused identity is explicit safe-host replay protection, not a recovered
native field or proof of outer Horus scheduling. This is one selected-island
sweep, not the runtime-counted Contact
-> Joint -> Drive loop, DynamicUpdate consumption, or gameplay solver.

The independent selected-island integration audit packet hashes to
`A6151632F66F1E040E3BC3CD2C6EBB1D0FBE2679AE9299A743CB9D22B499D4A5`.
Its audited Joint-pool, preparation, pure-consumer, and indexed-RF source hashes
are respectively
`02D6AA788ADBBC7948582235D552031A287EBE1AD153A0C77D304B3496CC7AA7`,
`3EA4625A94C4D341C1D673E02CF7781D105FCB953DE8B6AAF26C3F2FCB913179`,
`28B9E33C21BA069FCF54057B4F1E506C4B7FB4D78A34AEB6F7D5E308C2E1BC8F`,
and `A3E52CCCB3CB5D0D9D547DC5A910E3C16C900C055FDB74BFBE4379673EB9B919`.

Body record fourth words are persistent fields, not padding. RF identity is
body `+0x1c`, inverse mass is `+0x7c`, and raw state is `+0x8c`. The adjacent
metadata words at `+0x2c/+0x3c/+0x4c/+0x5c/+0x6c/+0x9c/+0xac` enter native
intermediate W-lane arithmetic but are dead at the estimate transcript and
complete final record. The independent discriminator hashes to
`ECD4C60552A84DA3937F2030C60CDA318C3C43CD5443ECA37C5164F8F68E9E3E`;
zeroing all fourteen A/B words and individually replacing each with `123.0`
preserve final SHA-256
`8B63ED0D2811272B1583F7C25FC75A1CC1A9981BDA2EF0A3F609BF3F879C9B07`
and estimate-trace SHA-256
`D9AE9D3F8263A95C66F00E2063D7609A6EA8F60D668725653C5854A404233B41`.
Accordingly, zero synthesis is authorized only at this final/estimate adapter;
it is not a claim of exact native reduction operands or intermediate W images.

The preparation evidence, record-contract audit, and body-lane provenance
audit SHA-256 values are
`8990753CC6C499480CA4F17E74720254CD2D88E7C5784FBCEF50B61F40BD7F61`,
`6D59E6EC168F91A6C108C54E975E11F1934E0B06588A104919B4843E94D5ADA0`,
and `901E8C0F19A4AB4E62C997A0ACD051CE422DE16CCD828F21C2BF2510C673B48C`.
Empty/one-sided admission, nonmonotonic Active order, rejected-payload
chronology, stale/foreign endpoints with B-first priority, signed label
selection, map binding, real and dishonest reservations, token bounds, all
single-record callback failure counts, later-record attribution, and inert batch
drops are tested. The tables remain private prepared values: checked Joint
execution and RF capability pairing now cover one selected island, while full
Horus outer scheduling, Contact preparation/execution, DynamicUpdate
consumption, manager concurrency, cross-island RF ownership, and Xenon
exceptional/estimate/reduction parity remain unclaimed.

The separate Contact packet has SHA-256
`1375100DCDF641B90CE5F1D84C56D22CE18E577149F23B24732C7278322DD989`.
Contacts have no body-state admission and select A's label unless A is exactly
`-1`, unlike Joint/Drive. Gameplay passes zero separation, so the Contact and
ContactJacobian entries initially alias one `0x100` record which
`ContactBatchBuild` compiles in place. The consumer loads all eight endpoint RF
vectors before effects, stores its packed accumulator, then stores A/B row
pairs in 3, 1, 2, 0 order. This now has a distinct all-four-row transaction;
the Joint/Drive 0/2 executor is not generalized to Contact.

The complete pure `ContactBatchBuild` per-record compiler is now integrated
for ordinary finite inputs. Its evidence packet SHA-256 is
`279D9D7A94A37C646648847C3FFC930BA419ECA23CBB66009D94CDE6DD2FB1A4`;
the normalized raw hash is
`1FD5F76321A415183ACD9EE1FF837B89EC5FC5FD1C7F2BC8FC2F750D8F9DA877`,
and the standalone oracle source/output hashes are
`47E377914FB8D20F5CE4D2948386E945323299B48AD560FE320C6B5D5C259C59`
and `60BB5AB4531AB44980C50FF524B3956DB60742F3EF2160C3B63E79FA710FFAC7`.
The private composer snapshots all sixteen vectors, applies independent
side-0/side-1 bit-four gates, publishes `nb=(state0|state1)&8`, performs one
typed four-lane reciprocal-estimate request, and returns the existing exact
`0x100` Contact Jacobian. Literal output/denominator, direct state-route,
backend-failure, and test-only sixteen-store-order fixtures pass. Backend
failure returns no record but retains already-issued numeric trace effects.
The independently audited typed draft, 256-vector Rust corpus, and generator
hash to
`C0956D005DA3EA31ABB20BF608D21A6ECA4F9E2947DC45F7109EE51420D5CE5F`,
`D8DA6C8712DD2917963F92A5B2B75166A28F4ECFFD227BE26D80C82E1B7729DB`,
and `627CEE825AD101FAFCBD81F39ED888EDB6C224996549598D5B0A4A1A503E8EBA`.
Raw evidence and Rust use the dead AX/AY/AZ W residues
`At.y/At.x/At.x`; the older Python oracle's AZ.W `At.y` typo reaches no final
word, so its output stays valid but the corpus does not prove that intermediate
lane. Producer owner/occurrence/frame identity, other producer variants,
append chronology, table execution, Contact-record-driven live RF effects,
Xenon estimate bits, and exceptional VMX/FPSCR parity remain open.

The independent producer/seal census packet hashes to
`9EC8914EABBC001C7AE3AD160977E0DA27D5B1F6A721559769C31014D9DB196E`.
Raw `AddNetworkContacts` stores prove its concrete role mapping
`[table A, table B, RF side 0, RF side 1] = [static, selected, static, selected]`;
Contact table selection therefore falls back from static A's `-1`
label to selected B. The four roles remain independent in the compiler API
because `ContactBatchBuild` performs no equality check and the other producer
families are not yet occurrence-bound. The packet establishes the Contact
count/record writer census and the successful `Step_Collision3` return as the
semantic completion boundary before table creation and compilation. It does
not reconstruct complete collision candidate geometry, prove universal role
wiring or deterministic cross-job dense order, or implement a producer, seal,
prepared owner, island table, or selected-island Contact executor.

The network path now has one deliberately narrower private queue foundation.
Its packet, `network_contact_queue_evidence.md`, has SHA-256
`12CE63BB44A4833F825BFFE289CFA51811AB9A5D3B7581E3253ADD9894EADADE`
and freezes the native PDB `0x40` request evidence and 432-entry fixed-vector
capacity plus the raw `Island::AddNetworkContact` append at
`0x827b6f40..0x827b6fb4`. The semantic host request intentionally makes no
native-layout claim. The queue keeps
every point/normal word including W, retains caller order, and binds the selected
body to one pool occurrence and body frame without adding a state predicate.
Wrong-pool, wrong-frame, foreign/stale/free, requests against structurally
corrupt pools, and 433rd requests refuse before queue effects; unique queue
identity and those refusal routes are safe-host policy. Until a checked Part registry exists, the caller resolves
Part-to-body and Material before enqueue rather than reading them live at native
drain time, so
intervening Part mutation parity is explicitly unclaimed. This is not a
universal Contact producer and does not yet drain, publish `0x100` records,
seal a dense batch, build tables, or execute Contact constraints.

The successful ordinary-finite one-record network compositor is now a separate
private pure boundary. Its packet hashes to
`2010F8D2B8B4E3211FA5802F5252B5B2ABCD0CAE41EB228713CB18BA3BB64607`.
Raw `0x827b3c04..0x827b3ec4` and an independent fourteen-case oracle freeze the
fused selected-body transform, displaced point, both point velocities, relative
velocity, dynamic and X/Z fallback tangent routes, exact minimum-normal
threshold, one four-lane estimate reused for both axes, scalar material routes,
all final W sources, and alternating A/B scratch rows. The oracle's complete
output hashes to
`8CA8067FB84338587C239C28F33B059729677CF9D88DC6441A9143CDEBC6C1A3`;
its row words hash to SHA-256
`C4D94A2C2D8D667787911A95DB1AC739EFCB41F031AD26F7F80F5CE50CD071F1`
and FNV-1a-64 `B5702605D82046F8`. Independent review caught an initially
algebraic but one-ULP-wrong binormal FMA association; the raw association and
its discriminator are now frozen in Rust. A fallible estimate backend and its
no-value return are safe-host policy, not native behavior.

The pure one-record result does not itself consume the queue or publish a dense
Contact record. A subsequent private network-only boundary now consumes the
whole queue into an unpublished non-Clone prepared contribution. It checks
queue/pool/frame against an exclusive DynamicUpdate phase, reserves both local
vectors, validates every static/selected occurrence and four independent role
tokens before numeric work, and seals queue/pool/frame/phase/static identities
plus literal queue ordinals. Success consumes the queue; every returned typed
failure returns it unchanged. Panic recovery is explicitly unclaimed because
reservation/backend panic can drop the owned queue and retain a backend trace
prefix.

Preparation samples numeric body fields under the exclusive phase, while
point, normal, displacement, Material, and selected occurrence remain queue
copies. A same-frame DynamicUpdate regression test proves this timing split.
The prepared-contribution evidence packet hashes to
`CC1F353B0175B14B9BBD41B5DEFDB6CA874008A2BF3B03693996099B8D476D46`.
It supersedes only the older producer packet's semantic singular `old_count`
wording: raw uses three distinct count loads at `0x827b3bd8`, `0x827b3be4`,
and `0x827b3be8`, with no atomic/stable reservation.

This owner still publishes no global Contact count or record and does not
recover native failure/store chronology, manager static-body authority, a live
Part authority, exact Xenon estimate bits, exceptional VMX/FPSCR behavior,
tables, compilation/execution ownership, or RF effects. Those remain distinct
evidence boundaries.

The complete ordinary-finite Contact per-record solver is frozen under packet
SHA-256
`F771B6FD8EF1C8FF8A0467B6D6FD4C28BDB44ED7C057CA880E88D13730B4F0FC`;
its standalone Rust oracle source/output hashes are
`7E49801CE27587775021D6EE8F35699A54CE6992C1D7998CDA5285BE8D28F1E6`
and `1A1B7D961B55996B628CD4BA26491A757848F21115A3C85A6EC365E5CC2525D3`.
It freezes distinct position and velocity point channels, position-only packed
W projection, static-friction admission with dynamic-bound fallback, full
record/W mutation, and exact A3/B3/A1/B1/A2/B2/A0/B0 RF stores. All eight rows
are preloaded, proving self-pair B-wins and sequential shared-body visibility.
The exact public `0x100` layout and crate-private pure host helper are now
implemented. Binding/address words remain private and coefficient columns keep
their unexpanded PDB `irac_*`/`irbc_*` names. Complete record/RF, nine-store
prefix, and self-pair tests pass; the checked table executor remains open.

The exclusive body phase now also provides a Contact-specific indexed RF
transaction, separate from the Joint/Drive 0/2 contract. It validates the
complete plan before effects, copies both complete live A/B blocks per selected
entry, and stores A3, B3, A1, B1, A2, B2, A0, B0. A four-entry `[2,0,2,1]`
fixture proves literal selector order, repeated-item mutation, then-live
shared-body history, self-pair staging, B-wins stores, and all 32 observed
commits. Separate refusal coverage freezes later out-of-range/stale-B failures
plus real and dishonest reservation failures before callbacks; a test-only
observer freezes the exact through-B1 panic prefix. Complete A/B copying is
host mechanics under exclusive ownership, not a claim of native RF load
instruction order. No Contact record or table is connected to this
transaction. The independent audit packet and audited source hash are
`780EF13E20F240C2584BFF84E804994B7AD2D58D1808BF820DFDDF1B5961CDA5`
and `A547E7A1FFFCAD0E04140C3EB2BC2F61AA40C6C72A6C48D4B666B6ACAB748EA5`.

The outer-schedule packet has SHA-256
`EA078BDFBD4DFE02517E8A9574AA481EAF6AAF9E643C21684D50972C45D773B0`.
`HorusPipelineIslandUsingIndirectJacobians` snapshots Contact, Joint, and Drive
counts/list bases plus the runtime `Simulation+0xb0` maximum before its zero
gate, then executes exactly that many full Contact -> Joint -> Drive passes.
Zero phases are skipped but an empty island still performs the three count
checks each pass. The job graph makes all three compilers siblings, each island
a successor of all compilers, and DynamicUpdate a successor of every island.
It provides no edge between islands; their native execution order is therefore
unproven and may be concurrent. World initialization writes 25 into the manager
parameters, but the runtime field remains authoritative. Any serial ascending
host island schedule must be labeled policy until RF disjointness is proven.

## Verification boundary

Focused tests discriminate exact default bits/types, list order and FIFO reuse,
raw state canonicalization, spy-state removal, Reset preservation, quaternion/
inertia integration, parent/child reversal, payload retention, teardown after
body removal, live Drive-input mutation, capacity early exits, wrapping counts,
foreign/stale/double handles, occurrence exhaustion, corruption atomicity, and
read-vs-remove capability separation.

`SimulationAssemblyBackend` now exclusively owns one body, Joint, and Drive
pool and implements the already recovered `DynamicAssemblySession` backend
surface. Its authority is the body-pool authority, its Drive pool is bound to
that same body pool, and session-private handles preserve the proven attach,
stage-specific rollback, pose-copyback, and body-first detach orders. Joint
Add validates parent then child after the native capacity early exit. Drive Add
uses the equivalent bound-pool validation. Non-capacity host-safety refusals
are failure-atomic and retained as typed diagnostics even while the session
performs rollback; native-shaped capacity exhaustion remains an undiagnosed
`None`.

`Arc` payload retention, `RwLock` synchronization for live Drive inputs, typed
diagnostic compression through the existing optional backend trait, and panic
on direct violation of that trait's live/exactly-once precondition are explicit
safe-host policy. The public session API maintains those preconditions. There
is no implicit `Drop` cleanup and no solver invented by the composition.

The complete pools and transaction now provide a concrete attach/detach
lifecycle for reconstructed Assemblies. The checked Drive and Joint sweeps can
apply live constraints for one selected island, while the private network
Contact path now reaches a checked compiled record/table batch and one selected
RF sweep. The next physics milestone is the full Horus outer interleave and
solver-to-body integration tests, then end-to-end Assembly attachment against streamed
collision. The bounded body integrator has its own provenance record in
`simulation-dynamic-update.md`.

## Manager-owned Contact workspace and frame destination (superseding boundary)

The earlier statements that Contact lacked checked preparation or a manager
Static-body authority are superseded for the private network producer only.
The frozen workspace-lifetime packet is
`world_probe_temp\contact_workspace_lifetime_evidence.md`,
SHA-256
`0A31BF4D9C4A2DA7EB3CE6F685F51520C1508AD56AC19D3A6698324DB9F1D62B`.
Its normalized raw evidence and extractor hash to
`15B0C7DFFBDE0BFD33420BEB6D434E9AC58565D41F05D1D647F61005FDC5B7B0`
and `037CFBF2482CDEEE02D0350192BA4FE24063382AF902BFC55EB1A04FA7E74C2D`.

PDB and raw establish one manager-owned workspace allocation, a Simulation
Contact backing/count/maximum at `+0x10/+0x64/+0x7c`, a secondary island region
at `+0x2f0`, and manager `+0xdc` as an alias over those fields. With
`A(x)=(x+127)&~127`, the exact island-enabled descriptor is
`A(384*(J+D)+512*C+10*A(4*B)+20*A(4*J)+20*A(4*D)+20*A(4*C))` in native
32-bit arithmetic. The two shipped World managers use exact Contact maxima
`2040` and `760` and workspace sizes `0x195f00` and `0x7f300`.
`StartFrame` resets the count without changing the maximum/backing or clearing
record bytes; only `[0,count)` is live. The optional backing swap is not an
observed gameplay route because both recovered table callers pass zero.

The private Rust domain therefore reserves persistent semantic storage to a
logical maximum, retains the manager-created identity Static-body occurrence in
raw state `1`, and exposes only an exclusive Producing frame followed by an
immutable unpublished seal. Whole-queue preparation uses that retained Static
body and checks global remaining capacity before local allocation or numeric
work. Append revalidates manager/pool/frame/phase/static seals, the complete
contribution, queue ordinals, four role occurrences, and four one-based binding
words before its first write. Typed failures return the exact owner and preserve
the prior dense prefix. Successful multi-contribution order is explicit host
call order, not a claim about native collision-job reservation interleaving.

Checked overflow/allocation failures, pristine-pool construction, unique host
authorities, same-frame reopen refusal, and Producing/Sealed typestate are host
safety policy. Native passes a Static inertia address even though Static Add
does not retain it; the host's absent inertia represents the resulting state,
not literal argument parity. No native count publication, arbitrary/public
producer, non-network collision enumeration, Part registry, island table,
compiler/executor owner, RF effect, or complete Horus schedule is claimed.

## Sealed network Contact compilation and stable island subsequences

The final sentence above is superseded for table/compiler ownership on the
private network path. The frozen Contact table packet is
`world_probe_temp\contact_island_tables_horus_evidence.md`,
SHA-256
`1375100DCDF641B90CE5F1D84C56D22CE18E577149F23B24732C7278322DD989`;
its normalized raw output hashes to
`5EE8DAE8B4C76A26F8BF9B10D34064B3B0AACBADDD690A42C224D9CB5C713436`.

`CreateSimulationIslandTables` visits Contacts in global dense order at
`0x82ae3df8..0x82ae3ee8` with no state-bit admission. It resolves signed A then
B labels and selects `A == -1 ? B : A` at `0x82ae3e80..0x82ae3e90`, so Contact
is deliberately A-preferred unlike Joint/Drive. Null maps supply zero. The
count increment and paired stores at `0x82ae3eb4..0x82ae3edc` place every
ordinal exactly once in one stable per-island subsequence. Both observed callers
use zero separation, making the Contact and ContactJacobian pointers alias the
same record; later `ContactBatchBuild` compiles that global dense backing in
place.

Rust now consumes the immutable network seal into a source-bound, non-Clone
batch containing separately owned compiled Jacobians and ten semantic island
lists. Before any allocation or compiler callback it validates the exact
manager/pool/frame/phase/static source, dense ordinals, four independent table/
solver roles and occurrences, four one-based record words, map provenance, and
all selected labels. It then reserves all required output, builds stable table
sidecars, and runs the existing ordinary-finite compiler in dense order with one
phase timestep. Every typed failure returns the exact seal; no batch, manager
record mutation, or RF effect is published.

The fixed sealed count, combined table/compiler transaction, separate record
ownership, checked labels/allocations, and occurrence-qualified sidecars are
host safety policy. Host Vec capacity is not native physical-layout parity;
native workspace gives each of ten Contact and ten ContactJacobian lists full
`m_CT_Max` capacity. Non-network producers, native volatile-count concurrency,
nonzero separation, selected-island Contact execution, and the complete
Contact -> Joint -> Drive runtime interleave remain separate boundaries.

## Checked selected-island network Contact execution

The selected-island execution boundary above is now closed for the private
network batch only. The numeric authority is
`world_probe_temp\contact_island_solver_implementation_evidence.md`,
SHA-256
`F771B6FD8EF1C8FF8A0467B6D6FD4C28BDB44ED7C057CA880E88D13730B4F0FC`;
the independently audited RF transaction packet hashes to
`780EF13E20F240C2584BFF84E804994B7AD2D58D1808BF820DFDDF1B5961CDA5`.

The native Contact consumer at `0x82ae2990..0x82ae2c50` reads exactly the
selected `m_islandContactJacobians` list. For each supplied entry it loads the
Jacobian and `ia/ib`, snapshots both complete RF blocks, stores the record
accumulator at `0x82ae2ba4`, and commits `A3,B3,A1,B1,A2,B2,A0,B0` through
`0x82ae2c2c`; it branches to the next entry only at `0x82ae2c48`. It does not
test state or `m_nb`, sort, deduplicate, or replace the indirect list with a
dense traversal.

The private checked sweep revalidates the selected island, exact sealed source,
table-to-compiled-to-source Contact identity, all four table/solver body roles
and occurrences, and all four one-based compiled words before any effect. The
body-owned transaction also preflights its complete selector/body plan before
callback zero. Execution then calls the existing pure Contact solver, mutates
that record's accumulator, and commits all eight returned rows in the recovered
order. The batch remains reusable for a later outer iteration.

Supplied order and duplicates are preserved. Integrated oracles prove repeated
record accumulator history, shared-body sequential visibility, self-pair
preload with B winning every row, repeated same-phase sweeps, empty-island and
drop inertness, and later-entry refusal before any earlier effect. Returned
errors are pre-effect; panic prefixes are deliberately not rolled back and are
covered at the underlying pure-solver and live RF transaction seams.

This does not yet own runtime iteration count, interleave Contact -> Joint ->
Drive, prove cross-island disjointness or scheduling, consume DynamicUpdate,
publish native volatile counts, authorize non-network producers, or extend the
ordinary-finite claim to exceptional Xenon behavior.

## Runtime-counted one-island Horus interleave

The runtime-counted interleave boundary above is now closed for one private
prepared island invocation. The authoritative schedule packet is
`world_probe_temp\horus_outer_schedule_evidence.md`, SHA-256
`EA078BDFBD4DFE02517E8A9574AA481EAF6AAF9E643C21684D50972C45D773B0`.
The new owner takes the caller's snapshotted unsigned `m_MaxIteration`, returns
at zero, otherwise preflights all three exact list/body plans, and executes
Contact -> Joint -> Drive for each pass while rereading live RF at every record.

The reusable plans own their exact mutable record slices; the combined
transaction owns the exclusive DynamicUpdate phase. This prevents family/phase
cross-pairing and keeps occurrence-resolved body indices stable. Complete
three-family preflight before pass zero is safe-host failure atomicity. Panic is
not rolled back and retains the exact earlier/current mutation prefix.

A nonempty two-island integration test matches literal `C,J,D,C,J,D` across all
record bits and RF rows, differs from `C,C,J,J,D,D`, and leaves the unselected
island unchanged. Zero, range, later-Drive refusal, repeated history, and
cross-family panic boundaries are independently covered. Full provenance and
audited source hashes are in
[`simulation-horus-pipeline.md`](simulation-horus-pipeline.md).

Production batch construction/orchestration, inter-island RF policy, the native
job barrier, DynamicUpdate phase consumption, non-network Contact producers,
and exceptional Xenon floating-point parity remain separate work.
