use futures::TryFutureExt;
use mc_launcher_core::account::AccountInfo;
use mc_launcher_core::config::LauncherConfig;
use mc_launcher_core::instance::Instance;

use anyhow::{Result, anyhow};
use serde_json::value::Value as Json;
use tokio::sync::mpsc;

fn main() {
    tokio_uring::start(run()).unwrap()
}
async fn run() -> Result<()> {
    let mut info = AccountInfo::default();

    test_login(&mut info).await?;

    test_login(&mut info).await?;

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
