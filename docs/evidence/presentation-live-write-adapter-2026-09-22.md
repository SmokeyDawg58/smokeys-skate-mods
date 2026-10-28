# Same Actor presentation writer

Source/code confidence: high within the explicitly admitted initialized,
synchronized pose route. Registered September22; compilation/original-data
validation pending. This is not full DataProvider or renderer publication.

WriteSkater827d8ab8 captures GetLocalPose825f04e0, GetFace825f05f8,
GetRight825f06e8 and GetLeft825f0670 in that order. Local getter ultimately
reads actual HiLOD82dffd88 AnimOutPhysIn local pose; the adapter validates
that physical and Actor publication refer to the same allocation. Hands and
face retain actual OwnedAnimationSqtBuffer allocations. Unknown/unconstructed
non-null hands are not substituted with the source's null-hand zero branch.

Matrix Q4 loop827d8c94..8e20 reuses the existing source-reviewed compression
primitive and actual InitOffsets ordered indices. Matrix translation8e54..8eac
loads matrix translation, overwrites W with one, and compresses at range0.5.
Left hand precedes right hand. IsInitFace825f0230 reads actual known flag23
(00800000) only after both hands. Face0/1/2 use existing combined Q3/T at0.5;
false face writes alternating zero Q/T records without reading face values.
Scratch records remain caller-owned and retain source partial prefixes.

Native WriteData827dbf98 (constant setup827dc564) supplies ratio bits46fffe00, zero+0 and minus-one
vectors. These are that caller's constants, not tuning defaults. Q4 uses
basis XYZ only; explicit translation W overwrite means unknown affine W is
not an error on this path. Existing reciprocal estimate/truncate numeric
services remain explicit host/reference numeric boundaries.

Submission90ec precedes9100 (second call address827d9100): first block is
Q4 XYZW then left/right/face Q3, second is translations, all logical halfwords
encoded big endian, alignment2. DataProvider::SubmitData82990858 asserts
AllowedToSubmit then calls DataBuffer::AddData829908a8. The writer requires an
explicit submission service; its corpus recorder does not claim to implement
that provider. A second submission failure retains the first publication.

Independent reviewer approved source adapter and writer, including same owner,
getter order, numerical vectors, matrix layout, flag timing and byte stream.
New original corpus call occurs only after selected post-solve returns. It
checks both ordered block sizes/alignment, big-endian Q prefix and retention
when second submission is deliberately rejected. Capturing source allocations
alone is not a frame completion token. Full WriteData header/hips-board producer,
actual buffer lifecycle, ReadSkater and rendering remain separate dependencies.

Validation update: fmt-presentation-startup-1, clippy-presentation-startup-5
and workspace-presentation-startup-1 pass (4,539unit+6doc). Original fresh
frames0/1 each compress860 bytes and pass second-submission retention checks;
source-owned initialized148-bone presentation startup/face restore also passes.
Frame2 stops later in Actor ChannelBlendAnim before another pose write. New
unit tests preserve first Q4 on missing later matrix and left-hand output on
right-hand failure; missing face-query owner is observed only after both hands.
