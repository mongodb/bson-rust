use bson::{self, Bson, Document};

fn print_doc_with_indent_level(doc: &Document, n: i32) {
    macro_rules! print_spaces {
        ( $n:expr ) => {
            for _ in 0..$n {
                print!(" ");
            }
        };
    }

    println!("{{");

    for (key, value) in doc.iter() {
        print_spaces!(n + 4);
        print!("{}: ", key);

        print_bson_with_indent_level(value, n);
        println!("");
    }

    print_spaces!(n);
    println!("}}");
}

#[macro_export]
macro_rules! print_doc {
    ( $doc:expr ) => {
        print_doc_with_indent_level($doc, 0)
    };
}

fn print_bson_with_indent_level(bson: &Bson, n: i32) {
    match bson {
        &Bson::Document(ref d) => print_doc_with_indent_level(&d, n + 4),
        &Bson::String(ref s) => print!("\"{}\",", s),
        &Bson::Boolean(b) => print!("{},", b),
        &Bson::Array(ref v) => {
            print!("[ ");

            for b in v {
                print_bson_with_indent_level(b, n);
                print!(" ");
            }

            print!("]");
        },
        ref bson => print!("{:?},", bson)
    };
}


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

    print_doc!(&doc);
}
