# BIG archive evidence

Confidence: **high** for `BIG4/BIGF` and `Viv4` parsing and lookup. Compression
payloads use RefPack; the buffer decompressor is reconstructed and bounds-checked.

## Reference functions and types

| Address | Symbol |
|---:|---|
| `0x8272a2d0` | `Sk8::BigFileMan::MountBigFileGroup` |
| `0x8296e958` | `xcore::BigFile::Mount` |
| `0x82bb6d50` | `rw::core::Big::GetHeaderType` |
| `0x82bb7510` | `rw::core::Big::GetHeaderSize` |
| `0x82bb7c40` | `rw::core::Big::LocateEntry` |
| `0x82bb8398` | `rw::core::Big::FindFileByHash` |
| `0x82bb8680` | `rw::core::Big::Locate` |
| `0x82bb91d8` | `rw::core::BigHandler::AddArchiveFromFile` |
| `0x82bc9140` | `rw::core::codec::RefpackBuf::Decompress` |
| `0x82bc93e8` | `rw::core::codec::RefpackBuf::GetDecompressedSize` |

`Sk8::BigFileMan` is exactly 80 bytes in the 32-bit reference ABI: an array of
20 four-byte `xcore::BigFile` handles. Its static metadata records are 264 bytes:
a 32-bit mount group, a boolean selecting an external `.bh` table, a 256-byte
path, and tail padding.

## Formats

Named `BIG4/BIGF` headers contain a 16-byte fixed header followed by variable
records: big-endian offset, big-endian stored size, and a null-terminated path.
Path lookup is ASCII case-insensitive.

`Viv4` headers are normally external `.bh` files. Each 16-byte entry contains
big-endian offset, stored size, unpacked size (zero means stored size), and a
32-bit DJB2 path hash. `rw::core::Big::Big` sorts these entries by hash before
binary search. `AddArchiveFromFile` falls back to the `.big` header if a requested
`.bh` cannot be opened.

## Exact mount groups

- Boot: `render`, `miscboot`, `colourcubes`, `db`.
- Load: `scene`, `worldproxy`, `worldbam`, `worldmisc`, the three `assettoc`
  archives, `miscload`, `globallocators`, `anim`.
- Game: `missions`, `aipaths`.
- Characters: `livingworld`, `marquee`, `ropa2`.
- Create-a-character: `createacharacter`.

The supplied data root currently has no `data\\big\\colourcubes.big`; whether it
is optional, folded into another archive, or a build mismatch remains open.
