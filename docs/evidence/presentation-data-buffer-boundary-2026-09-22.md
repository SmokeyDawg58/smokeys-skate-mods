# Presentation submission and readable backing boundary

Read-only source audit; no duplicate DataProvider implementation introduced.

WriteData827dbf98 submits validCount (4 bytes, alignment4), then all208-byte
skater headers (alignment16), then WriteSkater827d8ab8 Q/T blocks (alignment2).
GetRootMatrix825f04a0 reads actual PhysOut Skeleton root matrix. Fresh
GetLocalPose at827dc3c0 supplies hips and board matrices; these are copied to
header before character damage/wipeout fields and before pose compression.
Compressed offset is208*(validCount-currentIndex)+precedingPoseBytes, packed
into high16 at827dc4e0. Root matrix, peer/local/collidable/opacity/damage/state
metadata must come from their actual owners, not inferred compressed bones.

GetPeerId825f0068 calls GetPlayerID then PlayersBackEnd::GetPlayerPeerID8255aba8,
which loads the actual160-byte player row+152 qword. IsLocal825f0138 compares
local index first then local simulation index. IsCollidable825f01c8 queries
actual Ghost and short-circuits true only on actual null Ghost.
GetBoardSpeed825f0420 reads SkateboardMotion surface speed;
GetTimeInState825f0460 reads StateUpdateCount, not elapsed float time.
GetCharacterDamageData825f05b8 reads actual Scoring2 seven packed area pairs.

DataProvider::SubmitData82990858 asserts AllowedToSubmit then calls
DataBuffer::AddData829900c8. AddData writes zero alignment padding first,
updates the six-bit maximum alignment and asserts representability, calls
SimpleDataBuffer::WriteData8298fd70, then updates25-bit written size. WriteData
asserts negative size, and for positive size validates location/capacity and
copies the exact bytes. Its return is size. DataBuffer ctor82990dd0 allocates
the requested capacity without a proved zero fill; metadata reset preserves
bit0 and clears size/alignment. Unwritten capacity is not known zero.

PostGather82990a40 uncompressed path copies only written submission bytes into
actual gather buffer, honoring recorded alignment. A following provider's
subframe or alignment padding can therefore supply adjacent bytes, but those
bytes depend on actual ordering. Compressed path uses actual codec output and
separate restore buffers; do not fabricate their contents. End of PostGather
clears submission size/alignment while retaining allocation contents.

ReadSkater packed scalar expansion can observe the sign of a following
halfword (independently audited by read author/reviewer). Its logical final
T record does not prove an extra known zero halfword. The read API must accept
actual readable backing or report the missing boundary. Any eventual portable
allocation policy for undefined original backing must be explicit and cannot
be presented as source-proven zero padding. Frame producer/restore lifecycle
and exact original-data end-of-buffer behavior remain unvalidated.

Portable allocation decision (independently reviewed): initial host backing
may be explicitly zero-initialized once, retained across frames, and mutated
only at native data/padding writes. This selects unspecified allocator contents,
not source-proven padding. ReadSkater observes actual backing bytes; no decoder
appended zeros or per-frame tail clearing. Xbox tail contents may differ and
thus influence packed widening; this numerical platform boundary is documented.
