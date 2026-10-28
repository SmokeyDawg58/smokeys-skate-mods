# Mood startup constructed-field fix

Runtime failed during BootPrefix in MoodMan initialization with
`unsupported constructed value 4694b0039eaacdb8`. This is RESULT_LIST, not a
missing behavior or an unknown mood event. Its existing consumer already reads
the original array through the retained Vault; the generic numeric projection
rejected the definition's handler flag before it reached those fields.

## IDA and asset evidence

MoodMan.Init82ce9638 reads array length at82ce98b8, then each result collection
key at entry+8 via GetAttributePointer82ce98e0. Type-handler ID table82ff0f48 and
pointer table82ff10a8 index7 bind37eeae390851cff9 to singleton82ff1298 and
vtable82167b24. Index41 binds a0a8bf1ba3b36a68 to singleton82ff1290 and
vtable82167b10. Both Construct/Destruct slots point to825187c0, raw `blr`.

PDB tLWRefspecPriorityPair is32 bytes: RefSpec0..24, unsigned priority24..28,
padding28..32. Assign828b55f8 copies that RefSpec and priority. PDB
tLWMoodEventPrerequisites is40 bytes: RefSpec0..24, prerequisite24..28,
operator28..32, float32..36, padding36..40. Assign828b5738 copies those fields.
RefSpec cache pointer16..20 is runtime state. Node flag8 is kHasHandler, not
evidence that a constructor transforms every field.

Original Jan17 skater.vlt/bin: both definitions have flags0x25 and strides32/40.
Across17 mood collections,204 result entries and122 prerequisite entries have
zero bytes16..24 and no serialized pointer relocations within their records.
The next prerequisite lookup used the same rejecting path, so both audited
types are handled together.

## Change and validation

The existing typed array reader admits only PlainField8..16 and24..28 for the
priority-pair type, and8..16 and24..36 for the prerequisite type. Exact type,
record size, node storage, array stride, index, inheritance and relocation
checks are retained. Cache fields, full-record projections and other constructed
types still fail explicitly. There is no second Vault or alternate mood logic.

Added synthetic regression coverage for inherited padded arrays, second-element
stride, original key/suffix bytes, out-of-range indices, and rejection of cache
and full-record reads. Syntax parsing passed. Regression test execution and
runtime verification have not been performed. Release compilation completed:
`cargo build --release -p skate2-pc -j 1` returned exit0 (7m18s), producing the
updated executable. Existing warnings remain.
