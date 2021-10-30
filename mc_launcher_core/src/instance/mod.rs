use std::path::PathBuf;
use futures::stream::Map;
use serde::{Deserialize, Serialize, de::{self, Visitor}};
use serde_json::{Value as Json};
use anyhow::{Result, anyhow};

pub mod deserialize;
use deserialize::*;

#[derive(Deserialize)]
pub struct Instance {
    pub arguments: LaunchArguments,
    #[serde(rename="assetIndex")]
    pub asset_index: AssetConfig,
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

// impl InstanceConfig {
//     async fn from_version_json(json: &Json, condition: &Json) -> Result<(InstanceConfig, ReadResult)> {
//         let invalid_json_err = anyhow!("Invalid json!");
//         let allow = false;

//         let if_part = false;
//         let if_overflow = false;
        
        
//     }
// }

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