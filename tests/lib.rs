#[macro_use(bson, doc)]
extern crate bson;
extern crate chrono;
extern crate rustc_serialize;

mod modules;

use bson::Bson;
use bson::spec::BinarySubtype;
use bson::oid::ObjectId;
use chrono::offset::utc::UTC;

#[test]
fn test_format() {
    let id_string = "thisismyname";
    let string_bytes : Vec<_> = id_string.bytes().collect();
    let mut bytes = [0; 12];

    for i in 0..12 {
        bytes[i] = string_bytes[i];
    }

    let id = ObjectId::with_bytes(bytes);
    let date = UTC::now();

    let doc = doc! {
        "float" => 2.4,
        "string" => "hello",
        "array" => ["testing", 1],
        "doc" => {
            "fish" => "in",
            "a" => "barrel",
            "!" => 1
        },
        "bool" => true,
        "null" => (Bson::Null),
        "regexp" => (Bson::RegExp("s[ao]d".to_owned(), "i".to_owned())),
        "code" => (Bson::JavaScriptCode("function(x) { return x._id; }".to_owned())),
        "i32" => 12,
        "i64" => (-55),
        "timestamp" => (Bson::TimeStamp(229999444)),
        "binary" => (Bson::Binary(BinarySubtype::Md5, "thingies".to_owned().into_bytes())),
        "_id" => id,
        "date" => (Bson::UtcDatetime(date))
    };

    let expected = format!("{{ float: 2.4, string: \"hello\", array: [\"testing\", 1], doc: {{ fish: \"in\", a: \"barrel\", !: 1 }}, bool: true, null: null, regexp: /s[ao]d/i, code: function(x) {{ return x._id; }}, i32: 12, i64: -55, timestamp: Timestamp(0, 229999444), binary: BinData(5, \"thingies\"), _id: ObjectId(\"{}\"), date: Date(\"{}\") }}", id_string, date);

    assert_eq!(expected, format!("{}", doc));
}
