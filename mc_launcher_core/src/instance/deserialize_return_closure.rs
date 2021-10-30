use serde_json::{Value as Json};
use serde::{Deserialize, Deserializer};

use std::{fmt, path::PathBuf};

use crate::download::Task;

#[derive(Deserialize)]
pub struct LaunchArguments {
    pub game: Vec<Argument>,
    pub jvm: Vec<Argument>,
}

pub enum Argument {
    Value(String),
    ValueVec(Vec<String>),
    Condition(Box<dyn FnOnce(Rule) -> Option<Argument>>),
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
                if rules.is_empty() {
                    Ok(out)
                }
                else { 
                    Ok(Argument::Condition(Box::new(move |condition| {
                        for rule in rules {
                            if !rule.check_rule(&condition) {
                                return None;
                            }
                        }
                        Some(out)
                    })))
                }
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where 
                    A: serde::de::SeqAccess<'de>, {
                let mut out: Vec<String> = Vec::new();

                while let Some(item)= seq.next_element::<String>()? {
                    out.push(item);
                }

                Ok(Argument::ValueVec(out))
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
    pub fn check_rule(&self, condition: &Rule) -> bool {
        // self.action generally as "allow"
        let allow = self.action == condition.action;
        
        if let Some(os_req) = &self.os {
            if let Some(os_con) = &condition.os {
                if let Some(arch_req) = &os_req.arch {
                    if let Some(arch_con) = &os_con.arch {
                        if !arch_req.contains(arch_con) {
                            return !allow;
                        }
                    } else {
                        return !allow;
                    }
                }

                if let Some(name_req) = &os_req.name {
                    if let Some(name_con) = &os_con.name {
                        if !name_req.contains(name_con) {
                            return !allow;
                        }
                    } else {
                        return !allow;
                    }
                }

                if let Some(version_req) = &os_req.version {
                    if let Some(version_con) = &os_con.version {
                        if !version_req.contains(version_con) {
                            return !allow;
                        }
                    } else {
                        return !allow;
                    }
                }
            }
            else {
                return !allow;
            }
        }
        if let Some(features_req) = &self.features {
            if let Some(features_con) = &condition.features {
                if let Some(idu_req) = &features_req.is_demo_user {
                    if let Some(idu_con) = &features_con.is_demo_user {
                        if idu_req != idu_con {
                            return !allow;
                        }
                    } else {
                        return !allow;
                    }
                }
                if let Some(hcr_req) = &features_req.has_custom_resolution {
                    if let Some(hcr_con) = &features_con.has_custom_resolution {
                        if hcr_req != hcr_con {
                            return !allow;
                        }
                    } else {
                        return !allow;
                    }
                }
            } else {
                return !allow;
            }
        }
        allow
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

pub enum DownloadItemGen {
    Generator(Box<dyn FnOnce(&Rule) -> Option<DownloadItem>>),
    DownloadItem(DownloadItem),
    None,
}

pub struct Library {
    pub download_item: DownloadItemGen,
    pub name: String,
    pub has_rule: bool,
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
                let mut has_rule: bool = false;
                let mut is_native: bool = false;
                let mut extract_exclude: Vec<String> = Vec::new();

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "name" => name = map.next_value::<String>()?,
                        "downloads" => {
                            download = map.next_value::<LibraryDownload>()?;
                            is_native = download.classifier != None;
                        },
                        "natives" => {
                            let natives = map.next_value::<Json>()?;
                            if cfg!(linux) {
                                native = natives["linux"].as_str().unwrap_or_default().to_owned();
                            }
                            else if cfg!(windows) {
                                native = natives["windows"].as_str().unwrap_or_default().to_owned();
                            }
                            else if cfg!(macos) {
                                native = natives["macos"].as_str().unwrap_or_default().to_owned();
                            }
                            else {
                                native = "".to_owned()
                            }
                        },
                        "rules" => {
                            rules = map.next_value::<Vec<Rule>>()?;
                            has_rule = true;
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

                let download_item = if has_rule {
                    DownloadItemGen::Generator(Box::new(move |condition| {
                        for rule in rules {
                            if !rule.check_rule(&condition) {
                                return None;
                            }
                        }
                        if is_native {
                            let classifier = download.classifier.unwrap();
                            match serde_json::from_value::<DownloadItem>(classifier[native].to_owned()) {
                                Ok(download) => Some(download),
                                Err(_) => None,
                            }
                        } else {
                            Some(download.artifact)
                        }
                    }))
                } else {
                    if is_native {
                        let classifier = download.classifier.unwrap();
                        match serde_json::from_value::<DownloadItem>(classifier[native].to_owned()) {
                            Ok(download) => DownloadItemGen::DownloadItem(download),
                            Err(_) => DownloadItemGen::None,
                        }
                    } else {
                        DownloadItemGen::DownloadItem(download.artifact)
                    }
                };

                Ok(Library {
                    download_item: download_item,
                    name: name,
                    has_rule: has_rule,
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
    pub classifier: Option<Json>
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