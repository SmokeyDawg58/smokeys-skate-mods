//! Actual Actor constructor corpus continuation through body Dispatch and Sync.
use super::*;
use crate::{
    actor_animation_construction::initialization::bind_pose::BindPoseCommand,
    actor_graph_construction::ActorGraphIntents,
};

pub(crate) fn check(
    actor: &ActorConstruction,
    startup: &SimulationAnimationStartup,
    cvars: &InputCvarReferences,
) {
    let mut services = ActorBodyAnimationServices::try_from_startup(startup, cvars).unwrap();
    let retained = actor.retained_animation().unwrap().unwrap();
    let low = retained.lock().unwrap().player().unwrap().flags_bits & 0x0800_0000 != 0;
    let player = actor.prefix().unwrap().fields().unwrap().player_index;
    let key = (
        ActorGraphIntents::native_name(b"Skater"),
        i32::from_ne_bytes(player.to_ne_bytes()),
    );
    let commands = startup.managers.command.lock().unwrap().entries[&key].clone();
    let buffered = commands.lock().unwrap().buffered.clone().unwrap();
    for iteration in 0..2 {
        actor.try_dispatch_body_frame(&mut services).unwrap();
        {
            let animation = retained.lock().unwrap();
            let state = animation.initialization().unwrap();
            assert!(state.local_matrices.matrices().unwrap().is_none());
            assert!(state.global_matrices.matrices().unwrap().is_none());
        }
        {
            let pending = buffered.lock().unwrap();
            assert_eq!(pending.initiated_commands, 1);
            assert_eq!(
                pending.last_body_cull_threshold.unwrap().to_bits(),
                cvars
                    .try_skater_blend_cull_threshold(low)
                    .unwrap()
                    .to_bits()
            );
            assert!(matches!(
                pending.last_submitted.as_slice(),
                [
                    BindPoseCommand::FetchPose { .. },
                    BindPoseCommand::SqtToLocal { sqt: 0, matrix: 0 },
                    BindPoseCommand::ExtractMatrix { matrix: 0 },
                    BindPoseCommand::LocalToGlobal { matrix: 0 },
                    BindPoseCommand::ExtractMatrix { matrix: 0 },
                ]
            ));
        }
        if iteration == 0 {
            // Equal registry key is insufficient: another real player's ACS
            // must not replace this Actor's in-flight body command owner.
            let foreign_key = (key.0, i32::from(key.1 == 0));
            let foreign = startup.managers.command.lock().unwrap().entries[&foreign_key].clone();
            startup
                .managers
                .command
                .lock()
                .unwrap()
                .entries
                .insert(key, foreign);
            let result = actor.try_sync_body_frame(&mut services);
            startup
                .managers
                .command
                .lock()
                .unwrap()
                .entries
                .insert(key, Arc::clone(&commands));
            assert!(matches!(
                result,
                Err(BodyDispatchError::CommandOwnerMismatch)
            ));
            assert_eq!(buffered.lock().unwrap().initiated_commands, 1);
        }
        actor.try_sync_body_frame(&mut services).unwrap();
        let animation = retained.lock().unwrap();
        let state = animation.initialization().unwrap();
        assert_eq!(state.local_matrices.matrices().unwrap().unwrap().len(), 36);
        assert_eq!(state.global_matrices.matrices().unwrap().unwrap().len(), 36);
        assert_eq!(buffered.lock().unwrap().initiated_commands, 0);
    }
    eprintln!(
        "SAME published Actor -> runtime named ACS + live blend threshold -> retained local/global dispatch buffers -> separate same-owner Sync twice; GetPhysUpdateData/SetUpPhysics remain pending"
    );
}
