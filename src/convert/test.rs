use crate::{
    RawDocumentBuf,
    convert::{Format, FromBsonBytesOwned, IntoBsonBytes},
};

fn doc() -> RawDocumentBuf {
    rawdoc! { "a": 1, "b": "two" }
}

/// Checks supported by every format: serialize `&RawDocumentBuf`, deserialize an owned
/// `RawDocumentBuf` from a slice and from a `Vec`.
fn check_owned<F: Format>()
where
    for<'a> &'a RawDocumentBuf: IntoBsonBytes<F>,
    RawDocumentBuf: FromBsonBytesOwned<F>,
{
    let doc = doc();
    let expected = doc.as_bytes();

    assert_eq!(F::into_bson_bytes(&doc).unwrap(), expected);

    // Append preserves existing buffer contents.
    let mut buf = vec![0xAA, 0xBB];
    F::append_bson_bytes(&doc, &mut buf).unwrap();
    F::append_bson_bytes(&doc, &mut buf).unwrap();
    let mut want = vec![0xAA, 0xBB];
    want.extend(expected);
    want.extend(expected);
    assert_eq!(buf, want);

    // Owned result from a borrowed slice and from an owned buffer.
    let from_slice: RawDocumentBuf = F::from_bson_bytes(expected).unwrap();
    assert_eq!(from_slice, doc);
    let from_vec: RawDocumentBuf = F::from_bson_bytes_owned(expected.to_vec()).unwrap();
    assert_eq!(from_vec, doc);

    // Invalid input is rejected.
    assert!(F::from_bson_bytes::<RawDocumentBuf>(&expected[..expected.len() - 1]).is_err());
    assert!(F::from_bson_bytes_owned::<RawDocumentBuf>(vec![1, 2, 3]).is_err());
}

#[cfg(feature = "unstable-convert-direct")]
mod direct {
    use super::*;
    use crate::{RawDocument, convert::Direct};

    #[test]
    fn round_trip() {
        check_owned::<Direct>();

        let doc = doc();
        let bytes = doc.as_bytes();

        // `&RawDocument`, serialized and borrowed from the input.
        assert_eq!(Direct::into_bson_bytes(&*doc).unwrap(), bytes);
        let mut buf = vec![0xAA];
        Direct::append_bson_bytes(&*doc, &mut buf).unwrap();
        assert_eq!(&buf[1..], bytes);
        let _: &RawDocument = Direct::from_bson_bytes(bytes).unwrap();
        assert!(Direct::from_bson_bytes::<&RawDocument>(&bytes[..bytes.len() - 1]).is_err());

        // By-value `RawDocumentBuf` hands over its buffer.
        assert_eq!(Direct::into_bson_bytes(self::doc()).unwrap(), bytes);
        let mut buf = vec![0xAA];
        Direct::append_bson_bytes(self::doc(), &mut buf).unwrap();
        assert_eq!(&buf[1..], bytes);
    }
}

#[cfg(feature = "unstable-convert-serde")]
mod serde {
    use super::*;
    use crate::{RawDocument, convert::Serde};

    #[test]
    fn round_trip() {
        check_owned::<Serde>();

        let doc = doc();
        let bytes = doc.as_bytes();
        let _: &RawDocument = Serde::from_bson_bytes(bytes).unwrap();
        assert!(Serde::from_bson_bytes::<&RawDocument>(&bytes[..bytes.len() - 1]).is_err());
    }
}

#[cfg(feature = "unstable-convert-facet")]
mod facet {
    use super::*;

    use crate::convert::{Facet as FacetFormat, Format};
    use ::facet::Facet;

    #[test]
    fn round_trip() {
        check_owned::<FacetFormat>();
    }

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
