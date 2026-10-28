# Actual Update1 environmental-grab publication and return

Source call82e03ad8 -> PlayerGrabSpline.UpdateProcessedPhysIn82dcbb18, then actual PhysOut locked=true store3ae8, live kSpewJumpStuff target3aec/value3af0 and optional debug through3bcc, return3bd0..3be0. This environmental spline/interactable publication is separate from airborne board-grab gesture/control implementation.

Existing constructor82dd1c10 clears primary future.ready100 at1c34, alternate210 at1c3c, interactable2c6c at1c70. Actual same-world PlayerGrabSpline::try_finish verifies its retained query manager and deferred-output identity; adapter requires that completion and never constructs a replacement. Current source targets are unrepresented (UnresolvedGrabTarget); active target copies report typedpending before their native copy/readiness clear.

Primary: bb38 readsready, bb44 validatesready, bb64 reads actualpointerb4 only onready. Valid target copies all GrabSpline into processed6c0 atbbac. bbb8 inserts valid bit8d0/01000000; bbbc clearsready. Alternate repeats atbbc0..bc48, pointer1c4/copyprocessed7c0/flag00800000. False orready-null preserves priorpayload while clearingvalidflag/readiness.

Interactable:bc4c readsready. Unready skips all result/flag/ID reads and preserves processedflag00400000. Ready reads countbc7c, zeroscursorbc80; positive count writescursor1bc90, setsprocessedflagbc9c then loads firstobjectIDbca0 and publishes8c0bca4, clearingreadybca8. Constructor's scalarfirstID has no completed query provenance yet: nonempty branch stops atbca0 after cursor/flag prefix, without publishing guessedID or clearingready. Readyempty clears onlyprocessedflagbcbc, then readybcc0. No count or target is fabricated.

Final lock uses existing PhysicalPlayerPhysOut::lock_after_update1 immediately after successful grabpublication. Late debugmissingtarget/overlay errors leave it locked. Enabled debug loads retained intended-velocityXYZ beforeoverlay; actual finalcallee825187c0 is an empty function despite IDA's aliased AIProcess::OnEnd name. No logging/drawing behavior is invented, and no W check is introduced for XYZ-only debug reads.

Cvar83 planned: reference830e0d38, initializer82f9e9b8, table82fd62b8; sectionPhysics_Jumping,labelkSpewJumpStuff,title'spew jump stuff',false,nullcallback. Static order is after19(table62b0),before22(table62bc).

Focused tests use existing constructor retained futures to verify idleflagpreservation; ready-null consumption; nonemptycursor/flag prefix before missingpayload; and sameowner readyempty finalclear. Root owns same-pair corpus integration and all Cargo gates. Grinds remain explicitly parked in host scheduling; completion here is only the selected suffix.
