# XSF payload transport

This record covers logical BIG-entry range access and the payload-relative XSF
framing used below the outer collection scheduler. It does not claim that the
live scheduler yet materializes or publishes collection resources.

## Native boundary

`assman::cAssetStreamer::Update` at `0x82d60ec0` passes the filename, full
64-bit collection offset, raw stream format, and signed asset count to
`assman::tLoadCollectionRequest::Load` at `0x82d54bd0`. The native load calls
`Stream::QueueFile` at that offset; it does not pass the XSM size to the raw
parser. `ParseRawFunction` at `0x82d546b0` treats byte zero at that offset as a
24-byte `tCollectionAsset`, obtains every block size from its stride at `+0x10`,
and validates every stride, including the final one, in `1..=0x280000`.

`MountSet::read_path_range` therefore defines its range relative to the
logical archive entry. It preserves first-mounted-match ownership. For a raw
entry it probes the actual entry start for the RefPack marker and then seeks
and reads only the requested range. For an outer-RefPack entry it reads and
expands the complete stored entry before slicing because decoded offsets cannot
be mapped to physical compressed offsets independently. Hash-table unpacked
sizes remain authoritative for raw and compressed entries; named BIG entries
do not falsely treat stored size as decoded size.

`CollectionPayload::parse` is deliberately separate from the existing
full-file `StreamFile::parse`: its input begins with the first collection-asset
record, not `SFIL`. It preserves asset order and full IDs, returns
payload-relative header/data ranges, counts the final stride in
`consumed_size`, and permits bytes after that consumed span. Checked range and
overflow failures are high-confidence safe host behavior for malformed data,
whose native path relies on assertions and valid authored bytes. The parser
also rejects a declared count larger than the payload could possibly hold
before reserving its output vector and maps reservation failure to a typed
error; neither behavior is claimed as native recovery.

## Rob's Park corpus gate

The supplied `worldbam.big` is 2,445,763,840 bytes with SHA-256
`d304bd10c08375ab09edbcaeb0897897a8347ed7b310cf2d2eaabde5c096fe66`.
At the exact `A_RobsPark` focus, the recovered selectors choose six
Presentation, four Simulation, and four Texture collections. Across all 14:

- every catalog path resolves in the first owning mount;
- every `SFIL` is version 3 with a 128-byte header;
- every XSM payload offset is `0x80` and `offset + size` equals logical XSF
  length;
- every outer BIG entry is raw;
- range parsing agrees field-for-field with the independent full-file parser;
- parsed counts are 2,528 Presentation, 68 Simulation, and 685 Texture blocks;
- `consumed_size` equals the registered XSM size for every payload;
- all 3,281 block IDs resolve in the matching XST and all arena envelopes
  decode successfully.

Cross-file asset repetition is valid: the 3,281 occurrences represent 1,780
unique IDs. Neither the transport nor the request pump may deduplicate them.

These observations are exact for the supplied corpus. The range API's generic
outer-RefPack fallback is covered by synthetic fixtures because these selected
world entries are raw.

## Remaining boundary

The native count/stride-driven chunk producer, device scheduling, cancellation,
decompression worker, asset-type constructors, registry publication,
resident-reference map, and unload destruction remain separate work. Reading
the registered XSM range into one host buffer would match the supplied corpus
but would be only a host envelope, not the recovered native I/O cadence.

The request-side parser, ring, and cadence are recorded separately in
[`collection-load-pump.md`](collection-load-pump.md). The next integration must
join these layers through an offset-aware operation backend without reusing the
whole-path manifest loader or reporting completion before opaque resource
materialization completes.
