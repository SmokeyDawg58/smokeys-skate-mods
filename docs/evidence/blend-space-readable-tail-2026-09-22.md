# BlendSpace adjacent serialized normal lane ? 2026-09-22

Original b_bump set0 has48 volumes with logical size180, normalsOffset116,
heightsOffset100 and animation indexes84. Native ComputeParameterization3
reads four normal vectors at120,136,152,168. The last W therefore consumes
bytes180..184 from adjacent original serialized storage. Source delta.W is
zero, but IEEE NaN/Inf multiplication means this lane cannot simply be dropped.
Fresh raw/effective-address proof is retained in
artifacts/blendspace-parameterization-independent-2026-09-22/geometry-readable-tail.md.

Parser now retains an optional exact adjacent word from the same controller
payload, separately from unchanged logical volume bytes. Kernel admission can
read this word at the exact logical end; unavailable source bytes remain a
VolumeBounds error. It neither fabricates padding nor changes offsets/stride.
The controller payload bound is still the admission boundary; this does not
claim knowledge of memory beyond the actual supplied allocation.

Independent source/code review approved. Parser test preserves exact NaN bits
from real appended data and distinguishes absent tail; kernel test consumes
that last normal W and rejects an unavailable word. Registered under validation.
Confidence high for source addressing and retained known bytes; real execution
and existing host floating-point limitations remain separate checks.

Registered readable-tail-input-1 validation passed fmt, strict clippy and
4,636 unit plus six doc tests. Original-data 18/20: neutral frame 2 now
passes parameterization/Complete/GetAttributes and reaches BlendSpace Eval.
