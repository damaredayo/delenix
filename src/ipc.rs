#[cfg(target_os = "linux")]
use tokio::net::UnixListener;

#[cfg(target_os = "windows")]
use tokio::net::windows::named_pipe::{NamedPipeClient, NamedPipeServer};

use serde_derive::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::sync::Mutex;

use delenix_lib::{config::Config, screenshot::ScreenshotType, upload};

#[derive(Debug, Serialize, Deserialize)]
pub enum Request {
    SetConfig(SetConfig),
    GetConfig(GetConfig),
    Upload(Upload),
    Screenshot(ScreenshotType),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetConfig {
    pub config: Config,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetConfig;

#[derive(Debug, Serialize, Deserialize)]
pub struct Upload {
    pub data: Vec<u8>,
    pub format: String,
}

#[derive(Serialize, Deserialize)]
pub struct UploadResponse {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct GetImage;

#[derive(Serialize, Deserialize)]
pub struct GetImageResponse {
    pub data: Vec<u8>,
    pub format: String,
}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new(error: String) -> Self {
        Self { error }
    }
}

pub async fn start_ipc(conf: Arc<Mutex<Config>>) {
    let server_task = tokio::spawn(async move {
        // Server code
        #[cfg(target_os = "windows")]
        {
            {
                let pipe_path = "\\\\.\\pipe\\delenix";
                let server = NamedPipeServer::new(pipe_path).await.unwrap();

                while let Ok(client) = server.accept().await {
                    let conf_clone = Arc::clone(&conf);

                    tokio::spawn(async move {
                        handle_client(conf_clone, client).await;
                    });
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let listener = UnixListener::bind("/tmp/delenix").unwrap();

            tracing::info!("Listening on /tmp/delenix");

            while let Ok((stream, _)) = listener.accept().await {
                let localaddr = stream.local_addr().unwrap();
                tracing::info!("Got a connection from {:?}", localaddr);
                let conf = Arc::clone(&conf);
                tokio::spawn(async move {
                    handle_client(&conf, Box::pin(stream)).await;
                });
            }
        }
    });

    // let client_task = tokio::spawn(async move {
    //     // Client code
    //     #[cfg(windows)]
    //     {
    //         // Connect to the named pipe server on Windows
    //         let mut client = tokio_named_pipes::NamedPipeClient::connect("\\\\.\\pipe\\delenix")
    //             .await
    //             .unwrap();
    //         handle_client(conf.clone(), client).await;
    //     }
    //
    //     #[cfg(unix)]
    //     {
    //         // Connect to the Unix socket server on Linux
    //         let stream = UnixStream::connect("/tmp/delenix").await.unwrap();
    //         handle_client(conf.clone(), stream).await;
    //     }
    // });

    server_task.await.unwrap();
    // client_task.await.unwrap();
}

trait AsyncRW: AsyncRead + AsyncWrite + Send {}

impl<T> AsyncRW for T where T: AsyncRead + AsyncWrite + Send {}

async fn handle_client(config: &Mutex<Config>, mut stream: Pin<Box<dyn AsyncRW + Send>>) {
    let mut buffer = [0; 1024];
    loop {
        let bytes_read = stream.read(&mut buffer).await.unwrap();
        if bytes_read == 0 {
            break;
        }

        let request: Request = serde_json::from_slice(&buffer[..bytes_read]).unwrap();

        tracing::info!("Got request: {:?}", request);

        match request {
            Request::SetConfig(set_config) => {
                let mut config = config.lock().await;
                *config = set_config.config;
            }
            Request::GetConfig(_get_config) => {
                let config = config.lock().await;
                let response = serde_json::to_vec(&*config).unwrap();
                stream.write_all(&response).await.unwrap();
            }
            Request::Upload(upload) => {
                let config: tokio::sync::MutexGuard<'_, Config> = config.lock().await;
                let results = upload::upload(&*config, &upload.data, &upload.format).unwrap();
                let response = serde_json::to_vec(&results).unwrap();
                stream.write_all(&response).await.unwrap();
            }
            Request::Screenshot(screenshot_type) => {
                let config = config.lock().await;
                let results = match config.screenshot(screenshot_type) {
                    Ok(results) => results,
                    Err(err) => {
                        let err = ErrorResponse::new(err.to_string());
                        serde_json::to_vec(&err).unwrap()
                    }
                };
                let response = serde_json::to_vec(&results).unwrap();
                stream.write_all(&response).await.unwrap();
            }
        }
    }
}
