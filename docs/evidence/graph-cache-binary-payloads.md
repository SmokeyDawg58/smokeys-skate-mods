# Graph cache fronts and compiled payloads

## Scope and confidence

This checkpoint reconstructs two separate boundaries:

- the `StateGraph` and `BehaviourGraph` cache-front path, hash, hit, flush,
  negative-entry, replacement, capacity, and ownership rules; and
- the lossless recursive wire grammar of compiled `.stategraph` payloads.

The successful well-formed binary paths are **exact** against the matched
debug XEX/PDB and shipped corpus. Checked path failures, malformed-buffer
errors, allocation failures, and Rust ownership types are explicit host-safety
substitutions for native asserts, fixed-pool corruption, unbounded C-string
scans, and intrusive pointers. XML parsing, concrete state factory classes,
controller construction, graph execution, and `InitPropData` remain open.

Principal authorities are:

- `StateGraph::Cache::Load`, `0x82cf8af0..0x82cf8cf4`;
- `StateGraph::Cache::Flush`, `0x82cf3060`;
- `StateGraph::Parser::ParseBinary`, `0x82cf8938..0x82cf8af0`;
- `StateGraph::Parser::ReadElement`, `0x82cf8740..0x82cf8868`;
- `BehaviourGraph::Cache::Load`, `0x82cf6ca0`;
- `BehaviourGraph::BehaviourParser::ParseBinary`, `0x82cf6750..0x82cf68dc`;
- `BehaviourGraph::BehaviourParser::ReadElement`, `0x82cf6070..0x82cf6188`;
- `StateGraph::BinaryAttributeMap`, `0x82cf4298..0x82cf43ac`;
- `stringhash32`, `0x82985a40`, and `hash32`, `0x82985788`.

## Cache-front rules

The State cache performs a case-sensitive substring search for `data\` before
slash conversion. It prefixes `data\` only when that byte sequence appears
nowhere in the caller path. The Behaviour cache always prefixes another
`data\`. Both convert `/` to `\` and use only
`GetHashValue32(normalized_original_path)` as identity; they do not retain a
path for collision disambiguation.

A present entry is a hit even when its value is null. `flushCache=false`
returns that positive or negative result without decoding. A miss or true
flush decodes first, then replaces/inserts the hash slot. Backend failure is a
typed host boundary and leaves the cache unchanged.

The State cache is a fixed map with 32 no-overflow nodes. Native attempts an
invalid write on a 33rd unique key after parsing. The safe host preserves that
ordering but reports `StateCapacity` after the decoder returns. Negative
entries consume nodes. Whole-cache flush/destruction releases entries in
descending unsigned-key order. State loads return borrowed values.

The Behaviour cache is dynamically allocated and returns an owning handle.
The readable `Arc` model preserves caller ownership across replacement and
cache destruction. Native hash-bucket teardown order is not represented or
claimed because this cache-front API exposes no iteration or destructor
effects.

Both binary parsers bounded-copy the normalized original path, replace from
the last dot with `.stategraph`, then convert slashes. Missing-dot and buffer
overflow behavior is unsafe natively; the host checks embedded NUL, input and
expanded lengths, and the 255-byte usable boundary. Cache helpers accept
NUL-free UTF-8 paths; native functions accept arbitrary C bytes.

The complete loader additionally tries compiled data first with flags zero and
alignment `0x10`, then XML on a null compiled result. State XML uses flags
zero; Behaviour XML uses flags one. Those loader calls and XML parsers remain
outside the cache-front closure in this checkpoint.

## Compiled recursive grammar

Both compiled formats use the same headerless preorder record:

```text
Element :=
    name: NUL-terminated bytes
    attribute_count: big-endian u32
    attributes: Attribute[attribute_count]
    child_count: big-endian u32
    children: Element[child_count]

Attribute :=
    key: NUL-terminated bytes
    text: NUL-terminated bytes
    float_bits: big-endian raw IEEE-754 binary32 bits
    boolean_raw: u8
```

There is no header, magic, version, total count, alignment, padding, or footer.
State `ProcessElement` and Behaviour `StartProcessingElement` occur after all
attributes and before child count/descendants in native code. The format layer
retains serialized order and all raw representations but deliberately does not
invent those semantic effects.

State attributes use an element-local fixed hash map with capacity 16. Keys
are hashed by the recovered Jenkins lookup2-style routine seeded with
`0xabcdef00`; only the hash persists and the first serialized duplicate or
collision wins. The readable element keeps original raw keys for diagnostics
while `attribute()` reproduces hash-only first-wins lookup. The distinct test
keys `a05763` and `a063d3` both hash to `0xfe11cd43`.

The State parser owns a fixed 24-entry object/null stack. The checked wire
reader permits 24 nested element records and rejects the 25th rather than
reproducing fixed-vector overflow. It preserves arbitrary non-NUL byte strings
without a UTF-8 assumption. Native `ParseBinary` accepts bytes after the first
root; `parse_native_prefix` reports the consumed prefix, while the separate
corpus policy requires exact consumption.

The Behaviour reader preserves raw float bits and raw boolean bytes and checks
the native 16-attribute capacity. Its UTF-8, recursion, bounded-read,
allocation, and no-trailing-data rules are documented safe-host validation
policies for the shipped descriptor corpus.

## Shipped-corpus proof

All ten compiled living-world Behaviour descriptors parse exactly:

- 5,147 bytes and 100 nodes;
- root `BehaviourSource` in every file;
- exactly one `BehaviourPlugin` per file;
- its textual `file` attribute maps in order to state preload paths 3 through
  12.

All 36 named compiled State payloads parse and consume exactly:

- 1,017,704 bytes, 17,257 nodes, and 31,302 attributes;
- root `state` in every file;
- maximum root-zero depth 12, attributes per element 8, and children per
  element 38;
- tags: 2,681 `state`, 2,457 `transition`, 3,561 `expression`, 4,485
  `condition`, 3,828 `behaviour`, 165 `param`, and 80 `hook`;
- raw boolean bytes: 30,163 zero and 1,139 one;
- no compiled `stategraph` wrapper, `include`, unknown tag, duplicate key, or
  within-element attribute-hash collision.

The application freezes every State file's exact path, unpacked byte size,
node count, attribute count, and maximum depth, rather than accepting aggregate
totals alone. A mounted release run through the production `MountSet` passes
all ten Behaviour and all 36 State oracles. These payloads resolve through the
reference archive set; this module does not infer runtime semantic completion
from successful byte parsing.

## Remaining boundary

The next semantic layer must invoke State `ProcessElement` and Behaviour
`StartProcessingElement` at their exact pre-child boundary, reconstruct the
State/Transition/Expression direct constructors, and recover registered
Condition/Behaviour/TransitionHook and transferable plugin factories. XML
fallback and root `Process` effects are separate. Until those exist, the
preload can prove cache and data delivery but cannot execute the player,
pedestrian, camera, or living-world graphs.

## Evidence hashes

- final StateGraph payload/cache packet
  `35BF0FF55C5A783F99FE33B2A67B48C5B44151DC4902C8F72BCBF2DC722F6AC3`;
- StateGraph validation report
  `A3331D2603E0D1332B1DE7DBA736C65B4C02F3156B9D9C856D1E14316E5FA957`;
- StateGraph 36-file oracle
  `48D12DD98802694718FA8A5084E484A5C025E5BF1E17ED9B6A6668071F39F472`;
- collision finder
  `4DEB65E54D94FA9E88A8B5C03F17059F811D65AB684664DB8C7A390E918D27A0`;
- graph-cache proof packet
  `749FD10302F46150F8D628572CAFABF3D5C1301CEE55BDAD8911BA7A8AFBB677`;
- graph-cache extractor
  `21549E51CBCE370BAC05EF970A11E6F156F2B58AF495DA7B5A18B7FA2BCA6FFB`;
- Behaviour decoder/`InitPropData` packet
  `0884392FD9FA945A98701E897EEB90FC2C8BFE7CEED517989C6257AD1ACF991E`;
- supplied `miscload.big`
  `0512157AAADD88A6E1992509944D0AAFF49EFAD2DB7BDD8C32AA4774A92F7F13`;
- debug XEX
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`;
- PDB
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`.
