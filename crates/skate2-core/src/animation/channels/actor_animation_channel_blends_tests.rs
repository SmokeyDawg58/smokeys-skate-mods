//! Original-asset regression, child `channel_runtime::blend_tests`.
//! Invoke after `check_initialized_actor` on its disposable post-teardown Actor.
//! This creates actual trees/channels and must not run on a continuing skater.
use super::*;

#[expect(
    clippy::too_many_lines,
    reason = "one disposable original Actor validates successive real channel allocation and publication"
)]
pub(crate) fn check_disposable_initialized_actor(
    actor: &crate::actor_construction::ActorConstruction,
) {
    let retained = actor.retained_animation().unwrap().unwrap();
    let mut animation = retained.lock().unwrap();
    let snapshot =
        |animation: &crate::actor_construction::animation_mount::MountedActorAnimation| {
            let channels = animation
                .initialization
                .as_ref()
                .unwrap()
                .channels
                .as_ref()
                .unwrap();
            (
                channels.free_channels.clone(),
                channels.free_trees.clone(),
                channels.dictionary.len(),
                channels.root,
            )
        };
    let before = snapshot(&animation);
    assert_eq!(
        before.0.len(),
        4,
        "run after existing two-channel disposable fixture"
    );
    let next_channel = *before.0.last().unwrap();
    let next_tree = *before.1.last().unwrap();
    let missing = FastString36::encode("__MISSING_BLEND_REGRESSION__").unwrap();
    assert_eq!(
        animation
            .try_new_channel_with_blends(missing, missing, 0.125, 0.625, true)
            .unwrap(),
        NewGestureChannelStatus::AnimationMissing
    );
    assert_eq!(snapshot(&animation), before);
    // Native ec44 writes this prefix before GetAnimTree returns NULL; no rollback.
    assert_eq!(
        animation
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap()
            .channels[next_channel]
            .channel_tree,
        Some(next_tree)
    );
    assert!(
        !animation
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap()
            .dictionary
            .contains_key(&missing)
    );

    let clip = FastString36::encode("B_GSTR_SKY_BOTH").unwrap();
    for (ordinal, label, input, output, last, expected_flags) in [
        (0, "BlendPositive", 0.125_f32, 0.625_f32, true, 0x4800_0000),
        (1, "BlendZero", -0.0_f32, -0.0_f32, true, 0x4000_0000),
        (2, "BlendNegative", -0.375_f32, -0.25_f32, false, 0),
        (
            3,
            "BlendUnordered",
            0.875_f32,
            f32::from_bits(0x7fc0_4321),
            true,
            0x4000_0000,
        ),
    ] {
        let name = FastString36::encode(label).unwrap();
        let prior = snapshot(&animation);
        let expected_index = *prior.0.last().unwrap();
        let expected_tree = *prior.1.last().unwrap();
        assert_eq!(
            animation
                .try_new_channel_with_blends(name, clip, input, output, last)
                .unwrap(),
            NewGestureChannelStatus::Created
        );
        let channels = animation
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap();
        assert_eq!(channels.free_channels.len(), 3 - ordinal);
        assert_eq!(channels.free_trees.len(), prior.1.len() - 1);
        assert_eq!(channels.dictionary.len(), prior.2 + 1);
        assert_eq!(channels.root, Some(expected_index));
        let index = *channels.dictionary.get(&name).unwrap();
        assert_eq!(index, expected_index);
        let channel = &channels.channels[index];
        assert_eq!(channel.channel_tree, Some(expected_tree));
        assert_eq!(channel.blend_in_length.unwrap().to_bits(), input.to_bits());
        assert_eq!(
            channel.blend_out_length.unwrap().to_bits(),
            output.to_bits()
        );
        assert_eq!(channel.channel_flags, Some(expected_flags));
        assert_eq!(channel.base, prior.3);
        assert_eq!(channels.channels[prior.3.unwrap()].parent, Some(index));
        let runtime = channel.runtime.as_ref().unwrap();
        assert_eq!(runtime.blend_out_last_frame, last);
        assert_eq!(runtime.blending_out, ordinal == 0);
        assert!(
            !runtime.cycle
                && !runtime.blend_in_first_frame
                && !runtime.blend_out_early
                && !runtime.expired
                && !runtime.use_attributes
        );
        assert!(
            matches!(runtime.base, ChannelBaseReference::Channel(parent) if Some(parent) == prior.3)
        );
        let pool = &channels.trees[expected_tree].runtime;
        assert!(!pool.node(&runtime.animation).unwrap().evaluated);
        for actual in [
            channel.blend_coefficient,
            channel.blend_in_time,
            channel.blend_out_time,
            channel.resurrect_time,
            channel.resurrect_length,
        ] {
            assert_eq!(actual.unwrap().to_bits(), 0);
        }
        assert_eq!(channel.influence, Some(1.0));
        assert_eq!(channel.priority, 0);
        let created = snapshot(&animation);
        // Existing-name short circuit precedes allocation and must not replace
        // parameters, even with different lengths and an absent animation name.
        assert_eq!(
            animation
                .try_new_channel_with_blends(name, missing, 9.0, 8.0, !last)
                .unwrap(),
            NewGestureChannelStatus::AlreadyPresent
        );
        assert_eq!(snapshot(&animation), created);
        let channel = &animation
            .initialization
            .as_ref()
            .unwrap()
            .channels
            .as_ref()
            .unwrap()
            .channels[index];
        assert_eq!(channel.blend_in_length.unwrap().to_bits(), input.to_bits());
        assert_eq!(
            channel.blend_out_length.unwrap().to_bits(),
            output.to_bits()
        );
        assert_eq!(channel.channel_flags, Some(expected_flags));
    }
    let full = snapshot(&animation);
    assert_eq!(
        animation
            .try_new_channel_with_blends(missing, clip, 0.1, 0.1, false)
            .unwrap(),
        NewGestureChannelStatus::NoFreeChannel
    );
    assert_eq!(snapshot(&animation), full);
    animation.check_remove_channels_corpus();
    animation.check_fakie_channel_transition_corpus();
    {
        let playback = animation
            .initialization
            .as_ref()
            .unwrap()
            .playback
            .as_ref()
            .unwrap()
            .lock()
            .unwrap();
        let map = animation
            .base
            .as_ref()
            .unwrap()
            .construction_map
            .occupied_slots()
            .iter()
            .map(|r| r.unwrap())
            .collect::<Vec<_>>();
        crate::actor_animation_tree::sequence::tests::check_authored_run_sequence_corpus(
            &playback, &map,
        );
    }
    // Reuse the returned actual slots. Expiry must follow native dictionary
    // order, and the last surviving root must be reread before AddTime.
    let names = [
        "EXPIRE_A", "EXPIRE_B", "EXPIRE_C", "EXPIRE_D", "EXPIRE_E", "EXPIRE_F",
    ]
    .map(|name| FastString36::encode(name).unwrap());
    for name in names {
        assert_eq!(
            animation
                .try_new_channel_with_blends(name, clip, 0.0, 0.0, false)
                .unwrap(),
            NewGestureChannelStatus::Created
        );
    }
    for (i, name) in names.iter().enumerate() {
        let length = if i == 0 {
            f32::NAN
        } else if i == 5 {
            0.125
        } else {
            0.0
        };
        animation.try_end_body_spin_channel(name, length).unwrap();
    }
    let channels = animation
        .initialization
        .as_ref()
        .unwrap()
        .channels
        .as_ref()
        .unwrap();
    let survivor = channels.dictionary[&names[5]];
    let expected_free = channels
        .dictionary
        .iter()
        .filter(|(name, _)| **name != names[5])
        .map(|(_, index)| *index)
        .collect::<Vec<_>>();
    animation.try_add_animation_time(0.0).unwrap();
    let channels = animation
        .initialization
        .as_ref()
        .unwrap()
        .channels
        .as_ref()
        .unwrap();
    assert_eq!(channels.free_channels, expected_free);
    assert_eq!(channels.dictionary.len(), 1);
    assert_eq!(channels.root, Some(survivor));
    animation.try_end_body_spin_channel(&names[5], 0.0).unwrap();
    // DisallowPumping uses the same native overload with current-frame=false.
    // Replacing a preceding true bit must clear it, retain early blend flags,
    // and carry the authored duration into the actual channel owner.
    animation.try_end_channel_with_blend(&names[5], 0.1, false).unwrap();
    {
        let channels = animation.initialization.as_ref().unwrap().channels.as_ref().unwrap();
        let channel = &channels.channels[survivor];
        assert_eq!(channel.channel_flags.unwrap() & 0x6800_0000, 0x2800_0000);
        assert_eq!(channel.blend_out_length, Some(0.1));
        assert!(!channel.runtime.as_ref().unwrap().blend_out_last_frame);
    }
    animation.try_end_body_spin_channel(&names[5], 0.0).unwrap();
    animation.try_add_animation_time(0.0).unwrap();
    let channels = animation
        .initialization
        .as_ref()
        .unwrap()
        .channels
        .as_ref()
        .unwrap();
    assert!(channels.dictionary.is_empty());
    assert_eq!(channels.root, None);
    assert_eq!(channels.free_channels.last(), Some(&survivor));
}
