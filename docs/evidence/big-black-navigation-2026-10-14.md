# Big Black travel: missing mesh target iterator

Date: 2026-10-14. PC only, solo. **Source fix; not compiled or run.**

A playtest hired Big Black, moved to Matrix Plaza, and hit a queued-frame
failure after 661 completed frames:

```text
Tick(LivingWorld(Pedestrians("mesh target iterator absent")))
```

## Live reference

The IDA reference is the live debug database
`sk82_na_m.xex.i64`, with
auto-analysis and Hex-Rays ready. This investigation used that live connection.
Input SHA-256:
`3706cedabd24fcd3dd16c99405ff5725deab5b09739ae926324db88a0b7c0fe2`.

Eight functions' decompiled code and raw instructions are retained in
`.scratch/big-black-navigation-20261014/native.json`.

## Confirmed mismatch

`MeshNavigation::ResolvePath` (`0x82e76120`) checks the pending A* flag, the
allow-A* cvar, and the entity navigation-rig enable bit. It then calls
`GetCurrentTargetIterator` at `0x82e761d4`; `cmplwi r3, 0` at `0x82e761d8` and
`beq` at `0x82e761dc` exit to the epilogue at `0x82e768b8` when it is null.
This branch does not clear the pending A* flag, reset the path, change the
destination, or notify a pathing failure.

The port's `PedestrianConstruction::resolve_current_navigation` implemented
the earlier gates but omitted that null check. It then took the NavRig and
called `with_current_mesh_iterator`, which returned the exact reported error.
Added the missing return before borrowing the rig or dispatching the solver.

An empty iterator is valid here:

- `TrackProtectee::Begin` (`0x8270d1d8`) pushes head tracking, selects mesh
  navigation, and enables the rig. It does not assign a target iterator.
- `TrackProtectee::Update` (`0x8270d2a8`) resolves the protectee's position
  and calls `TargetExplicit` with mode 1. The existing Big Black callbacks
  retain this behavior.
- `SwitchNavigationMode` (`0x82e6d7d8`) selects the retained navigation owner;
  it does not initialize or transfer an iterator.
- `Navigation::SetCurrentTargetIterator` (`0x82e6bb68`) assigns the pointer
  only. No replacement iterator should be synthesized for an empty selection.

## Adjacent paths reviewed

`Task_AStarNavigation` (`0x82e733f8`) calls the active navigation's
`ResolvePath` when the navigation owner exists. The host follows that routing.
`MeshNavigation::Seek` (`0x82e75b68`) and `Update` (`0x82e7dc70`) also accept
a null iterator. Their existing host adapters already pass `None`; position,
heading, speed, and seek-direction work retain their original ordering.

All three callers of `with_current_mesh_iterator` were inspected: Seek and
Update already handle `None`, and ResolvePath now has the reference guard.
The solver has one production call site, through this guarded dispatch.
The repair therefore covers all pedestrians using it, including Big Black.

## Validation and limits

- One Rust file syntax-parsed successfully with Rustfmt `--emit stdout`,
  `skip_children=true`. The command did not rewrite the source.
- Source change is four added lines: the reference comment and null guard.
  No physics, navigation tuning, solver algorithm, iterator selection, or
  service lifetime changes.
- Backups and the scoped source diff/hashes were kept outside the repository.
- No build, tests, or game launch. The specific Matrix Plaza trip has
  not been reproduced; runtime confirmation requires rebuilding the PC game.

Confidence: **high** for the demonstrated source mismatch and repair.

