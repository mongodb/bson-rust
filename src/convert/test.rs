#[cfg(feature = "unstable-convert-facet")]
mod facet {
    use crate::convert::{Facet as FacetFormat, Format};
    use ::facet::Facet;

    #[derive(Debug, PartialEq, Facet)]
    struct Borrowing<'a> {
        name: &'a str,
        count: i32,
    }

    // Serialize through a reference whose lifetime is shorter than that of the data borrowed by
    // the value.
    fn serialize_short_borrow<'long>(val: Borrowing<'long>) -> Vec<u8> {
        let local = Borrowing {
            name: val.name,
            count: val.count,
        };
        let short: &Borrowing<'_> = &local;
        FacetFormat::into_bson_bytes(short).unwrap()
    }

    #[test]
    fn short_reference_to_long_lived_borrow() {
        let name = String::from("hello");
        let bytes = serialize_short_borrow(Borrowing {
            name: &name,
            count: 3,
        });
        let back: Borrowing = FacetFormat::from_bson_bytes(&bytes).unwrap();
        assert_eq!(
            back,
            Borrowing {
                name: "hello",
                count: 3
            }
        );
    }

    // The value is borrowed for a strictly shorter time than the data it holds, and the data
    // outlives the call.
    #[test]
    fn borrow_shorter_than_contents() {
        let name = String::from("hello");
        let val = Borrowing {
            name: &name,
            count: 1,
        };
        let first = FacetFormat::into_bson_bytes(&val).unwrap();
        // `val` is still usable (and its contents still borrowed) after the short borrow ends.
        let second = FacetFormat::into_bson_bytes(&val).unwrap();
        assert_eq!(first, second);
    }

    // Owned data, 'static contents, short borrow.
    #[test]
    fn static_contents_short_borrow() {
        #[derive(Facet)]
        struct Owned {
            s: String,
        }
        let bytes = {
            let o = Owned { s: "x".into() };
            FacetFormat::into_bson_bytes(&o).unwrap()
        };
        assert!(!bytes.is_empty());
    }

    // A type invariant in its lifetime can't be shortened by subtyping, so the borrow lifetime
    // of the reference must be independent of the facet lifetime.
    #[derive(Facet)]
    struct Invariant<'a> {
        count: i32,
        #[facet(skip)]
        _marker: std::marker::PhantomData<fn(&'a ()) -> &'a ()>,
    }

    #[test]
    fn invariant_type_short_borrow() {
        fn go<'long>(val: Invariant<'long>) -> Vec<u8> {
            let r: &Invariant<'long> = &val;
            FacetFormat::into_bson_bytes(r).unwrap()
        }
        let bytes = go(Invariant {
            count: 5,
            _marker: std::marker::PhantomData,
        });
        assert!(!bytes.is_empty());
    }
}
