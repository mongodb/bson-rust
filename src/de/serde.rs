use std::{borrow::{Borrow, Cow}, convert::{TryFrom, TryInto}, fmt, vec};

use lazy_static::__Deref;
use serde::de::{
    self,
    DeserializeSeed,
    Deserializer as _,
    EnumAccess,
    Error,
    MapAccess,
    SeqAccess,
    Unexpected,
    VariantAccess,
    Visitor,
};
use serde::Deserialize;
use serde_bytes::ByteBuf;

use crate::{
    bson::{Binary, Bson, DbPointer, JavaScriptCodeWithScope, Regex, Timestamp},
    datetime::DateTime,
    document::{Document, IntoIter},
    oid::ObjectId,
    spec::BinarySubtype,
    Decimal128,
};

use super::raw::Decimal128Access;

pub(crate) struct BsonVisitor;

impl<'de> Deserialize<'de> for ObjectId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum OidHelper {
            HexString(String),
            Bson(Bson),
        }

        match OidHelper::deserialize(deserializer)
            .map_err(|_| de::Error::custom("expected ObjectId extended document or hex string"))?
        {
            OidHelper::HexString(s) => ObjectId::parse_str(&s).map_err(de::Error::custom),
            OidHelper::Bson(bson) => match bson {
                Bson::ObjectId(oid) => Ok(oid),
                bson => {
                    let err = format!(
                        "expected objectId extended document or hex string, found {}",
                        bson
                    );
                    Err(de::Error::invalid_type(Unexpected::Map, &&err[..]))
                }
            },
        }
    }
}

impl<'de> Deserialize<'de> for Document {
    /// Deserialize this value given this `Deserializer`.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_map(BsonVisitor).and_then(|bson| {
            if let Bson::Document(doc) = bson {
                Ok(doc)
            } else {
                let err = format!("expected document, found extended JSON data type: {}", bson);
                Err(de::Error::invalid_type(Unexpected::Map, &&err[..]))
            }
        })
    }
}

impl<'de> Deserialize<'de> for Bson {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Bson, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(BsonVisitor)
    }
}

impl<'de> Visitor<'de> for BsonVisitor {
    type Value = Bson;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("expecting a Bson")
    }

    #[inline]
    fn visit_bool<E>(self, value: bool) -> Result<Bson, E>
    where
        E: Error,
    {
        Ok(Bson::Boolean(value))
    }

    #[inline]
    fn visit_i8<E>(self, value: i8) -> Result<Bson, E>
    where
        E: Error,
    {
        Ok(Bson::Int32(value as i32))
    }

    #[inline]
    fn visit_u8<E>(self, value: u8) -> Result<Bson, E>
    where
        E: Error,
    {
        convert_unsigned_to_signed(value as u64)
    }

    #[inline]
    fn visit_i16<E>(self, value: i16) -> Result<Bson, E>
    where
        E: Error,
    {
        Ok(Bson::Int32(value as i32))
    }

    #[inline]
    fn visit_u16<E>(self, value: u16) -> Result<Bson, E>
    where
        E: Error,
    {
        convert_unsigned_to_signed(value as u64)
    }

    #[inline]
    fn visit_i32<E>(self, value: i32) -> Result<Bson, E>
    where
        E: Error,
    {
        Ok(Bson::Int32(value))
    }

    #[inline]
    fn visit_u32<E>(self, value: u32) -> Result<Bson, E>
    where
        E: Error,
    {
        convert_unsigned_to_signed(value as u64)
    }

    #[inline]
    fn visit_i64<E>(self, value: i64) -> Result<Bson, E>
    where
        E: Error,
    {
        Ok(Bson::Int64(value))
    }

    #[inline]
    fn visit_u64<E>(self, value: u64) -> Result<Bson, E>
    where
        E: Error,
    {
        convert_unsigned_to_signed(value)
    }

    #[inline]
    fn visit_f64<E>(self, value: f64) -> Result<Bson, E> {
        Ok(Bson::Double(value))
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<Bson, E>
    where
        E: de::Error,
    {
        self.visit_string(String::from(value))
    }

    #[inline]
    fn visit_string<E>(self, value: String) -> Result<Bson, E> {
        Ok(Bson::String(value))
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Bson, E> {
        Ok(Bson::Null)
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Bson, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Bson, E> {
        Ok(Bson::Null)
    }

    #[inline]
    fn visit_seq<V>(self, mut visitor: V) -> Result<Bson, V::Error>
    where
        V: SeqAccess<'de>,
    {
        let mut values = Vec::new();

        while let Some(elem) = visitor.next_element()? {
            values.push(elem);
        }

        Ok(Bson::Array(values))
    }

    fn visit_map<V>(self, mut visitor: V) -> Result<Bson, V::Error>
    where
        V: MapAccess<'de>,
    {
        use crate::extjson;

        let mut doc = Document::new();

        /// A wrapper struct used to deserialize a string that is either borrowed or owned.
        /// When deserializing from raw BSON, this can be borrowed, but from a `Document` it has to be
        /// owned.
        struct MaybeBorrowed<'a>(Cow<'a, str>);
        impl<'de> Deserialize<'de> for MaybeBorrowed<'de> {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de> {
                struct MaybeBorrowedVisitor;

                impl<'de> Visitor<'de> for MaybeBorrowedVisitor {
                    type Value = MaybeBorrowed<'de>;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a string")
                    }

                    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
                    where
                        E: Error, {
                        Ok(MaybeBorrowed(Cow::Borrowed(v)))
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: Error, {
                        Ok(MaybeBorrowed(Cow::Owned(v.to_string())))
                    }
                }

                deserializer.deserialize_str(MaybeBorrowedVisitor)
            }
        }

        while let Some(k) = visitor.next_key::<MaybeBorrowed>()? {
            match k.0.borrow() {
                "$oid" => {
                    let hex: String = visitor.next_value()?;
                    return Ok(Bson::ObjectId(ObjectId::parse_str(hex.as_str()).map_err(
                        |_| {
                            V::Error::invalid_value(
                                Unexpected::Str(&hex),
                                &"24-character, big-endian hex string",
                            )
                        },
                    )?));
                }
                "$symbol" => {
                    let string: String = visitor.next_value()?;
                    return Ok(Bson::Symbol(string));
                }

                "$numberInt" => {
                    let string: String = visitor.next_value()?;
                    return Ok(Bson::Int32(string.parse().map_err(|_| {
                        V::Error::invalid_value(
                            Unexpected::Str(&string),
                            &"32-bit signed integer as a string",
                        )
                    })?));
                }

                "$numberLong" => {
                    let string: String = visitor.next_value()?;
                    return Ok(Bson::Int64(string.parse().map_err(|_| {
                        V::Error::invalid_value(
                            Unexpected::Str(&string),
                            &"64-bit signed integer as a string",
                        )
                    })?));
                }

                "$numberDouble" => {
                    let string: String = visitor.next_value()?;
                    let val = match string.as_str() {
                        "Infinity" => Bson::Double(std::f64::INFINITY),
                        "-Infinity" => Bson::Double(std::f64::NEG_INFINITY),
                        "NaN" => Bson::Double(std::f64::NAN),
                        _ => Bson::Int64(string.parse().map_err(|_| {
                            V::Error::invalid_value(
                                Unexpected::Str(&string),
                                &"64-bit signed integer as a string",
                            )
                        })?),
                    };
                    return Ok(val);
                }

                "$binary" => {
                    let v = visitor.next_value::<extjson::models::BinaryBody>()?;
                    return Ok(Bson::Binary(
                        extjson::models::Binary { body: v }
                            .parse()
                            .map_err(Error::custom)?,
                    ));
                }

                "$code" => {
                    let code = visitor.next_value::<String>()?;
                    if let Some(key) = visitor.next_key::<MaybeBorrowed>()? {
                        if key.0.deref() == "$scope" {
                            let scope = visitor.next_value::<Document>()?;
                            return Ok(Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
                                code,
                                scope,
                            }));
                        } else {
                            return Err(Error::unknown_field(key.0.deref(), &["$scope"]))
                        }
                    } else {
                        return Ok(Bson::JavaScriptCode(code));
                    }
                }

                "$scope" => {
                    let scope = visitor.next_value::<Document>()?;
                    if let Some(key) = visitor.next_key::<MaybeBorrowed>()? {
                        if key.0.deref() == "$code" {
                            let code = visitor.next_value::<String>()?;
                            return Ok(Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
                                code,
                                scope,
                            }));
                        } else {
                            return Err(Error::unknown_field(key.0.deref(), &["$code"]))
                        }
                    } else {
                        return Err(Error::missing_field("$code"));
                    }
                }

                "$timestamp" => {
                    let ts = visitor.next_value::<extjson::models::TimestampBody>()?;
                    return Ok(Bson::Timestamp(Timestamp {
                        time: ts.t,
                        increment: ts.i,
                    }));
                }

                "$regularExpression" => {
                    let re = visitor.next_value::<extjson::models::RegexBody>()?;
                    return Ok(Bson::RegularExpression(Regex {
                        pattern: re.pattern,
                        options: re.options,
                    }));
                }

                "$dbPointer" => {
                    let dbp = visitor.next_value::<extjson::models::DbPointerBody>()?;
                    return Ok(Bson::DbPointer(DbPointer {
                        id: dbp.id.parse().map_err(Error::custom)?,
                        namespace: dbp.ref_ns,
                    }));
                }

                "$date" => {
                    let dt = visitor.next_value::<extjson::models::DateTimeBody>()?;
                    return Ok(Bson::DateTime(
                        extjson::models::DateTime { body: dt }
                            .parse()
                            .map_err(Error::custom)?,
                    ));
                }

                "$maxKey" => {
                    let i = visitor.next_value::<u8>()?;
                    return extjson::models::MaxKey { value: i }
                        .parse()
                        .map_err(Error::custom);
                }

                "$minKey" => {
                    let i = visitor.next_value::<u8>()?;
                    return extjson::models::MinKey { value: i }
                        .parse()
                        .map_err(Error::custom);
                }

                "$undefined" => {
                    let b = visitor.next_value::<bool>()?;
                    return extjson::models::Undefined { value: b }
                        .parse()
                        .map_err(Error::custom);
                }

                "$numberDecimalBytes" => {
                    let bytes = visitor.next_value::<ByteBuf>()?;
                    let arr = bytes.into_vec().try_into().map_err(|v: Vec<u8>| {
                        Error::custom(format!(
                            "expected decimal128 as byte buffer, instead got buffer of length {}",
                            v.len()
                        ))
                    })?;

                    #[cfg(not(feature = "decimal128"))]
                    return Ok(Bson::Decimal128(Decimal128 { bytes: arr }));

                    #[cfg(feature = "decimal128")]
                    unsafe {
                        return Ok(Bson::Decimal128(Decimal128::from_raw_bytes_le(arr)));
                    }
                }

                k => {
                    let v = visitor.next_value::<Bson>()?;
                    doc.insert(k, v);
                }
            }
        }

        Ok(Bson::Document(doc))
    }

    #[inline]
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Bson, E>
    where
        E: Error,
    {
        Ok(Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: v.to_vec(),
        }))
    }

    #[inline]
    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Bson, E>
    where
        E: Error,
    {
        Ok(Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: v,
        }))
    }
}

fn convert_unsigned_to_signed<E>(value: u64) -> Result<Bson, E>
where
    E: Error,
{
    if let Ok(int32) = i32::try_from(value) {
        Ok(Bson::Int32(int32))
    } else if let Ok(int64) = i64::try_from(value) {
        Ok(Bson::Int64(int64))
    } else {
        Err(Error::custom(format!(
            "cannot represent {} as a signed number",
            value
        )))
    }
}

/// Serde Deserializer
pub struct Deserializer {
    value: Option<Bson>,
}

impl Deserializer {
    pub fn new(value: Bson) -> Deserializer {
        Deserializer { value: Some(value) }
    }
}

macro_rules! forward_to_deserialize {
    ($(
        $name:ident ( $( $arg:ident : $ty:ty ),* );
    )*) => {
        $(
            forward_to_deserialize!{
                func: $name ( $( $arg: $ty ),* );
            }
        )*
    };

    (func: deserialize_enum ( $( $arg:ident : $ty:ty ),* );) => {
        fn deserialize_enum<V>(
            self,
            $(_: $ty,)*
            _visitor: V,
        ) -> ::std::result::Result<V::Value, Self::Error>
            where V: ::serde::de::Visitor<'de>
        {
            Err(::serde::de::Error::custom("unexpected Enum"))
        }
    };

    (func: $name:ident ( $( $arg:ident : $ty:ty ),* );) => {
        #[inline]
        fn $name<V>(
            self,
            $(_: $ty,)*
            visitor: V,
        ) -> ::std::result::Result<V::Value, Self::Error>
            where V: ::serde::de::Visitor<'de>
        {
            self.deserialize_any(visitor)
        }
    };
}

impl<'de> de::Deserializer<'de> for Deserializer {
    type Error = crate::de::Error;

    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> crate::de::Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = match self.value.take() {
            Some(value) => value,
            None => return Err(crate::de::Error::EndOfStream),
        };

        match value {
            Bson::Double(v) => visitor.visit_f64(v),
            Bson::String(v) => visitor.visit_string(v),
            Bson::Array(v) => {
                let len = v.len();
                visitor.visit_seq(SeqDeserializer {
                    iter: v.into_iter(),
                    len,
                })
            }
            Bson::Document(v) => {
                let len = v.len();
                visitor.visit_map(MapDeserializer {
                    iter: v.into_iter(),
                    value: None,
                    len,
                })
            }
            Bson::Boolean(v) => visitor.visit_bool(v),
            Bson::Null => visitor.visit_unit(),
            Bson::Int32(v) => visitor.visit_i32(v),
            Bson::Int64(v) => visitor.visit_i64(v),
            Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                bytes,
            }) => visitor.visit_byte_buf(bytes),
            binary @ Bson::Binary(..) => visitor.visit_map(MapDeserializer {
                iter: binary.into_extended_document().into_iter(),
                value: None,
                len: 2,
            }),
            Bson::Decimal128(d) => visitor.visit_map(Decimal128Access::new(d)),
            _ => {
                let doc = value.into_extended_document();
                let len = doc.len();
                visitor.visit_map(MapDeserializer {
                    iter: doc.into_iter(),
                    value: None,
                    len,
                })
            }
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> crate::de::Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.value {
            Some(Bson::Null) => visitor.visit_none(),
            Some(_) => visitor.visit_some(self),
            None => Err(crate::de::Error::EndOfStream),
        }
    }

    #[inline]
    fn deserialize_enum<V>(
        mut self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> crate::de::Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let value = match self.value.take() {
            Some(Bson::Document(value)) => value,
            Some(Bson::String(variant)) => {
                return visitor.visit_enum(EnumDeserializer {
                    val: Bson::String(variant),
                    deserializer: VariantDeserializer { val: None },
                });
            }
            Some(v) => {
                return Err(crate::de::Error::invalid_type(
                    v.as_unexpected(),
                    &"expected an enum",
                ));
            }
            None => {
                return Err(crate::de::Error::EndOfStream);
            }
        };

        let mut iter = value.into_iter();

        let (variant, value) = match iter.next() {
            Some(v) => v,
            None => {
                return Err(crate::de::Error::invalid_value(
                    Unexpected::Other("empty document"),
                    &"variant name",
                ))
            }
        };

        // enums are encoded in json as maps with a single key:value pair
        match iter.next() {
            Some((k, _)) => Err(crate::de::Error::invalid_value(
                Unexpected::Map,
                &format!("expected map with a single key, got extra key \"{}\"", k).as_str(),
            )),
            None => visitor.visit_enum(EnumDeserializer {
                val: Bson::String(variant),
                deserializer: VariantDeserializer { val: Some(value) },
            }),
        }
    }

    #[inline]
    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> crate::de::Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    forward_to_deserialize! {
        deserialize_bool();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_i8();
        deserialize_i16();
        deserialize_i32();
        deserialize_i64();
        deserialize_f32();
        deserialize_f64();
        deserialize_char();
        deserialize_str();
        deserialize_string();
        deserialize_unit();
        deserialize_seq();
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_tuple(len: usize);
        deserialize_identifier();
        deserialize_ignored_any();
        deserialize_byte_buf();
    }
}

struct EnumDeserializer {
    val: Bson,
    deserializer: VariantDeserializer,
}

impl<'de> EnumAccess<'de> for EnumDeserializer {
    type Error = crate::de::Error;
    type Variant = VariantDeserializer;
    fn variant_seed<V>(self, seed: V) -> crate::de::Result<(V::Value, Self::Variant)>
    where
        V: DeserializeSeed<'de>,
    {
        let dec = Deserializer::new(self.val);
        let value = seed.deserialize(dec)?;
        Ok((value, self.deserializer))
    }
}

struct VariantDeserializer {
    val: Option<Bson>,
}

impl<'de> VariantAccess<'de> for VariantDeserializer {
    type Error = crate::de::Error;

    fn unit_variant(mut self) -> crate::de::Result<()> {
        match self.val.take() {
            None => Ok(()),
            Some(val) => Bson::deserialize(Deserializer::new(val)).map(|_| ()),
        }
    }

    fn newtype_variant_seed<T>(mut self, seed: T) -> crate::de::Result<T::Value>
    where
        T: DeserializeSeed<'de>,
    {
        let dec = Deserializer::new(self.val.take().ok_or(crate::de::Error::EndOfStream)?);
        seed.deserialize(dec)
    }

    fn tuple_variant<V>(mut self, _len: usize, visitor: V) -> crate::de::Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.val.take().ok_or(crate::de::Error::EndOfStream)? {
            Bson::Array(fields) => {
                let de = SeqDeserializer {
                    len: fields.len(),
                    iter: fields.into_iter(),
                };
                de.deserialize_any(visitor)
            }
            other => Err(crate::de::Error::invalid_type(
                other.as_unexpected(),
                &"expected a tuple",
            )),
        }
    }

    fn struct_variant<V>(
        mut self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> crate::de::Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.val.take().ok_or(crate::de::Error::EndOfStream)? {
            Bson::Document(fields) => {
                let de = MapDeserializer {
                    len: fields.len(),
                    iter: fields.into_iter(),
                    value: None,
                };
                de.deserialize_any(visitor)
            }
            ref other => Err(crate::de::Error::invalid_type(
                other.as_unexpected(),
                &"expected a struct",
            )),
        }
    }
}

struct SeqDeserializer {
    iter: vec::IntoIter<Bson>,
    len: usize,
}

impl<'de> de::Deserializer<'de> for SeqDeserializer {
    type Error = crate::de::Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> crate::de::Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.len == 0 {
            visitor.visit_unit()
        } else {
            visitor.visit_seq(self)
        }
    }

    forward_to_deserialize! {
        deserialize_bool();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_i8();
        deserialize_i16();
        deserialize_i32();
        deserialize_i64();
        deserialize_f32();
        deserialize_f64();
        deserialize_char();
        deserialize_str();
        deserialize_string();
        deserialize_unit();
        deserialize_option();
        deserialize_seq();
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_tuple(len: usize);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_identifier();
        deserialize_ignored_any();
        deserialize_byte_buf();
    }
}

impl<'de> SeqAccess<'de> for SeqDeserializer {
    type Error = crate::de::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> crate::de::Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            None => Ok(None),
            Some(value) => {
                self.len -= 1;
                let de = Deserializer::new(value);
                match seed.deserialize(de) {
                    Ok(value) => Ok(Some(value)),
                    Err(err) => Err(err),
                }
            }
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

pub(crate) struct MapDeserializer {
    pub(crate) iter: IntoIter,
    pub(crate) value: Option<Bson>,
    pub(crate) len: usize,
}

impl MapDeserializer {
    pub(crate) fn new(doc: Document) -> Self {
        let len = doc.len();
        MapDeserializer {
            iter: doc.into_iter(),
            len,
            value: None,
        }
    }
}

impl<'de> MapAccess<'de> for MapDeserializer {
    type Error = crate::de::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> crate::de::Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.len -= 1;
                self.value = Some(value);

                let de = Deserializer::new(Bson::String(key));
                match seed.deserialize(de) {
                    Ok(val) => Ok(Some(val)),
                    Err(e) => Err(e),
                }
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> crate::de::Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        let value = self.value.take().ok_or(crate::de::Error::EndOfStream)?;
        let de = Deserializer::new(value);
        seed.deserialize(de)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de> de::Deserializer<'de> for MapDeserializer {
    type Error = crate::de::Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> crate::de::Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    forward_to_deserialize! {
        deserialize_bool();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_i8();
        deserialize_i16();
        deserialize_i32();
        deserialize_i64();
        deserialize_f32();
        deserialize_f64();
        deserialize_char();
        deserialize_str();
        deserialize_string();
        deserialize_unit();
        deserialize_option();
        deserialize_seq();
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_tuple(len: usize);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_identifier();
        deserialize_ignored_any();
        deserialize_byte_buf();
    }
}

impl<'de> Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        match Bson::deserialize(deserializer)? {
            Bson::Timestamp(timestamp) => Ok(timestamp),
            _ => Err(D::Error::custom("expecting Timestamp")),
        }
    }
}

impl<'de> Deserialize<'de> for Regex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        match Bson::deserialize(deserializer)? {
            Bson::RegularExpression(regex) => Ok(regex),
            _ => Err(D::Error::custom("expecting Regex")),
        }
    }
}

impl<'de> Deserialize<'de> for JavaScriptCodeWithScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        match Bson::deserialize(deserializer)? {
            Bson::JavaScriptCodeWithScope(code_with_scope) => Ok(code_with_scope),
            _ => Err(D::Error::custom("expecting JavaScriptCodeWithScope")),
        }
    }
}

impl<'de> Deserialize<'de> for Binary {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        match Bson::deserialize(deserializer)? {
            Bson::Binary(binary) => Ok(binary),
            _ => Err(D::Error::custom("expecting Binary")),
        }
    }
}

#[cfg(feature = "decimal128")]
impl<'de> Deserialize<'de> for Decimal128 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        match Bson::deserialize(deserializer)? {
            Bson::Decimal128(d128) => Ok(d128),
            _ => Err(D::Error::custom("expecting Decimal128")),
        }
    }
}

impl<'de> Deserialize<'de> for DateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        match Bson::deserialize(deserializer)? {
            Bson::DateTime(dt) => Ok(dt),
            b => {
                println!("expecting date time got {:?}", b);
                Err(D::Error::custom("expecting DateTime"))
            }
        }
    }
}

impl<'de> Deserialize<'de> for DbPointer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        match Bson::deserialize(deserializer)? {
            Bson::DbPointer(db_pointer) => Ok(db_pointer),
            _ => Err(D::Error::custom("expecting DbPointer")),
        }
    }
}
