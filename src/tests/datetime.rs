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

#[test]
fn invalid_rfc3339_to_datetime() {
    let a = "2020-06-09T10:58:07-095Z";
    let b = "2020-06-09T10:58:07.095";
    let c = "2020-06-09T10:62:07.095Z";
    assert!(crate::DateTime::from_rfc3339(a).is_err());
    assert!(crate::DateTime::from_rfc3339(b).is_err());
    assert!(crate::DateTime::from_rfc3339(c).is_err());
}
