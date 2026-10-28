# Retained Vault instances and scalar/layout fetching

Status: source packet recovered 2026-09-07; the selected immutable read graph
is implemented in attribute_runtime.rs and connected to wheel construction.
Independent corrected-code review and strict workspace/corpus gates pass. This is not a
complete mutable Attrib service or a native runtime mutation trace.

## Source and confidence

Reference is the symbolized `sk82_na_m.xex` database identified in the other
physics evidence. Confidence **exact** for checked PPC branches, offsets and
original words; **high** for their proposed readable typed read-graph boundary.
Decompiler argument splitting is unreliable for these 64-bit keys: in
FindCollection the class key enters r3 and collection key r4.

| Behavior | Source address |
| --- | --- |
| Attrib::FindCollection | 0x82976528 |
| Database::GetClass / Class::GetCollection | 0x82977868 / 0x8297a478 |
| Instance constructor / GetAttributePointer | 0x82975bb0 / 0x82976308 |
| Collection::GetNode / GetData | 0x8297b658 / 0x8297b7e8 |
| Node::GetPointer / GetArray | 0x82979ac8 / 0x82979b78 |
| Loaded Collection constructor / Node constructor | 0x8297bdc0 / 0x82977d00 |
| ClassPrivate constructor | 0x82978d50 |
| Generated physics constructor / DefaultDataArea | 0x827b2660 / 0x828b5340 |

FindCollection performs exact class then collection lookup, returning null on
either miss. There is no automatic retry of the collection named default.
Instance size0x10 holds collection+0, cached layout+4, message port+8, flags+c.
Construction retains the collection occurrence and caches its layout storage
identity, not vector contents. A null instance does not automatically rebind
when later collections appear. Each GetAttributePointer calls GetData anew.

GetNode searches explicit nodes on the requesting collection, then each
parent nearest first. Only after that complete chain misses does it search
the ORIGINAL requesting collection's class layout-node table, and only if
that original collection has layout storage. Parent explicit data therefore
wins over a child's layout-backed field. There is no independent parent-layout
search. The selected node includes its owner collection for addressing.
A found node blocks fallback even if its resolved pointer is null or an array
index is out of range. Non-array GetData ignores the supplied element index.

GetPointer flag precedence is inline0x40 (address of its four-byte word), then
layout0x10 (owning collection layout+offset), then static0x20 (owning class
static data+offset), then stored pointer. For arrays, the eight-byte header
contains u16 alloc/count/size/encoded_type_and_pad. Bounds are checked against
count; element address is base+8+((encoded_type_and_pad >>12)&0xffff8)+size*index.
Current physics/wheel inputs need no arrays or constructed object types;
unsupported representations must yield explicit host errors, not native-miss
zero defaults.

Loaded Collection copies its relocated layout pointer and retains Vault/class.
Nonzero full64-bit parent keys resolve in the SAME class; native immediately
AddRefs the result. Missing parents are invalid native load input, not a
graceful parentless fallback. Serialized nodes gain runtime occupied bit0x80;
inline0x40 becomes0xc0. Loader passes ptrIsRaw=false, retaining layout-relative
offsets. Serialized trailing entry_flags is not consumed by node construction.
ClassPrivate generates layout nodes only for definition.flags&2 && !(flags&8).
Flags begin0x10, add array0x2 for definition bit0 and constructed0x8 for bit0x20,
then occupied0x80. WorldGravity definitionflags6 yields layout node0x90 at0;
wheel definitionsflags4 do NOT create generic default values.

### Independent implementation-review corrections

The loaded ctor indexes the serialized node's type vector before Node
construction (8297bf04); the generic metadata parser does not validate that
index. The read subset now retains it and refuses an invalid index or a node
type differing from the definition used to size its view. Unsupported node
forms remain lazy errors so unrelated supported lookups can still succeed.

PtrN may target the returned data, not just the pointer leading to it. Any
fixup overlapping a plain returned byte range is refused: raw pointer
placeholders must not silently become numeric zeroes. This includes generated
physics's full32-byte range. A fixup targeting an explicit layout/static node's
union word also changes its offset BEFORE loaded construction (lwz8297bef8),
so that representation is refused rather than ignoring the relocation as a
losing pointer branch. Inline/layout/stored supported precedence is otherwise
unchanged. These refusals are safe-host unsupported errors, not native misses.

Tracing ClassPrivate's caller found an important static-data distinction.
ClassExportPolicy::Initialize82979098..82979160 initializes runtime static base
null. When raw static pointer is nonnull and static_size>0, it queries
ClassStaticDesc::GetStatic829790f0; if registered, it copies MIN(serialized size,
descriptor size) to the descriptor's mStruct at82979120 and passes that
REGISTERED destination at82979148. It does NOT pass the raw serialized static
pointer. GetStatic828b5350..53a8 has exactly one descriptor, at82167a38: key
ccb7c06868eb1c87, mStruct83059d24, size4. PDB descriptor size16: key+0, ptr+8,
size+c. None of the selected physics/wheel/world classes matches; their native
runtime static base is null. The reader deliberately refuses a winning static
node with UnsupportedClassStaticStorage until the registry/copy owner exists;
it never exposes raw static payload or substitutes a zero default. Initial
raw-static implementation/test expectations were removed before acceptance.
ClassExportPolicy SHA-256:
809b409315d4aff77fc885aa84a4d7d23aafcce0e7f178b927ced4ebbb34d77e.

The owned reader materializes a valid post-load graph, not native export/load
constructor chronology: all selected collections are staged, then parents
validated in the same class. It accepts forward references in this immutable
graph and refuses missing/cyclic parents. Native immediate AddRef requires
already registered parents. Instances retain the same immutable Vault occurrence
plus collection/layout identity; replacing an external Arc handle cannot rebind
them. Range checks guarantee dependency-buffer bounds, not full per-type
layout/static allocation extents. Type construction, arrays, class-static
storage, export-pointer targets, mutable edits/replacement, Instance::Change and
full load/unload registration sequencing remain explicitly unimplemented.

## Caller-specific gravity and wheel behavior

Generated physics constructor finds class04293d0f2adcfbb4 and its requested
collection, builds Instance, then substitutes shared zero DefaultDataArea(0x20)
ONLY if cached layout is null. First32bytes at0x83059d30 were verified zero.
Island constructs its own physics.default instance. Constructor827bab18,
stores827bad30/34, and UpdateWorldFromAttributes827b2cd8 load16bytes DIRECTLY
from that cached layout into Simulation+0x90. A generic WorldGravity node
lookup is not equivalent under hypothetical explicit overrides.

Wheel accessors call scalar GetAttributePointer(index0) and only a null result
gets DefaultDataArea(4). Seven keys/words and original raw/extracted byte
distinction remain in skateboard-wheels.md. GlobalAttributeCollection retains
the wheel instances (constructor828b1608). Native repeated scalar reads and
definition construction sampling are not automatically modeled by immutable
snapshotting; that boundary remains explicit in skateboard_wheels.rs.

Mounted dependency is asset skater.bin, ID16762145128419181554. Physics has
one default collection, parent0, 11explicit entries and layout dependency1
offset628672. Gravity words are 00000000/c11ccccd/00000000/00000000. Wheels have
one default, parent0, no layout, 13entries; physics_world has one scalar entry.
Existing formats/attrib.rs supplies metadata, PtrN, class definitions and
collection records; prior test-local inline selection was NOT a runtime service.

## Updates: source-ready notification, incomplete mutation ingress

Boot installs AttribSystemEditNotifier at82740b88; notifier82731f78 invokes
Monitor::NotifyAll8272fd00. Dispatch order: general subscribers with edited
attribute key, matching attribute-key, matching collection-key, matching
class-key subscribers. Keyed callbacks receive the matched SUBSCRIPTION key
(PPC helper8272dd00), not invariably the edited attribute key.

Physics watcher registration827bb098 subscribes to physics class and generic
default collection keyd7edbd362d7d2152, among others. Classification827baf00
therefore queues BOTH skateboard replacement and gravity update for an edit
to physics.default, assuming native notifier invocation. Deferred update
827bae80, called at end World::Update3 at827afa7c, runs ReplaceSkateboards,
then UpdateWorldFromAttributes, then ReplaceSkeletons, clearing each flag
after its operation returns. A gravity-only setter does not reproduce this.

Exact external edit ingress/storage mutation and complete ReplaceSkateboards
execution remain unrecovered. Direct gEditNotifier xrefs exposing only a setter
do not prove edits impossible. No guessed automatic reload/rebind is permitted.
Instance::Change82976260 has an additional unresolved integration quirk: changing
an unmodified instance to null does not explicitly clear old cached layout.
The new read graph must not claim that API or full live-edit support.

## Source hashes and validation targets

SHA-256 over independently read function bytes:

- GetNode: bb7d54b20ea6a60842a78026ec6344c4b1f2f7ab8b7719673cfde673ad9b65f8
- GetData: 206f1cd0bb6ef5b1062faea249c6bf4623455da7367e21579fbeb5360a9a5a9b
- Collection ctor: e64e1c23ca1c114ddeb7bb8b4fdc4d0eb99fd27e41c904550ec65ca941757c46
- Class ctor: a397a9a7a042930257e3f7d4d6c7b2e51df849e911248b3950b66f7d13e0eaef
- Watcher update: f44aa837b5fe538ebe294327e15118b4e8334bccdb2d30096154e595da9bc1aa

Required discriminators: exact collection miss; child/parent explicit precedence;
parent explicit versus child layout; absent parent-layout fallback; found-null
blocking fallback; ignored scalar index; pointer-flag precedence; distinct
generated physics layout default; retained bindings; unsupported != missing.
Actual raw db.big now feeds the shared reader, gravity layout and all seven wheel
inputs; frozen words are assertions only. The old test-local inline decoder is
retained as an independent assertion oracle, never owner-construction input.
GlobalAttributeCollection's retained binding slice uses native world-before-wheel
order (world Find/Instance828b2324/2334, wheels828b2380/2390). Immutable sampling
is still not native mutable CreateWheels lookup/dereference timing.

Root strengthened generated gravity testing with the actual WorldGravity key,
an explicit16-byte override distinct from its cached layout, and retained-owner
drop checks. Seventeen synthetic tests pass, including type errors, relocated
payload/union refusal, unsupported-static refusal and consumer error-versus-zero
behavior. Independent live-IDA implementation review approves this bounded
immutable numeric profile after the above corrections.

Final root gate: formatting;2012core+187formats+38app+3symbols=2240unit tests;
six doctests; all-targets Clippy with warnings denied. Three ignored core
original-data tests explicitly rerun and pass: generated gravity, shared-fetch
wheel construction, retained eight-frame DMO/terrain. The pre-existing ignored
app corpus is not rerun. Ordinary workspace tests ignore all four corpus cases.
No captured native mutable/runtime numeric trace, full service, app session or
skating test is claimed. Relative union guarding checks exact PtrN slot matches;
exhaustive malformed overlapping-relocation/type/allocation validation is not
claimed. External assets and parked PlayRemappedAnimation remain unchanged.
