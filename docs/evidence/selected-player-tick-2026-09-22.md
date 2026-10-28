# Selected production tick extraction

This packet composes the same five APIs previously called by the three-frame
original-data driver: GameInputFrameSession::try_step; Actor selected prefix;
ground pre-solve; authored-ground solve; selected post-solve. Their existing source
evidence and numeric boundaries are unchanged. It is not the full Simulation
schedule: late predictor harvest, StepSelector/locomotion and camera/presentation
are not silently supplied by this facade.

All per-frame policy is supplied: mapped sample and receiver, sync service,
recognizer, feedback, timestep, LOD/wait, collision schedule/groups/culling/spy,
iteration count and explicit pre/post grind policies. The numeric bundle contains
the same existing host numerical implementations; no tolerance or tuning values
are cached or invented. Source cvars and player settings are supplied actual
retained owners. The hand token pool is retained across successful ticks.

Hands locality reads the actual Actor player field and actual local manager only
when native dispatch requests it. The clamp is a supplied fallible service, called
only at the existing source branch. The original-data selected High/LOD0 test
supplies an explicit unsupported error if it is ever requested; production has no
panic or substitute clamp value. Broader LOD admission awaits a live clamp producer.

The result retains input delivery, hand/face buffers, pre-solve capability, solver
reports and completed post-solve output. Errors are typed by stage and retain all
prior native writes. A caller must not retry the entire tick after partial failure.

The shared patch moves the existing repeated-frame loop onto this facade while
leaving world/query construction, mounted static/grind residency, explicit neutral
sample, sync2, fixed selected schedule and assertions in the test. The test retains
owner identity, dispatch, processed-assembly and final output/numeric checks.
An optional read-only host observer preserves all four existing finite-state
probes, the actual submitted-count capture before solve, and the immediate
after-solve assertion that every submitted retained future is ready. This observer
is supplied by the corpus, with assertions confined to that driver. Passing None
does not read state or call a substitute service. It changes no native algorithm
or publication order. First-harvest implementation and dedicated coverage remain
untouched. Root owns original-data validation and Cargo; no pass claimed.
