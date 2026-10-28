# Withdrawn animation-only preview16

The owner rejected this change: a shorter wind-up did not provide the requested
stronger physical push. Its animation-rate changes are removed in the next
local preview. Retain the following as historical evidence only.

# Stationary mongo start: local host adjustment

The owner reports that regular-stance held X barely moves from rest. The new
preview15 journal captured real failures: tick175..204 (0.50s) stays at
0.019..0.038m/s; tick373..403 also requests zero speed while held strength
reaches4.5. First contact follows the original lead-in around0.53s. S3 mode is
active; the source bank/graph is used. Earlier synthetic X tests began in switch
stance and did not exercise this same mongo tree.

Original Push.xml selects B_PUSH_INTO / B_MONGO_PUSH_INTO using the mirror bit.
Original bank metadata confirms the low-speed strong mongo lead-in is about
0.53s, with push_contact strength0; CYC1 provides strength1. These source timings
are evidence, not a defect in the retail data. The concrete native stance getter
82B97128 reads relative stance; retain that selection and both button mappings.

The local mod now plays only B_MONGO_PUSH_INTO at2x when completed board speed
is finite and has magnitude below0.5m/s. Its normal completion/sequence event
enters the original CYC1 at1x. Do not manufacture an early contact, apply an
extra force, change global time, or increase the original push strength/speed
caps. Rolling mongo, other pushes, all other animations and grading stay stock.
This is an explicit owner-requested feel adjustment, not claimed retail1:1
timing. Confidence: exact scope and rate selection; actual game validation is
recorded externally in local-gameplay-review-r13 before installation.

Original assets and raw reference pages remain external. Keep this preview
local until the owner approves publishing; preserve volumes and isolated silent
diagnostics per the task instructions.
