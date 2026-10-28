# Marquee stance scalar projection

Root original-data test found RelocatedPayload at dependency1 offset468948
while ApplyPlayerAttributes read the entire20-byte marquee cached layout.
That layout includes relocated text pointers. The native stance helper does
not read those pointer bytes: fresh raw IsSkaterRegular82cfea68 loads cached
layout atfea88 and executes `lbz r31,0x10(r11)` atfea90. Only byte16 is read.

The existing Vault API has checked whole-layout and relocated-text accessors,
but no scalar field projection. The bounded patch adds try_cached_layout_field
using the SAME cached pointer resolution and existing Vault.range validation.
It returns only offset16,size1 to the marquee caller. Unrelated relocated text
bytes are excluded from that range; overlapping pointer bytes, overflow and
out-of-bounds access still fail. Missing cached layout remains None, allowing
the existing generated DefaultDataArea20 stance-zero fallback. No default is
substituted for malformed nonnull layout or overlapping relocation.

Two regression tests distinguish unrelated pointer relocation from every
possible four-byte relocation overlap with stance16, plus bounds/null checks.
The old whole-layout reader still rejects the same synthetic relocated layout.
Original player-load test remains the full producer-consumer regression.

Shared patch only; no registered edits or agent/reviewer Cargo. Confidence high
for the exact source read and checked implementation. Independent reviewer
approved the fresh lbz+10 proof, range overlap/bounds checks and unchanged null
default. git apply --check passed. Packet sealed; root owns execution of focused
and original-data tests.
