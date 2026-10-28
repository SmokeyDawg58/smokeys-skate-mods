# Actor physics auxiliary and retained settings owners

Fresh September12 IDA review; selected fields are high confidence, not a complete
Actor/backend constructor or custom-character loading claim.
Raw source bytes and full disassembly are archived outside the repository at
`%TEMP%/skate2-actor-physics-auxiliary-source.json`.

Actor's external reset allocation825fecdc is 80 bytes from Sim, published+21c
at825fece4, with no member constructor. Late impulse allocation825ff458 is16
bytes from Sim, published+218 at825ff45c. AI input allocation825ff470 is176
bytes from Sim, published+228 at825ff4ec. Sim category variable82ff1af0 is55;
category row82ff0eb0 contains maskFFFFFFFF and OR00000100. Its zero-fill policy
establishes initial residual bytes, independently of constructor stores.

AI vectors use v127 zeroed at825feb94. The late constructor initializes matrix,
desired velocity, trajectory position/velocity/gravity/offset to zero and
trajectory time to four -1 lanes at825ff4d0. The flags at+A0 preserve low25 and
clear high7 at825ff4e0; the low25 are already zero from Sim allocation. Actor
AI-controller+214 is explicitly null at825fe964. Auxiliary construction exposes
these retained allocations without claiming the intervening breadcrumbs or
subsequent submittable/jobs have run.

The existing earlier ExternalPlayerReset owner now retains its known Sim-zero
matrix/onboard values. The selected actual HiLOD Processed allocation82e03ee4
likewise establishes zero residual impulse/reset payloads before ctor03ef0.
Generic Processed new/Reset still preserve unknown or previous payloads.

`AIPhysicsInput::operator=`825f0f60 copies ten vectors at offsets0..90, followed
by seven distinct masked stores of flags+A0, from bit31 down to25. Destination
low25 are preserved. Whole-struct clone would incorrectly overwrite those bits.
Actor.SetUpPhysics825ff8dc copies Actor+258 bit02000000 into AnimOut+2aa0
bit01000000 (rotate31 then mask), preserving all other destination bits.

`PlayersBackEnd::GetPhysicsSetting`8255b318 checks player index<=8 and setting<2,
then directly loads backend+a0+160*index+4*setting. `SetPhysicsSetting`8255b3a0
uses the same address without value clamping. PlayersBackEnd ctor8256bf30
resets nine PlayersData rows in ascending addresses then calls
ResetPlayers82564338, which resets all nine again. PlayersData.Reset82561e68
writes wheel hardness .7 bits3f333333 at82561ef4 (constant8202e96c), then truck
tightness .3 bits3e99999a at82561efc. The retained selected settings owner stores
these source constructor values and admits later genuine writes; frame code
must query that same owner.

Custom character loading can overwrite settings: PlayerLoaderImpl's
ApplyPlayerAttributes82815998..b0 reads CACRecord then writes each setting;
Sk8CACLoader.ApplyAttributesImpl82819d14 copies its tuningValues. Neither loader
is replaced with a fixed setting in this change. The default/marquee corpus may
use actual backend constructor fields; it does not establish arbitrary custom
CAC or full backend module registration.

The existing retained PhysicalGameInfo Arc now includes ignoreWipeoutResetButton
at+298, initializedfalse by GameInfo ctor825594e8. Static owner83040500 is
constructed through initializer82f536a8, table82fd0d48. WorldServices forwards
reads of that same Arc. Other source writers include challenge GameLoad825b783c,
GameUnload825b78a0, BackToSP8273db10, and online UpdateGameInfo827543c8/54558;
those flows are not inferred by offline construction.

Focused tests cover retained settings mutation without clamping, valid ninth
slot, independent row Reset, AI negative time versus Simzero, and one-shot
Processed Sim provenance that cannot overwrite a later publication on retry.
Root owns integrated validation and publication-suffix implementation.

Selected live cvar additions for the other reviewed frame closures:
feet-line radius initializer82f9ea08/table82fd62bc/ref830e0d98, section
Physics_Systems_OffboardIK, labelkFeetLineTestRadius, titleLineTestRadius,
default3cf5c28f, GUI0..1. Fade-in initializer82f563b8/table82fd0ff4/ref83047d78
and fade-out82f56418/table82fd0ff8/ref83047da4 use Render_Skaters,
kSkaterFadeInTime/kSkaterFadeOutTime, titles Fade-in/Fade-out time (seconds),
default1.0 and GUI0..5. Raw constructor call sites prove null callbacks for all
three. Native reference registration follows initializer-table address order;
INI load stays unclamped and each getter samples the current linked target.
