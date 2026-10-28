# Presentation category2 job affinity amendment

Unregistered source review and patch proposal, September 12, 2026.
`animation-job-category2-amendment.patch` is an UNAPPLIED narrow change to the
existing `animation_job_execution.rs` backend. New children
`animation_job_affinity.rs` and `animation_job_affinity_tests.rs` await that
patch. No registered file was edited, and no Cargo command was run.

## Original source and retained category

`Anim::AnimJobScheduler::SetCategoryForJob824d4f44..824d4f58` sets affinity1
for category0 and affinity4 for nonzero categories. The intermediate compare
against2 at4f48 does not alter this setter's nonzero path. This does not mean
all nonzero categories have the same wait behavior.

`WaitForJob824d53a8` raw53dc..53e8 dispatches category0 versus category2;
other nonzero categories return without waiting or checking IsDone. Category2
checks the selected job at53f0, resolves the actual job manager only if not
done, then joins one job on the SAME local queue with mask4 at5458..5460.
It repeats IsDone at5468 and loops while not done. Category0 similarly uses
mask1 at54ec..54f4. The checked host extension admits categories0/2 only;
other categories produce an explicit UnsupportedCategory error rather than
claiming a completed job or incorrectly joining affinity4.

`AnimCmdSys::AnimCommandSystemImpl` retains the supplied category as
mJobAffinity at82a32288. Actual `Sync82a30088` reads this SAME retained
Impl+0x9c at300c0, passing it with the null-root job to the scheduler virtual
at300d4. Thus caller wait selection must use retained ACS category, not infer
category from a mutable entry-point object or use a new default value.

## Same queue and exact admitted ready ordering

LocalBackend::AddNotReady829afa58 retains the entry-point affinity and packs
priority data at829afb18..44. Rawfb24 truncates affinity to a byte; fb30..34
complements its low6 bits; priority is the unsigned low byte atfb2c. The final
unsigned64 ordering key contains ready/state bits, complemented affinity,
priority, and submission low47 bits.

RunSingleJob829b00b0 first forms a caller mask from complemented low6 affinity
bits at00bc..00d8. It tests readiness and nonempty affinity intersection at
0108..0124, ORs that caller mask into each packed key at012c, and chooses the
unsigned64 minimum at0130..013c. It rechecks readiness and affinity intersection
at017c..0198 before claiming the chosen job. The equivalent key among admitted
ready jobs of source priority128 is:

    ((! (job_affinity & caller_affinity)) & 63, submission & ((1 << 47) - 1))

All admitted entry points continue to require priority128, environmentLOCAL,
no sleep and no break-on-entry. For ANY63 workers, an otherwise equal affinity4
job precedes affinity1 even if submitted later. With a category0 caller mask1,
only affinity1 jobs qualify; with category2 mask4, only affinity4 jobs qualify.
Same-affinity ties retain submission order up to native low47 wrap behavior.
Filtering solely by affinity while retaining the old insertion-first choice
would miss the newly observable mixed-category ANY ordering.

The amendment copies each actual entry affinity into its retained Instance at
submission. Subsequent mutation of a dormant entry-point owner cannot alter
already submitted work. take_ready accepts the caller mask, filters without
claiming ineligible jobs, and selects the native admitted ordering key. Worker
threads receive their actual retained backend.worker_affinity (63), instead of
calling an unfiltered function. Category2 introduces no separate queue, thread,
manager, work owner, or job implementation.

## Interfaces and integration boundary

The existing backend.wait(root) retains category0 behavior. New
wait_in_category(root,category) implements the selected source wait masks.
The same release_selected method continues retiring only that completed ACS's
jobs and leaves peer handles intact. Unsupported categories are host admission
errors, not a reconstruction of native return semantics for those branches.

This scheduler patch deliberately does NOT broaden RAW ACS construction
admission. Root's actual presentation integration must also:

1. Retain actual setup.category on the existing BufferedBodyCommands owner at
   the corresponding AnimCommandSystemImpl construction step; this maps to
   native mJobAffinity+0x9c, not Data's SQT allocation fields.
2. In submit_sqt_programs, replace the hardcoded entry affinity1 with
   selected_category_affinity(the retained category) for every selected job,
   including the null root. Preserve its current source scheduling order.
3. In sync_sqt_programs, pass the same retained category to wait_in_category.
   Keep the existing exact backend Arc identity checks and only publish SQT
   outputs after actual completion.
4. Admit source-proven full/FacePres setup capacities/averages and matrix/RAW
   storage separately. Accepting category2 as category0 or weakening unrelated
   setup guards would not complete those construction requirements.

No invented default category is needed: existing actual body/hand/face setup
records carry0; source presentation full/FacePres setup records carry2.

## Focused validation and limits

Seven tests exercise distinct category masks/unsupported admission, category2
wait leaving an affinity1 peer ready, submitted affinity surviving later entry
mutation, ANY native mixed-category priority, ineligible masks and low47 tie
keys, actual selected FetchRaw->SqtBlend->null-root dependency completion, and
unadmitted entry affinity rejection before queue mutation. Deterministic tests
use the same queue/backend implementation with no independently racing workers;
production Init retains its three actual LOCAL/ANY worker configurations.
The drafts are formatted; tests are not compiled until root applies the patch.

Confidence: high for decoded category control flow, per-entry retention and
admitted priority/affinity ordering. Host mutex/condition-variable blocking
replaces native lock-free busy joining; timeout callbacks, queue-pressure
self-running, arbitrary priority values, and other affinity/category families
remain outside this bounded amendment. This is not full job-scheduler parity
or proof that presentation ACS construction/publication is already mounted.
