use crate::{spec::BinarySubtype, tests::LOCK, Binary};

#[test]
fn binary_from_base64() {
    let _guard = LOCK.run_concurrently();

    let input = base64::encode("hello");
    let produced = Binary::from_base64(input, None).unwrap();
    let expected = Binary {
        bytes: "hello".as_bytes().to_vec(),
        subtype: BinarySubtype::Generic,
    };
    assert_eq!(produced, expected);

    let produced = Binary::from_base64("", BinarySubtype::Uuid).unwrap();
    let expected = Binary {
        bytes: "".as_bytes().to_vec(),
        subtype: BinarySubtype::Uuid,
    };
    assert_eq!(produced, expected);
}
