# Live presentation compression continuation

September22 status note: the historical missing hand/face producer described below
has since been implemented in the retained Actor hands/face session and selected
frame path. Presentation InitAnimation/offset/restore/compression/root/twist drafts
already exist but remain unregistered; do not recreate them. The live simulation
publication to WriteSkater/ReadSkater/full-pose/GPU consumer chain is still missing.
This note updates integration status, not the historical source evidence below.

Root source review, September 12, 2026. This is a continuation packet, not a
registered implementation or rendered-skater claim. Preserve the existing
Andale decompression code; the missing direction is live simulation output to
the original presentation pose stream.

WriteSkater827d8ab8 obtains actual local matrices at8b08, face SQTs at8b20,
right-hand SQTs at8b38 and left-hand SQTs at8b4c, in that order. It writes body
Q4 rotations using InitOffsets' ordered indices, then truck translations,
left-hand Q3, right-hand Q3, and Face0/1/2 Q3 plus translations. Null hand
pointers and the actual false IsInitFace branch have explicit zero writes;
those branches must not replace missing initialized buffers. SubmitData90ec
publishes the quaternion block and9100 the translation block, both alignment2.

InitOffsets827dd548 separately derives matrix Q4 indices from Torso, Head,
Arms, Legs, Skateboard and Skateboard_Trucks. Trucks also supply matrix
translations. Hand bone counts and Face0/1/2 ranges come from their original
hierarchies. Rawd944 checks the completed Q4 count against120. Rawddaec..ddb14
computes pose bytes as8*Q4+6*(Q3+T). The existing unregistered offsets draft
matches this valid-data order; host checked-range errors are admission bounds,
not evidence of the native malformed-data failure behavior.

QuatCompressQuantize3 is827d5af0..5c54. It scales the quaternion before testing
scaled W against the supplied zero vector. Ordered negative W multiplies the
scaled quaternion by the supplied minus-one vector; other values retain it.
It converts Y, X and Z using fctiwz, retaining each integer's low16 bits and
writing Y, X, Z into their respective destination positions. The loop has a
separate final-element tail and reads the first source quaternion even for a
nonpositive count; do not invent a safe empty native call. Reuse the existing
ReckoningNumeric integer-conversion contract for explicit unsupported numeric
cases instead of relying on Rust saturating float casts.

TransCompress827d6200..646c checks ranges before conversion, then computes a
reciprocal estimate with two original VNMSUB refinements, multiplies it by the
supplied ratio, and multiplies the source vector. fctiwz conversions occur Z,
X,Y; destination writes X,Z,Y preserve source partial-store ordering.
Independent review resolves IDA's erroneous high-register rendering: raw
62b0=1b1fc965,6334=1a5f9965,63b8=199f6965 decode VA63, the corresponding
XYZ splat, rather than v127. Each axis requires strictly -range < value < range;
NaN and equality assert. No W range check occurs. The caller explicitly writes
temporary truck translation W=1 at8e8c while supplying range0.5.

WriteData supplies exact splats ratio32767 (46fffe00), zero+0, one+1 and
minusone-1 atc544..5b4. Truck and face translation ranges are0.5. The combined
QuatTransCompressQ3 at827d5c58..6200 uses scalar fdivs(32767,range) at5cb0,
then direct translation multiplies, unlike TransCompress's estimate/refinement
graph. Its Q/T conversions and stores interleave. Reuse common range and
hemisphere operations only where ordering agrees; preserve the distinct numeric
graphs and stores. Review of its complete mutation schedule is still pending.
Body Q4's
inline matrix-to-quaternion candidate at8c98..8dbc should reuse the existing
reviewed quaternion candidate only after comparing its operations and absence
or presence of final normalization. ReadSkater, live hips/board overrides,
UpdateBodies, expressions and palette publication remain subsequent work.

September22 bounded re-review: combined mutation schedule and Q4 conversion/store
order match fresh raw disassembly. Later September12 compression-primitives.md
append had already superseded the historical pending note above. Reuse existing
primitives; no recreation is required. Registration and same-Actor channel
publication remain open, with host numeric limitations unchanged.
