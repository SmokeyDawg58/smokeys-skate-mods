# Board constructor wheel-output reset

September12, root implementation after independent agent source discovery.
SkateboardBody constructor82b382dc calls CollisionInfo.Reset82b23630 before
the later body Init. Reset loads positive zero at23650, stores wheel height
at CollisionInfo+318 (Body+358) at23710, then writes four gJ normals at+260
and four positive-zero distances at+2a0 in2373c..2375c.

The retained SkateboardWheelLines projection previously left these fields
unknown even after body construction. Its host allocation still starts unknown;
SkateboardBodyConstruction.try_construct now publishes the reset values at its
existing constructor-field boundary before attribute reads and body Init.
It preserves the same retained wheel state, without completing a query or
changing deck-line payloads. A poisoned owner lock is an explicit host error.

The old selected system-logic corpus expected a missing wheel height on a
constructed body; that expectation was contradicted by the native constructor.
It now checks the actual initial positive-zero height before its existing real
terrain query. The main same-Actor corpus can consequently attempt Update2
immediately after Update1 using real constructor output. No early query is
inserted and no Ground result is forced. Combined validation remains pending.
