# Board FillPhysOut Pumping and contact continuation

Registered, pending gates, September 12, 2026. Confidence: high for the raw control and
field mapping; no original-data or Cargo result is claimed for this batch.
Source is Skateboard.FillPhysOut82b2f3e8, entered after the prior f7f8 speed store.

## Implemented f7fc..f94c

The existing same-owner continuation must also retain the Part6 body observation
captured at f43c. It is the operand of f894, not a freshly resolved Part body.
No new velocity/position owner is introduced. Body+10 uses the existing
`try_contact_spy_center_of_mass` semantic XYZ getter with unknown intrusive W;
despite the historical method name, it only checks pool authority/occurrence and
integrity and does not require contact-spy registration or a frame.

| Store | Source and output | Read/guard order |
|---|---|---|
| f828 | Pumping+2c to Motion+bc pump_potential | capture Pumping pointer f7fc, latch f800, scalar f808, assertion |
| f85c | Processed+8cc bit4 to Motion+115 intentional pump | latch f82c, fresh flags f838, assertion |
| f88c | Board+530 bit40000000 to Skeleton+240 abort push | latch f860, flags f864, assertion |
| f8bc | captured Body+10 to Reckoning+90 deck position | latch f890, vector f894, assertion |
| f8f0 | fresh Processed+8c4 bit00100000 to Reckoning+110 flip | latch f8c4, flags f8cc, assertion |
| f94c | conditional Board+b0 JumpDV or native full zero to Air+70 | fresh event bit00400000 f8f8 chooses address; guard f91c; actual selected vector read f944 |

Pumping is the already constructed shared HiLOD+2c0/Board+108 allocation. Its
private `pump_amount` corresponds to source +2c and has actual constructor/reset
zero from82dd8a60. A narrow accessor exposes that field. Reading this field does
not imply Pumping.Update ran. AbortPush is the existing SkateboardEntryState
field; the actual Update3 prefix clears it. Missing prior producer knowledge
remains an error, not an assumed false value.

The true JumpDV branch has a typed `PendingJumpDeltaProducer` boundary. It occurs
after the Air guard and preserves every earlier store. The false branch writes
the source's four-lane zero and never invokes the missing producer. No new
untracked JumpDV field or arbitrary zero initialization is introduced.

Root independently reviewed the raw read/guard order and same-owner captured
body mapping before registration. Focused tests cover locked-output Pumping read-before-assert behavior,
selected missing JumpDV preserving the entire prefix and previous Air value,
and the unselected branch's exact full-zero copy without a JumpDV query.

## Read-only next contact mapping f950..fdd4

All four wheels execute ascending, individually guarded. At f994 the output
latch is sampled before inContact f998 and body velocity f9ac. The captured
contact byte controls both subsequent vector choices. Wheel linear velocity
goes to Motion+d0+16*i. Position Body+d0+16*i goes to Collision+cd0+16*i; normal
Body+60+16*i goes to Collision+d10+16*i. If contact is false, each vector is
native full zero, and unused source payloads need not be known. Audio Body+310
and pattern Body+32c go to Collision+d50/d60 respectively, with each scalar read
after its output guard. Existing collision_state arrays retain all these values,
from actual CollisionInfo.Reset and PostPhysics selection/motion operations.

The later front/back truck contact and audio stores use the same arrays;
overall physics surface at fc20 already has a selected publication helper.
Do not reconstruct a second surface voting algorithm.

The actual deck line record is Body+1f10. Guarded valid byte+34 goes to
Ground+123; guarded start+0 goes to Ground+90. A fresh valid-byte read at fc94
controls guarded position+10 to Ground+a0, normal+20 to Ground+b0, and surface+30
to Ground+f0. Existing wheel_lines.start/end own and publish this record.

Afterwards, raw fd6c18642065 compares the splatted deck-position X with itself;
NaN selects an additional guarded full-zero position store. No W validity check
or arbitrary finite-value clamp belongs here. Finally the latch sample fdac
precedes collision.acceleration[6] at Body+210, loaded fdb4 and stored to
Motion+70 at fdd4.

The existing surface-motion implementation covers fdd8..3004c and should be
reused with the actual retained Reckoning/Processed owners. Its selected wrapper
has extra contact-frame policy that does not appear in native direct reads;
preserve that legacy API while designing the actual contiguous adapter. Its
shared length refinement also requires the same local VNMSUB signed-zero audit
already completed for the earlier board-speed graph. No duplicate math is
implemented in this draft.
