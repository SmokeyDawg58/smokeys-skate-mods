# Animated VBR zero significance count

Author source review, 2026-09-12. Confidence: **high** for the recovered
stream ownership and sign handling; Xbox numerical parity still requires a
reference execution trace. This packet supplements `andale-animation.md`.

## Native producers and consumers

`ANIM_CODEC::VBRDecompressor::DecompressFrame` at `0x82edc788` calls
`Vector_UnPackFrameBlockAndDecompressOneFrame` (`0x82edbd18`, call
`0x82edc804`) after advancing past the header's channel-count quantization
selector bytes. `DecompressTwoConsecFrames` at `0x82edc8d8` repeats the same
unpack logic inline, before reconstructing both requested frame samples.

For pointer `p` to the serialized count byte, the single-frame routine reads
the initial significance word little-endian from `p+1..p+4`
(`0x82edbdb0..0x82edbde8`), and establishes its refill pointer at `p+5`.
It reads the actual byte at `p` at `0x82edbdd4`; `cmplwi cr6,count,0` and
`beq` at `0x82edbdf4/8` skip the `(count+7)>>3` pointer advance when zero.
The coefficient pointer is then `p+1+ceil(count/8)` at `0x82edbe0c`.
Raw comparison/branch bytes are `2b0b0000 419a0010` (BO12, BI26, EQ).
The inline two-frame path has the same count load at `0x82edca98`,
comparison/branch at `0x82edcab8/bc`, and final pointer at `0x82edcad0`.

Thus zero **does not encode an advance of 32 bytes**. When the width table
demands 256 flags, both streams start at the same allocation byte. Their
logical reads overlap. The width loop at `0x82edbed4..0x82edbf84` consumes
one significance flag for every nonzero nibble, independent of the count
byte. A set flag consumes `width+1` coefficient bits. Width-zero entries
consume neither stream. The loop processes low width nibbles first from each
big-endian width word, and consumes both streams least-significant-bit first.
There is no native assertion that the declared flag count equals consumption,
or that coefficients consume the entire sized frame block. Prefetch/refill
loads may exceed logical consumption; reconstructing values requires only
the logically read bytes and must never synthesize missing bytes.

The native output coefficients are float vectors, not signed integers.
`0x82edbf18` subtracts the packed vector from all-ones and
`0x82edbf38/50` selects the sign bit; `0x82edbf98` converts the unsigned
magnitude to float and `0x82edbfac` ORs the sign into it. A present coefficient
with sign zero and magnitude zero is negative floating-point zero. An absent
coefficient remains positive zero. An `i16` diagnostic representation cannot
preserve that distinction for runtime reconstruction.

## Current original-data observation

Read-only script `artifacts/vbr-zero-count-review.py` opens the original
external `data/big/anim.big`. Its matching Viv4 BH record has hash `07138dc7`,
offset 6912, stored size 4597256, unpacked-size field zero. No original bytes
are copied into this repository. Root used:
`<game folder>`.

`ofb_runout_run_0_toloco` record offset `0x1f4360`, zero-based part index 4
(part offset `0x11d0`) has 61 frames, 32 scalar rows, and 256 nonzero width
nibbles. All eight unique frame blocks declare count zero:

| Block | Compressed offset | Block bytes | Coefficient bytes read | Available bytes | Negative zero coefficients |
|---|---:|---:|---:|---:|---:|
| 0 | 180 | 243 | 207 | 239 | 4 |
| 1 | 423 | 237 | 201 | 233 | 5 |
| 2 | 660 | 244 | 208 | 240 | 3 |
| 3 | 904 | 231 | 195 | 227 | 5 |
| 4 | 1135 | 236 | 200 | 232 | 5 |
| 5 | 1371 | 235 | 199 | 231 | 6 |
| 6 | 1606 | 229 | 193 | 225 | 5 |
| 7 | 1835 | 240 | 204 | 236 | 6 |

Every logical read remains inside its own sized block. Each has 32 bytes
after the logically consumed coefficient bytes, which must remain retained
as serialized bytes rather than treated as an extra stream advance. The
historical evidence reported 43 exceptional blocks; that count is not
reproduced by this direct read and must not be repeated as a new result.
The adjacent part index 3 declares 251 flags and consumes each of its eight
coefficient payloads exactly, with no negative zero coefficients.

## Implementation and validation

`AndaleVbrFrameBlock` retains the actual serialized count byte.
`decode_frame_block` admits the observed count-zero/256-required-flags case,
copies the 32 logically read flag bytes, and retains the coefficient stream
starting at the same byte, including its unused suffix. Other count mismatches
remain rejected. Both logical streams must fit inside their own sized block;
the implementation does not emulate speculative native overreads.
Ordinary blocks retain their previous exact coefficient-payload-size check.

The shared unpack loop has integer and float conversions. The public integer
API remains a diagnostic view; `sample_vbr_sqt` now selects the private float
path through reconstruction, preserving native negative zero. The existing
public integer scalar reconstruction helper delegates to that same float
reconstructor after converting its arguments.

Three focused tests cover native overlap/sign handling, truncation of each
logical stream, and unchanged ordinary mismatch/trailing-byte rejection.
The ignored original-data test asserts eight unique blocks and the negative
zero counts above, then evaluates all 61 part frames through the production
SQT sampler. Rust tests are written but not run by the author: root owns the
coherent Cargo gates. The read-only Python probe ran successfully and narrow
Rust formatting completed. No Xbox bitwise numerical claim is made.
