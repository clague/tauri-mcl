use crate::account::AccountInfo;
use crate::instance::Instance;

use serde::{Serialize, Deserialize};
use anyhow::{Result};
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, AsyncReadExt};

#[derive(Serialize, Deserialize, Default)]
pub struct LauncherConfig {
    pub accounts: Vec<AccountInfo>,

    pub download_chunk_size: u64,
    pub download_parallels_count: u32,
}

impl LauncherConfig {
    pub async fn save(&self, path: &str) -> Result<usize>{
        let mut _file = File::create(path).await?;

        _file.write(serde_json::to_string(self)?.as_bytes())
            .await
            .map_err(anyhow::Error::msg)
    }

    pub async fn load(path: &str) -> Result<LauncherConfig>{
        let mut _file = File::open(path).await?;

        let mut buf = String::new();

        _file.read_to_string(&mut buf).await?;

        let config: LauncherConfig = serde_json::from_str(buf.as_str())?;
        
        Ok(config)
    }
}
