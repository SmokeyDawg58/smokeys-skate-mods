# Presentation Q3 and translation compression primitives

Unregistered implementation packet, September 12, 2026. Files:
`skater_presentation_compression.rs`, its `combined` child, and focused tests.
These implement arithmetic over borrowed source records and existing caller
output slices. They do not initialize presentation, publish streams, or render a
skater. Root retains WriteSkater/Q4 and later read/publication integration.

## Source and ownership

`ANIM_CODEC::RawDiscretize::QuatCompressQuantize3` is
`827d5af0..827d5c54`; `TransCompress` is `827d6200..827d646c`;
`QuatTransCompressQ3` is `827d5c58..827d61f8`. WriteSkater calls them for
hands at `8edc/8f44`, trucks at `8e98`, and face ranges at
`8fe4/9040/9094`. The primary `CompressionSource::Sqts` borrows actual
Andale SQT records, directly accessing rotation and translation at the source
load/prefetch sites. The vector-array variant supports controlled callers with
one arithmetic implementation. No intermediate gathered pose or fresh retained
output owner is created. Rust borrowing excludes concurrent mutation and native
source/destination aliasing; no parity for such aliases is claimed.

`WriteData827dc544..827dc5b4` constructs splatted ratio32767 (`46fffe00`),
positive zero, one and minus one. Q3 primitives do not read the supplied one
parameter. Parameters retain the other full four-lane vectors, including the
all-true test of supplied zero versus the splatted scaled quaternion W. An
ordered all-true result multiplies all scaled lanes by supplied minus one;
unordered comparisons do not flip. There is no normalization or clamping.

## Corrected translation range operands

TransCompress lower comparisons raw `62b0=1b1fc965`, `6334=1a5f9965`,
`63b8=199f6965` decode VA63, not the misleading IDA-rendered v95. v63 is
X/Y/Z splatted at `623c/62e4/6368`. Upper compares use range splats
v60/v54/v48 against v63. CR bit24, extracted by `extrwi ...,1,24`, must be
nonzero after each compare. Thus ordered `range > value` then ordered
`value > -range` are required for each axis in XYZ order; equality and NaN
assert, and no W bound is tested. Caller `WriteSkater8e8c` explicitly sets
truck temporary W=1 before compression with range0.5, independently confirming
that a four-source-lane range interpretation would be incorrect. Combined
loop/tail compare operands use the same scalar-splat pattern.

Standalone translation requests one reciprocal estimate of the splatted range,
then executes two refinements: residual is the NEGATION of rounded
`fma(seed,range,-1)`, followed by `fma(seed,residual,seed)`. It multiplies the
refined reciprocal by each supplied ratio lane, then each translation lane.
This preserves native VNMSUB exact-zero sign semantics. Combined compression
instead reads the constant32767 at `820229b8`, executes scalar `fdivs` at
`5cb0` and reuses the rounded splat for every translation. These paths are kept
separate. The selected WriteSkater range is0.5, loaded at `8e44`.

## Conversions and partial stores

Every fctiwz uses existing ReckoningNumeric::truncate_i32. Low16 extraction uses
integer bit bytes, not a saturating cast or a floating clamp. Destinations are
logical `[u16;3]` XYZ records; the later stream writer must emit big-endian
halfwords into the actual retained caller stream.

* Q3 loop converts Y,X,Z before storing Y,X,Z (`5b84..5bb0`). Its tail
  converts Y,X, stores Y at `5c34`, then converts Z and stores X,Z.
* Standalone translation converts Z,X,Y before storing X,Z,Y (`641c..6450`).
* Combined loop converts Qz,Tx,Ty,Qx,Qy,Tz, then stores
  Qz,Tz,Ty,Tx,Qy,Qx (`5ef0..5f68`).
* Combined tail converts Qy,Qz,Qx, stores Qy at `61a4`, converts Ty,Tz,Tx,
  then stores Qx,Tx,Qz,Tz,Ty (`6174..61e4`).

Next-record Q (then T for combined) prefetch occurs before current-record
arithmetic. The checked borrowed views preserve that ordering on a missing
source record. A later numeric/range failure preserves all already completed
destination writes. Destination capacity is checked as a host memory-admission
prerequisite. Nonpositive counts return an explicit admission error: the native
invalid call still reads a first input and executes its final-element tail,
so this is deliberately not represented as an empty successful operation.

## Validation and limits

Nine focused tests cover low16 wrapping, scaled-W and nonuniform-zero gates,
strict range boundaries/NaNs/ignored W, estimate/refinement distinction,
standalone and combined conversion order, loop-versus-tail partial stores,
late range failure, early prefetch failure, and nonpositive-count admission.
Files are formatted. No Cargo or test execution was performed by this agent;
registration and full gates belong to root.

Confidence: high for raw instruction order, original caller constants, and
borrowed-record/destination behavior. Native exceptional FP status, Xenon
estimate seeds, host division/FMA denormal and NaN-payload details remain
explicit numerical boundaries. This is not full stream or animation parity.


## Root independent code review, September12

Root read the Q3, standalone T and combined Q3/T code against the retained raw
source review. Fresh disassembly5ef0..5f68 and6170..61f8 confirms combined
conversion/store ordering and the final tail's early QY store. The standalone
reciprocal-refinement graph stays distinct from combined scalar division.
These files remain unregistered and untested pending presentation integration;
review acceptance is not an end-to-end presentation result.
