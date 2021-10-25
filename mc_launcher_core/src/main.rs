use mc_launcher_core::account::AccountInfo;
use mc_launcher_core::config::LauncherConfig;
use mc_launcher_core::download::{*, deserialize::Index};

use anyhow::{Result, anyhow};
use serde_json::value::Value as Json;
use tokio::sync::mpsc;

fn main() {
    tokio_uring::start(run());
    let mut a = String::new();
    std::io::stdin().read_line(&mut a);
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

    let client = reqwest::Client::new();
    DownloadTask::new(MANIFEST_URL, "versions/")?.download_file(&client).await?;

    let file = std::fs::File::open("versions/version_manifest.json")?;
    let json: Json = serde_json::from_reader(std::io::BufReader::new(file))?;

    for version in json["versions"].as_array().ok_or(anyhow!("can't parse json!"))? {
        if (version["id"]) == "1.17.1" {
            std::fs::create_dir_all("versions/1.17.1")?;

            DownloadTask::new(version["url"].as_str().ok_or(anyhow!("can't parse json!"))?,
                                "versions/1.17.1/")?
                .download_file(&client)
                .await?;
            break;
        }
    }

    let file = std::fs::File::open("versions/1.17.1/1.17.1.json")?;
    let json: Json = serde_json::from_reader(std::io::BufReader::new(file))?;

    std::fs::create_dir_all("assets/indexes/")?;

    DownloadTask::new(json["assetIndex"]["url"].as_str().ok_or(anyhow!("can't parse json!"))?,
                    "assets/indexes/")?
        .download_file(&client)
        .await?;

    let file = std::fs::File::open("assets/indexes/1.17.json")?;
    let index: Index = serde_json::from_reader(std::io::BufReader::new(file))?;

    let (tx, mut rx) = mpsc::channel(55);
    std::fs::create_dir_all("assets/objects/")?;

    tokio::select! {
        r = download_objects(RESOURCE_URL, &index.objects.0, "assets/objects/", &tx, 50) => {
            match r {
                Ok(_) => println!("Download Complete!"),
                Err(e) => eprintln!("{}", e),
            }
        },
        _ = async { 
            loop {
                if let Some(progress) = rx.recv().await {
                    println!("received {}/{}", progress, index.objects.0.len());
                } else {
                    eprintln!("channel is closed!");
                }
            }
        } => {},
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
