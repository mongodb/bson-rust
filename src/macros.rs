// BSON macro based on the serde_json json! implementation.

/// Construct a bson::BSON value from a literal.
///
/// ```rust
/// # use bson::bson;
/// #
/// # fn main() {
/// let value = bson!({
///     "code": 200,
///     "success": true,
///     "payload": {
///       "some": [
///           "pay",
///           "loads",
///       ]
///     }
/// });
/// # }
/// ```
#[macro_export]
macro_rules! bson {
    //////////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an array [...]. Produces a vec![...]
    // of the elements.
    //
    // Must be invoked as: bson!(@array [] $($tt)*)
    //////////////////////////////////////////////////////////////////////////

    // Finished with trailing comma.
    (@array [$($elems:expr,)*]) => {
        vec![$($elems,)*]
    };

    // Finished without trailing comma.
    (@array [$($elems:expr),*]) => {
        vec![$($elems),*]
    };

    // Next element is `null`.
    (@array [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::bson!(@array [$($elems,)* $crate::bson!(null)] $($rest)*)
    };

    // Next element is an array.
    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::bson!(@array [$($elems,)* $crate::bson!([$($array)*])] $($rest)*)
    };

    // Next element is a map.
    (@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::bson!(@array [$($elems,)* $crate::bson!({$($map)*})] $($rest)*)
    };

    // Next element is an expression followed by comma.
    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::bson!(@array [$($elems,)* $crate::bson!($next),] $($rest)*)
    };

    // Last element is an expression with no trailing comma.
    (@array [$($elems:expr,)*] $last:expr) => {
        $crate::bson!(@array [$($elems,)* $crate::bson!($last)])
    };

    // Comma after the most recent element.
    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::bson!(@array [$($elems,)*] $($rest)*)
    };

    //////////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an object {...}. Each entry is
    // inserted into the given map variable.
    //
    // Must be invoked as: bson!(@object $map () ($($tt)*) ($($tt)*))
    //
    // We require two copies of the input tokens so that we can match on one
    // copy and trigger errors on the other copy.
    //////////////////////////////////////////////////////////////////////////

    // Finished.
    (@object $object:ident () () ()) => {};

    // Insert the current entry followed by trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        $object.insert::<_, $crate::Bson>(($($key)+), $value);
        $crate::bson!(@object $object () ($($rest)*) ($($rest)*));
    };

    // Insert the last entry without trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        $object.insert::<_, $crate::Bson>(($($key)+), $value);
    };

    // Next value is `null`.
    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!(null)) $($rest)*);
    };

    // Next value is an array.
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!([$($array)*])) $($rest)*);
    };

    // Next value is a map.
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!({$($map)*})) $($rest)*);
    };

    // Next value is an expression followed by comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!($value)) , $($rest)*);
    };

    // Last value is an expression with no trailing comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!($value)));
    };

    // Missing value for last entry. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::bson!();
    };

    // Missing key-value separator and value for last entry.
    // Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::bson!();
    };

    // Misplaced key-value separator. Trigger a reasonable error message.
    (@object $object:ident () (: $($rest:tt)*) ($kv_separator:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `:`".
        unimplemented!($kv_separator);
    };

    // Found a comma inside a key. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `,`".
        unimplemented!($comma);
    };

    // Key is fully parenthesized. This avoids clippy double_parens false
    // positives because the parenthesization may be necessary here.
    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object ($key) (: $($rest)*) (: $($rest)*));
    };

    // Munch a token into the current key.
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    //////////////////////////////////////////////////////////////////////////
    // The main implementation.
    //
    // Must be invoked as: bson!($($bson)+)
    //////////////////////////////////////////////////////////////////////////

    (null) => {
        $crate::Bson::Null
    };

    ([]) => {
        $crate::Bson::Array(vec![])
    };

    ([ $($tt:tt)+ ]) => {
        $crate::Bson::Array($crate::bson!(@array [] $($tt)+))
    };

    ({}) => {
        $crate::Bson::Document($crate::doc!{})
    };

    ({$($tt:tt)+}) => {
        $crate::Bson::Document($crate::doc!{$($tt)+})
    };

    // Any Into<Bson> type.
    // Must be below every other rule.
    ($other:expr) => {
        <_ as ::std::convert::Into<$crate::Bson>>::into($other)
    };
}

/// Construct a bson::Document value.
///
/// ```rust
/// # use bson::doc;
/// #
/// # fn main() {
/// let value = doc! {
///     "code": 200,
///     "success": true,
///     "payload": {
///       "some": [
///           "pay",
///           "loads",
///       ]
///     }
/// };
/// # }
/// ```
#[macro_export]
macro_rules! doc {
    () => {{ $crate::Document::new() }};
    ( $($tt:tt)+ ) => {{
        let mut object = $crate::Document::new();
        $crate::bson!(@object object () ($($tt)+) ($($tt)+));
        object
    }};
}

/// Construct a [`crate::RawBson`] value from a literal.
///
/// ```rust
/// use bson::rawbson;
///
/// let value = rawbson!({
///     "code": 200,
///     "success": true,
///     "payload": {
///       "some": [
///           "pay",
///           "loads",
///       ]
///     }
/// });
/// ```
#[macro_export]
macro_rules! rawbson {
    //////////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an array [...]. Produces a
    // RawArrayBuf containing the elements.
    //
    // Must be invoked as: bson!(@array [] $($tt)*)
    //////////////////////////////////////////////////////////////////////////

    // Finished with trailing comma.
    (@array [$($elems:expr,)*]) => {
        $crate::RawArrayBuf::from_iter(vec![$($elems,)*])
    };

    // Finished without trailing comma.
    (@array [$($elems:expr),*]) => {
        $crate::RawArrayBuf::from_iter(vec![$($elems),*])
    };

    // Next element is `null`.
    (@array [$($elems:expr,)*] null $($rest:tt)*) => {
        $crate::rawbson!(@array [$($elems,)* $crate::rawbson!(null)] $($rest)*)
    };

    // Next element is an array.
    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        $crate::rawbson!(@array [$($elems,)* $crate::rawbson!([$($array)*])] $($rest)*)
    };

    // Next element is a map.
    (@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        $crate::rawbson!(@array [$($elems,)* $crate::rawbson!({$($map)*})] $($rest)*)
    };

    // Next element is an expression followed by comma.
    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        $crate::rawbson!(@array [$($elems,)* $crate::rawbson!($next),] $($rest)*)
    };

    // Last element is an expression with no trailing comma.
    (@array [$($elems:expr,)*] $last:expr) => {
        $crate::rawbson!(@array [$($elems,)* $crate::rawbson!($last)])
    };

    // Comma after the most recent element.
    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        $crate::rawbson!(@array [$($elems,)*] $($rest)*)
    };

    //////////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an object {...}. Each entry is
    // inserted into the given map variable.
    //
    // Must be invoked as: rawbson!(@object $map () ($($tt)*) ($($tt)*))
    //
    // We require two copies of the input tokens so that we can match on one
    // copy and trigger errors on the other copy.
    //////////////////////////////////////////////////////////////////////////

    // Finished.
    (@object $object:ident () () ()) => {};

    // Insert the current entry with followed by trailing comma, with a key literal.
    (@object $object:ident [$key:literal] ($value:expr) , $($rest:tt)*) => {{
        $object.append($crate::raw::cstr!($key), $value);
        $crate::rawbson!(@object $object () ($($rest)*) ($($rest)*));
    }};

    // Insert the current entry with followed by trailing comma, with a key expression.
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {{
        $object.append($($key)+, $value);
        $crate::rawbson!(@object $object () ($($rest)*) ($($rest)*));
    }};

    // Insert the last entry without trailing comma, with a key literal.
    (@object $object:ident [$key:literal] ($value:expr)) => {
        $object.append($crate::raw::cstr!($key), $value);
    };

    // Insert the last entry without trailing comma, with a key expression.
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        $object.append($($key)+, $value);
    };

    // Next value is `null`.
    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::rawbson!(@object $object [$($key)+] ($crate::rawbson!(null)) $($rest)*);
    };

    // Next value is an array.
    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::rawbson!(@object $object [$($key)+] ($crate::rawbson!([$($array)*])) $($rest)*);
    };

    // Next value is a map.
    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::rawbson!(@object $object [$($key)+] ($crate::rawbson!({$($map)*})) $($rest)*);
    };

    // Next value is an expression followed by comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::rawbson!(@object $object [$($key)+] ($crate::rawbson!($value)) , $($rest)*);
    };

    // Last value is an expression with no trailing comma.
    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::rawbson!(@object $object [$($key)+] ($crate::rawbson!($value)));
    };

    // Missing value for last entry. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::rawbson!();
    };

    // Missing key-value separator and value for last entry.
    // Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::rawbson!();
    };

    // Misplaced key-value separator. Trigger a reasonable error message.
    (@object $object:ident () (: $($rest:tt)*) ($kv_separator:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `:`".
        unimplemented!($kv_separator);
    };

    // Found a comma inside a key. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `,`".
        unimplemented!($comma);
    };

    // Key is fully parenthesized. This avoids clippy double_parens false
    // positives because the parenthesization may be necessary here.
    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        $crate::rawbson!(@object $object ($key) (: $($rest)*) (: $($rest)*));
    };

    // Munch a token into the current key.
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        $crate::rawbson!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    //////////////////////////////////////////////////////////////////////////
    // The main implementation.
    //
    // Must be invoked as: rawbson!($($bson)+)
    //////////////////////////////////////////////////////////////////////////

    (null) => {
        $crate::RawBson::Null
    };

    ([]) => {
        $crate::RawBson::Array($crate::RawArrayBuf::new())
    };

    ([ $($tt:tt)+ ]) => {
        $crate::RawBson::Array($crate::rawbson!(@array [] $($tt)+))
    };

    ({}) => {
        $crate::RawBson::Document($crate::rawdoc!{})
    };

    ({$($tt:tt)+}) => {
        $crate::RawBson::Document($crate::rawdoc!{$($tt)+})
    };

    // Any Into<RawBson> type.
    // Must be below every other rule.
    ($other:expr) => {
        <_ as ::std::convert::Into<$crate::RawBson>>::into($other)
    };
}

/// Construct a [`crate::RawDocumentBuf`] value.
///
/// ```rust
/// use bson::rawdoc;
///
/// let value = rawdoc! {
///     "code": 200,
///     "success": true,
///     "payload": {
///       "some": [
///           "pay",
///           "loads",
///       ]
///     }
/// };
/// ```
#[macro_export]
macro_rules! rawdoc {
    () => {{ $crate::RawDocumentBuf::new() }};
    ( $($tt:tt)+ ) => {{
        let mut object = $crate::RawDocumentBuf::new();
        $crate::rawbson!(@object object () ($($tt)+) ($($tt)+));
        object
    }};
}

/// Like [`serde_with::serde_conv!`], but with additional functionality:
/// 1. Supports attaching documentation (`///`) and other attributes to the generated struct
/// 2. Allows serializers that return a [`Result`]`, enabling error handling during serialization
///
/// This macro generates a `SerializeAs`/`DeserializeAs` implementation for a given type,
/// with optional struct-level attributes like `#[derive(...)]` or `/// doc comments`.
#[cfg(feature = "serde")]
macro_rules! serde_conv_doc {
    ($(#[$meta:meta])* $vis:vis $m:ident, $t:ty, $ser:expr, $de:expr) => {
        #[allow(non_camel_case_types)]
        $(#[$meta])*
        $vis struct $m;

        // Prevent clippy lints triggering because of the template here
        // https://github.com/jonasbb/serde_with/pull/320
        // https://github.com/jonasbb/serde_with/pull/729
        #[allow(clippy::all)]
        #[allow(missing_docs)]
        const _:() = {
            impl $m {
                $vis fn serialize<S>(x: &$t, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    let y = $ser(x).map_err(serde::ser::Error::custom)?;
                    Serialize::serialize(&y, serializer)
                }

                $vis fn deserialize<'de, D>(deserializer: D) -> Result<$t, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    let y = Deserialize::deserialize(deserializer)?;
                    $de(y).map_err(serde::de::Error::custom)
                }
            }

            #[cfg(feature = "serde_with-3")]
            impl serde_with::SerializeAs<$t> for $m {
                fn serialize_as<S>(x: &$t, serializer: S) -> Result<S::Ok, S::Error>
                where
                    S: Serializer,
                {
                    Self::serialize(x, serializer)
                }
            }

            #[cfg(feature = "serde_with-3")]
            impl<'de> serde_with::DeserializeAs<'de, $t> for $m {
                fn deserialize_as<D>(deserializer: D) -> Result<$t, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    Self::deserialize(deserializer)
                }
            }
        };
    };
}

#[cfg(feature = "serde")]
pub(crate) use serde_conv_doc;
