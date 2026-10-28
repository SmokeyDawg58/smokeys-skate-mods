# Coffin/offboard five reported gaps ? 2026-09-25

SOLO. No compilation or game launch.

- Motion Pre120: native8290de98 reads HasIntent condition200 (sConditions+320). Sustain independently selects true default829221c8.
- Motion Pre334: native82915034..150fc reads condition234 (sConditions+3a8), then only if true condition273 (+444). Both existing HasIntent. Sustain independently selects true default829221c8.
- Motion update828f0158 (2190410072), CoffinToBS268: parent Coffin23 at828ee6d0 then CreateAttribute, native empty HandBusy/DisallowPumping updates, PlayAnimation. Coffin parent calls OnBoard3 then ForcePhysics empty update, AttachIntent, SetTurning, CreateAttribute, ScoringGrabs89. Existing handlers reused. Both authored aliases268/269 receive their exact flattened ancestry once, 29 slots.
- Motion update828f1588 (2190415240), ToOffBoard668: parent Lateflip80 at828eedc0 then CreateAttribute, PlayAnimation, AirDismounting. Parent calls OnBoard3 then ForcePhysics empty update, AttachIntent, CreateAttribute. Both authored aliases657/668 receive their exact 26-slot ancestry. Reuses the prior retained AirDismounting instance lifecycle and corrected before_sprint frame setter.
- Action79 Pre829034f4..3718: (c44 && !c42 && !c45) || (c45 && c42 && !c44), preserving lazy ordered reads and repeated cached queries. c42 is existing IsMirrored; c44/c45 are HasAGIntent. Independent Sustain829086a8..881c: (c44 && !c45) || (c45 && !c44), omitting stance. Raw false exits8290024c/82906a2c both verified li0; true exits verified li1. Uses existing Action condition cache.

Static audit PASS: all four authored state ancestry arrays match parent declarations in order, exactly one registration per alias and no repeated callbacks. Motion condition descriptors checked. Rustfmt parse PASS. Added Action79 truth-table regression covering both stances and both masks, including no stance query during Sustain; test NOT executed. No claim of runtime validation. Quiet play unchanged. Unrelated Boneless physical602 remains unfinished.
