use serde::{Deserialize, de::{self, Visitor}};
use serde_json::{Value as Json};

use crate::deserialize::{LaunchArguments, AssetConfig,
    MainDownloadItems, JavaVersion, Library, Logging};

#[derive(Deserialize)]
pub struct Instance {
    pub arguments: LaunchArguments,
    #[serde(rename="assetIndex")]
    pub assets_index: AssetConfig,
    #[serde(rename="downloads")]
    pub main_downloads: MainDownloadItems,

    #[serde(rename="id")]
    pub version: String,
    #[serde(rename="javaVersion")]
    pub java_version: JavaVersion,

    #[serde(deserialize_with="deserialize_skip_error")]
    pub libraries: Vec<Library>,
    pub logging: Logging,

    #[serde(rename="mainClass")]
    pub main_class: String,
}

pub fn deserialize_skip_error<'de, D>(deserializer: D) -> Result<Vec<Library>, D::Error>
where
        D: de::Deserializer<'de>, {
    struct OuterVisitor;
    impl<'de> Visitor<'de> for OuterVisitor {
        type Value = Vec<Library>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a sequence of library stuff")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
                A: de::SeqAccess<'de>, {
            let mut out = Vec::new();
            while let Some(element) = seq.next_element::<Json>()? {
                match serde_json::from_value::<Library>(element) {
                    Ok(library) => out.push(library),
                    Err(_) => {},
                }
            }
            Ok(out)
        }
    }
    deserializer.deserialize_seq(OuterVisitor)
}
pub enum ReadResult {
    Part,
    Fixed,
    Overflow,
    PartOverflow
}