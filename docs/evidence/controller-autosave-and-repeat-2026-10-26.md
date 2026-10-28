# Controller Settings preview27

Supersedes preview26's explicit Save/Cancel and preview switch. This is a PC UI
policy, not a claim about reconstructed console physics. Controller Settings is
between Gameplay Settings and Display Settings in both Options variants.
Its root contains Left Stick Settings, Right Stick Settings and Reset to Default.
Every value adjustment persists atomically and becomes active immediately.
Back navigates normally and never discards a successful adjustment. A failed
write preserves the prior active values and reports an error without losing them.

Only continuous physical left/right D-pad levels accelerate adjustment. Native
PRES map states86/87 are the same held levels Replay2 already uses; normal menu
navigation keeps its native24/12-tick repeat. Numeric step sizes scale1/2/5 after
0/0.8/1.8seconds. Releasing, reversing, changing row/page or losing focus resets
acceleration. Unrelated menus and game simulation do not use this PC policy.

Focused production-module checks cover menu placement, removed rows, immediate
two-stick persistence and fresh-process reload, failed writes, independent values,
hold/release/reversal/row/focus behavior, and unchanged audio/gameplay preferences.
Actual in-game visual/presentation-path checks remain required before delivery.
