use std::{
    convert::TryFrom,
    time::{Duration, SystemTime},
};

use crate::{
    doc,
    oid::ObjectId,
    spec::BinarySubtype,
    tests::LOCK,
    Binary,
    Bson,
    DateTime,
    Document,
    JavaScriptCodeWithScope,
    Regex,
    Timestamp,
};
use serde_json::{json, Value};

#[test]
fn to_json() {
    let _guard = LOCK.run_concurrently();
    let mut doc = Document::new();
    doc.insert(
        "_id",
        Bson::ObjectId(ObjectId::from_bytes(*b"abcdefghijkl")),
    );
    doc.insert("first", Bson::Int32(1));
    doc.insert("second", Bson::String("foo".to_owned()));
    doc.insert("alphanumeric", Bson::String("bar".to_owned()));
    let data: Value = Bson::Document(doc).into();

    assert!(data.is_object());
    let obj = data.as_object().unwrap();

    let id = obj.get("_id").unwrap();
    assert!(id.is_object());
    let id_val = id.get("$oid").unwrap();
    assert!(id_val.is_string());
    assert_eq!(id_val, "6162636465666768696a6b6c");

    let first = obj.get("first").unwrap();
    assert!(first.is_number());
    assert_eq!(first.as_i64().unwrap(), 1);

    let second = obj.get("second").unwrap();
    assert!(second.is_string());
    assert_eq!(second.as_str().unwrap(), "foo");

    let alphanumeric = obj.get("alphanumeric").unwrap();
    assert!(alphanumeric.is_string());
    assert_eq!(alphanumeric.as_str().unwrap(), "bar");
}

#[test]
fn bson_default() {
    let _guard = LOCK.run_concurrently();
    let bson1 = Bson::default();
    assert_eq!(bson1, Bson::Null);
}

#[test]
fn document_default() {
    let _guard = LOCK.run_concurrently();
    let doc1 = Document::default();
    assert_eq!(doc1.keys().count(), 0);
    assert_eq!(doc1, Document::new());
}

#[test]
fn from_impls() {
    let _guard = LOCK.run_concurrently();
    assert_eq!(Bson::from(1.5f32), Bson::Double(1.5));
    assert_eq!(Bson::from(2.25f64), Bson::Double(2.25));
    assert_eq!(Bson::from("data"), Bson::String(String::from("data")));
    assert_eq!(
        Bson::from(String::from("data")),
        Bson::String(String::from("data"))
    );
    assert_eq!(Bson::from(doc! {}), Bson::Document(Document::new()));
    assert_eq!(Bson::from(false), Bson::Boolean(false));
    assert_eq!(
        Bson::from(Regex {
            pattern: String::from("\\s+$"),
            options: String::from("i")
        }),
        Bson::RegularExpression(Regex {
            pattern: String::from("\\s+$"),
            options: String::from("i")
        })
    );
    assert_eq!(
        Bson::from(JavaScriptCodeWithScope {
            code: String::from("alert(\"hi\");"),
            scope: doc! {}
        }),
        Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
            code: String::from("alert(\"hi\");"),
            scope: doc! {}
        })
    );
    //
    assert_eq!(
        Bson::from(Binary {
            subtype: BinarySubtype::Generic,
            bytes: vec![1, 2, 3]
        }),
        Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: vec![1, 2, 3]
        })
    );
    assert_eq!(Bson::from(-48i32), Bson::Int32(-48));
    assert_eq!(Bson::from(-96i64), Bson::Int64(-96));
    assert_eq!(Bson::from(152u32), Bson::Int32(152));
    assert_eq!(Bson::from(4096u64), Bson::Int64(4096));

    let oid = ObjectId::new();
    assert_eq!(
        Bson::from(b"abcdefghijkl"),
        Bson::ObjectId(ObjectId::from_bytes(*b"abcdefghijkl"))
    );
    assert_eq!(Bson::from(oid), Bson::ObjectId(oid));
    assert_eq!(
        Bson::from(vec![1, 2, 3]),
        Bson::Array(vec![Bson::Int32(1), Bson::Int32(2), Bson::Int32(3)])
    );
    assert_eq!(
        Bson::try_from(json!({"_id": {"$oid": oid.to_hex()}, "name": ["bson-rs"]})).unwrap(),
        Bson::Document(doc! {"_id": &oid, "name": ["bson-rs"]})
    );

    // References
    assert_eq!(Bson::from(&24i32), Bson::Int32(24));
    assert_eq!(
        Bson::try_from(&String::from("data")).unwrap(),
        Bson::String(String::from("data"))
    );
    assert_eq!(Bson::from(&oid), Bson::ObjectId(oid));
    assert_eq!(
        Bson::from(&doc! {"a": "b"}),
        Bson::Document(doc! {"a": "b"})
    );

    // Optionals
    assert_eq!(Bson::from(Some(4)), Bson::Int32(4));
    assert_eq!(
        Bson::from(Some(String::from("data"))),
        Bson::String(String::from("data"))
    );
    assert_eq!(Bson::from(None::<i32>), Bson::Null);
    assert_eq!(Bson::from(None::<String>), Bson::Null);
    assert_eq!(doc! {"x": Some(4)}, doc! {"x": 4});
    assert_eq!(doc! {"x": None::<i32>}, doc! {"x": Bson::Null});

    let db_pointer = Bson::try_from(json!({
        "$dbPointer": {
            "$ref": "db.coll",
            "$id": { "$oid": "507f1f77bcf86cd799439011" },
        }
    }))
    .unwrap();
    let db_pointer = db_pointer.as_db_pointer().unwrap();
    assert_eq!(Bson::from(db_pointer), Bson::DbPointer(db_pointer.clone()));
}

#[test]
fn timestamp_ordering() {
    let _guard = LOCK.run_concurrently();
    let ts1 = Timestamp {
        time: 0,
        increment: 1,
    };
    let ts2 = Timestamp {
        time: 0,
        increment: 2,
    };
    let ts3 = Timestamp {
        time: 1,
        increment: 0,
    };
    assert!(ts1 < ts2);
    assert!(ts1 < ts3);
    assert!(ts2 < ts3);
}

#[test]
fn from_chrono_datetime() {
    let _guard = LOCK.run_concurrently();

    fn assert_millisecond_precision(dt: DateTime) {
        assert!(dt.to_chrono().timestamp_subsec_micros() % 1000 == 0);
    }
    fn assert_subsec_millis(dt: DateTime, millis: u32) {
        assert_eq!(dt.to_chrono().timestamp_subsec_millis(), millis)
    }

    let now = chrono::Utc::now();
    let dt = DateTime::from_chrono(now);
    assert_millisecond_precision(dt);

    #[cfg(feature = "chrono-0_4")]
    {
        let bson = Bson::from(now);
        assert_millisecond_precision(bson.as_datetime().unwrap().to_owned());

        let from_chrono = DateTime::from(now);
        assert_millisecond_precision(from_chrono);
    }

    let no_subsec_millis: chrono::DateTime<chrono::Utc> = "2014-11-28T12:00:09Z".parse().unwrap();
    let dt = DateTime::from_chrono(no_subsec_millis);
    assert_millisecond_precision(dt);
    assert_subsec_millis(dt, 0);

    #[cfg(feature = "chrono-0_4")]
    {
        let dt = DateTime::from(no_subsec_millis);
        assert_millisecond_precision(dt);
        assert_subsec_millis(dt, 0);

        let bson = Bson::from(dt);
        assert_millisecond_precision(bson.as_datetime().unwrap().to_owned());
        assert_subsec_millis(bson.as_datetime().unwrap().to_owned(), 0);
    }

    for s in &[
        "2014-11-28T12:00:09.123Z",
        "2014-11-28T12:00:09.123456Z",
        "2014-11-28T12:00:09.123456789Z",
    ] {
        let chrono_dt: chrono::DateTime<chrono::Utc> = s.parse().unwrap();
        let dt = DateTime::from_chrono(chrono_dt);
        assert_millisecond_precision(dt);
        assert_subsec_millis(dt, 123);

        #[cfg(feature = "chrono-0_4")]
        {
            let dt = DateTime::from(chrono_dt);
            assert_millisecond_precision(dt);
            assert_subsec_millis(dt, 123);

            let bson = Bson::from(chrono_dt);
            assert_millisecond_precision(bson.as_datetime().unwrap().to_owned());
            assert_subsec_millis(bson.as_datetime().unwrap().to_owned(), 123);
        }
    }

    #[cfg(feature = "chrono-0_4")]
    {
        let bdt = DateTime::from(chrono::MAX_DATETIME);
        assert_eq!(
            bdt.to_chrono().timestamp_millis(),
            chrono::MAX_DATETIME.timestamp_millis()
        );

        let bdt = DateTime::from(chrono::MIN_DATETIME);
        assert_eq!(
            bdt.to_chrono().timestamp_millis(),
            chrono::MIN_DATETIME.timestamp_millis()
        );

        let bdt = DateTime::MAX;
        assert_eq!(bdt.to_chrono(), chrono::MAX_DATETIME);

        let bdt = DateTime::MIN;
        assert_eq!(bdt.to_chrono(), chrono::MIN_DATETIME);
    }
}

#[test]
fn system_time() {
    let _guard = LOCK.run_concurrently();

    let st = SystemTime::now();
    let bt_into: crate::DateTime = st.into();
    let bt_from = crate::DateTime::from_system_time(st);

    assert_eq!(bt_into, bt_from);
    assert_eq!(
        bt_into.timestamp_millis(),
        st.duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as i64
    );

    let st = SystemTime::UNIX_EPOCH
        .checked_add(Duration::from_millis(1234))
        .unwrap();
    let bt = crate::DateTime::from_system_time(st);
    assert_eq!(bt.timestamp_millis(), 1234);
    assert_eq!(bt.to_system_time(), st);

    assert_eq!(
        crate::DateTime::MAX.to_system_time(),
        SystemTime::UNIX_EPOCH + Duration::from_millis(i64::MAX as u64)
    );
    assert_eq!(
        crate::DateTime::MIN.to_system_time(),
        SystemTime::UNIX_EPOCH - Duration::from_millis((i64::MIN as i128).abs() as u64)
    );

    assert_eq!(
        crate::DateTime::from_system_time(SystemTime::UNIX_EPOCH).timestamp_millis(),
        0
    );
}

#[test]
fn debug_print() {
    let oid = ObjectId::parse_str("000000000000000000000000").unwrap();

    let doc = doc! {
        "oid": oid,
        "arr": Bson::Array(vec! [
            Bson::Null,
            Bson::Timestamp(Timestamp { time: 1, increment: 1 }),
        ]),
        "doc": doc! { "a": 1, "b": "data"},
    };
    let normal_print = "Document({\"oid\": ObjectId(\"000000000000000000000000\"), \"arr\": \
                        Array([Null, Timestamp { time: 1, increment: 1 }]), \"doc\": \
                        Document({\"a\": Int32(1), \"b\": String(\"data\")})})";
    let pretty_print = "Document({
    \"oid\": ObjectId(
        \"000000000000000000000000\",
    ),
    \"arr\": Array([
        Null,
        Timestamp {
            time: 1,
            increment: 1,
        },
    ]),
    \"doc\": Document({
        \"a\": Int32(
            1,
        ),
        \"b\": String(
            \"data\",
        ),
    }),
})";

    assert_eq!(format!("{:?}", doc), normal_print);
    assert_eq!(format!("{:#?}", doc), pretty_print);
}
