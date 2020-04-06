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
    (@object $object:ident ($($key:tt)+) (=> null $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!(null)) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!(null)) $($rest)*);
    };

    // Next value is an array.
    (@object $object:ident ($($key:tt)+) (=> [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!([$($array)*])) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!([$($array)*])) $($rest)*);
    };

    // Next value is a map.
    (@object $object:ident ($($key:tt)+) (=> {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!({$($map)*})) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!({$($map)*})) $($rest)*);
    };

    // Next value is an expression followed by comma.
    (@object $object:ident ($($key:tt)+) (=> $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!($value)) , $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!($value)) , $($rest)*);
    };

    // Last value is an expression with no trailing comma.
    (@object $object:ident ($($key:tt)+) (=> $value:expr) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!($value)));
    };

    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        $crate::bson!(@object $object [$($key)+] ($crate::bson!($value)));
    };

    // Missing value for last entry. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) (=>) $copy:tt) => {
        // "unexpected end of macro invocation"
        $crate::bson!();
    };

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
    (@object $object:ident () (=> $($rest:tt)*) ($kv_separator:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `=>`".
        unimplemented!($kv_separator);
    };

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
    (@object $object:ident () (($key:expr) => $($rest:tt)*) $copy:tt) => {
        $crate::bson!(@object $object ($key) (=> $($rest)*) (=> $($rest)*));
    };

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
        $crate::Bson::Document($crate::doc!{$($tt)+});
    };

    // Any Serialize type: numbers, strings, struct literals, variables etc.
    // Must be below every other rule.
    ($other:expr) => {
        ::std::convert::From::from($other)
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
