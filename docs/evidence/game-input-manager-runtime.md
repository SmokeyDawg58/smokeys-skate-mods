# Actual GameInputManager lifecycle and update

Confidence: **high** for the selected retained owner path. Root independently
reviewed the manager Update and PlayerInput constructor raw instructions. This
does not claim the missing Simulation frame caller or SkaterManager construction.

`GameInputManager::GameInputManager` 826e3308 initializes its fixed-capacity nine
player list, the `GameInputBroadcaster` port on queue 6, and two known-pattern
vectors, then calls Load 826e3260. It does not call Reset. LoadDataFile 826e3180
loads, parses and resets pattern caches before publishing each collection. The
seven original relative paths are retained in native order: five right-stick
collections followed by skaterls and skaterstep on the left. The Rust manager
loads and publishes sequentially; a fourth-file refusal retains the first three
collections and no players. Original assets remain external.

Reset 826e2c20 deletes the old players in list order and only then clears the
list end. Each new PlayerInput 826e2440 captures the supplied actual SIM input map
before allocating both histories, left then right, before constructing and
registering GameInputSK8. The manager publishes the returned player before
AddPatterns: all left collections, then all right collections. `PlayerInput`
offsets are matchers 0, histories 20, embedded pad 28, input pointer 1b0, flags
1b4. Flags are `(index == 8) + 1`, not a guessed local-player classification.
History backing capacity is 201 and its initially valid sample count is zero.
The original temporary copied into unused storage is uninitialized; reserved
Rust storage is not represented as source-known zero samples. Existing matcher
ring validity admits at most 200 published points.

PlayerInput destruction 826e12d0 destroys each stick's history then matchers,
left before right, before the input receiver unregister callback. The manager
uses the same actual CommandManager registrations. A retained safe-host alias
to a deleted player becomes unavailable, while a replacement receives a new
occurrence identity with the same native receiver ID. Checked failures preserve
completed destruction/construction prefixes; retry after a partially failed
lifecycle is not a recovery operation supplied by this implementation.

Update 826e2578 first requires the actual DebugOverlay singleton even for an
empty player list. For each retained player, its actual pad count is checked
before flags against the caller's update mask. Eligible players run
UpdateRecognizer 826e2008 followed by GenerateGameInputEvents 826dfdc0, before
the next player. Only after the entire loop does the same queue 6 Flush run.
Errors preserve earlier history and queued-event writes and do not flush or
process later players. The manager owns the existing GameInputEventRoute, so
Actor listeners subscribe to the same broadcaster and receiver occurrences.

Runtime recognizer services use linked live input cvar references, including
process-shared once-copied gesture-display references, the actual Vault and the
shared allocated DebugOverlay. Enabled debug paths create the generated
recognizer class instance 7f086875d621c88c for the source stick collection and
perform every requested attribute lookup. The selected executable discards the
returned data pointers: missing values select generated defaults without a
scalar read or a fabricated draw operation.

Matched gesture display additionally calls the actual Simulation ISkaterManager
local-index query (826e1cd8, concrete folded target 824859b0, field 174). No
SkaterManager retained owner exists yet. An explicit late collaborator is
required on this branch; neither zero, -1 nor eight is invented. Source cvar
defaults for input and gesture displays are false, so actual linked defaults
can pass the normal path. SetLocalIndex 82600380 has ghost/offline/network
remapping semantics and remains outside this implementation.

The original InputCfg mappings 59..63 are already supported, contrary to the
older recognizer-plan note: DebugPrev/Next/Pause/ReplaceSkateboard/ResetWorld use
DPadL/DPadR/Start/DPadU/Back pressed states, respectively. Controls 74..95 in the
selected original map use buttons, arithmetic and authored literals. These 27
mappings consume no global arguments; this does not authorize fabricated global
values for unsupported menu/configuration expressions.

Focused tests cover ordered partial Load publication, nine-player Reset and
occurrence replacement, native flag categories, overlay-before-empty-list,
count/mask gates before unlinked cvars, a later-player lock failure retaining an
earlier queued event, and both flag categories completing the final Flush.
Tests and original-data integration are pending the root's combined Cargo gate;
no independent Cargo run was made by this implementation agent.

Root validation23/26 PASS: workspace23 3,799units/6docs, original23all17, Clippy26/format23/diff23. Independent source review approved construction/Reset/destruction/Update andruntime services. Originalcorpus takes actualmanager.player(0) Arc forsameActor, feedsoriginalcodec/map/patterns andcallsmanagerUpdatewithoneFlush per sample, verifiesactualOllieevent andActionGraphintents. Command sync2/sampletiming and laterActor scheduling remain controlled; sourceSkaterManager localindex remains a late dependency.
