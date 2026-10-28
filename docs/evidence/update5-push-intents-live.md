# Update5 push pulses and final input intents

Native PhysicalPlayerHiLOD::Update5_ProcessOutput82e04ea8, selected60d0..61e8.
The caller follows animation output60cc and precedes DoPhysicsSurfaceCheck61ec.
Root author; independent raw review output_continuation. Registered/gates pending.

Read actual Processed8c4 once. Bit02000000 gates all stance reads and the need
for right-foot04000000. One actual AnimOutPhysIn pointer yields regular2886
then switch2883. Mongo is right-foot == (switch == regular); otherwise regular.
Actual HiLOD120 bit00400000/00800000 gates rising-edge output pulses. Writes
set true only and the regular branch is else-if. No implicit output reset.

Native6168 rereads HiLOD word;616c captures Processed before617c replaces only
mask00c00000. Then sample output lock6180, read captured Processed8c8 bit0,
assert/lookup Intents, publish coffin61ac. Final grabbing uses fresh Processed
8c4 mask0x20 before its own guard and store61e8. The retained source stores the
non-collision word; separately owned collision bit01000000 remains untouched.
The selected adapters read the same actual Actor animation/Processed owners.

Four tests cover all stance/right-foot combinations, repeated held pushes and
release/rearm through actual PhysOut.Reset, capture failure after pulse before
history, and null Intents preserving prior history. No complete Update5/frame
claim: surface, physical-animation intents/Fill/Reset and update count remain.
