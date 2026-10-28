//! Private kernels used by actual list creation. Controlled XYZ values/callbacks
//! discriminate source ordering without constructing a Body, Spy, or frame.
use super::*;

#[test]
fn both_sides_independently_read_other_then_this_in_source_order() {
    let mut events = Vec::new();
    let values = [
        [1.0, 2.0, 3.0],
        [4.0, 8.0, 16.0],
        [9.0, 8.0, 7.0],
        [3.0, 5.0, 10.0],
    ];
    let mut read = |endpoint| {
        let index = events.len();
        events.push(endpoint);
        Ok::<_, ()>(values[index])
    };
    let a = contact_relative_velocity(true, &mut read).unwrap();
    let b = contact_relative_velocity(false, &mut read).unwrap();
    assert_eq!(
        events,
        [
            ContactEndpoint::B,
            ContactEndpoint::A,
            ContactEndpoint::A,
            ContactEndpoint::B
        ]
    );
    assert_eq!(a.map(f32::to_bits), [3.0_f32, 6.0, 13.0].map(f32::to_bits));
    assert_eq!(b.map(f32::to_bits), [-6.0_f32, -3.0, 3.0].map(f32::to_bits));
}

#[test]
fn source_signed_multiply_and_subtraction_preserve_zero_discriminators() {
    let right = [0.0, -0.0, 2.0];
    assert_eq!(
        contact_normal(right, true).map(f32::to_bits),
        [0, 0x8000_0000, 0x4000_0000]
    );
    assert_eq!(
        contact_normal(right, false).map(f32::to_bits),
        [0x8000_0000, 0, 0xc000_0000]
    );
    let relative = contact_relative_velocity(true, |endpoint| {
        Ok::<_, ()>(match endpoint {
            ContactEndpoint::B => [0.0, -0.0, 3.0],
            ContactEndpoint::A => [-0.0, 0.0, 1.0],
        })
    })
    .unwrap();
    assert_eq!(relative.map(f32::to_bits), [0x8000_0000, 0, 0xc000_0000]);
}

#[test]
fn full_vector_cap_precedes_all_reads_and_preserves_contents() {
    let mut contacts = vec![0x1234_u32; 16];
    let before = contacts.clone();
    let capacity = contacts.capacity();
    assert_eq!(
        try_push_capped(
            &mut contacts,
            || (),
            || -> Result<u32, ()> { panic!("capped side must not read velocity or build output") }
        ),
        Ok(false)
    );
    assert_eq!(contacts, before);
    assert_eq!(contacts.capacity(), capacity);
}

#[test]
fn shared_vector_length_fifteen_accepts_only_a_before_b_cap() {
    let mut contacts = vec![[0_u32; 3]; 15];
    let mut events = Vec::new();
    assert_eq!(
        try_push_capped(
            &mut contacts,
            || (),
            || {
                contact_relative_velocity(true, |endpoint| {
                    events.push(endpoint);
                    Ok::<_, ()>(match endpoint {
                        ContactEndpoint::B => [1.0; 3],
                        ContactEndpoint::A => [4.0; 3],
                    })
                })
                .map(|value| value.map(f32::to_bits))
            }
        ),
        Ok(true)
    );
    assert_eq!(
        try_push_capped(
            &mut contacts,
            || (),
            || {
                contact_relative_velocity(false, |_| -> Result<[f32; 3], ()> {
                    panic!("B cap before either reread")
                })
                .map(|value| value.map(f32::to_bits))
            }
        ),
        Ok(false)
    );
    assert_eq!(events, [ContactEndpoint::B, ContactEndpoint::A]);
    assert_eq!(contacts.len(), 16);
    assert_eq!(contacts[15], [3.0_f32.to_bits(); 3]);
}

#[test]
fn later_side_read_error_keeps_earlier_append_but_never_appends_partial_side() {
    let mut contacts = Vec::new();
    let mut events = Vec::new();
    assert_eq!(
        try_push_capped(
            &mut contacts,
            || "allocation",
            || {
                contact_relative_velocity(true, |endpoint| {
                    events.push(endpoint);
                    Ok::<_, &str>(match endpoint {
                        ContactEndpoint::B => [2.0; 3],
                        ContactEndpoint::A => [5.0; 3],
                    })
                })
                .map(|value| value.map(f32::to_bits))
            }
        ),
        Ok(true)
    );
    assert_eq!(
        try_push_capped(
            &mut contacts,
            || "allocation",
            || {
                contact_relative_velocity(false, |endpoint| {
                    events.push(endpoint);
                    match endpoint {
                        ContactEndpoint::A => Ok([7.0; 3]),
                        ContactEndpoint::B => Err("second read"),
                    }
                })
                .map(|value| value.map(f32::to_bits))
            }
        ),
        Err("second read")
    );
    assert_eq!(
        events,
        [
            ContactEndpoint::B,
            ContactEndpoint::A,
            ContactEndpoint::A,
            ContactEndpoint::B
        ]
    );
    assert_eq!(contacts, [[3.0_f32.to_bits(); 3]]);
}

#[test]
fn first_read_error_does_not_sample_second_endpoint_or_append() {
    for side in [true, false] {
        let mut contacts: Vec<[f32; 3]> = Vec::new();
        let mut events = Vec::new();
        assert_eq!(
            try_push_capped(
                &mut contacts,
                || 0,
                || {
                    contact_relative_velocity(side, |endpoint| {
                        events.push(endpoint);
                        Err::<[f32; 3], _>(17)
                    })
                }
            ),
            Err(17)
        );
        assert_eq!(
            events,
            [if side {
                ContactEndpoint::B
            } else {
                ContactEndpoint::A
            }]
        );
        assert!(contacts.is_empty());
    }
}
