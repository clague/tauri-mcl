use parking_lot::Mutex;
use reqwest::Client;
use tokio::time::Duration;

use std::path::PathBuf;
use std::sync::Arc;
use mc_launcher_core::download::*;
use mc_launcher_core::instance::Instance;
use mc_launcher_core::deserialize::{AssetsIndex, VersionManifest};

use crate::state::MainState;
use crate::error::{Result, SerializedError};

#[tauri::command]
pub async fn download_json(state: tauri::State<'_, MainState>, version_id: String) -> Result<()> {
    let CHUNK_SIZE = 3000000;
    let PARALLELS = 64;
    let POLL_DURATION = Duration::from_millis(100);
    let VERSION_ROOT = "versions/";
    let LIBRARY_ROOT = "libraries/";
    let ASSETS_ROOT = "assets/";
    let MANIFEST_URL = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
    let RESOURCE_URL = "http://resources.download.minecraft.net";

    // let (progress_sender, mut progress_receiver) = mpsc::channel(PARALLELS);
    // let (controller_sender, mut controller_receiver) = mpsc::channel(5);
    // let (speed_sender, mut speed_receiver) = mpsc::channel(5);

    let client = Client::new();

    let manifest_file = PathBuf::from(VERSION_ROOT).join("version_manifest.json");

    let manifest: VersionManifest =
        if !manifest_file.is_file() {
            Task::new(MANIFEST_URL, &manifest_file, 0).download_file_std(&client).await?;
            serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(&manifest_file)?))?
        }
        else {
            if let Ok(r) = serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(&manifest_file)?)) {
                r
            }
            else {
                Task::new(MANIFEST_URL, &manifest_file, 0).download_file_std(&client).await?;
                serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(&manifest_file)?))?
            }
        };

    let mut version_file = PathBuf::from(VERSION_ROOT);
    version_file.push(&version_id);
    version_file.push(format!("{}.json", version_id));

    let instance: Instance = 
        if !version_file.is_file() {
            let mut url: String = String::new();
            for version in manifest.versions {
                if version.id == version_id {
                    url = version.url;
                }
            }
            if url.is_empty() {
                return Err(SerializedError::from("Invalid version"));
            }
            Task::new(&url, VERSION_ROOT, 0).download_file_std(&client).await?;
            serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(&version_file)?))?
        }
        else {
            if let Ok(r) = serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(&version_file)?)) {
                r
            }
            else {
                let mut url: String = String::new();
                for version in manifest.versions {
                    if version.id == version_id {
                        url = version.url;
                    }
                }
                if url.is_empty() {
                    return Err(SerializedError::from("Invalid version"));
                }
                Task::new(&url, VERSION_ROOT, 0).download_file_std(&client).await?;
                serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(&version_file)?))?
            }
        };

    let mut assets_index = PathBuf::from(ASSETS_ROOT);
    assets_index.push("indexes");
    assets_index.push(format!("{}.json", instance.assets_index.id));

    let assets: AssetsIndex =
        if !assets_index.is_file() {
            Task::new(&instance.assets_index.url, &assets_index, 0).download_file_std(&client).await?;
            serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(&assets_index)?))?
        }
        else {
            if let Ok(r) = serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(&assets_index)?)) {
                r
            }
            else {
                Task::new(&instance.assets_index.url, &assets_index, 0).download_file_std(&client).await?;
                serde_json::from_reader(std::io::BufReader::new(std::fs::File::open(&assets_index)?))?
            }
        };

    Ok(())
}

pub async fn check_json() {

}

pub struct DownloadState {
    queues: Vec<Arc<Mutex<Queue>>>,
}

impl DownloadState {
    pub fn new() -> DownloadState{
        DownloadState {
            queues: Vec::new(),
        }
    }
}