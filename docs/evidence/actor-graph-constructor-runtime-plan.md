# Actor graph constructor continuation

Read-only original IDA/PDB audit, 2026-09-08. This is the concrete continuation
after the same Actor's listener. Graph selection/load, generated execution,
animation, physical-pair construction and final Actor publication are separate.

## Actual caller and interface publication

Actor825fe850 stores listener+1f0 at825fec58. It then allocates ActionGraph
0xa68 bytes in the Sim category at ec6c. Null allocation skips its constructor
and stores null; non-null calls82cea340 at ec84, with SAME Entity and debug
level/flush0. Actor+1f4 is assigned ec90, after the complete selected constructor.
Next allocation178c at eca4 calls SkaterMotionGraph825fdda8 at ecb4. Actor+1f8
receives its ISkaterMotionGraph subobject, base+0xa78, at ecc8; null stays null.
This pointer is not the base IMotionGraph pointer. No graph load happens here.

IActionGraph ctor82cea168 first stores SAME UCOM owner a190, asserts non-null
a19c..b0, Find by `_IName` identity a1c0, duplicate assertion a1d8/dc, Add a1f0.
Its identity is825ebe88. Motion base registers IMotionGraph via82cea08c before
its fields; identity825ec090. Skater derived registers ISkaterMotionGraph
825efbd0 at825fdde0, identity825ec0a0. Its owner writefbf8, null assertionfc04,
Findfc28, duplicate assertionfc40/44, Addfc58 precede derived construction.
All interfaces belong to Actor+4's existing registry. Do not issue another
Entity authority for either graph, or conflate their distinct controller tokens.

Interface Add occurs before graph field/controller initialization. A checked
host failure must retain that prefix; registration cannot be postponed until
the entire graph is ready. Final Actor IMotionGraph cache resolution remains
much later at825ff614/storef618, after animation, physics and job construction.

## Shared empty hierarchical wrapper, not a loaded session

ActionGraph82cea340..a420:

1. Register IActionGraph a36c; controller+8=null a374.
2. Construct actual empty intent map+0xc a394.
3. Form six-word Context: controller=null a3a8, Entity=actual pEntity a3a0,
   remaining four pointers zero via memset a3b0.
4. StateGraphAlloc(0x20,"unknown",0x10) a3c0. Non-null copies all six slots
   a3e8/ec loop, then replaces Context.controller with this new wrapper a3f4,
   sets root-name hash+18=0 a3fc, abstract-controller+1c=null a400, publishes
   ActionGraph.controller a404. Allocation-null is retained as null a410;
   native constructor still returns its ActionGraph, not a fabricated loaded one.

MotionGraph82cea060..a164 follows the same graph-independent pattern:
IMotionGraph registration a08c, controller null a094, intent map a0b4. Then
next-transition type0 a0c4, placement0 a0d0, high two flag bits clear a0e0,
blend length source constant3e4ccccd a0e4. Context actual Entity a0d8,
controller null a0f0, four nulls a0f4. Allocate wrapper a104, copy a12c/130,
self-controller a138, hash0 a140, inner controller null a144, publish a148;
null branch a154.

PDB HierarchicalController size20: Context+0 (six pointer slots, size18),
root-name hash+18, AbstractHierarchicalController*+1c. StateGraphAlloc82cef4e8
checks actual XAllocator singleton f504..524, obtains StateGraph category
allocator f530 and calls its allocation method. Existing graph-asset loading
does not supply this native allocator service. A host selected allocation can
own the real typed wrapper with checked allocation failure, but cannot claim
native allocator/category parity. A callback that simply reports success
without retaining this wrapper is not a replacement.

## Real SkaterMotionGraph fields and constructor Reset

PDB sizes: base MotionGraph a78; ISkaterMotionGraph subobject+a78; filtered
intents+a80; MgAttributes+14dc (size1f0); last good landing velocity+16cc;
grab+16d0; push+16d8; slide+16fc; kickturn timer+1710, teleport+1714,
manual-out+1718; packed word+171c; trick+1720; gesture+175c; wipeout+1764;
Random+176c (size20). Total178c.

After base and derived registration, filtered map ctor runs de10. MgAttributes
is actual fixed vector capacity20 of 0x18-byte entries (FastString30 at0,
float+14). End=inline begin de24, begin de2c, capacity=begin+1e0 de34. This is
real empty output storage, not a generated metadata descriptor.

Constructor zero writes: landing velocity de3c; both busy hands de44/4c;
eight push floats de50/54/5c/60/64/68/6c/70; push packed high bit clears de7c.
Left/right slide timer and high-three-bit clears de80..9c repeat dea0..bc;
combined slide high two bits clear dec8. Kickturn timer decc, teleport ded8,
manual-out dee0 are zero. Packed171c writes `(old & 00ffffff)|03000000` dedc.
AnimOutTrickData ctor825f7598 at dee4 clears its FastString storage then calls
Reset825f68f0: invalid trick name691c and gesture6954, tweakX/Y0 6980/84,
trickbits0 6988. The invalid-string initializer82f7be08 uses source8225fe5b,
whose first byte is00: it is an empty FastString, not uninitialized BSS evidence.
Gesture isGesturing0 deec, gesture enum Count def4; wipeoutXY0 def8/fc.
Random::Reset82a6dba8 at df00 sets index0 dbb0 then seed0 dbb8 and copies six
original words from8220554c at dbc4:
`f22d0e56 883126e9 c624dd2f 0702c49c 9e353f7d 6fdf3b64`.

The last constructor call df08 is **SkaterMotionGraphReset with this=base+a78**.
IDA applies the wrong base type in its pseudocode; apparent intent-node offsets
are actual derived fields. Correct graph:

- Clear filtered intents (this+8)612c; set count0 6138.
- Virtual slot+18 at6148 resolves through vtable82069688 to
  ClearMgAttributes825fe1d0: load this+a64 (base+14dc begin), store this+a68
  (base+14e0 end). It does not set transition type despite bad pseudocode.
- Reinitialize landing/grab/push/slide/kickturn fields6168..61d0, with exact
  same scalar and packed-mask writes. Set packed171c to
  `(old & 01ffffff)|02000000`61dc. Teleport/manual-out are not reset here.
- Clear base motion intents61e8, count0 61ec.
- Base virtual slot+20 at6200 resolves via vtable8206975c to
  ClearNextAnimTransOveride825fdf50. Raw clears type/placement to0, blend
  3e4ccccd. Store order placementdf8c, blenddf90, typedf94, flagsdf98. Flags
  low30 come from temporary stack storage, NOT retained old graph flags;
  high2 are known zero. Preserve unknown low bits rather than manufacturing0.

The Reset contains no graph load, graph tick, Vault/cvar lookup or Actor physics
query. The entire selected constructor graph above can own real state without
success-only external reset callbacks. Generated behaviour remains later.

GetMgAttributes825fdfa0 returns ISkaterMotionGraph-this+a64, i.e. SAME retained
vector at base14dc. Future AddMgAttribute825fe7a0 must append to that vector;
no copied output owner. Base and filtered intent maps are distinct, as is the
ActionGraph intent map. They must not share an arbitrary listener test sink.

## Existing Rust reuse and precise missing pieces

- `actor_construction.rs`: owns actual selected Entity prefix and the sole
  `ActorGraphAuthority<ActorIdentity>`. Extend its same registry/stages after
  mounted listener; actor fields currently record graph nulls only.
- `motion_graph_entity_authority.rs`: reuse existing issuance, shallow Entity
  handle, controller occurrence and `BoundMotionGraphContext`. The latter
  already encodes the four source-null slots with an uninhabited type.
  `ActorGraphAuthority::motion_graph_facade` shares its issuance; no new issuer
  is required. Action needs equivalent access to SAME authority, not a new key.
- `state_graph_context.rs`: exact existing six-slot storage/shallow copy;
  construct two separate contexts with distinct wrapper/controller identities.
- `motion_graph_authority_session.rs::try_new`: currently requires a loaded
  StateGraphRuntime, complete four-leaf capability preflight, runtime facade,
  allocator, then creates DynamicStateGraphController. This is **later** than
  constructor-null AbstractHierarchicalController and cannot be eagerly called.
  Its current private token creation must be reconciled with the retained
  native wrapper token at a future load boundary, rather than silently issuing
  a second controller owner.
- `motion_graph_dynamic_session.rs`, `dynamic_state_graph_controller.rs`:
  already implement dynamic execution; do not create a competing graph engine.
- `state_graph_controller_route.rs`, `generated_onboard_state_graph.rs`,
  `living_world_state_graph_preload.rs`: existing lazy route/generated metadata
  selection. Use later InitPlayer load path; selected generated execution must
  not quietly fall back to the implemented dynamic engine.
- `play_animation.rs` existing FastString30/36 types and
  `play_animation_lifecycle.rs::RawAnimationTransitionParams` are reusable.
  Raw transition flags currently demand a whole u32; constructor unknown low
  bits need an explicit known-mask projection or typed semantic flags until a
  consumer actually demands the whole raw word.

Missing retained components are therefore small but real: unloaded hierarchical
wrapper storage; actual three capacity64 intent maps; capacity20 MgAttribute
output; derived scalar/packed/trick/gesture/random state; same-registry graph
interface references and Actor graph member publication. Native fixed-map
ctor825fd668 uses64 nodes of0x20, bucket buffer66, requested65 buckets through
prime-policy GetPrevBucketCountOnly, max load10000, overflow disabled. Empty
storage can be constructed now; later intent insertion/traversal must reuse
the exact recovered map/string semantics rather than assume BTreeMap iteration.
There is no existing full retained intent-map owner found in this repository.

Recommended implementation is a retained `ActorGraphComponents` owned by the
same Actor construction state, with staged Action then SkaterMotion allocation,
interface registration, concrete empty state initialization and publication.
Each allocation result records null versus real owned component; null is not
“loaded”. End state names should say `GraphFacadesConstructed` and preserve
unloaded/null inner controllers. Expose SAME intent/output owners for listener
and generated consumers later. No Runtime callback supplied merely to return
success is needed for the recovered constructor Reset. Remaining native
allocator/world services and later Actor stages remain explicit boundaries.

### Listener ownership prerequisite for the graph continuation

The current original corpus separately retains a listener borrowing the Actor
authority. Actor remains `ListenerRequired`; successful external registrations
do not themselves prove native Actor+1f0 ownership/publication. Graph continuation
must not infer that milestone merely because `authority_for_listener` was read.

The smallest non-self-referential extension is for the same listener implementation
to retain a shallow owned `MotionGraphEntityAuthority<ActorIdentity>` handle
instead of a borrow of the surrounding `ActorGraphAuthority`. Add a narrow
authority accessor that shallow-copies the existing private handle using the
existing Context-handle contract; it must never invoke `issue` or `new`.
Actor then owns the actual listener `Arc<Mutex<...>>`, the event route retains
its existing weak handler references, and both observe the same issuance.
This introduces no Actor cycle: the authority inner stores the gameplay/world
identity, not the outer Actor containing the listener. A stable externally
allocated constructor arena is an alternative, but the current borrowed corpus
must not be relabeled as that native storage owner.

Use one attempted listener stage before allocation/registration. Retain its
actual partial listener and registration capability while registrations proceed,
including failure teardown already implemented by the route. Only completion
of all three registrations and the ec58 member publication advances to
`GraphRequired`. Allocation-null is a distinct recorded native member outcome,
not successful listener construction. Destruction must preserve native listener
before Action before Motion order; selected route's explicit failed-delivery
teardown remains separate from successful constructor advancement.

## Original byte anchors

SHA256 from actual function bytes, end exclusive:

| Function range | SHA256 |
|---|---|
|82cea340..82cea420|722fd9a995c4f2cac80476f5610e159489be68f0e27d9c04a5e89c39e7ec3742|
|82cea060..82cea164|73942e9b9525fc9044c5c8f53d69dc71a5aad2191b9048af6b199a869a5e7a21|
|825fdda8..825fdf1c|6d832b4ef6328777b954ffa9559e3c4d053a8e10131bab2e2ffba9b4fa7b6132|
|825f6110..825f620c|a4f4c9456443a9f70a52c1a3d786ce6c3ab883123fccd5470ed14279ed7a8318|
|825fd668..825fd730|8e28648c2cb2cbad764e175ac94d7c00dbcf67d4acaa4e349159ea90408bafe0|

No Rust edits or Cargo checks were performed for this audit.
