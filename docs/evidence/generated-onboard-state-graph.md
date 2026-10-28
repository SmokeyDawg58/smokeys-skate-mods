# Generated onboard StateGraph descriptor boundary

Date: 2026-09-07. This checkpoint reconstructs the matched debug build's
read-only generated State descriptor and controller-selection seam. It does not
construct or tick the generated Action/Motion controllers.

## Evidence identity

- Matched debug XEX SHA-256:
  `3706CEDABD24FCD3DD16C99405FF5725DEAB5B09739AE926324DB88A0B7C0FE2`.
- PDB SHA-256:
  `0B060CEC3CAA2F174999EBFDEC1D7C90CBCB212815E7933E68D273D687D56EF0`.
- MAP SHA-256:
  `51E60F0993C40FF1E473F3D28E1E2F954DFEC605A785CA2EE02320C8B8997E88`.
- Generated-seam proof SHA-256:
  `465E0612BA72F5192ED47CB8BBD826F8691A5BDAB6F2CA74EA034DAC90A6043B`.
- Main raw packet SHA-256:
  `3A5899FEAA53E59A9A4F9C7E79A65845FBDC372F06CE99F8288D87EEBAA7BB81`.
- Exact StateDesc-table dump SHA-256:
  `2D0D320F85F4CAE4BC85C29B6874902C075D81C6F141352982C7A732DB23B879`.
- Generated/dynamic comparison report SHA-256:
  `EAFD50DF6F0BB6295E7206725BCC8EE57F3EF1EFB5CF4C805CDFDD3A774E0D60`.

The Action accessor is `HCDelegate_ActionGraph_OnBoard::State` at
`0x82928268`; it bounds indices below 224 and addresses records from
`0x821766B0`. Motion's accessor is at `0x8292A538`; it bounds indices below
1,354 and addresses records from `0x82195FA0`.

## Descriptor seam

`GeneratedStateDesc` decodes the exact big-endian `0x20`-byte record by named
offsets rather than host transmutation:

- guest name address at `+0x00`;
- interrupt type and active raw bytes at `+0x04/+0x05`;
- interrupt target, parent, and expression indices at `+0x06/+0x08/+0x0A`;
- child, Behaviour, and Transition `{u16 count,u16 index}` sections at
  `+0x0C/+0x10/+0x14`;
- opaque generated update guest address at `+0x18`;
- PDB-unassigned trailing bytes at `+0x1C`.

Raw values are preserved, including the difference between an empty section
whose index is `0xFFFF` and one whose index is zero. The update address remains
opaque metadata and cannot be invoked through this API.

Action and Motion generated indices are domain-tagged and bounds-checked before
any provider access. No conversion to the dynamic runtime's `StateId` exists.
The core owns generated path construction: it follows same-kind parent indices,
checks every parent before access, detects cycles before rereading a record,
rejects embedded NULs in resolved C-string names, and returns ordered raw-byte
name components. A provider cannot certify a caller-invented flat path.

The core contains no proprietary StateDesc bytes. A
`GeneratedOnboardDescriptorProvider` must supply legally available exact record
bytes and resolve their guest C-string name addresses.

## Controller selection

`AbstractHierarchicalController::Create` at `0x829513F0` supplies the route
contract. Input bytes are treated as a C string ending at the first NUL. Native
basename handling first searches for the final `/`; it searches for `\` only
when no `/` exists. Comparison is exact and case-sensitive.

When generated use is allowed, all 20 known delegates are tested in frozen
order and any nonzero raw enable byte counts as enabled. Action and Motion hits
return their typed onboard kinds; the other 18 return a distinct
`GeneratedOther` route. With no hit, require-generated returns a typed miss;
otherwise the route is the real dynamic fallback. A generated hit never
silently becomes Dynamic.

The descriptor-only generated tick entry always returns
`GeneratedOnboardUnsupported`. `call_behaviours=false` is not treated as a
shortcut because native construction and selection still require concrete
Behaviour, Condition, Hook, Param, direct-helper, and generated-update logic.

Twelve focused tests cover every scalar offset and endian conversion, trailing
bytes and sentinel preservation, exact Action/Motion metadata and bounds,
bounds-before-provider ordering, root and multi-level component paths,
out-of-range parents, self/indirect cycles, embedded NULs, path lookup,
case/mixed-separator C-string selection, all 20 delegates, non-boolean enable
bytes, the allow/require matrix, domain separation, and unsupported ticking.
Independent source audit passed.

| Source | SHA-256 |
|---|---|
| `generated_onboard_state_graph.rs` | `8B643EA72C8111BE9ED6E6AD1C166B5ED76F0F598C75B5CB07C8D6F2488D1F2E` |
| `lib.rs` | `D8ED96CC319042BA8BB1431BA1CBAB70CE419935F6AA133E91F38FD3A3537A19` |

## Nonclaims

- No checked-in provider currently maps a matched external XEX into guest
  memory; synthetic tests prove the core contract without embedding its bytes.
- No generated Condition, Behaviour, Hook, Param, BitCache, StateUpdate, or
  controller lifecycle body is implemented here.
- The generated update guest address is never called.
- Generated and decoded numeric State indices are not equal; both graphs first
  diverge at index 2.
- The matched default generated path is never substituted with the implemented
  dynamic fallback.
- Native assertion continuation, allocation failure, initialization races, and
  destructor timing remain outside this checked metadata boundary.
