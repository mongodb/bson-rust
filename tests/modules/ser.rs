use bson::{Bson, Document, to_bson, from_bson};
use bson::oid::ObjectId;
use std::collections::BTreeMap;

#[test]
fn map() {
    let obj = Bson::ObjectId(ObjectId::new().unwrap());
    let s: BTreeMap<String, String> = from_bson(obj).unwrap();
    println!("{:?}", s);
    let deser: Bson = to_bson(&s);
    println!("{:?}", deser);
    assert_eq!(1, 2);
}
