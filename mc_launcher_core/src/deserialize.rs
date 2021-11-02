use serde_json::{Value as Json};
use serde::{Deserialize, Deserializer, de::{MapAccess, Visitor}};

use std::fmt;

use crate::download::Task;

#[derive(Deserialize)]
pub struct LaunchArguments {
    pub game: Vec<Argument>,
    pub jvm: Vec<Argument>,
}

pub enum Argument {
    Value(String),
    Vec(Vec<String>),
    None,
}

impl<'de> Deserialize<'de> for Argument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
            D: Deserializer<'de>, {
        struct OuterVisitor;

        impl<'de> serde::de::Visitor<'de> for OuterVisitor {
            type Value = Argument;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("string, map, or a sequence of string")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                    V: serde::de::MapAccess<'de>, {
                let mut rules: Vec<Rule> = Vec::new();
                let mut out: Argument = Argument::None;

                while let Some(key) = map.next_key::<String>()? {
                    if key == "rules" {
                        rules = map.next_value::<Vec<Rule>>()?;
                    }
                    else if key == "value" {
                        out = map.next_value::<Argument>()?;
                    }
                }
                for rule in rules {
                    if !rule.check_rule() {
                        return Ok(Argument::None);
                    }
                }
                Ok(out)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where 
                    A: serde::de::SeqAccess<'de>, {
                let mut out: Vec<String> = Vec::new();

                while let Some(item)= seq.next_element::<String>()? {
                    out.push(item);
                }

                Ok(Argument::Vec(out))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                    E: serde::de::Error, {
                Ok(Argument::Value(v.to_owned()))
            }
        }
        deserializer.deserialize_any(OuterVisitor)
    }
}

#[derive(Deserialize, Clone)]
pub struct Rule {
    pub action: String,
    pub os: Option<OsRule>,
    pub features:  Option<FeatureRule>,
}

impl Rule {
    pub fn check_rule(&self) -> bool {
        let default = self.action == "allow";

        if let Some(os) = &self.os {
            if let Some(arch) = &os.arch {
                if (arch == "x86" && !cfg!(target_arch="x86")) ||
                    (arch == "arm" && !cfg!(target_arch="arm"))  {
                    return !default;
                }
            }
            if let Some(name) = &os.name {
                if (name == "windows" && !cfg!(target_os="windows")) ||
                    (name == "linux" && !cfg!(target_os="linux")) ||
                    (name == "osx" && !cfg!(target_os="macos")) {
                    return !default;
                }
            }
            if let Some(version) = &os.version {
                if let os_info::Version::Semantic(os_version, _, _) = os_info::get().version() {
                    if !version.contains(os_version.to_string().as_str()) {
                        return !default;
                    }
                }
                else {
                    return !default
                }
            }
        }
        if let Some(features) = &self.features {
            if let Some(_) = features.is_demo_user {
                return !default;
            }
        }
        default
    }
}

#[derive(Deserialize, Clone)]
pub struct OsRule {
    pub arch: Option<String>,
    pub name: Option<String>,
    pub version: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct FeatureRule {
    pub is_demo_user: Option<bool>,
    pub has_custom_resolution: Option<bool>,
}


#[derive(Deserialize)]
pub struct AssetConfig {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(rename="totalSize")]
    pub total_size: u64,
    pub url: String,
}

#[derive(Deserialize)]
pub struct MainDownloadItems{
    pub client: DownloadItem,
    pub client_mappings: DownloadItem,
}

#[derive(Deserialize)]
pub struct JavaVersion {
    pub component: String,
    #[serde(rename="majorVersion")]
    pub major_version: u32,
}

#[derive(Deserialize, Default)]
pub struct DownloadItem {
    #[serde(alias="id")]
    #[serde(default)]
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

impl Into<Task> for DownloadItem {
    fn into(self) -> Task {
        Task::new(&self.url, &self.path, self.size)
    }
}

pub struct Library {
    pub download_item: DownloadItem,
    pub name: String,
    pub is_native: bool,
    pub extract_exclude: Vec<String>,
}

impl<'de> Deserialize<'de> for Library {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
            D: Deserializer<'de>, {
        struct OuterVisitor;
        impl<'de> serde::de::Visitor<'de> for OuterVisitor {
            type Value = Library;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a library download option")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                    A: serde::de::MapAccess<'de>, {
                let mut native: String = String::new();
                let mut rules: Vec<Rule> = Vec::new();

                let mut download: LibraryDownload = LibraryDownload::default();
                let mut name: String = String::new();
                let mut is_native: bool = false;
                let mut extract_exclude: Vec<String> = Vec::new();
                        
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "name" => name = map.next_value::<String>()?,
                        "downloads" => {
                            download = map.next_value::<LibraryDownload>()?;
                            is_native = download.classifiers != None;
                        },
                        "natives" => {
                            let natives = map.next_value::<Json>()?;
                            if cfg!(target_os="linux") {
                                native = natives["linux"].as_str().unwrap_or_default().to_owned();
                            }
                            else if cfg!(target_os="windows") {
                                native = natives["windows"].as_str().unwrap_or_default().to_owned();
                            }
                            else if cfg!(target_os="macos") {
                                native = natives["osx"].as_str().unwrap_or_default().to_owned();
                            }
                            else {
                                native = "".to_owned()
                            }
                        },
                        "rules" => {
                            rules = map.next_value::<Vec<Rule>>()?;
                        },
                        "extract" => {
                            #[derive(Deserialize)]
                            struct Extract {
                                exclude: Vec<String>
                            }
                            extract_exclude = map.next_value::<Extract>()?.exclude;
                        },
                        _ => {},
                    }
                }

                for rule in rules {
                    if !rule.check_rule() {
                        return Err(serde::de::Error::custom("Rule not fit"));
                    }
                }

                let download_item: DownloadItem = if is_native {
                    let classifiers = download.classifiers.unwrap();
                    match serde_json::from_value::<DownloadItem>(classifiers[native].to_owned()) {
                        Ok(download) => download,
                        Err(_) => return Err(serde::de::Error::custom("No classifier")),
                    }
                } else {
                    download.artifact
                };

                Ok(Library {
                    download_item: download_item,
                    name: name,
                    is_native: is_native,
                    extract_exclude: extract_exclude,
                })
            }
        }
        deserializer.deserialize_map(OuterVisitor)
    }
}

#[derive(Deserialize, Default)]
pub struct LibraryDownload {
    pub artifact: DownloadItem,
    pub classifiers: Option<Json>
}

#[derive(Deserialize)]
pub struct Logging {
    pub client: ClientLogging,
}

#[derive(Deserialize)]
pub struct ClientLogging {
    pub argument: String,
    pub file: DownloadItem,
    #[serde(rename="type")]
    pub _type: String,
}

#[derive(Deserialize)]
pub struct VersionManifest {
    pub latest: LatestVersion,
    pub versions: Vec<Version>,
}

#[derive(Deserialize)]
pub struct LatestVersion {
    pub release: String,
    pub snapshot: String,
}

#[derive(Deserialize)]
pub struct Version {
    pub id: String,
    #[serde(rename="type")]
    pub _type: String,
    pub url: String,
    pub time: String,
    #[serde(rename="releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Default)]
pub struct WrapperVec(pub Vec<ResourceObject>);

#[derive(Debug, Default, Deserialize)]
pub struct ResourceObject {
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Default, Deserialize)]
pub struct AssetsIndex {
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
