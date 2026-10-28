# Update5 animation stores after trick payload

Bounded actual interval `82e05cd0..60cc`; existing output/Processed/animation owners only.

Initial carving guard is sampled `5cd8` before Processed flags `5cdc`, then asserted before store `5d04`. Mask `80000000`. Guards precede turn980 and spin97c reads/stores `5d38/5d6c`.

COM sequence preserves guards `5d74`, `5d9c`, sampled `5dc8` before vectors/subtraction and asserted `5dd8`, then dot `5e08` before final guard `5e0c` and store `5e38`. Raw `17ee787d` decodes VD127/VA46/VB47; IDA's v78 is incorrect. v46 is output SystemReckoning COM, v47 is output SkateboardReckoning transform position (not effective transform). Raw `159f6db5` dots their difference with output SystemReckoning up vector. Existing numeric service supplies dot3; W is unused and never fabricated. Xenon exceptional reduction behavior remains that service's documented host boundary.

Guarded animation-input booleans mirrored2882, switch2883, fakie2884, regular2886 publish in that order. Fresh animation locomotion2aa0 mask04000000 precedes guard and OffBoard jumping. Processed8cc mask00100000 precedes standing-jump guard. Byte load8c8 bit0 means **whole big-endian word mask01000000**, publishing no-input. Guard precedes input-controls time_since_last_input9bc. Remaining fresh flags precede guards: 8c4 mask08000000 generally pushing; 8d0 mask20 transitioning; 8d4 mask40000000 offboard; 8d0 mask4 running out.

No new field producers are required. Existing retained masks must be known; no default flags are substituted. Animation input pointer is freshly captured for each native pointer read; each callback reads one scalar. Processed owner identity is retained by the same Reckoning owner.

Register `construct::update5_animation`, crate `physical_player_update5_animation`, then call runtime `try_publish_update5_animation(owner)` after trick payload before selected conditioner. Later push-edge/intents/surface/completion suffix remains outside this span. Three tests cover actual transform position, unknown W, source-order partial publication, and locked carving sampling. No Cargo run by drafting agent.
Root integration: registered under assembly::update5_animation and construct::update5_animation. Root corrected fresh Processed capture before carving guard, and turn/spin use actual input_controls.turn980 and jump_events.spin97c, not body_spin960. Full tail now continues through push, surfaces and finish. Accepted workspace67/original58/Clippy115; see current checkpoint.
