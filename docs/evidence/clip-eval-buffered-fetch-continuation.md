# Clip evaluation and buffered fetch continuation

Confidence: high for mapped control flow and descriptor construction; dynamic
SQT evaluation is incomplete pending native interpolation and trajectory kernels.

The production VBR integer decoder already exists as
`AndaleClipPart::sample_vbr_sqt`. The normalized-time and whole-pose interpolation
convenience APIs are not a replacement for the runtime fetch path: they skip
buffered command time quantization, packed 16-bit interpolation weights, original
global part matching, authored translation.w channel weights, and trajectory
previous/current-frame displacement. Their generic nonfused linear interpolation
and square-root normalization also differ from the original VMX extractor.

The next source chain is Clip::Eval82d6ef78 -> AnimCommandSystem::Fetch82a2f740
-> FetchEngine::EnQueueCmd82a30458 -> FillExtractBuffers82d74420 -> clip
BatchFetch82d737d0 -> Vector_ExtractPackedNVBR82d71268. The latter uses actual
integer VBR frame decoding, source FMADD interpolation, two reciprocal-square-root
refinements, original per-bone channel weights, and TrajectoryCalculation82d6e2c8.
Pose825852d0, AddSQT82588308 and Mirror82c92818 then require the same ACS stack
and actual retained hierarchy; they cannot substitute an already evaluated pose.

Implemented prerequisite: `AnimationTreePool::prepare_clip_fetch` reads a real
Clip node and prepares its original Fetch request without prematurely writing
mWasEvaluated/mOldTime/mNumLoops. It uses baseSpeed*speed, source FSEL time clamps,
and the zero-loop previous>current correction. `ClipFetchPlan` retains the actual
clip database occurrence, command owner and destination hierarchy. No SQT output
or successful pose evaluation is claimed by this plan.

The buffered timing module preserves low18 previous-time bits using multiplier
44b60b33, source epsilon3a34002d, low11 delta/current bits using447fe000,
and EnQueueCmd inverse constants3a34002d/3a801002. Nonzero loop count becomes the
native loop bit; current time decodes from an absolute field when looped and from
previous+delta otherwise. ALL_FETCHES2 resolves through the actual setup; only
FETCHBLEND1 enables interpolation. Floating-to-integer conversion outside the
representable source domain remains explicit instead of saturating.

BatchFetch uses destination global part IDs in hierarchy order. A successful
match at the current clip cursor consumes it; a later match does not advance that
cursor. Frame positions use source FPS*time clamping, real truncation offset for
unblended fetch, and u16 fraction*65535. The trajectory part at bone offset0
retains a previous pair and actual full-cycle delta from clip fixed words when
looped; zero loops produce the original zero translation/identity quaternion.
Channel weights follow the actual 16-byte AnimationPart header (PDB fields
mHeader/mCompressedDataSize/mCompressionHeader/mCompressedData), and come from
the retained original part bytes, not decoded names
or invented uniform values. No-matched-parts currently returns an explicit
boundary before the native dummy extraction descriptor.

New retained `AnimationCommandSetup.fetch_time_error` starts at FetchSys
constructor zero. Both actual NewACS registration paths now perform the explicit
post-construction, pre-registry write at824d6540 using QueryFetchTimeError
82a22950, constant bits3ada1019. This distinguishes construction failure prefixes
from the completed setup required by BatchFetch range checks.

The initialized Actor corpus fixture prepares plans for both actual clip children
after its real Play/AddTime calls and checks command identity, part bounds, packed
frame order, authored weight lengths and trajectory descriptor pairing. Root owns
compilation and execution. External source packet
`%TEMP%/skate2-native-clip-eval-fetch-source.json` retains ten complete function
decompilations and original bytes. Remaining work: command error/capacity/stack
admission, exact native VBR interpolation and
trajectory extraction, compound ACS nodes, then actual queued body-job publication.
