use std::fmt;
use serde::de::{Deserializer, Visitor, MapAccess};
use serde::Deserialize;

#[derive(Debug, Default)]
pub struct WrapperVec(pub Vec<ResourceObject>);

#[derive(Debug, Default, Deserialize)]
pub struct ResourceObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct Index {
    pub objects: WrapperVec,
}

impl<'de> Deserialize<'de> for WrapperVec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de>, {
        struct OuterVisitor;

        impl<'de> Visitor<'de> for OuterVisitor {
            type Value = WrapperVec;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a nonempty sequence of a sequence of numbers")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where V: MapAccess<'de>,
            {
                let mut out: Vec<ResourceObject> = Vec::new();

                while let Some((_, value)) = map.next_entry::<String, ResourceObject>()? {
                    out.push(value);
                }
                Ok(WrapperVec(out))
            }
        }
        deserializer.deserialize_map(OuterVisitor)
    }
}
