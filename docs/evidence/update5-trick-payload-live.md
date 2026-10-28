# Update5 retained trick payload after current-state Fill

The runtime captures a single animation input Arc after each pair's guard, at native `5c44` and separately `5c80`. Each scalar is read later from that captured owner; replacing the assembly input cannot redirect the second scalar. Late read failures preserve earlier writes.

Raw source `82e05b78..82e05ccc`, after virtual Fill `5b74`:

- Guard `5b7c` precedes reading output Grinds substate `+8c` at `5ba0`. Compare to **1 or 2**; registers r25/r26 are 1/0 (`539c/53a0`). A fresh guard `5bc0` precedes the `is_grinding+138` store `5be4`. Read the actual retained output even with the explicit first-test GrindManager exclusion; never synthesize substate zero.
- Guard `5bec` precedes Trick component resolution and input `AnimOutPhysIn+2894` copy at `5c1c`. Assignment `825f6f80` copies thirteen words then two floats (`+34/+38`). Source and output already share the existing `AnimOutTrickData` representation; its typed clone preserves payload bits. No conversion or replacement trick schema is needed.
- Guard `5c24` precedes input `+28d0` gesture enum to Trick `+6c` (`5c50`), then input `+28d4` gesturing to output `+70` (`5c58`). IDA types confirm gesture first, boolean second. The semantic boolean representation omits native padding; enum bits are reinterpreted without saturation.
- Guard `5c60` precedes input wipeout X/Y `+28d8/+28dc` to output `+74/+78` (`5c8c/5c94`). Copies preserve float bits; the first write survives a failed second source read.
- Processed flags `+8d0` are read `5ca0` before guard `5ca8`. Mask `00004000` publishes Trick `is_push_tricking+7d` at `5ccc`.

Runtime uses the retained assembly `animation_physics_input`, existing constructor-produced trick/gesture/wipeout fields in `actor_anim_out_phys_in.rs`, and retained Processed physics flags. It requires explicit `GrindManagerFillPolicy`; native policy remains pending until the prior native callback exists. Exclusion preserves and consumes the existing Grinds substate without rewriting it.

Register output child `construct::update5_trick_payload` and crate module `physical_player_update5_trick_payload`; call runtime `try_publish_update5_trick_payload(owner, grind_policy)` immediately after Ground.Fill and before the existing selected conditioner. This bounded span does not complete the later Update5 suffix.

Four tests cover actual substate values 0/1/2/4/-1, payload bit preservation and source order, partial copies on late source failure, and locked-output lazy failure. Standalone formatting only; root integrator owns Cargo gates and shared registration.
