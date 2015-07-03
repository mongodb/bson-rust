use bson::Bson;

#[test]
fn recursive_macro() {
    let doc = doc! {
        "a" => ("foo"),
        "b" => {
            "bar" => {
                "harbor" => ["seal", false],
                "jelly" => (42.0)
            },
            "grape" => (27)
        },
        "c" => [-7]
    };

    match doc.get("a") {
        Some(&Bson::String(ref s)) => assert_eq!("foo", s),
        _ => panic!("String 'foo' was not inserted correctly."),
    }

    // Inner Doc 1
    match doc.get("b") {
        Some(&Bson::Document(ref doc)) => {

            // Inner doc 2
            match doc.get("bar") {
                Some(&Bson::Document(ref inner_doc)) => {

                    // Inner array
                    match inner_doc.get("harbor") {
                        Some(&Bson::Array(ref arr)) => {
                            assert_eq!(2, arr.len());

                            // Match array items
                            match arr[0] {
                                Bson::String(ref s) => assert_eq!("seal", s),
                                _ => panic!("String 'seal' was not inserted into inner array correctly."),
                            }
                            match arr[1] {
                                Bson::Boolean(ref b) => assert!(!b),
                                _ => panic!("Boolean 'false' was not inserted into inner array correctly."),
                            }
                        },
                        _ => panic!("Inner array was not inserted correctly."),
                    }

                    // Inner floating point
                    match inner_doc.get("jelly") {
                        Some(&Bson::FloatingPoint(ref fp)) => assert_eq!(42.0, *fp),
                        _ => panic!("Floating point 42.0 was not inserted correctly."),
                    }
                },
                _ => panic!("Second inner document was not inserted correctly."),
            }
        },
        _ => panic!("Inner document was not inserted correctly."),
    }

    // Outer array
    match doc.get("c") {
        Some(&Bson::Array(ref arr)) => {
            assert_eq!(1, arr.len());

            // Integer type
            match arr[0] {
                Bson::I32(ref i) => assert_eq!(-7, *i),
                _ => panic!("I32 '-7' was not inserted correctly."),
            }
        },
        _ => panic!("Array was not inserted correctly."),
    }
}
