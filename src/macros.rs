/// Construct a BSON value
#[macro_export]
macro_rules! bson {
    ([]) => {{ $crate::Bson::Array(Vec::new()) }};

    ([$($val:tt),*]) => {{
        let mut array = Vec::new();

        $(
            array.push(bson!($val));
        )*

        $crate::Bson::Array(array)
    }};

    ([$val:expr]) => {{
        $crate::Bson::Array(vec!(::std::convert::From::from($val)))
    }};

    ({ $($k:expr => $v:tt),* }) => {{
        $crate::Bson::Document(doc! {
            $(
                $k => $v
            ),*
        })
    }};

    ($val:expr) => {{
        ::std::convert::From::from($val)
    }};
}

/// Construct a BSON Document
#[macro_export]
macro_rules! doc {
    () => {{ $crate::Document::new() }};

    ( $($key:expr => $val:tt),* ) => {{
        let mut document = $crate::Document::new();

        $(
            document.insert_bson($key.to_owned(), bson!($val));
        )*

        document
    }};
}
