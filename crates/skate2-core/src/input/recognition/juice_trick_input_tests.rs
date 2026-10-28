use super::*;
struct Reader {
    bytes: Option<Vec<u8>>,
    status: bool,
    fail: bool,
}
impl Reader {
    fn name(name: &[u8]) -> Self {
        Self {
            bytes: Some(name.to_vec()),
            status: true,
            fail: false,
        }
    }
}
impl JuiceTrickReader for Reader {
    type Error = &'static str;
    fn get_string(&mut self) -> Result<JuiceInputString, Self::Error> {
        if self.fail {
            Err("reader")
        } else {
            Ok(JuiceInputString {
                succeeded: self.status,
                bytes: self.bytes.clone(),
            })
        }
    }
}
#[test]
fn static_zero_state_and_shared_handles_publish_and_consume_actual_request() {
    let owner = JuiceTrickInput::new_static_initialized();
    let mut same = owner.clone();
    assert!(!owner.try_is_trick_active().unwrap());
    assert_eq!(owner.state.lock().unwrap().name, [0; 64]);
    owner.try_do_trick(&mut Reader::name(b"Kickflip")).unwrap();
    assert!(same.is_trick_active().unwrap());
    assert_eq!(same.take_trick_name().unwrap(), "Kickflip");
    assert!(!owner.try_is_trick_active().unwrap());
    // GetTrickName never erases the name and does not require active=true.
    assert_eq!(same.take_trick_name().unwrap(), "Kickflip");
}
#[test]
fn c_string_copy_caps_at_63_and_preserves_trailing_storage() {
    let owner = JuiceTrickInput::new_static_initialized();
    owner.try_do_trick(&mut Reader::name(&[b'x'; 90])).unwrap();
    let mut destination = [0xa5; 64];
    assert_eq!(owner.try_get_trick_name(&mut destination).unwrap(), 63);
    assert_eq!(&destination[..63], &[b'x'; 63]);
    assert_eq!(destination[63], 0);
    owner
        .try_do_trick(&mut Reader::name(b"A\0ignored"))
        .unwrap();
    let name = owner.state.lock().unwrap().name;
    assert_eq!(&name[..3], &[b'A', 0, b'x']);
    let mut destination = [0xa5; 64];
    assert_eq!(owner.try_get_trick_name(&mut destination).unwrap(), 1);
    assert_eq!(&destination[..3], &[b'A', 0, 0xa5]);
}
#[test]
fn reader_status_is_ignored_but_null_and_failure_do_not_mutate_retained_state() {
    let owner = JuiceTrickInput::new_static_initialized();
    let mut reader = Reader::name(b"Ollie");
    reader.status = false;
    owner.try_do_trick(&mut reader).unwrap();
    assert!(owner.try_is_trick_active().unwrap());
    reader.bytes = None;
    assert_eq!(
        owner.try_do_trick(&mut reader),
        Err(JuiceTrickRequestError::State(
            JuiceTrickInputError::MissingString
        ))
    );
    reader.fail = true;
    assert_eq!(
        owner.try_do_trick(&mut reader),
        Err(JuiceTrickRequestError::Reader("reader"))
    );
    assert!(owner.try_is_trick_active().unwrap());
    let mut same = owner.clone();
    assert_eq!(same.take_trick_name().unwrap(), "Ollie");
}
#[test]
fn unsupported_utf8_refuses_only_after_original_name_copy_and_latch_clear() {
    let mut owner = JuiceTrickInput::new_static_initialized();
    owner
        .try_do_trick(&mut Reader::name(&[0xff, b'X']))
        .unwrap();
    assert_eq!(
        owner.take_trick_name(),
        Err(JuiceTrickInputError::UnsupportedNameEncoding)
    );
    assert!(!owner.try_is_trick_active().unwrap());
    assert_eq!(&owner.state.lock().unwrap().name[..3], &[0xff, b'X', 0]);
}
