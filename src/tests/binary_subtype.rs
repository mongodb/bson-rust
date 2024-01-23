use crate::{spec::BinarySubtype, tests::LOCK};

#[test]
fn from_u8() {
    let _guard = LOCK.run_concurrently();
    // Check the endpoints of the defined, reserved, and user-defined subtype ranges.
    assert_eq!(BinarySubtype::from(0x00), BinarySubtype::Generic);
    assert_eq!(BinarySubtype::from(0x06), BinarySubtype::Encrypted);
    assert_eq!(BinarySubtype::from(0x07), BinarySubtype::Column);
    assert_eq!(BinarySubtype::from(0x08), BinarySubtype::Sensitive);
    assert_eq!(BinarySubtype::from(0x7F), BinarySubtype::Reserved(0x7F));
    assert_eq!(BinarySubtype::from(0x80), BinarySubtype::UserDefined(0x80));
    assert_eq!(BinarySubtype::from(0xFF), BinarySubtype::UserDefined(0xFF));
}
