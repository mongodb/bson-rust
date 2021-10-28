use std::str::FromStr;

#[test]
fn rfc3339_to_datetime() {
    let rfc = "2020-06-09T10:58:07.095Z";
    let date = chrono::DateTime::<chrono::Utc>::from_str(rfc).unwrap();
    assert_eq!(
        crate::DateTime::from_rfc3339(rfc).unwrap(),
        crate::DateTime::from_chrono(date)
    );
}
