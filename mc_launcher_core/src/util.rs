use std::path::Path;

use std::fs::*;

use zip;
use anyhow::{Result, anyhow};

async fn extract_zip<P>(zip: P, extract_path: P) -> Result<()>
where 
        P: AsRef<Path>, {
    let zip_file = zip.as_ref();
    let extract_file = extract_path.as_ref();

    if !zip_file.is_file() {
        return Err(anyhow!("File doesn't exist"));
    }

    if !extract_file.exists() {
        std::fs::create_dir_all(extract_file.parent().ok_or(anyhow!("No parent dir"))?)?;
    }

    let file = File::open(zip_file)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;

    }

    Ok(())
}