use bson::{Document,Bson};

#[derive(RustcEncodable,RustcDecodable ,Debug , Clone, PartialEq, Eq)]
pub struct User {
    id: u64,
    name : String
}

#[test]
pub fn test_to_object(){
    let mut doc = Document::new();
    doc.insert("id".to_owned() , Bson::I64(10i64));
    doc.insert("name".to_owned() , Bson::String("ZhuGe".to_owned()));

    let bson = Bson::Document(doc);
    match bson.to_object::<User>() {
        Err(err) => panic!("{}" , err),
        Ok(ref user) => {
            println!("user : {:?}", user);
            assert_eq!(user , &User{id : 10u64 , name : "ZhuGe".to_owned()});
        }
    }
}