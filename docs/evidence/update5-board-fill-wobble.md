# BoardFill wobble and steering stores

Registered modules cover only 82b30050..82b300e0, following the surface
continuation. They reuse the constructor-retained SpeedWobble in
SkateboardForceRuntime and its existing steering state, with no replacement
objects, scalar summaries, or numeric Update claims.

Source captures outer+10c at 30050, samples output lock at 30054, reads
SpeedWobble.effect+24 at 3005c, then asserts and writes Motion+b0 at 3007c.
It freshly captures outer+10c at 30080, samples the lock at 30084, reads
wobbleTilt+28 at 3008c, then asserts and writes Motion+b4 at 300ac.
The third assertion 300b0..300cc and Motion pointer lookup 300d0 precede
outer.deckTilt+e0 read 300d4 and Motion+b8 store 300e0. Capturing a null
SpeedWobble pointer alone does not consume it; the draft reports a missing
constructor only at the selected scalar dereference after the latch sample.

The existing SpeedWobble constructor 82b2eaf8 writes effect and tilt positive
zero at eb4c/eb50, after binding the real attribute instance. Its selective
Reset writes those same fields at 82b26644/26648 and preserves amplitude+14.
Outer Skateboard constructor deckTilt zero is 82b38b08; actual steering owns
later tilt updates. Shared changes are just read-only effect() and
wobble_tilt() getters, plus two module registrations. No fields were added.

Focused tests distinguish locked-output read-before-assert ordering, fresh second
pointer capture, preservation of the first store on a late scalar failure,
and exact negative-zero/NaN scalar bits. They have not been run. The returned
same-owner token stops before the fresh effective deck transform and forward
velocity calculation beginning 300e4.
