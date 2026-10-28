# Pumping.Fill after Skeleton

Confidence high. Fresh raw82dd4348..447c, actual Update5 call82e04f24.
New unregistered output child physical_player_phys_out_pumping_fill.rs and owner
child physical_player_pumping_fill.rs; three focused tests, not run.

Initial output Skeleton+200 is explicitly +0 at4394 after the entry guard; no
Pumping field is read first. Then the actual captured Pumping pointer supplies
absorption+38 -> Skeleton+204 at43c0; groundNormalAbsorption+3c -> Skeleton+208
at43ec; groundNormalMinCrouch+40 -> Skeleton+210 at4418; deckAngleAbsorption+44
-> Skeleton+214 at4444; pumpAcceleration+30 -> Ground+fc at4470.

Each load is distinct and late. Raw4398/439c,43c4/43cc,43f0/43f4 and4448/444c
sample the output lock byte before loading the source value, but only act on
the sampled assertion afterward. Deck absorption instead loads441c before lock
sample4420. The kernel preserves this distinction using existing guard tokens;
it does not prefetch a payload snapshot. Each store checks its actual component.

The same-owner adapter captures owner.pumping Arc once and uses owner.phys_out;
it does not allocate/reset Pumping or require its constructor before the first
source use. Thus missing Pumping produces the actual initial-zero output prefix
before its first field read fails. Existing private Pumping fields are accessed
from the new owner child and no parallel scalar owner or tuning is introduced.

Proposed registrations: construct::pumping_fill under PhysOut, and pumping::fill
under physical_player_pumping. API try_fill_pumping_output(&Arc<actual owner>).
Caller must reach the real selected post-Skeleton continuation. This does not
claim complete Update5; next call82e04f30 is GrindManager.Fill, explicitly parked.

Tests cover the first zero before missing-owner failure, late deck-value failure
preserving three prior compressions and ordered reads, and locked entry with no
Pumping reads. Formatting passed; no Cargo or original-data probe run.
