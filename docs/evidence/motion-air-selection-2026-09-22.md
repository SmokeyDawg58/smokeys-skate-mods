# Reached Air Motion selection chain

Original-compass reaches frame10 Motion expression157 after Action43 and Motion405/142. The retained generated declaration tree places ordered siblings beneath InAir21: GrabsTweaks77 (DBL642, FS647, BS658, Mute669, Stale676), BoardAdjusts78 (left248,right249,up685,down687), then InAirStatic79/Air252/Sequence690 or Default691. Existing generated evaluator and condition cache remain the sole selector and intent owners.

Fresh native Pre switch table8290aa90 (evaluator8290aa50):

|Expression|Target|Condition|
|---|---|---|
|157|8290ed44|245 BSGrab|
|161|8290eef4|233 MuteGrab|
|164|8290f02c|256 StaleGrab|
|166|8290f104|258 BoardAdjustLeft|
|167|8290f164|261 BoardAdjustRight|
|168|8290f1c4|262 BoardAdjustUp|
|175|8290f4ac|268 BoardAdjustDown|
|184|8290f854|275 HasAnimAttribute sequencetoair|
|185..190|8290f8b4,f914,f974,f9d4,fa34,fa94|276..281 lateflip intents|

Each directly returns its existing cached condition. Sustain table8291e254 entries all target829221c8 constanttrue, with no reads. Adjacent AirStatic transition1423 expression292 at82912dc4 reads HasIntent9, then only iftrue PhysFilteredState85 (calls12de8/12e40). Lateflip gate335 at82915100 reads Air223 then HasIntent139 (calls15124/15178); Sustain335 points8291fa8c, the same Air223 predicate as134.

HasAnimAttribute82caa1d0 first finds IAnimatable; absence returnsfalse. Slot22 at82caa25c resolves through SkaterAnim vtable82302d28+58 to GetTreeAttr82d79170. GetTreeAttr scans retained mTreeAttrs begin/end in source order, compares all five packednamewords and copies the first match. It does not recompute tree attributes. The existing Actor animation phase producer already owns these rows. The new query verifies existing interface/allocation/entity identity and samples that same storage. Empty storage gives false; uninitialized occupied rows are typed missingowner errors, not synthetic misses.

Native HasAnimAttribute checks optional sequence then NOP/comparison. Current runtime attribute producer does not retain native sequence metadata: a found row with requestedsequence !=-1 is explicitly pending. Actual condition275 has defaultsequence-1/NOP, so its full path is connected. NOP does not require numeric attribute data. NonNOP uses existing scalar comparison for retained float data; unsupported value representations remain explicit. Native type2 scalar representation is not invented. Packed names reuse existing checked FastString30 encoding, preserving its shipped-alphabet/length admission.

Tests cover packed firstmatch/laterunknown laziness, emptyactualstorage, directleaf true/false/errors, exact ordered InAir21 search through Default691, and lazy twooperand transitions. This is selection coverage, not a claim that all Air behaviour callbacks or all Air exit conditions are recovered. Native338 requires actual IsBodyFlipping160,339 requires PrelandTilt282 afterphysical89; those remain pending in existing dispatch. One-foot branches similarly require their actual conditions. No grind/offboard expansion.

Author ran standalone rustfmt and git apply --check --ignore-space-change (registered CRLF context); no Cargo or original-data execution. Root integrates and runs those gates.
