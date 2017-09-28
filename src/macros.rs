#[macro_export]
macro_rules! bson {
    // Hide distracting implementation details from the generated rustdoc.
    ($($bson:tt)+) => {
        bson_internal!($($bson)+)
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! bson_internal {
    //////////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an array [...]. Produces a vec![...]
    // of the elements.
    //
    // Must be invoked as: bson_internal!(@array [] $($tt)*)
    //////////////////////////////////////////////////////////////////////////

    // Done with trailing comma.
    (@array [$($elems:expr,)*]) => {
        vec![$($elems,)*]
    };

    // Done without trailing comma.
    (@array [$($elems:expr),*]) => {
        vec![$($elems),*]
    };

    // Next element is `null`.
    (@array [$($elems:expr,)*] null $($rest:tt)*) => {
        bson_internal!(@array [$($elems,)* bson_internal!(null)] $($rest)*)
    };

    // Next element is `true`.
    (@array [$($elems:expr,)*] true $($rest:tt)*) => {
        bson_internal!(@array [$($elems,)* bson_internal!(true)] $($rest)*)
    };

    // Next element is `false`.
    (@array [$($elems:expr,)*] false $($rest:tt)*) => {
        bson_internal!(@array [$($elems,)* bson_internal!(false)] $($rest)*)
    };

    // Next element is an array.
    (@array [$($elems:expr,)*] [$($array:tt)*] $($rest:tt)*) => {
        bson_internal!(@array [$($elems,)* bson_internal!([$($array)*])] $($rest)*)
    };

    // Next element is a map.
    (@array [$($elems:expr,)*] {$($map:tt)*} $($rest:tt)*) => {
        bson_internal!(@array [$($elems,)* bson_internal!({$($map)*})] $($rest)*)
    };

    // Next element is an expression followed by comma.
    (@array [$($elems:expr,)*] $next:expr, $($rest:tt)*) => {
        bson_internal!(@array [$($elems,)* bson_internal!($next),] $($rest)*)
    };

    // Last element is an expression with no trailing comma.
    (@array [$($elems:expr,)*] $last:expr) => {
        bson_internal!(@array [$($elems,)* bson_internal!($last)])
    };

    // Comma after the most recent element.
    (@array [$($elems:expr),*] , $($rest:tt)*) => {
        bson_internal!(@array [$($elems,)*] $($rest)*)
    };

    //////////////////////////////////////////////////////////////////////////
    // TT muncher for parsing the inside of an object {...}. Each entry is
    // inserted into the given map variable.
    //
    // Must be invoked as: bson_internal!(@object $map () ($($tt)*) ($($tt)*))
    //
    // We require two copies of the input tokens so that we can match on one
    // copy and trigger errors on the other copy.
    //////////////////////////////////////////////////////////////////////////

    // Done.
    (@object $object:ident () () ()) => {};

    // Insert the current entry followed by trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr) , $($rest:tt)*) => {
        $object.insert_bson(($($key)+).into(), $value);
        bson_internal!(@object $object () ($($rest)*) ($($rest)*));
    };

    // Insert the last entry without trailing comma.
    (@object $object:ident [$($key:tt)+] ($value:expr)) => {
        $object.insert_bson(($($key)+).into(), $value);
    };

    // Next value is `null`.
    (@object $object:ident ($($key:tt)+) (=> null $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!(null)) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: null $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!(null)) $($rest)*);
    };

    // Next value is `true`.
    (@object $object:ident ($($key:tt)+) (=> true $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!(true)) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: true $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!(true)) $($rest)*);
    };

    // Next value is `false`.
    (@object $object:ident ($($key:tt)+) (=> false $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!(false)) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: false $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!(false)) $($rest)*);
    };

    // Next value is an array.
    (@object $object:ident ($($key:tt)+) (=> [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!([$($array)*])) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: [$($array:tt)*] $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!([$($array)*])) $($rest)*);
    };

    // Next value is a map.
    (@object $object:ident ($($key:tt)+) (=> {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!({$($map)*})) $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: {$($map:tt)*} $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!({$($map)*})) $($rest)*);
    };

    // Next value is an expression followed by comma.
    (@object $object:ident ($($key:tt)+) (=> $value:expr , $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!($value)) , $($rest)*);
    };

    (@object $object:ident ($($key:tt)+) (: $value:expr , $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!($value)) , $($rest)*);
    };

    // Last value is an expression with no trailing comma.
    (@object $object:ident ($($key:tt)+) (=> $value:expr) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!($value)));
    };

    (@object $object:ident ($($key:tt)+) (: $value:expr) $copy:tt) => {
        bson_internal!(@object $object [$($key)+] (bson_internal!($value)));
    };

    // Missing value for last entry. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)+) (=>) $copy:tt) => {
        // "unexpected end of macro invocation"
        bson_internal!();
    };

    (@object $object:ident ($($key:tt)+) (:) $copy:tt) => {
        // "unexpected end of macro invocation"
        bson_internal!();
    };

    // Missing colon and value for last entry. Trigger a reasonable error
    // message.
    (@object $object:ident ($($key:tt)+) () $copy:tt) => {
        // "unexpected end of macro invocation"
        bson_internal!();
    };

    // Misplaced colon. Trigger a reasonable error message.
    (@object $object:ident () (=> $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `:`".
        unimplemented!($colon);
    };

    (@object $object:ident () (: $($rest:tt)*) ($colon:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `:`".
        unimplemented!($colon);
    };

    // Found a comma inside a key. Trigger a reasonable error message.
    (@object $object:ident ($($key:tt)*) (, $($rest:tt)*) ($comma:tt $($copy:tt)*)) => {
        // Takes no arguments so "no rules expected the token `,`".
        unimplemented!($comma);
    };

    // Key is fully parenthesized. This avoids clippy double_parens false
    // positives because the parenthesization may be necessary here.
    (@object $object:ident () (($key:expr) => $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object ($key) (=> $($rest)*) (=> $($rest)*));
    };

    (@object $object:ident () (($key:expr) : $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object ($key) (: $($rest)*) (: $($rest)*));
    };

    // Munch a token into the current key.
    (@object $object:ident ($($key:tt)*) ($tt:tt $($rest:tt)*) $copy:tt) => {
        bson_internal!(@object $object ($($key)* $tt) ($($rest)*) ($($rest)*));
    };

    //////////////////////////////////////////////////////////////////////////
    // The main implementation.
    //
    // Must be invoked as: bson_internal!($($bson)+)
    //////////////////////////////////////////////////////////////////////////

    (null) => {
        $crate::Bson::Null
    };

    (true) => {
        $crate::Bson::Boolean(true)
    };

    (false) => {
        $crate::Bson::Boolean(false)
    };

    ([]) => {
        $crate::Bson::Array(vec![])
    };

    ([ $($tt:tt)+ ]) => {
        $crate::Bson::Array(bson_internal!(@array [] $($tt)+))
    };

    ({}) => {
        $crate::Bson::Document($crate::Document::new())
    };

    ({$($tt:tt)+}) => {
        $crate::Bson::Document({
            let mut object = $crate::Document::new();
            bson_internal!(@object object () ($($tt)+) ($($tt)+));
            object
        })
    };

    // Any Serialize type: numbers, strings, struct literals, variables etc.
    // Must be below every other rule.
    ($other:expr) => {
        ::std::convert::From::from($other)
        //$crate::to_bson(&$other).unwrap()
    };
}

// /// Construct a BSON value
// #[macro_export]
// macro_rules! bson {
//     ([]) => {{ $crate::Bson::Array(Vec::new()) }};

//     ([$($val:tt),*]) => {{
//         let mut array = Vec::new();

//         $(
//             array.push(bson!($val));
//         )*

//         $crate::Bson::Array(array)
//     }};

//     ([$val:expr]) => {{
//         $crate::Bson::Array(vec!(::std::convert::From::from($val)))
//     }};

//     ({ $($k:expr => $v:tt),* }) => {{
//         $crate::Bson::Document(doc! {
//             $(
//                 $k => $v
//             ),*
//         })
//     }};

//     ($val:expr) => {{
//         ::std::convert::From::from($val)
//     }};
// }

#[macro_export]
macro_rules! doc {
    () => {{ $crate::Document::new() }};
    ( $($tt:tt)+ ) => {{
        let mut object = $crate::Document::new();
        bson_internal!(@object object () ($($tt)+) ($($tt)+));
        object
    }};
}

// /// Construct a BSON Document
// #[macro_export]
// macro_rules! doc {
//     () => {{ $crate::Document::new() }};

//     ( $($key:expr => $val:tt),* ) => {{
        
//         let mut document = $crate::Document::new();

//         $(
//             document.insert_bson($key.to_owned(), bson!($val));
//         )*

//         document
//     }};
// }
