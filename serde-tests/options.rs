use std::collections::HashMap;

use bson::{doc, Bson, DeserializerOptions, SerializerOptions};

use serde::{
    ser::{
        SerializeMap,
        SerializeSeq,
        SerializeStruct,
        SerializeStructVariant,
        SerializeTupleStruct,
        SerializeTupleVariant,
    },
    Deserialize,
    Serialize,
};

/// Type whose serialize and deserialize implementations assert that the (de)serializer
/// is not human readable.
#[derive(Deserialize)]
struct Foo {
    a: i32,
    unit: Unit,
    tuple: Tuple,
    map: Map,
    unit_variant: Bar,
    tuple_variant: Bar,
    struct_variant: Bar,
    seq: Seq,
}

impl Serialize for Foo {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        assert!(!serializer.is_human_readable());

        let mut state = serializer.serialize_struct("Foo", 7)?;
        state.serialize_field("a", &self.a)?;
        state.serialize_field("unit", &self.unit)?;
        state.serialize_field("tuple", &self.tuple)?;
        state.serialize_field("map", &self.map)?;
        state.serialize_field("unit_variant", &self.unit_variant)?;
        state.serialize_field("tuple_variant", &self.tuple_variant)?;
        state.serialize_field("struct_variant", &self.struct_variant)?;
        state.serialize_field("seq", &self.seq)?;
        state.end()
    }
}

#[derive(Deserialize)]
enum Bar {
    Unit,
    Tuple(Unit),
    Struct { a: Unit },
}

impl Serialize for Bar {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        assert!(!serializer.is_human_readable());
        match self {
            Self::Unit => serializer.serialize_unit_variant("Bar", 0, "Unit"),
            Self::Tuple(t) => {
                let mut state = serializer.serialize_tuple_variant("Bar", 1, "Tuple", 1)?;
                state.serialize_field(t)?;
                state.end()
            }
            Self::Struct { a } => {
                let mut state = serializer.serialize_struct_variant("Foo", 2, "Struct", 1)?;
                state.serialize_field("a", a)?;
                state.end()
            }
        }
    }
}

struct Unit;

impl Serialize for Unit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        assert!(!serializer.is_human_readable());
        serializer.serialize_unit_struct("Unit")
    }
}

impl<'de> Deserialize<'de> for Unit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        assert!(!deserializer.is_human_readable());
        Ok(Unit)
    }
}

#[derive(Deserialize)]
struct Tuple(Unit);

impl Serialize for Tuple {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        assert!(!serializer.is_human_readable());
        let mut state = serializer.serialize_tuple_struct("Tuple", 1)?;
        state.serialize_field(&self.0)?;
        state.end()
    }
}

struct Map {
    map: HashMap<String, Unit>,
}

impl Serialize for Map {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        assert!(!serializer.is_human_readable());

        let mut state = serializer.serialize_map(Some(self.map.len()))?;
        for (k, v) in self.map.iter() {
            state.serialize_entry(k, &v)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for Map {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        assert!(!deserializer.is_human_readable());
        let map = Deserialize::deserialize(deserializer)?;
        Ok(Self { map })
    }
}

struct Seq {
    seq: Vec<Unit>,
}

impl Serialize for Seq {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        assert!(!serializer.is_human_readable());

        let mut state = serializer.serialize_seq(Some(self.seq.len()))?;
        for v in self.seq.iter() {
            state.serialize_element(&v)?;
        }
        state.end()
    }
}

impl<'de> Deserialize<'de> for Seq {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        assert!(!deserializer.is_human_readable());
        let v = Vec::<Unit>::deserialize(deserializer)?;
        Ok(Self { seq: v })
    }
}

#[test]
fn to_bson_with_options() {
    let options = SerializerOptions::builder().human_readable(false).build();

    let mut hm = HashMap::new();
    hm.insert("ok".to_string(), Unit);
    hm.insert("other".to_string(), Unit);
    let f = Foo {
        a: 5,
        unit: Unit,
        tuple: Tuple(Unit),
        unit_variant: Bar::Unit,
        tuple_variant: Bar::Tuple(Unit),
        struct_variant: Bar::Struct { a: Unit },
        map: Map { map: hm },
        seq: Seq {
            seq: vec![Unit, Unit],
        },
    };
    bson::to_bson_with_options(&f, options).unwrap();
}

#[test]
fn from_bson_with_options() {
    let options = DeserializerOptions::builder().human_readable(false).build();

    let doc = doc! {
        "a": 5,
        "unit": Bson::Null,
        "tuple": [Bson::Null],
        "unit_variant": { "Unit": Bson::Null },
        "tuple_variant": { "Tuple": [Bson::Null] },
        "struct_variant": { "Struct": { "a": Bson::Null } },
        "map": { "a": Bson::Null, "b": Bson::Null },
        "seq": [Bson::Null, Bson::Null],
    };

    let _: Foo = bson::from_bson_with_options(doc.into(), options).unwrap();
}
