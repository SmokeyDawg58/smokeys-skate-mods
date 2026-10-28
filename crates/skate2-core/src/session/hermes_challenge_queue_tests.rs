use super::*;

#[test]
fn constructor_then_two_notifications_copy_source_headers_without_actor_handles() {
    let queue = ChallengeEventQueue::try_construct().unwrap();
    assert_eq!(queue.snapshot().unwrap(), (0, 0, Vec::new()));
    let broadcaster = ChallengeEventBroadcaster::construct(Arc::clone(&queue));
    broadcaster.send(SkaterNotification::Created, 4).unwrap();
    broadcaster
        .send(SkaterNotification::LocalChanged, -1)
        .unwrap();
    let (creation, deletion, entries) = queue.snapshot().unwrap();
    assert_eq!((creation, deletion), (96, 0));
    assert_eq!(entries.len(), 2);
    for entry in &entries {
        assert_eq!(entry.event_size, 48);
        assert!(entry.message_published);
        let message = entry.copied_message.unwrap();
        assert_eq!(message.size, 20);
        assert_eq!(message.id, 0);
        assert_eq!(
            message.port,
            skate2_formats::state_graph::state_graph_string_hash32(b"Challenge")
        );
    }
    assert_eq!(entries[0].copied_message.unwrap().payload,
        ChallengePayload::Skater { notification: SkaterNotification::Created, player_id: 4 });
    assert_eq!(
        entries[0].copied_message.unwrap().kind,
        (-340_057_085_i32).cast_unsigned()
    );
    assert_eq!(entries[1].copied_message.unwrap().payload,
        ChallengePayload::Skater { notification: SkaterNotification::LocalChanged, player_id: -1 });
    assert_eq!(entries[1].copied_message.unwrap().kind, 2_104_924_600);
}

#[test]
fn queue_owner_retains_earlier_producers_and_stops_at_native_diagnostic() {
    let queue = ChallengeEventQueue::try_construct().unwrap();
    let first = ChallengeEventBroadcaster::construct(Arc::clone(&queue));
    let second = ChallengeEventBroadcaster::construct(Arc::clone(&queue));
    for _ in 0..1365 {
        first.send(SkaterNotification::Created, 0).unwrap();
    }
    let before = queue.snapshot().unwrap();
    assert_eq!(before.0, 65_520);
    assert_eq!(
        second.send(SkaterNotification::LocalChanged, 0),
        Err(ChallengeQueueError::PendingOverflowDiagnostic)
    );
    assert_eq!(queue.snapshot().unwrap(), before);
}
