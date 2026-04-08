mod corpus;

use facet::Facet;
use facet_json;

use crate::Bson;

use super::ExtJson;

fn assert_roundtrip<T: Facet<'static> + PartialEq + std::fmt::Debug>(value: &T, expected: &str) {
    let json = facet_json::to_string_pretty(value).unwrap();
    assert_eq!(json, expected);
    let back: T = facet_json::from_str(&json).unwrap();
    assert_eq!(value, &back);
}

#[test]
fn roundtrip_i32() {
    #[derive(Debug, Facet, PartialEq)]
    struct Foo {
        a: i32,
        #[facet(proxy = ExtJson)]
        b: i32,
        #[facet(opaque, proxy = ExtJson)]
        c: Bson,
    }
    assert_roundtrip(
        &Foo {
            a: 13,
            b: 42,
            c: Bson::Int32(1066),
        },
        r#"{
  "a": 13,
  "b": {
    "$numberInt": "42"
  },
  "c": {
    "$numberInt": "1066"
  }
}"#,
    );
}

#[test]
fn roundtrip_symbol() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        s: Bson,
    }
    assert_roundtrip(
        &Foo {
            s: Bson::Symbol("hello".into()),
        },
        r#"{
  "s": {
    "$symbol": "hello"
  }
}"#,
    );
}

#[test]
fn roundtrip_double() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(proxy = ExtJson)]
        v: f64,
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    assert_roundtrip(
        &Foo {
            v: 1.5,
            b: Bson::Double(2.5),
        },
        r#"{
  "v": {
    "$numberDouble": "1.5"
  },
  "b": {
    "$numberDouble": "2.5"
  }
}"#,
    );
}

#[test]
fn roundtrip_i64() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(proxy = ExtJson)]
        v: i64,
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    assert_roundtrip(
        &Foo {
            v: 9_000_000_000,
            b: Bson::Int64(1_000_000_000_000),
        },
        r#"{
  "v": {
    "$numberLong": "9000000000"
  },
  "b": {
    "$numberLong": "1000000000000"
  }
}"#,
    );
}

#[test]
fn roundtrip_object_id() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        v: crate::oid::ObjectId,
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    let id = crate::oid::ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
    assert_roundtrip(
        &Foo {
            v: id,
            b: Bson::ObjectId(id),
        },
        r#"{
  "v": {
    "$oid": "507f1f77bcf86cd799439011"
  },
  "b": {
    "$oid": "507f1f77bcf86cd799439011"
  }
}"#,
    );
}

#[test]
fn roundtrip_datetime() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        v: crate::DateTime,
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    let dt = crate::DateTime::from_millis(1_000_000_000_000);
    assert_roundtrip(
        &Foo {
            v: dt,
            b: Bson::DateTime(dt),
        },
        r#"{
  "v": {
    "$date": {
      "$numberLong": "1000000000000"
    }
  },
  "b": {
    "$date": {
      "$numberLong": "1000000000000"
    }
  }
}"#,
    );
}

#[test]
fn roundtrip_binary() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        v: crate::Binary,
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    let bin = crate::Binary {
        subtype: crate::spec::BinarySubtype::Generic,
        bytes: vec![1, 2, 3],
    };
    assert_roundtrip(
        &Foo {
            v: bin.clone(),
            b: Bson::Binary(bin),
        },
        r#"{
  "v": {
    "$binary": {
      "base64": "AQID",
      "subType": "00"
    }
  },
  "b": {
    "$binary": {
      "base64": "AQID",
      "subType": "00"
    }
  }
}"#,
    );
}

#[test]
fn roundtrip_timestamp() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        v: crate::Timestamp,
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    let ts = crate::Timestamp {
        time: 1234,
        increment: 5,
    };
    assert_roundtrip(
        &Foo {
            v: ts,
            b: Bson::Timestamp(ts),
        },
        r#"{
  "v": {
    "$timestamp": {
      "t": 1234,
      "i": 5
    }
  },
  "b": {
    "$timestamp": {
      "t": 1234,
      "i": 5
    }
  }
}"#,
    );
}

#[test]
fn roundtrip_decimal128() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        v: crate::Decimal128,
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    let d: crate::Decimal128 = "3.14".parse().unwrap();
    assert_roundtrip(
        &Foo {
            v: d,
            b: Bson::Decimal128(d),
        },
        r#"{
  "v": {
    "$numberDecimal": "3.14"
  },
  "b": {
    "$numberDecimal": "3.14"
  }
}"#,
    );
}

#[test]
fn roundtrip_regex() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        v: crate::Regex,
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    let r = crate::Regex::from_strings("abc", "i").unwrap();
    assert_roundtrip(
        &Foo {
            v: r.clone(),
            b: Bson::RegularExpression(r),
        },
        r#"{
  "v": {
    "$regularExpression": {
      "pattern": "abc",
      "options": "i"
    }
  },
  "b": {
    "$regularExpression": {
      "pattern": "abc",
      "options": "i"
    }
  }
}"#,
    );
}

#[test]
fn roundtrip_db_pointer() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        v: crate::DbPointer,
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    let id = crate::oid::ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
    let dp = crate::DbPointer {
        namespace: "test.coll".to_string(),
        id,
    };
    assert_roundtrip(
        &Foo {
            v: dp.clone(),
            b: Bson::DbPointer(dp),
        },
        r#"{
  "v": {
    "$dbPointer": {
      "$ref": "test.coll",
      "$id": {
        "$oid": "507f1f77bcf86cd799439011"
      }
    }
  },
  "b": {
    "$dbPointer": {
      "$ref": "test.coll",
      "$id": {
        "$oid": "507f1f77bcf86cd799439011"
      }
    }
  }
}"#,
    );
}

#[test]
fn roundtrip_bson_string_and_bool() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(proxy = ExtJson)]
        sv: String,
        #[facet(opaque, proxy = ExtJson)]
        sb: Bson,
        #[facet(proxy = ExtJson)]
        bv: bool,
        #[facet(opaque, proxy = ExtJson)]
        bb: Bson,
        #[facet(opaque, proxy = ExtJson)]
        n: Bson,
    }
    assert_roundtrip(
        &Foo {
            sv: "hello".into(),
            sb: Bson::String("hello".into()),
            bv: true,
            bb: Bson::Boolean(true),
            n: Bson::Null,
        },
        r#"{
  "sv": "hello",
  "sb": "hello",
  "bv": true,
  "bb": true,
  "n": null
}"#,
    );
}

#[test]
fn roundtrip_array() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        a: Bson,
        #[facet(opaque, proxy = ExtJson)]
        b: crate::Array,
    }
    let arr = vec![
        Bson::Int32(1),
        Bson::String("hello".into()),
        Bson::Boolean(false),
        Bson::Array(vec![Bson::Int64(9_000_000_000)]),
    ];
    assert_roundtrip(
        &Foo {
            a: Bson::Array(arr.clone()),
            b: arr,
        },
        r#"{
  "a": [
    {
      "$numberInt": "1"
    },
    "hello",
    false,
    [
      {
        "$numberLong": "9000000000"
      }
    ]
  ],
  "b": [
    {
      "$numberInt": "1"
    },
    "hello",
    false,
    [
      {
        "$numberLong": "9000000000"
      }
    ]
  ]
}"#,
    );
}

#[test]
fn roundtrip_javascript_code() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    assert_roundtrip(
        &Foo {
            b: Bson::JavaScriptCode("console.log(1)".into()),
        },
        r#"{
  "b": {
    "$code": "console.log(1)"
  }
}"#,
    );
}

#[test]
fn roundtrip_javascript_code_with_scope() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    let jsc = crate::JavaScriptCodeWithScope {
        code: "function(x) { return x + n; }".into(),
        scope: doc! { "n": 1 },
    };
    assert_roundtrip(
        &Foo {
            b: Bson::JavaScriptCodeWithScope(jsc),
        },
        r#"{
  "b": {
    "$code": "function(x) { return x + n; }",
    "$scope": {
      "n": {
        "$numberInt": "1"
      }
    }
  }
}"#,
    );
}

#[test]
fn roundtrip_document() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        #[facet(opaque, proxy = ExtJson)]
        v: crate::Document,
        #[facet(opaque, proxy = ExtJson)]
        b: Bson,
    }
    let doc = doc! {
        "x": 1,
        "y": "hello",
        "nested": {
            "flag": true,
        },
    };
    assert_roundtrip(
        &Foo {
            v: doc.clone(),
            b: Bson::Document(doc),
        },
        r#"{
  "v": {
    "x": {
      "$numberInt": "1"
    },
    "y": "hello",
    "nested": {
      "flag": true
    }
  },
  "b": {
    "x": {
      "$numberInt": "1"
    },
    "y": "hello",
    "nested": {
      "flag": true
    }
  }
}"#,
    );
}
