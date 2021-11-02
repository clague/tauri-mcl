use std::io::{SeekFrom, prelude::*};
use std::path::{PathBuf, Path};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use parking_lot::Mutex;
#[cfg(target_os="linux")]
use tokio_uring::fs::{File, OpenOptions};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration, Instant};

use reqwest::{Client, header::RANGE};

use anyhow::{Result, anyhow, Error};
use bytes::Bytes;

#[derive(Clone)]
pub struct Task {
    pub url: String,
    pub path: PathBuf,
    pub size: u64,
    pub start: u64,
}

impl Task {
    pub fn new<P>(url: &str, path: P, size: u64) -> Task
    where P: AsRef<Path>, 
    {
        Task {
                url: url.to_owned(),
                path: path.as_ref().into(),
                size,
                start: 0,
        }
    }
    pub async fn get_part(&self, client: &Client, chunk_size: u64) -> Result<Bytes> {
        let end = core::cmp::min(self.start + chunk_size - 1 , self.size);

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
        let resp = client.get(&self.url).send().await?;

        if self.path.exists() {
            if self.path.is_dir() {
                let basename = resp
                    .url()
                    .path_segments()
                    .and_then(|segments| segments.last())
                    .and_then(|name| if name.is_empty() { None } else { Some(name) })
                    .ok_or(anyhow!("Can't parse url"))?;
                
                self.path.push(basename);
            }
        }
        else {
            std::fs::create_dir_all(self.path.parent().ok_or(anyhow!("No parent dir"))?)?;
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
    pub async fn download_file_std(&mut self, client: &Client) -> Result<usize> {
        let resp = client.get(&self.url).send().await?;

        if self.path.exists() {
            if self.path.is_dir() {
                let basename = resp
                    .url()
                    .path_segments()
                    .and_then(|segments| segments.last())
                    .and_then(|name| if name.is_empty() { None } else { Some(name) })
                    .ok_or(anyhow!("Can't parse url"))?;
                
                self.path.push(basename);
            }
        }
        else {
            std::fs::create_dir_all(self.path.parent().ok_or(anyhow!("No parent dir"))?)?;
        }

        let bytes = resp.bytes().await?;
        let mut file = std::fs::File::create(&self.path)?;
        file.write_all(&bytes)?;
        let res = bytes.len();

        println!("RESPONSE: {} bytes from {}", res, self.url);
        Ok(res)
    }

    pub async fn download_part_std(&mut self, client: Client, chunk_size: u64) -> Result<u64> {
        let end = core::cmp::min(self.start + chunk_size - 1 , self.size);

        let resp = if end == 0 {
            client.get(&self.url).send().await?
        }
        else {
            client.get(&self.url).header(RANGE, format!("bytes={}-{}", self.start, end)).send().await?
        };

        if self.path.exists() {
            if self.path.is_dir() {
                let basename = resp
                    .url()
                    .path_segments()
                    .and_then(|segments| segments.last())
                    .and_then(|name| if name.is_empty() { None } else { Some(name) })
                    .ok_or(anyhow!("Can't parse url"))?;
                
                self.path.push(basename);
            }
        }
        else {
            std::fs::create_dir_all(self.path.parent().ok_or(anyhow!("No parent dir"))?)?;
        }

        let bytes = resp.bytes().await?;

        let mut file = std::fs::OpenOptions::new().write(true).create(true).open(&self.path)?;
        file.seek(SeekFrom::Start(self.start))?;
        file.write_all(&bytes)?;
        let res = bytes.len();

        Ok(res as u64)
    }

    pub async fn download_part(&mut self, client: Client, chunk_size: u64) -> Result<u64> {
        let end = core::cmp::min(self.start + chunk_size - 1 , self.size);

        let resp = if end == 0 {
            client.get(&self.url).send().await?
        }
        else {
            client.get(&self.url).header(RANGE, format!("bytes={}-{}", self.start, end)).send().await?
        };

        if self.path.exists() {
            if self.path.is_dir() {
                let basename = resp
                    .url()
                    .path_segments()
                    .and_then(|segments| segments.last())
                    .and_then(|name| if name.is_empty() { None } else { Some(name) })
                    .ok_or(anyhow!("Can't parse url"))?;
                
                self.path.push(basename);
            }
        }
        else {
            std::fs::create_dir_all(self.path.parent().ok_or(anyhow!("No parent dir"))?)?;
        }

        let bytes = resp.bytes().await?;

        let file = OpenOptions::new().write(true).create(true).open(&self.path).await?;
        let (res, _) = file.write_at(bytes, self.start).await;

        file.sync_all().await?;
        file.close().await?;

        res.map(|size| size as u64).map_err(anyhow::Error::new)
    }
}

pub enum ControlSignal {
    Pause,
    Continue,
    Abort,
}

pub struct Message {
    pub path: PathBuf,
    pub success: bool,
    pub fail_reason: Option<Error>,
    pub writed: u64,
}
impl Message {
    pub fn new(path: PathBuf, success: bool, fail_reason: Option<Error>, writed: u64) -> Message {
        Message { path, success, fail_reason, writed }
    }
}

pub struct Queue {
    pub chunk_size: u64,
    pub parallels: usize,
    
    progress_sender: Option<mpsc::Sender<Message>>,
    poll_duration: Duration,
    tasks: Vec<Task>,
    client: Client,
    speed: f64,
    completed: usize,
    failed: usize,
    pended: usize,
    in_running: usize,

    abort: bool,
    stop: bool,
}

impl Queue {
    pub fn new(chunk_size: u64,
        parallels: usize,
        progress_sender: Option<mpsc::Sender<Message>>, 
        poll_duration: Duration) -> Queue {
        Queue {
            chunk_size,
            parallels,
            progress_sender,
            poll_duration,
            speed: 0.0,
            tasks: Vec::new(),
            client: Client::new(),
            completed: 0,
            failed: 0,
            pended: 0,
            in_running: 0,
            abort: false,
            stop: false,
        }
    }
    pub fn push_task(&mut self, task: Task) {
        self.tasks.push(task);
    }
    pub fn run_in_background(self) -> Arc<Mutex<Queue>> {
        let lock = Arc::new(Mutex::new(self));
        let handle = lock.clone();

        thread::spawn(move || tokio_uring::start(async {
            let mut queue = handle.lock();
            let mut index = 0;

            let mut task_count = queue.tasks.len();
            while index < task_count {
                if queue.tasks[index].start + queue.chunk_size > queue.tasks[index].size || queue.tasks[index].size == 0 {
                    index += 1;
                    continue;
                }
                let mut task = queue.tasks[index].clone();
                while task.start + queue.chunk_size < task.size {
                    task.start += queue.chunk_size;
                    queue.tasks.push(task.clone());
                }
                index += 1;
            }
            task_count = queue.tasks.len();

            let (tx, mut rx) = mpsc::channel(queue.parallels);

            let mut stamp = Instant::now();
            let mut period_writed: u64 = 0;
            let mut stop = false;

            drop(queue);

            loop {
                let queue = handle.lock();
                if queue.abort || (task_count <= (queue.completed + queue.failed)) {
                    break;
                }
                stop = queue.stop;
                drop(queue);

                tokio::select! {
                    Some((path, res)) = rx.recv() => {
                        if let Ok(size) = res {
                            let mut queue = handle.lock();
                            queue.completed += 1;
                            queue.in_running -= 1;
                            if queue.progress_sender.is_some() {
                                queue.progress_sender.as_ref().unwrap().send(Message::new(path, true, None, size)).await;
                                period_writed += size;
                            }
                            drop(queue);
                        }
                        else if let Err(e) = res {
                            let mut queue = handle.lock();
                            queue.failed += 1;
                            queue.in_running -= 1;
                            println!("fail a  task!, in running: {}", queue.in_running);
                            if queue.progress_sender.is_some() {
                                queue.progress_sender.as_ref().unwrap().send(Message::new(path, false, Some(e), 0)).await;
                            }
                            drop(queue);
                        }
                    },
                    _ = async {
                        let queue = handle.lock();
                        let poll_duration = queue.poll_duration;
                        drop(queue);

                        sleep(poll_duration).await;
                    }, if !stop => {
                        let mut queue = handle.lock();

                        queue.speed = period_writed as f64 * (1.0 / stamp.elapsed().as_secs_f64());
                        stamp = Instant::now();
                        period_writed = 0;

                        if queue.in_running < queue.parallels {
                            if queue.tasks.len() == 0 {
                                return;
                            }
                            let n = std::cmp::min(queue.parallels - queue.in_running, queue.tasks.len());
                            for _ in 0..n {
                                let len = queue.tasks.len();
                                let mut task = queue.tasks.swap_remove(len - 1);

                                let client = queue.client.clone();
                                let sender = tx.clone();
                                let chunk_size = queue.chunk_size;

                                tokio_uring::spawn(async move {
                                    let res = task.download_part(client, chunk_size).await;
                                    sender.send((task.path, res)).await
                                });
                                queue.in_running += 1;
                                queue.pended += 1;
                            }
                        }
                        drop(queue);
                    }
                }
            }
            drop(handle);
            println!("download queue all done");
        }));
        lock
    }
}

// pub async fn download_libraries(version: &VersionConfig, condition: &Rule, dirpath: &str, progress_sender: &Sender<u32>, chunk_size: u64, parallels: u32) {
//     let path_buf = PathBuf::from(&dirpath);
//     let reqwest_client = reqwest::Client::new();

//     let mut tasks = stream::iter(vec![&version.main_downloads.client, &version.main_downloads.client_mappings])
//         .flat_map(|download_item| {
//             let task = Task {
//                 url: download_item.url.clone(),
//                 path: path_buf.join(format!("versions/{0}/{0}.jar", version.version)),
//                 size: download_item.size,
//                 chunk_size: chunk_size,
//                 start: 0,
//             };
//             stream::iter(task).map(|mut part| {
//                 let client = &reqwest_client;
//                 async move {
//                     part.download_part(client).await
//                 }
//             })
//         });
    
//     tasks.extend(stream::iter(version.libraries)
//         .flat_map(|library| {
//             let item = match library.download_item {
//                 ItemGen::Generator(f) => {
//                     f(condition)
//                 },
//                 ItemGen::Item(i) => Some(i),
//                 ItemGen::None => None,
//             }
//             if let Some(item) = item {
//                 let task = Task {
//                     url: item.url.clone(),
//                     path: path_buf.join(item.path),
//                     size: item.size,
//                     chunk_size: chunk_size,
//                     start: 0,
//                 };
//                 stream::iter(task).map(|mut part| {
//                     let client = &reqwest_client;
//                     async move {
//                         part.download_part(client).await
//                     }
//                 })
//             } else {
//                 stream::empty()
//             }
//         }));
// }