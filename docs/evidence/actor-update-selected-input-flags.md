# Selected Actor.Update after physics setup

September 19, 2026. Confidence high: root and independent reviewer read fresh
Actor.Update825f10f0 and Chasee.UpdateByChasee82de5200.

SkaterManager.UpdatePhysics825ffd98 calls Sync825ffea8, SetUpPhysics825ffed4,
then Actor.Update825ffee0. Actor.Update first checks its actual AI controller;
an existing controller requires its Update before any flag clear. At825f1138
Actor+258 is masked by97ffffff, clearing impulse40000000, reset20000000 and
AI-input08000000. This must not clear the already copied AnimOutPhysIn flags.

The next virtual call825f1140 is Chasee.UpdateByChasee. Its first
GetNumChasers returns vector end-minus-begin; zero returns before any pursuit
services or timer changes. Existing ActorConstructorPrefix.chase_end_empty is
the actual constructor projection; the selected adapter checks it at this
point, preserving the flag clear if a nonempty Chasee is unsupported.

The adapter requires actual constructed auxiliaries and their null AI pointer.
It neither creates pursuit ownership nor implements nonempty pursuit. Actor
sound-produced/grind-sound timer maintenance after1140 remains explicitly
outside this selected first-skating schedule, along with parked audio.

Validation pending root integration and original-data tests. This prefix does
not establish complete Actor.Update or continuous-frame parity.
