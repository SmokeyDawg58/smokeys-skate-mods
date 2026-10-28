# Skater breadcrumb gate attributes and live cvars

The shipped `data\db\skater.vlt` and `skater.bin` in the original BIG archive have the mounted corpus signatures `(851568, f7db0ed527db57b4)` and `(812520, 6b4fc086777fe4df)`. The extracted sibling files were inspected with the existing read-only `target\debug\examples\inspect_vault.exe` and the repo's `hash_attribute.exe`. No numeric gate value below is a host fallback.

## Correcting the class key

`BreadCrumbManager::Implementation` constructor `0x82ce3480..348c` contains big-endian instruction bytes `3e a0 12 b6 62 c3 08 53 62 b6 4c 0e 7a c3 00 0e`: `lis r21,0x12b6; ori r3,r22,0x0853; ori r22,r21,0x4c0e; insrdi r3,r22,32,0`. `insrdi` inserts `r22` into the high half of `r3`, giving class `0x12b64c0e804b0853`. Original Vault metadata independently exports class `skater_breadcrumbs` as `0x12b64c0e804b0853`, and the repo's hash executable returns the same number. The previous Rust `ActorBreadCrumbManager::CLASS = 0x804b085312b64c0e` and its corresponding earlier evidence had the 32-bit halves reversed; that value does not select the shipped class. The default collection key `0xd7edbd362d7d2152` is already correct. The root integrator independently reproduced the mounted class export, and a second PPC reviewer confirmed the `insrdi` half order before the Rust correction.

The same `insrdi` order matters for two inline collision keys. At `0x82ce1a94..1ab4`, instruction bytes `3d 60 cf 6a 3d 40 ad d0 61 64 1c 15 61 49 32 ca 3b be 01 30 79 24 00 0e 38 a0 00 00` produce `0xadd032cacf6a1c15` (normal threshold). At `0x82ce1b0c..1b28`, bytes `3d 60 38 06 3d 40 8a be 61 64 d2 73 61 49 09 8d 38 a0 00 00 79 24 00 0e` produce `0x8abe098d3806d273` (vertical-distance threshold). The earlier bridge note wrote both keys with halves reversed. The original `default` collection has explicit inline scalar nodes with raw words `0x3f400000` (0.75) and `0x3f800000` (1.0), respectively.

Other reached generated accessors form these keys in their PPC code and the same original collection has these inline values:

| Native accessor | Address | Full key | Raw word | Value |
| --- | --- | --- | --- | ---: |
| CapsuleHeightForCollisionTest | `0x82ce1120` | `c0526c883af0ecca` | `3ecccccd` | 0.4 |
| CapsuleRadiusForCollisionTest | `0x82ce1168` | `ceb092e418a5b001` | `3f333333` | 0.7 |
| MaxNavMeshScoreForBreadCrumb | `0x82ce11b0` | `2d567511a6f190eb` | `447a0000` | 1000 |
| MinDistToPedestriansForRespawn | `0x82ce11f8` | `e64c980ee2114070` | `3f800000` | 1 |
| MinDistToVehicleForRespawn | `0x82ce1240` | `59e1f2b3cdc4e658` | `40000000` | 2 |
| MinTicksInStateBeforeBreadCrumb | `0x82ce1288` | `10b7c3a9cc8d3721` | `0000000f` | 15 |

The collision predicate `0x82ce19b0..1cb4` and navmesh predicate `0x82ce14d0..159c` read these through the retained `mAttribs` instance and `DefaultDataArea(4)` only on a null pointer. The implementation should read the live Vault value on each reached call and let missing values use the same zero-word fallback, rather than embed the shipped numbers. The `_TestForCollisionConstraints` PPC rejects if the first scalar is strictly greater than the hit normal's Y lane (`0x82ce1af8..1b08`), then rejects if `(start_y - hit_y)` is strictly greater than the second scalar (`0x82ce1b5c..1b70`). It then applies `_IsSurfaceOK` and the capsule line. The vector-register source aliases in IDA should receive separate review before mounting the live collision gate. The navmesh predicate reads `MaxNavMeshScoreForBreadCrumb` only after a valid cell and accepts strictly `cell_score < max_score` (`fcmpu` and `bge` at `0x82ce1574..1580`).

The cvar initializers `0x82f895a8` and `0x82f895f8` construct two separate `cVarBoolRef` entries in section `BreadCrumbs`, both true by default and with null callbacks. Their registration table entries are `0x82fd51f0` and `0x82fd51f4`; their ref globals are `0x830a6200` and `0x830a61c4`. The exact labels are `kbEnableNavMeshConstraints` (title `Enable NavMesh Constraints`) and `kEnableWipeoutSpotBreadCrumb` (title `Enable Wipeout Spot Bread Crumb`). `GetWipeoutSpotBreadCrumb` samples the second live target before forming a candidate; `_TestForNavMeshConstraints` samples the first before network and cell checks. The mounted startup INI must be checked through the existing cvar deserializer before assuming the default values in a live selected session.

Rust now samples these seven float fields through the retained `ActorBreadCrumbManager::attributes` instance on each reached read. A null pointer uses the same four-byte zero fallback as the native generated accessors. The ignored original-data test `mounted_breadcrumb_class_binds_original_gate_nodes` asserts the corrected class/default binding and all seven non-null shipped raw scalar words; this test is pending the root integrator's combined build. The two cvars are registered in source initializer order and sampled from live linked targets. The live respawn bridge remains separate until collision, navmesh, LivingWorld and per-frame last-known breadcrumb publication are all mounted.
