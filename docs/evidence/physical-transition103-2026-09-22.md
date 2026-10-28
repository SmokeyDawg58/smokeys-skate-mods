# Actual frame20 Air200 to GroundAnimation103

Original queued frame20 source selector returns103 after both modes completed0..19. CalcSuggestedState AirLanding selects103 only through actual Skeleton.IsSkateboardAnimated. No forced result is introduced.

SetPhysicsState82dff180: controller.EnterNewPhysicsState1b0 precedes statecount0/stateTimer0, processed timer0/laststate/lastcategory/next/current stores1c0..204. Then old state's Exit218. Existing separate Actor ID queries/telemetry are retained after Exit. Destination103 selects mPhysState_GroundAnimation at82dff394; shared Enter callffd1c. The packet expands the existing transition runner only for Air200?103, preserving Ground?200/300. No second transition runner or state allocation.

Air.Exit82d84190 clears actual mUseCentreOfMassVel at419c BEFORE SAME Reckoning.ResetBodySpin41a0. GroundAnimation's actual96byte instance already resides in RemainingStates. Constructor82d83dd0 only initializes JumpInfo, not jumpedThisFrame/jumpVel. Enter82d83e68 stores jumpedfalse3e90, all four jumpVel lanes positivezero3e94, SAME Skeleton trucktilttrue3e9c, existing angular-drive-only1.0 helper3ea8, SAME Wipeout.SetState Ground1 at3eb4. Host checks actual retained allocation/published flag and pair identity; any late drive/Wipeout failure retains earlier stores. No substitute state or force tuning.

Success publishes EntryComplete then original Update2 suffix clears requests once. Update3 retains actual count/prediction/environmental prefix and resolves bindings from that SAME RemainingStates allocation. Explicit StateUpdatePending103 occurs at state.Update boundary before Ground100 logic. This packet intentionally does not run PhysicsGround.Update for103 or claim a complete frame20.

Next implementation immediately required: Update82d98d68 clears jumped flag, actual fixed-heading Reckoning, existing Skeleton.UpdateAnimatedSkateboard(true), addSkateboardError, then UpdateSkateboard82d8a7b0. That method performs steering0, surface wheel material, actual CalcGroundJump and retained JumpInfo writes before jump/nonjump branching. Existing legacy math and live force/selector services must be reused; no assumed false jump. Exit103 and Fill103 remain distinct later lifecycle work.

Focused dispatch regression admits only the three audited predecessor/destination pairs. Independent source review and root original-data gate verify concrete owner/store chain; author runs no Cargo. New file physical_player_ground_animation_entry.rs plus shared artifact patch; root integrates and tests.
