# Andale animation database and codec evidence

Confidence is `high` for the packed names, database/record boundaries,
hierarchy, clip metadata, pose/clip part layout, raw scalar primitives, VBR
fixed headers, constant preludes, and VBR frame-block ranges described here.
Confidence is `high` for ordinary animated VBR coefficient bit unpacking,
dequantization, inverse DCT, and integer-frame SQT construction. Confidence is
not yet claimed for raw packed-channel ordering or the one observed 256-flag
VBR edge case.

This reconstruction is source-level Rust. The Xbox executable, PDB, and PPC
disassembly are evidence for behavior; no guest CPU state, translated
instruction stream, or emulator subsystem is part of the runtime.

## Packed names and database records

`Util::FastString<6>::Assign` (`0x824b1a78`) and the 30-character assignment
operator (`0x824ca698`) prove the serialized base-38 alphabet. Six characters
occupy each big-endian `u32`; zero terminates, digits are 1 through 10, letters
are 11 through 36, and underscore is 37. Assignment is case-insensitive.

`Andale::DataBase::SetDBContent` (`0x82d7f7e8`) proves aligned `DataHeader`
records. The typed record kinds are file header 1, clip 2, pose 3, hierarchy 4,
physics parameters 5, blend space 6, phase blend 7, construction selector 8,
sequence 9, and selector 10. `DataBase::GetAnimTree` (`0x82d7def0`) proves the
controller mapping and every child-name stride. The retail corpus validates
205 OffBoard controllers and 944 OnBoard controllers: 1,149 nodes and 3,289
unique edges, with zero unresolved child references.

Type 5 contains a 16-byte header and 112-byte physics-bone records. The body
databases each expose 24 named bones with parent/joint orientations,
dimensions, pose, twist, swing, and mass. Types 6 through 10 are represented as
named Rust controllers rather than opaque payloads. Blend-space volume table
bytes whose weighting role has not yet been established remain explicitly
retained as serialized evidence.

`SkaterPresentationDB.abin` contains the `fullskeleton` database and a
`rig_tpose` hierarchy with 148 bones, trajectory enabled, and 18 named parts.
The body animation databases use a 36-bone, eight-part subset. The real
character graph remaps all 426 selected model bone slots to the 148-bone
presentation hierarchy with zero fallback slots; LOD 0 and 2 gather 131 and
129 packed matrices.

## Clips and pose parts

`Andale::Clip::Init` (`0x82807260`) proves these clip fields relative to the
aligned record body:

| Offset | Field |
| ---: | --- |
| `0x20` | frames per second |
| `0x24` | frame count |
| `0x28` | base speed |
| `0x2c` | attribute-chain offset |
| `0x34` | flags, including loop bit `0x10000000` |
| `0x38` | encoded animation-part offset table |

Duration is `(frame_count - 1) / frames_per_second`. `OffBoard.abin` has 1,016
clips and 84 poses; `OnBoard.abin` has 1,988 clips and 72 poses. All 3,004 body
clips use stored codec tag `0x00524256` (VBR in PPC byte order). Every part owns
its bounded header, compressed payload, and clip-local trailing region.

`Clip::GetAttribute` (`0x82d77c98`) proves that the low byte at body offset
`0x37` is the attribute count and offset `0x2c` locates a variable-size
`AttributeHeader` chain. Each header is 36 bytes and advances by its own size;
its value begins at the next 16-byte boundary. `Attribute::Init`
(`0x82d751a0`) identifies type 0 as scalar f32, type 1 as four f32 lanes, and
type 3 as a 24-byte value. These are now typed and bounded independently of
the final animation part, eliminating the former ambiguous trailing region.

`PhaseBlend::SetAttributes` (`0x82d79528`) asks every child for the controller's
named scalar attribute, sorts children by that coordinate, selects the two
neighbors around the runtime input, and computes a clamped linear blend
coefficient. `GetWeight` (`0x82d6bf10`) distributes `(1-coef)` and `coef`;
`EvalBlendPose` (`0x82d793b8`) evaluates the two selected poses and blends them,
with endpoint culling. Recursive Rust evaluation now also preserves normalized
time while child duration changes, uses the raw coefficient for duration, and
applies the reference cubic outer-edge shaping around the selected skater cull
threshold before blending SQT values. For the proof route, the serialized tree and static
clip coordinates are:

`b_antic_360shuvit_cyc` (`angle`) → center `0`, right `1`, left `-1`;
the left child (`compression`) → normal clip `1`, high-antic clip `0`.

The native probe supplies the exact endpoints `angle=-1` and `compression=1`,
therefore resolving the real leaf clip through data rather than naming it.
At normalized time `0.25`, `angle=-0.75`, `compression=1`, and the recovered
high cull threshold `0.1`, the same evaluator produces weights `0.8857422` for
`r_antic_360shuvit_l_0_cyc` and `0.11425781` for
`r_antic_360shuvit_n_0_cyc`, plus a finite blended 36-bone pose.

## Runtime attributes

`Clip::GetAttributes` (`0x82d80808`) and `Clip::GetAttributeStatus`
(`0x82d6d3b0`) establish the time-dependent collection rules. Static attributes
have status `6` and are always emitted. Timed attributes are active (`5`),
crossed/ended (`9`), or inactive (`17`) according to the exact previous/current
time and loop branches. The normal physics caller uses filter `7`, retaining
static and active attributes.

`PhaseBlend::GetAttributes` (`0x82d7fab0`) evaluates the selected lower child
with the caller's filter and the upper child with all-status filter `31`, then
calls `Attributes::Blend` (`0x82d7f3d0`). Unlike pose weighting, attribute
values use the raw linear neighbor coefficient. Endpoint children are culled;
otherwise only names present in both lists survive, preserving lower-child
order. Scalars and vectors interpolate linearly. Type-3 values preserve the
lower value's first 20 bytes and interpolate their final f32. Timed begin/end
seconds also interpolate unless the attribute is static.

The recursive Rust evaluator implements those rules through the same
controller route as pose sampling. On the real shuvit endpoint it emits static
`angle=-1`, `anticstrength=1`, and `compression=1`. At the non-endpoint input
`angle=-0.75`, it emits the same ordered set with the blended scalar exactly
`angle=-0.75`, while the independently shaped pose weights remain
`0.8857422/0.11425781`.

The type-3 view is now typed further as a packed `FastString<30>` bone name and
big-endian f32 transition. Complete `OnBoard.abin` validation decodes all 864
records canonically: push/brake contacts target the two toe-base bones, and
hand-grab contacts target the authored left/right hand bones. No 24-byte value
is interpreted as a guest structure by the native runtime.

An `Andale::AnimationPart` is 16 bytes: packed part header, compressed byte
count, compression-header offset, and compressed-data offset. Pose part tables
use their top offset byte as a hierarchy-part name index. The packed header
also exposes part index, bone count, and compression-header byte count.

## Raw-discretize primitives

The PDB types and named functions establish three scalar formats:

| Type | Bytes | Decoder |
| --- | ---: | --- |
| `tQuatQuantize3` | 6 | `QuatDecompressQuantize3` at `0x827d1220` |
| `tQuatQuantize4` | 8 | `QuatDecompressQuantize4` at `0x827d1300` |
| `tTransCompressed` | 6 | `TransDecompress` at `0x827d13b8` |

All stored components are big-endian signed `i16` values scaled by `1/32767`.
Quaternion-3 reconstructs a non-negative fourth component from unit length;
quaternion-4 normalizes all four expanded components. Translation multiplies
the three expanded components by the supplied range and writes homogeneous
`w = 1`. Readable Rust implements these operations directly.

The full presentation `rig_tpose` uses a fourth format established by
`RawDiscretize::QuatCompress` / `QuatDecompress` at `0x825e9b00` /
`0x825e9e68`. Its u32 stores 11-bit azimuth, 10-bit elevation, and an 11-bit
vector magnitude. The scalar is the non-negative unit-length remainder. In the
real 18-part pose, channel mask 2 selects that quaternion and mask 4 selects
three unquantized big-endian f32 translation lanes; selected values are
interleaved per bone. The exact `[format 0, format 1]` combination decodes all
148 authored SQT transforms. Other raw selector combinations are deliberately
unsupported until evidenced.

## VBR fixed header and constant prelude

`VBRDecompressor::InitPerAnim` (`0x82edc1f8`) and `UnPackHeaderBits`
(`0x82edbbe8`) establish this compression-header layout:

1. A 16-byte value header: constant-map RLE count, frame count, DCT minimum and
   maximum, channel-block count, and constant-palette count.
2. The normalized constant palette as big-endian `f32` values.
3. One big-endian `u16` compressed size for each group of up to eight frames.
4. Padding to 16 bytes.
5. One 16-byte descriptor per channel block: animated and constant entry
   counts, dequantization minimum/maximum, and scalar component count.

`Andale::FetchSys::FetchSys` (`0x82d6f370`) configures exactly three 48-byte SQT
channels: scale has 3 components at byte 0, quaternion has 4 at byte 16, and
translation has 3 at byte 32.

The compressed payload begins with one palette index per constant scalar.
Values expand as `minimum + palette[index] * (maximum - minimum)`. Alternating
run lengths, starting with non-constant, classify each channel entry. A
four-byte work-table slot follows for every animated scalar, then the sized
eight-frame blocks begin.

All real headers and preludes validate. Constant-only parts materialize as
native `AndaleSqt` values: 3,024 OffBoard body bones, 2,592 OnBoard body bones,
and 62 face-simulation bones. The clip size tables resolve these animated
regions exactly:

| Database | Eight-frame blocks | Bytes | Unexplained trailing bytes |
| --- | ---: | ---: | ---: |
| OffBoard | 22,464 | 1,440,725 | 0 |
| OnBoard | 86,720 | 3,585,516 | 0 |

`ANIM_CODEC::InitIDctTablesInv` (`0x82edb3d0`) constructs the exact inverse
DCT table used for each eight-frame block. For output sample `n`, coefficient
zero is weighted by `1/8`; coefficient `k` from one through seven is weighted
by `cos((n + 1/2) * k * pi / 8) / 4`. This transform is now implemented as
readable Rust and locked down with DC and first-frequency basis-vector tests.

Each ordinary frame block begins with one quantization-table index for each of
the three channel types. A one-byte significance-bit count and LSB-first
significance bitset follow. The four-byte-per-scalar work table contains eight
four-bit coefficient widths; a present coefficient stores a sign plus the
configured magnitude bits in a second LSB-first stream. Native Rust expands
these sign-magnitude integer rows, and the packed payload size is derived from
the exact bits consumed rather than trusted as padding.

All 86,720 OnBoard blocks and 22,421 of 22,464 OffBoard blocks split and consume
exactly, including 2,650,752 and 1,124,007 coefficient-payload bytes. The 43
exceptions are all `ofb_runout_run_0_toloco` part 4. Its width table requests
256 significance flags while the serialized one-byte count is zero; the
reference PPC path also branches on that byte before locating the coefficient
stream. The historical 43-exception count was not reproduced by the 2026-09-12
direct read: eight unique blocks occur in this part. Fresh single/two-frame
source review proves overlapping streams when the actual count byte is zero;
it does not mean a 32-byte stream advance. The bounded decoder closure and
signed-zero preservation are documented in
[`animated-vbr-zero-count.md`](animated-vbr-zero-count.md), with root
validation pending.

`VBRDecompressor::Init` and `DecompressFrameBlock` establish the quantization
row construction and per-channel range mapping. Readable Rust now reconstructs
raw scale/quaternion/translation samples and implements the caller's linear
scale/translation interpolation plus shortest-path normalized quaternion
interpolation. `r_antic_360shuvit_l_0_cyc` frame 0 evaluates into all 36 SQT
bones across the eight real OnBoard hierarchy parts; the probe reports unit
quaternion lengths after the same normalized blend boundary.

## Presentation pose and skinning

`SkaterPresentation::ReadSkater` (`0x827ddb60`) copies body animation into
named ranges of the full presentation hierarchy. The OnBoard hierarchy's eight
parts match full-rig trajectory, torso, head, arms, legs, skateboard,
skateboard-trucks, and skateboard-procedural ranges, totaling 36 bones.
Readable `LocalPose` code performs that named overlay on top of the decoded
148-bone authored bind pose.

`Anim_Tools::SQTtoLocal` (`0x82a22bb0`) builds column-axis matrices from SQT.
`Anim_Tools::Pose` (`0x82a23068`) composes each local matrix with its parent,
except that roots and children of the trajectory node copy their local matrix
directly. `PoseWithMatMult` (`0x82a23528`) then multiplies each global matrix by
the hierarchy-ordered inverse bind pose. Native Rust implements those rules as
named matrix operations rather than translated PPC instructions.

The real `r_antic_360shuvit_l_0_cyc` frame now passes through that complete
path: 36 sampled body transforms overlay 148 bind transforms, producing 148
finite global and skin matrices. The existing character remaps gather all 131
LOD-0 and 129 LOD-2 palette matrices with zero `Hips` fallback. As an
independent convention check, the authored bind globals times real model
inverse-bind matrices differ from identity by at most `0.030320402` across the
106 referenced hierarchy bones at both LODs.

## Next reconstruction boundary

`VBRDecompressor::Vector_UnPackFrameBlockAndDecompressOneFrame`
(`0x82edbd18`), `DecompressFrameBlock` (`0x82edb790`), `DecompressFrame`
(`0x82edc788`), and `DecompressTwoConsecFrames` (`0x82edc8d8`) are the primary
remaining codec evidence. The next implementation should:

1. Resolve the isolated 256-significance-flag OffBoard block behavior against
   reference execution, and lift the unused raw-discretize selectors.
2. Lift the remaining clip trajectory extraction and controller kinds beyond
   the now-recursive PhaseBlend path.
3. Reconstruct the specialized hand, face, hips, truck, and board correction
   stages visible in `ReadSkater`.
4. Capture reference-frame pose and skin matrices for direct numeric parity
   checks.

This checkpoint has a native animated-trick skinning and animation-to-physics
handoff path, but makes no playable-skater or physics-parity claim.


### 2026-09-11 buffered bind-pose numeric review

Root independently re-read SQTtoLocal82a22bb0, Pose82a23068 and
ACSpose_job82a23e00 before admitting the buffered startup evaluator. The
existing generic quaternion formula doubled inputs before products; the
source82a22d24..64 rounds products first, then sums/differences at2d6c..8c,
then doubles off-diagonal values and applies scale. Diagonals use fused
1 - 2*sum at2d90/db0/dd8. The native VA128 decoding displayed by this IDA
instance adds32 to source-A registers reported>=64; raw field checks and
independent lane-definition/use traces recover the actual registers.
Ground-path independently repeated the lane trace and approved these orders.

Pose82a23488..d4 starts each orientation axis with rounded parent-X times
local-X, then fused parent-Y/local-Y and parent-Z/local-Z additions.
Translation starts with fused parent-X/local-X plus parent translation.
The runtime hierarchy evaluator now uses this affine composition; the
separate generic inverse-bind matrix multiplication is unchanged. Regression
cases distinguish fused cancellation residue and premature doubling overflow.
This establishes the recovered arithmetic order; host subnormal/NaN payload
behavior has not been certified against Xbox VMX hardware. Updated original
asset and regression gates are pending at this entry.

Follow-up independent review caught exact-zero sign: VNMSUB negates the
rounded fused subtraction, rather than negating a multiplicand before FMA.
The diagonal implementation now uses `-sum.mul_add(2.0, -1.0)` and a bitwise
negative-zero regression. Architectural reference: [NXP AltiVec Programming
Interface Manual, vec_nmsub](https://www.nxp.com/docs/en/reference-manual/ALTIVECPIM.pdf),
Figure4-85, explicitly specifies negative RndToFPNearest(a*b-c).
Original-data gate39095 passed15/15 in2.55s before this signed-zero follow-up;
that gate includes the corrected retained-output allocation and fixture lock.
