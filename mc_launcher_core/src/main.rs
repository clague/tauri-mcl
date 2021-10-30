use futures::TryFutureExt;
use mc_launcher_core::account::AccountInfo;
use mc_launcher_core::config::LauncherConfig;
use mc_launcher_core::download::{*, deserialize::Index};
use mc_launcher_core::instance::Instance;

use anyhow::{Result, anyhow};
use mc_launcher_core::instance::deserialize::{DownloadItem, FeatureRule, Library, OsRule, Rule};
use serde_json::value::Value as Json;
use tokio::sync::mpsc;

fn main() {
    tokio_uring::start(run()).unwrap()
}
async fn run() -> Result<()> {
    let mut info = AccountInfo::default();

    // test_login(&mut info).await?;
    // test_save(&mut info).await?;

    // info = test_load().await?;
    // print_profile(&info);
    // println!("{}", info.refresh_token);
    let MANIFEST_URL = "https://launchermeta.mojang.com/mc/game/version_manifest.json";
    let RESOURCE_URL = "http://resources.download.minecraft.net";
    //let RESOURCE_URL = "https://bmclapi2.bangbang93.com/assets";

    let client = reqwest::Client::new();
    Task::new(MANIFEST_URL, "versions/", 0).download_file(&client).await?;

    let file = std::fs::File::open("versions/version_manifest.json")?;
    let json: Json = serde_json::from_reader(std::io::BufReader::new(file))?;

    for version in json["versions"].as_array().ok_or(anyhow!("can't parse json!"))? {
        if (version["id"]) == "1.17.1" {
            std::fs::create_dir_all("versions/1.17.1")?;

            Task::new(version["url"].as_str().ok_or(anyhow!("can't parse json!"))?,
                                "versions/1.17.1/", 0)
                .download_file(&client)
                .await?;
            break;
        }
    }

    let file = std::fs::File::open("versions/1.17.1/1.17.1.json")?;
    let instance: Instance = serde_json::from_reader(std::io::BufReader::new(file))?;

    std::fs::create_dir_all("assets/indexes/")?;

    Task::new(&instance.asset_index.url, "assets/indexes/", instance.asset_index.size)
        .download_file(&client)
        .await?;

    let file = std::fs::File::open("assets/indexes/1.17.json")?;
    let index: Index = serde_json::from_reader(std::io::BufReader::new(file))?;

    let (tx, mut rx) = mpsc::channel(55);
    std::fs::create_dir_all("assets/objects/")?;

    download_objects(RESOURCE_URL, &index.objects.0, "assets/objects/", tx, 3000000, 64).await?;
    loop {
        if let Some(progress) = rx.recv().await {
            if progress.success {
                println!("received {} bytes of {:#?}", progress.writed, progress.path);
            }
            else {
                println!("fail in download {:#?} because of {:#?}", progress.path, progress.fail_reason);
            }
        } else {
            println!("download abort!");
            break;
        }
    }

    let (tx, mut rx) = mpsc::channel(55);
    std::fs::create_dir_all("libraries/")?;
    download_libraries(&instance, "libraries/", tx, 3000000, 64).await?;
    loop {
        if let Some(progress) = rx.recv().await {
            if progress.success {
                println!("received {} bytes of {:#?}", progress.writed, progress.path);
            }
            else {
                println!("fail in download {:#?} because of {:#?}", progress.path, progress.fail_reason);
            }
        } else {
            println!("download abort!");
            break;
        }
    }

    Ok(())
}

async fn test_login(info: &mut AccountInfo) -> Result<()> {
    info.oauth2_login().await
}

async fn test_save(info: &mut AccountInfo) -> Result<()> {
    print_profile(&info);
    
    let mut config = LauncherConfig::default();
    config.accounts.push(info.clone());

    match config.save("./.RMCL.config.json").await {
        Ok(n) => {
            println!("Write into {} bytes", n);
            Ok(())
        }
        Err(e) => Err(e)
    }
}

async fn test_load() -> Result<AccountInfo> {
    let config = LauncherConfig::load("./.RMCL.config.json").await?;
    Ok(config.accounts[0].clone())
}

fn print_profile(info: &AccountInfo) {
    println!("{}", serde_json::to_string(info).unwrap());
}
