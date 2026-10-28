use super::*;

fn attribute(key: &str, text: &str, value: f32, boolean_raw: u8) -> StateGraphBinaryAttribute {
    StateGraphBinaryAttribute {
        key: key.as_bytes().to_vec(),
        key_hash: state_graph_string_hash32(key.as_bytes()),
        text: text.as_bytes().to_vec(),
        float_bits: value.to_bits(),
        boolean_raw,
    }
}

#[test]
fn binary_lookup_retains_typed_values_and_first_hash_match() {
    let mut first = attribute("x", "999", -0.0, 2);
    first.key = b"diagnostic name is not identity".to_vec();
    let values = [first, attribute("x", "later", 40.0, 0)];
    let lookup = BinaryAttributes(&values);
    assert_eq!(
        lookup.required_float("x").unwrap().to_bits(),
        (-0.0_f32).to_bits()
    );
    assert!(lookup.boolean("x", false).unwrap());
    assert!(!lookup.boolean("missing", false).unwrap());
    assert_eq!(lookup.get("x"), Some("999"));
}

#[test]
fn binary_print_text_uses_shared_constructor_with_serialized_numbers() {
    let values = [
        attribute("x", "not a number", 3.75, 0),
        attribute("y", "999", -2.25, 0),
        attribute("numFrames", "999", 7.0, 0),
        attribute("text", "camera", 0.0, 0),
    ];
    let BehaviourPayload::PrintText { x, y, frames, text } =
        print_text_from_lookup(&BinaryAttributes(&values)).unwrap()
    else {
        panic!("wrong constructor payload");
    };
    assert_eq!((x, y, frames), (3, -2, 7));
    assert_eq!(&text[..7], b"camera\0");
}
