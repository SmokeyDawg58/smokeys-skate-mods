# Actual Wipeout output callback

September13 source/implementation packet; confidence high, independent review and
root gates pending. Wipeout.FillPhysOut82dd46b0..4764, called by Update5 at4f84,
does not detect a bail. It publishes the retained actual HeavyLanding6 request.

Request6 (+1a) is read before all output access. False returns unchanged even if
the output is locked or unconstructed. True checks the write guard, captures
State, then reads request value6 (+4c) and stores State+28 landing speed at4700.
Next guard precedes Air+1b1 left-foot flag. True stores state1 without a right
read. False calls GetAir at4748 (fresh guard) and reads right-foot+1b2: nonzero
stores state2, zero stores state0. Both-active therefore selects left. Stores
retain the original State pointer; there is no late State guard.

The implementation reuses retained Wipeout ledger and actual preceding FootPlant
output. No request, impact value or planting result is generated here. Source
owner failures and missing components retain already committed landing speed.
Native detector paths and subsequent outer Update5 fields remain separate.

Tests cover request-false no output access, raw negative-zero speed, all four
foot combinations/left priority and missing-Air prefix preservation. Same-pair
session consumes the actual Controller output token and carries grind exclusion.
