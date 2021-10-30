use std::path::{PathBuf, Path};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

use tokio_uring::fs::{File, OpenOptions};
use tokio::io::{AsyncWriteExt, AsyncSeek};
use tokio::sync::{mpsc, oneshot};
use tokio::time::{sleep, Duration, Instant};

use futures::{Future, FutureExt, Stream, StreamExt, stream};
use reqwest::{Client, Response, header::RANGE};

use anyhow::{Result, anyhow, Error};
use bytes::Bytes;

use deserialize::ResourceObject;
use crate::instance::{Instance, deserialize::{Rule, DownloadItem}};

pub mod deserialize;

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
    
    poll_duration: Duration,
    tasks: Vec<Task>,
    client: Client,
    progress_sender: Option<mpsc::Sender<Message>>,
    speed_sender: Option<mpsc::Sender<u64>>,
    controller: Option<mpsc::Receiver<ControlSignal>>,
    completed: usize,
    failed: usize,
    pended: usize,
    in_running: usize,
}

impl Queue {
    pub fn new(chunk_size: u64, parallels: usize, progress_sender: Option<mpsc::Sender<Message>>, 
        controller: Option<mpsc::Receiver<ControlSignal>>,
        speed_sender: Option<mpsc::Sender<u64>>, 
        poll_duration: Duration) -> Queue {
        Queue {
            chunk_size,
            parallels,
            poll_duration,
            tasks: Vec::new(),
            client: Client::new(),
            progress_sender,
            speed_sender,
            controller,
            completed: 0,
            failed: 0,
            pended: 0,
            in_running: 0,
        }
    }
    pub fn push_task(&mut self, task: Task) {
        self.tasks.push(task)
    }
    pub fn run_in_background(mut self) -> JoinHandle<()>{
        thread::spawn(move || tokio_uring::start(async {
            let task_count = self.tasks.len();
            let mut index = 0;

            while index < task_count {
                if self.tasks[index].start + self.chunk_size > self.tasks[index].size {
                    index += 1;
                    continue;
                }
                let mut task = self.tasks[index].clone();
                while task.start + self.chunk_size < task.size {
                    task.start += self.chunk_size;
                    self.tasks.push(task.clone());
                }
                index += 1;
            }

            let (tx, mut rx) = mpsc::channel(self.parallels);
            let mut has_task = true;
            
            let has_progress_sender = self.progress_sender.is_some();
            let has_speed_sender = self.speed_sender.is_some();
            let has_controller = self.controller.is_some();

            let mut stamp = Instant::now();
            let mut period_writed: u64 = 0;
            let mut stop = false;
            
            while has_task && self.in_running > 0 {
                tokio::select! {
                    biased;
                    Some(signal) = async {
                        if has_controller {
                            self.controller.as_mut().unwrap().recv().await
                        } else {
                            futures::future::pending::<Option<ControlSignal>>().await
                        }
                    } =>  {
                        match signal {
                            ControlSignal::Pause => stop = true,
                            ControlSignal::Continue => stop = false,
                            ControlSignal::Abort => has_task = false,
                        }
                    },
                    Some((path, res)) = rx.recv() => {
                        if let Ok(size) = res {
                            self.completed += 1;
                            self.in_running -= 1;
                            if has_progress_sender {
                                self.progress_sender.as_ref().unwrap().send(Message::new(path, true, None, size)).await;
                                period_writed += size;
                            }
                        }
                        else if let Err(e) = res {
                            self.failed += 1;
                            self.in_running -= 1;
                            println!("fail a  task!, in running: {}", self.in_running);
                            if has_progress_sender {
                                self.progress_sender.as_ref().unwrap().send(Message::new(path, false, Some(e), 0)).await;
                            }
                        }
                    },
                    _ = async {
                        sleep(self.poll_duration).await;
                        if has_speed_sender && stamp.elapsed() >= Duration::from_secs(1) {
                            stamp = Instant::now();
                            period_writed = 0;
                            self.speed_sender.as_ref().unwrap().send(period_writed);
                        }
                    }, if has_task && stop => {
                        if self.in_running < self.parallels {
                            if self.tasks.len() == 0 {
                                has_task = false;
                                return;
                            }
                            let n = std::cmp::min(self.parallels - self.in_running, self.tasks.len());
                            for _ in 0..n {
                                let mut task = self.tasks.swap_remove(self.tasks.len() - 1);

                                let client = self.client.clone();
                                self.in_running += 1;
                                self.pended += 1;
                                
                                let sender = tx.clone();
                                tokio_uring::spawn(async move {
                                    let res = task.download_part(client, self.chunk_size).await;
                                    sender.send((task.path, res)).await
                                });
                            }
                        } 
                    }
                }
            }
            drop(self);
            println!("download queue all done");
        }))
    }
}

pub async fn download_objects(baseurl: &str, objects: &Vec<ResourceObject>, dirpath: &str, progress_sender: mpsc::Sender<Message>, chunk_size: u64, parallels: usize) -> Result<()> {
    let path_buf = PathBuf::from(&dirpath);

    let mut queue: Queue = Queue::new(chunk_size, parallels,
        Some(progress_sender), None, None, Duration::from_millis(100));

    for object in objects {
        let short_name = &object.hash[..2];
        let url = format!("{}/{}/{}", baseurl, short_name, object.hash);
        let task = Task::new(&url, path_buf.join(short_name), object.size);
        queue.push_task(task);
    }

    queue.run_in_background();

    Ok(())
}

pub async fn download_libraries(instance: &Instance, dirpath: &str, progress_sender: mpsc::Sender<Message>, chunk_size: u64, parallels: usize) -> Result<()> {
    let path_buf = PathBuf::from(&dirpath);
    let mut queue: Queue = Queue::new(chunk_size, parallels,
        Some(progress_sender), None, None, Duration::from_millis(100));
    
    for library in &instance.libraries {
        let new_path = path_buf.join(&library.download_item.path);
        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        queue.push_task(Task::new(&library.download_item.url, 
            new_path, library.download_item.size))
    }

    queue.run_in_background();

    Ok(())
}

pub async fn download_main<P>(url: P, dest: P, progress_sender: mpsc::Sender<Message>, chunk_size: u64, parallels: usize) -> Result<()> 
where 
        P: AsRef<Path>, {
    let mut queue: Queue = Queue::new(chunk_size, parallels,
        Some(progress_sender), None, None, Duration::from_millis(100));
    
    
    
    Ok(())
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