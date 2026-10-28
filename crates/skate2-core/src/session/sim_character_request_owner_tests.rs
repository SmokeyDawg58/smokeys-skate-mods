use super::*;
#[test]
fn ambient_load_and_unload_share_player_request_slots_and_sequence() {
    let mut requests = SimCharacterRequests::construct_fields();
    let callback = Arc::new(());
    let local = requests.begin_load(Arc::new(()), 0).unwrap();
    let ambient = requests.begin_ambient_load(Arc::clone(&callback)).unwrap();
    let unload = requests.begin_unload(callback).unwrap();
    assert_eq!((local.id(), ambient.id(), unload.id()), (0, 1, 2));
    assert_eq!(requests.pending.iter().flatten().count(), 3);
    assert_eq!(requests.load_slot(&unload.0), Err(RequestError::WrongKind));
    requests.finish_unloaded_callback(&unload).unwrap();
    assert!(requests.pending[2].is_none());
    assert!(requests.load_slot(&ambient).is_ok());
    assert!(requests.load_slot(&local).is_ok());
}
#[test]
fn callback_allocates_registration_before_releasing_load_and_skips_sentinel() {
    let mut requests = SimCharacterRequests::construct_fields();
    let callback = Arc::new(());
    let load = requests.begin_load(Arc::clone(&callback), 3).unwrap();
    assert_eq!(load.id(), 0);
    assert_eq!(requests.begin_registration(&load).unwrap(), 1);
    assert_eq!(requests.pending.iter().flatten().count(), 2);
    requests.finish_loaded_callback(&load).unwrap();
    assert!(requests.pending[0].is_none());
    assert_eq!(requests.pending[1].as_ref().unwrap().kind, Kind::Register);
    requests.next = u32::MAX - 1;
    assert_eq!(
        requests.begin_load(Arc::clone(&callback), 3).unwrap().id(),
        u32::MAX - 1
    );
    assert_eq!(requests.begin_load(callback, 3).unwrap().id(), 0);
}
#[test]
fn capacity_and_foreign_callback_preserve_pending_load() {
    let mut requests = SimCharacterRequests::construct_fields();
    let load = requests.begin_load(Arc::new(()), 0).unwrap();
    let foreign = LoadTicket {
        id: load.id,
        callback: Arc::new(()),
        player: 0,
    };
    assert_eq!(
        requests.begin_registration(&foreign),
        Err(RequestError::ForeignCallback)
    );
    for _ in 1..256 {
        requests.begin_load(Arc::new(()), 0).unwrap();
    }
    assert_eq!(
        requests.begin_registration(&load),
        Err(RequestError::Capacity)
    );
    assert!(requests.load_slot(&load).is_ok());
}
