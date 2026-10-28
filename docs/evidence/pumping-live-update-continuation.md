# Pumping live update continuation

Root source inspection, September12. Complete Update/Calculate drafts now exist
in physical_player_pumping_update.rs, with physical_player_pumping_attributes.rs
and input_cvar_pumping.rs adapters. They remain unregistered pending independent
review, typed retained-vector migration, tests and the actual Ground caller.
This packet is not a claim of recovered runtime behavior.

Ground.Update calls Pumping.Update82de0428 at82da1014, after actual
Skeleton.UpdateGroundSkateboard0ea8 and the deck-angle calculation0eac..0fd0.
The retained Pumping owner already exists in physical_player_pumping.rs; reuse
its fields and Reset. Update must not allocate a new state per call.

Update clears acceleration0468, dot3(comToDeckWorld,normal)0478..488 captures
height BEFORE recordValid0490. Constructor-false goes directly to08a4: full
position store08ac, full normal store08b4, recordValid08c0, lastHeight08c8,
then live kDrawNewPump08d0. Record-valid executes scalar/graph Vault fetching,
height smoothing and time threshold, Calculate82dde2f8 at0704, amount070c,
acceleration07cc and lower/upper clamp08a0. Source ZAxisVelocity parameter is
not consumed by this function; do not invent a read for it.

Calculate82dde2f8..e908 computes displacement/DT velocity length, actual global
physics_pumping_default graph and attributes, normal/displacement cross terms,
angular-speed filtering, normal/deck-angle absorption, three layout graphs,
live kDrawDeckAngleCompression, and final angularSpeed times speed graph.
Raw VMX128 operand decoding is mandatory; decompiler loses high VA bits and
misrepresents the sign selection in Update0668 and return in Calculate.
All attributes must use retained original global collection/runtime fetching.
Do not implement only the constructor-invalid fast path as a complete Update.

The Ground caller captures Reckoning+1b0 and dot3s against raw deck.Z+60 and
skater.Z+20. Only ordered abs(deck dot)>abs(skater dot) computes two signed
angles around Reckoning+20, wraps each with floor/strict half-turn comparison,
then uses abs(their difference). Other branch uses +0. It captures
Reckoning groundNormal+1f0, then processed DT+93c, then actual
Skeleton.GetCOMToDeckWorld0ff4 before Pumping.Update. Those pointers/reads
must preserve order and W knowledge, especially actual retained full stores.

Generated Pumping class constructor828b11d0 independently disassembled:
classKey=ab3a006f_3a2956e2 (11e4..11fc), generated cached layout fallback
DefaultDataArea(c0) at1220..1228. Dynamic debug initializers:
kDrawDeckAngleCompression82f9c1c0 and kDrawNewPump82f9c210. This establishes
class/layout identity, not the retained GlobalAttributeCollection default
instance construction; inspect its actual caller before binding a live owner.

Raw Update attribute keys (all actual global+f8, index0; each scalar fallback4,
graph fallback50 and X/Y offset10/30):
04d8 873402e3_c7f1a2ca smoothing;0530 a898f4cd_23f5d9b0 threshold;
0588 4bae22d0_e9c5851c maximum delta;06ac ed5d4acc_75c0c47d time graph;
07a8 nonpositive branch4747cb3d_449b8780 acceleration multiplier;
0810 ca49d85f_a629f7e7 positive acceleration cap;
086c ced79378_356747bd negative acceleration cap.
Raw Calculate keys: e430411e900c_4dbf7e9f speed graph (fallback50),
e59491cf6ee0_4908570e angular smoothing scalar,
e690f83562fb_39c3031c ground absorption scalar (pointer retained across later
fresh global layout graph), e8a436a24a73_70143936 deck absorption scalar.
Calculate cached-layout graphs:40/60 ground absorption,00/20 minimum crouch,
80/a0 deck-angle absorption. Both debug cvars default false with null callbacks,
section Physics_Pumping and exact labels/titles given by their initializer.

Update positive acceleration multiplier branch0744/748:
1ad7b592_6a86dc10 (shared lookup07a8). Do not collapse source ordered sign
selection or its unordered behavior into an absolute-value multiplier.

Global binding now independently verified from raw constructor828b1608:
1630..1644 builds retained r28=d7edbd36_2d7d2152. Pumping allocation29e8,
nonnull29f4..29fc passes this collection key and msgPort0 to828b11d0;
nullable result is published at Global+f8 by2a08. Use a retained runtime Vault
instance of that class/default, preserving allocation failure explicitly.

Further Update scalar audit:05bc rounds (1-smoothing)*oldSmooth first;
05c0 then performs fused newHeight*smoothing+that rounded term. The threshold
branch05cc is PPC BGE (NOT LT, so unordered enters the retained-value branch),
not Rust ordered >=. The maximum-delta skip05e8 is BLE (NOT GT), so unordered
skips clamping. Sign selection0640..65c is ordered >0 and >=0, yielding +1
for positive, +0 for either zero, and -1 for negative or unordered;0668
multiplies that selected result by the fetched maximum. Preserve raw branch
conditions; Hex-Rays emits a misleading unconditional negative result here.

Validation update: workspace32 passes3,887 units and6doc tests;Clippy40 passes.
Original29 passes16/17 and now reaches board contact publication, stopping on
the missing overall surface producer. Pumping.Update/Calculate still await
integration and review; these results do not validate their live execution.

The two debug references are now appended at registry111/112 in native table
order6074/6078 after Reckoning debug references. Both raw initializers pass
defaultfalse and callbacknull. Original strings are Physics_Pumping,
kDrawDeckAngleCompression / Draw Deck Angle Compression and kDrawNewPump /
Draw New Pump. Source DefaultDataArea828b5340 returns83059d30; fresh192-byte
read verifies all zeros for cached-layoutc0 and smaller scalar/graph fallbacks.
Nonnull malformed payloads remain errors. The draft retains the scalar pointer
across Calculate's ground layout graph before loading that scalar, and keeps
both source NormalizeSafe calls even though the first result is overwritten.
