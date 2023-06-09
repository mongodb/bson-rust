use std::time::Duration;

use crate::tests::LOCK;

#[test]
fn rfc3339_to_datetime() {
    let _guard = LOCK.run_concurrently();

    let rfc = "2020-06-09T10:58:07.095Z";
    let date =
        time::OffsetDateTime::parse(rfc, &time::format_description::well_known::Rfc3339).unwrap();
    let parsed = crate::DateTime::parse_rfc3339_str(rfc).unwrap();
    assert_eq!(parsed, crate::DateTime::from_time_0_3(date));
    assert_eq!(crate::DateTime::try_to_rfc3339_string(parsed).unwrap(), rfc);
}

#[test]
fn invalid_rfc3339_to_datetime() {
    let _guard = LOCK.run_concurrently();

    let a = "2020-06-09T10:58:07-095Z";
    let b = "2020-06-09T10:58:07.095";
    let c = "2020-06-09T10:62:07.095Z";
    assert!(crate::DateTime::parse_rfc3339_str(a).is_err());
    assert!(crate::DateTime::parse_rfc3339_str(b).is_err());
    assert!(crate::DateTime::parse_rfc3339_str(c).is_err());
}

#[test]
fn datetime_to_rfc3339() {
    assert_eq!(
        crate::DateTime::from_millis(0)
            .try_to_rfc3339_string()
            .unwrap(),
        "1970-01-01T00:00:00Z"
    );
}

#[test]
fn invalid_datetime_to_rfc3339() {
    assert!(crate::DateTime::MAX.try_to_rfc3339_string().is_err());
}

#[test]
fn duration_since() {
    let _guard = LOCK.run_concurrently();

    let date1 = crate::DateTime::from_millis(100);
    let date2 = crate::DateTime::from_millis(1000);

    assert_eq!(
        date2.checked_duration_since(date1),
        Some(Duration::from_millis(900))
    );
    assert_eq!(
        date2.saturating_duration_since(date1),
        Duration::from_millis(900)
    );
    assert!(date1.checked_duration_since(date2).is_none());
    assert_eq!(date1.saturating_duration_since(date2), Duration::ZERO);
}
