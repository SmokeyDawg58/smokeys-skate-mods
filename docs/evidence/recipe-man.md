# LivingWorld RecipeMan initialization and queue order

## Scope and confidence

`RecipeMan::Init`, its Vault-to-mapping traversal, and its loading-queue
derivation are reconstructed at **exact** confidence for the supplied default
Vault and stable compiled-default cVar. The application now builds the 24
groups and 54 entries from `skater.vlt`/`skater.bin`; the previous decoded
fixture remains an independent golden oracle.

The executable authorities are `RecipeMan::Init` at
`0x827289f8..0x82728a48`, `InitRecipeMappings` at
`0x82728488..0x827289f4`, and `InitLoadingQueues` at
`0x8271ffb0..0x827202d4`. The map insertion helper is
`DoInsertValue` at `0x82727010..0x8272713c`; the prime bucket selector is
`eastl::prime_rehash_policy::GetPrevBucketCountOnly` at
`0x82bd2278..0x82bd22c0`.

## Exact mapping iteration

The native category mapping is an EASTL
`fixed_hash_map<u64, fixed_vector<Entry,4>,64,65>`. Hint 65 resolves to 61
buckets. Hashing uses the low 32 bits of the `u64` key followed by unsigned
modulo 61, while equality compares the full 64 bits. A new group is inserted
at the head of its bucket chain. Native traversal visits buckets from 0 through
60 and follows each head chain, so colliding groups appear in reverse unique-
group insertion order. Entries within a group remain in append order.

The readable Rust representation stores unique groups in insertion order and
derives that exact bucket/head traversal. It preserves capacities of 64 groups,
four entries per group, and 64 entries in each loading queue. Checked overflow
and allocation errors are safe-host behavior; native fixed-container failure
handling is not claimed.

## Exact queue derivation

The first traversal appends every `required` model in native map and per-group
order. The initial budget is `max(maximum_initial - required_count, 0)`. On the
second traversal, while that budget is not full, the algorithm examines the
first entry in a group and advances past it regardless of whether it is
required; it enters the initial queue only when non-required. Every remaining
non-required entry enters the secondary queue. Secondary limiting repeatedly
removes from the back, observably equivalent to truncating to its first
`maximum_secondary` IDs.

The shipped default corpus has 24 groups and 54 entries. With initial maximum
15 and secondary maximum 10 it produces 15 required, zero initial, and 39
secondary entries before back removal. The exact survivors are:

```text
BBA7197FAF017C2F
47AD17FD06F53ABA
A2B241F6AC35B367
C81C3AAC7F34F22D
3DCD5154E5526FA7
4CE45CA0FC8F0308
9BC1F7CB5A63B806
036EAABA3222EC24
C801DD94C3598622
34A2A830F2ABAEE2
```

The golden test freezes all 15 required IDs, all 39 pre-truncation secondary
IDs, and the exact surviving ten. A separate collision vector proves low-word
bucket hashing, reverse head order, and full-width equality.

`RecipeMan::Init` itself returns immediately for every nonzero initialized
byte without normalizing either field. Raw zero calls mapping initialization,
then queue initialization, then stores initialized byte one and frame delay
zero.

## Exact Vault-to-mapping traversal

`InitRecipeMappings` reads the already loaded Attrib database; it performs no
file open and does not parse character `.recipe` payloads. The model class is
`livingworld_models` (`4E740A6670F241A7`) and the inherited enable byte is
`Load_Model` (`A75AF7F3CFABE6E1`). Any nonzero byte enables a model; a missing
value uses the native zero default.

With `kUseForcedRecipes` false, seeds are constructed in this order:

```text
473C370392671785 required
C79A1101ADAB19DB optional
8BE1D151F9DA2D29 optional
21D3CDD93EAE05E5 optional
```

The cVar is sampled again for every seed rather than captured once. A true
per-seed sample uses only that seed ID as its candidate. A false sample obtains
all class collections and parents, sorts both `u64` arrays ascending, takes
the set difference to find leaves, then retains leaves whose parent chain
contains the seed. The descendant walk starts at the candidate's parent, so a
leaf seed is not its own descendant.

For each enabled candidate, model ID begins as the candidate and group ID also
begins as the candidate. If candidate differs from the seed and its immediate
parent differs from the seed, group ID becomes that immediate parent. Required
is copied from the seed; there is no per-model required field and no model
deduplication. Existing groups append in candidate order; new groups enter the
mapping at that point.

The forced seed branch reads the configuration collection
`E5874FC04C117535` of class `livingworld` (`F97B9086CAA50C53`). Its
`ForcedPedestrianModel` (`8BE70B1A756D9035`) and `ForcedVehicleModel`
(`7B5F7A6C78F3251D`) fields are 24-byte `RefSpec` values; native consumes only
the collection key at offset eight. Missing values become collection zero.
Forced seeds are optional and preserve pedestrian-then-vehicle order.

The core provider interface preserves the initial cVar read and every per-seed
read, including hot-toggle vectors. The mounted backend deliberately returns
the supplied build's stable compiled default false on every call because no
live cVar owner exists yet. It resolves exact inheritance through explicit
current/parent nodes before the original collection's layout/static default.
Malformed pointers, missing native preconditions, and parent cycles are
checked host errors rather than guessed continuation behavior.

## Evidence hashes

- `recipe_eastl_evidence.txt`:
  `69839CF92B5FCB81E10A98EB7D787F18BD38ABACB1EBA7A75417E89DF63D20B1`;
- `dump_recipe_eastl.py`:
  `53055E5D9E71F09E67382A3443943C1AAA6C48ED4B80809B5AFA5C0224FA567D`;
- `recipe_exact_queue_replay.txt`:
  `A7470C16B53B6C12B84031A92E11F726B767A441DD851BBA5149F4BF30B27453`;
- `recipe_pdb_evidence.txt`:
  `F152B99C546419CAC4DD7578EFDD9638C819CF9DE5E5988CF6D1406B8AA587C5`;
- `livingworld_models_vault.txt`:
  `9C26F69F21AC05E6B2D40B0D5A90B618E2A72A395AA40A9A638335B31488C725`;
- `livingworld_vault.txt`:
  `871FC6DA842725B2E4E0B3BD3E12E064E9486D0636C8F1B0B49B302A8F1A9E4`.

## Remaining boundary

The mounted backend is still a SecondaryLoad corpus collaborator rather than a
process-lifetime attribute database owner. A live mutable
`kUseForcedRecipes` source and arbitrary non-default Vault compatibility remain
outside this checkpoint. The supplied stable-false path is concrete and
differentially checked against all 54 `(group, model, required)` entries in
native construction order, then against the complete queue oracle. Present
explicit nodes with malformed/non-direct pointers are checked failures; they
cannot incorrectly fall through to an inherited value.
