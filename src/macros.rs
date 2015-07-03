#[macro_export]
macro_rules! add_to_doc {
    ($doc:expr, $key:expr => ($val:expr)) => {{
        $doc.insert($key.to_owned(), ::std::convert::From::from($val));
    }};

    ($doc:expr, $key:expr => [$($val:expr),*]) => {{
        let vec = vec![$(::std::convert::From::from($val)),*];
        $doc.insert($key.to_owned(), $crate::Bson::Array(vec));
    }};

    ($doc:expr, $key:expr => { $($k:expr => $v:tt),* }) => {{
        $doc.insert($key.to_owned(), $crate::Bson::Document(doc! {
            $(
                $k => $v
            ),*
        }));
    }};
}

#[macro_export]
macro_rules! doc {
    ( $($key:expr => $val:tt),* ) => {{
        let mut document = $crate::Document::new();

        $(
            add_to_doc!(document, $key => $val);
        )*

        document
    }};
}
