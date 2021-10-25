use tokio_uring::fs::{File, OpenOptions};
use std::path::{PathBuf, Path};
use tokio::io::{AsyncWriteExt, AsyncSeek};
use tokio::sync::mpsc::Sender;
use futures::{stream, StreamExt};
use reqwest::{Client, Response, header::RANGE};
use anyhow::{Result, anyhow};
use bytes::Bytes;

use deserialize::ResourceObject;

pub mod deserialize;


#[derive(Clone)]
pub struct DownloadTask {
    pub url: String,
    pub path: PathBuf,
    pub size: u64,
    pub chunk_size: u64,
    pub start: u64,
}

impl Iterator for DownloadTask {
    type Item = DownloadTask;

    fn next(&mut self) -> Option<Self::Item> {
        let clone = self.clone();
        if self.start <= self.size {
            self.start += self.chunk_size;
            Some(clone)
        }
        else {
            None
        }
    }
}

impl DownloadTask {
    pub fn new<P>(url: &str, path: P) -> Result<DownloadTask> 
    where P: AsRef<Path>, 
    {
        Ok(DownloadTask {
                url: url.to_owned(),
                path: path.as_ref().try_into()?,
                size: 0,
                chunk_size: 0,
                start: 0,
        })
    }
    pub async fn get_part(&self, client: &Client) -> Result<Bytes> {
        let end = core::cmp::min(self.start + self.chunk_size - 1 , self.size);

        let resp = if end == 0 {
            client.get(&self.url).send().await?
        }
        else {
            client.get(&self.url).header(RANGE, format!("bytes={}-{}", self.start, end)).send().await?
        };

        resp.bytes().await.map_err(anyhow::Error::new)
    }

    pub async fn get_whole(&self, client: &Client) -> Result<Bytes> {
        let resp = client.get(&self.url).send().await?;

        resp.bytes().await.map_err(anyhow::Error::new)
    }

    pub async fn download_file(&mut self, client: &Client) -> Result<usize> {
        let meta = std::fs::metadata(&self.path)?;
        let resp = client.get(&self.url).send().await?;

        if meta.is_dir() {
            let basename = resp
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .ok_or(anyhow!("Can't parse url"))?;
            
            self.path.push(basename);
        }

        let bytes = resp.bytes().await?;
        let file = File::create(&self.path).await?;
        let (res, _) = file.write_at(bytes, 0).await;

        let res = res?;
        println!("RESPONSE: {} bytes from {}", res, self.url);

        file.sync_all().await?;
        file.close().await?;

        Ok(res)
    }

    pub async fn download_part(&mut self, client: &Client) -> Result<usize> {
        let end = core::cmp::min(self.start + self.chunk_size - 1 , self.size);

        let resp = if end == 0 {
            client.get(&self.url).send().await?
        }
        else {
            client.get(&self.url).header(RANGE, format!("bytes={}-{}", self.start, end)).send().await?
        };

        let meta = std::fs::metadata(&self.path)?;

        if meta.is_dir() {
            let basename = resp
                .url()
                .path_segments()
                .and_then(|segments| segments.last())
                .and_then(|name| if name.is_empty() { None } else { Some(name) })
                .ok_or(anyhow!("Can't parse url"))?;
            
            self.path.push(basename);
        }

        let bytes = resp.bytes().await?;

        let file = OpenOptions::new().write(true).open(&self.path).await?;
        let (res, _) = file.write_at(bytes, self.start).await;

        file.sync_all().await?;
        file.close().await?;

        res.map_err(anyhow::Error::new)
    }
}
pub async fn download_objects(baseurl: &str, objects: &Vec<ResourceObject>, dirpath: &str, progress_sender: &Sender<usize>, parallels: usize) -> Result<()> {
    //let is_existed = metadata(path).is_ok();
    let path_buf = PathBuf::from(&dirpath);
    let reqwest_client = reqwest::Client::new();
    let mut completed = 0;

    let tasks = stream::iter(objects)
        .flat_map(|object| {
            let task = DownloadTask {
                url: format!("{}/{}/{}", baseurl, &object.hash[..2], object.hash),
                path: path_buf.join(&object.hash[..2]),
                size: object.size,
                chunk_size: 5000000,
                start: 0
            };
            stream::iter(task).map(|mut part| {
                let client = &reqwest_client;
                async move {
                    part.download_part(client).await
                }
            })
        })
        .buffer_unordered(parallels);
    
    tasks.for_each(move |b| {
        completed += 1;
        let tx = progress_sender.clone();

        async move {
            match b {
                Ok(_) => {
                    tx.send(completed).await.unwrap_or_default();
                },
                Err(e) => {
                    eprintln!("Got an error: {}", e);
                }
            }
        }
    }).await;

    Ok(())
}
