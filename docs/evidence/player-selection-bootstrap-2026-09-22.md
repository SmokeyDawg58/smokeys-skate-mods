# Selected player bootstrap and full character identity

Independent source/code audit. Exact Test-Path was false before creating this
authorized evidence file. No registered implementation or Cargo run.

## Existing owners and missing connection

Reuse player.rs PlayerLoaderSlot: requested/current selection, marquee load,
registration, stale completion and loaded phases already exist. Search found
only its own tests calling begin_marquee_load. Its character_loaded method
currently does not implement native ApplyPlayerAttributes before registration.
Do not introduce a second loader state machine.

Reuse PlayersBackendSettings for the retained backend. It currently owns nine
truck/wheel pairs, not marquee/stance/style/gesture/player-type fields. Fresh
PlayersBackEnd constructor8256bf30 runs nine PlayersData resets then
ResetPlayers82564338 repeats them. PlayersData.Reset82561e68 initializes
regular=false, male=true, style0, GestureSet.Reset, hatNone, playerTypeNone,
truck.3/wheel.7. A fixture's regular=true is not a native constructor default.

Selected first-load connection: OnSimCharacterLoaded82817f28 stores character
asset then calls ApplyPlayerAttributes828158f0 at82817fb8 before registration.
Its marquee branch reads MarqueeDefs.IsSkaterRegular82cfea68 and
GetSkaterHatType82cfe9e8, then SetPlayerCharacter8255b058,
SetPlayerStance8255b570 and SetHatType8255b4a0. IsSkaterRegular uses cached
characters_marquee layout byte0x10; hat uses FETCH/default4. Character face
selection already reads cached layout text0 in actor_animation_mount.rs.
The later local-Actor lookup/reload branch naturally skips with the actual
constructor-empty roster before first spawn. CAS/profile and replacement
remain separate paths; don't invent those results for a marquee session.

Actual local identity already exists in skater_manager_local_identity.rs:
SkaterManager constructor825fb300 has index-1/empty roster; selected offline
ResetGame8273a740..a91c produces index0 through SetLocalIndex826003f8 and
UpdateLocalPlayer825fff1c. PhysicalGameInfo already supplies actual retained
network=false constructor state. LocalBackendOwner owns ghost staticfalse and
the debug-menu sample; a fallible accessor can expose that same field. Network
and ghost reset paths explicitly remain pending. No local roster Actor should
be fabricated to satisfy selection.

Current AnimationStartupSource receives a u64 thread identity and the real
startup retains it in AnimationAttributePool. The mounted corpus source's1 is
a fixture. Production needs an actual host-thread identity adapter shared by
creation/bind-pose checks, not a new job scheduler. Existing job owners and
try_initialize_local_backend are reusable.

## Existing GetCACSettings width bug found by fresh raw audit

actor_animation_start.rs16 currently exposes skater_id as u32 and its match
at95..100 uses four truncated constants. The only implementation found is
actor_animation_mount_corpus.rs109. Thus migration is bounded to these two
files plus focused tests; no production adapter currently supplies the trait.

Native GetCACSettings825f0770 reads stance first, resets gestures, then calls
IPlayerInfo vtable+8 at07cc. Backend IPlayerInfo vtable8203b258+8 points to
GetPlayerCharacter8255b0b8, whose instruction8255b104 is `ld r3,0x5cc(r11)`:
the full64-bit character ID returns in r3. Constants are built with INS RDI
high32 merges, and comparisons are CMPLD, not 32-bit compares. The existing
u32 API therefore loses meaningful identity. Fresh raw branches are:

| Full character ID | Animation pro name | Comparison / target |
|---|---|---|
| 1d939610fea090ee | JerryHsu | 825f0800 /0898 |
| 0ded833242f2f857 | JasonDill | 825f081c /0884 |
| 146d3b7d88385267 | MikeCarroll | 825f0838 /0870 |
| 1733a97b7064d4c2 | PJLadd | 825f0854 /085c |
| 42545b0938e341c9 | DannyWay | 825f08c0 /0990 |
| be3309ce9ba41097 | RobDyrdek | 825f08dc /0980 |

The decompiler omits the last two branches and truncates the first four.
CAC is exactly u64::MAX (`cmpdi r3,-1` at08e4). Only that sentinel reads
GetPlayerGestures8255b6a0 followed by GetPlayerStyle8255b5d0. Normal/Loose/
Aggressive style behavior and existing lazy order should remain intact.
Unrecognized full IDs return no pro name, even if low32 matches a known pro.

Recommended minimal implementation: widen existing trait to u64, use all six
full IDs, retain exact CAC sentinel and lazy selection. A discriminating test
must reject a different high32 with a known low32, and confirm00000000ffffffff
does not select CAC. Then connect the existing loader's current selection to
the same retained backend fields at the verified ApplyPlayerAttributes boundary.
This is a first-load adapter cut, not completion of the whole front end,
network selection, profile service, or playable application.
