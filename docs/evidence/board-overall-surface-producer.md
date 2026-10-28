# Actual board overall surface producer

Original29 stopped at BoardFill because actual Body+348 overall physics surface
was unknown. That is not a constructor defect: CollisionInfo.Reset82b23630..23760
does not write relative+308, and Body constructor38270..38914 does not write
Body+348. Constructor382dc invokes Reset on Body+40. Reset's seven physics
types and contact flags are distinct fields and cannot stand in for overall.

The producer is ChoosePhysicsSurfaceType82b24878..49fc, called at82b3709c
after actual PostPhysics contact selection and seven motion updates. The selected
world API now invokes exactly this leaf using the same retained contact owner;
it does not claim that those preceding stages or the solver have completed.
No ContactSpyFrame is required by the source leaf. The corpus may explicitly
schedule this selected producer after line completion and before Update5.

Native order: zero twelve scratch counts; read wheel contact bytes34c..34f in
ascending order, only reading the associated physics word2f4..300 for a true
contact. Count types0..11, but choose only1..9, starting with winner1 and using
strict greater-than so ties keep the lower candidate. The new actual front end
reads collision_state directly. The older surface-only diagnostic parts array
is untouched; both routes reuse the same count-winner primitive. Unknown used
fields fail without invoking the cvar. Invalid scratch indices return a host
error rather than emulate native out-of-bounds stack access.

Finally source49d8..49e4 loads the live kForcedWorldSurface target and its value;
nonzero raw bits override, otherwise the voted type is stored at49f8. A failed
late cvar leaves old overall intact. Actual reset false contact flags therefore
produce1 if this leaf is invoked, without fabricating contacts or an overall
constructor default.

The previously absent cvar is index113: reference8308d858, initializer82f7b7a0,
table82fd4460. Fresh raw confirms default0 and null callback atb7c8/b7cc.
Section Physics_Surfaces, label kForcedWorldSurface, title World Surface;
sequential enum tags: as detected, smooth, rough, slow, slippery, very slow,
unrideable, doNotAlign. Actual shared manager linking and startup parsing own
the target. Runtime lookup is delayed until the leaf's final read. Registration
follows table order between existing55 and28; title-order oracle includes its
actual section. No controlled-zero provider is used by this route.

Parent independently reviewed all98 raw instructions and accepted lazy reads,
vote bounds/selection, late override, and absence of a constructor write.
Focused tests cover lazy unknown fields, reset-like false contacts, real
collision fields over diagnostic data, ties, non-candidate11, invalid12,
late-cvar failure preservation, raw override bits, and actual enum reloads.
Registered changes await parent-owned gates.

Parent checkpoint follow-up: Clippy43 passed. Workspace33 found an incorrect
new INI test expectation: ReadLabel82a8a7ac uses GetInt for this enum, as for
the already recovered gesture enums. The test now supplies numeric 7 and 2
and separately verifies that the display tag `rough` parses as zero. The
production parser is unchanged. Original30 remains16/17, now stopping earlier
in the newly selected GroundSkateboard path at an obsolete known-W assumption
in GeneralUpdate; this run does not establish completion of the new surface path.
