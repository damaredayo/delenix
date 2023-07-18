use std::collections::HashMap;

use serde_derive::{Deserialize, Serialize};

use crate::{screenshot, util::make_default_image_path};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub uploaders: Vec<Uploader>,
    pub screenshotter: Option<screenshot::custom::Screenshotter>, // If this is None, use the built-in screenshotter I have so graciously provided
    pub last_index: u32,
    pub copy_to_clipboard: bool,
    pub copy_url_to_clipboard: bool,
    pub freeze_screen: bool,
    pub tessdata_path: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Uploader {
    HTTP(HttpUploader),
    File(FileUploader),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum DestinationType {
    None,
    ImageUploader,
    TextUploader,
    FileUploader,
    URLShortener,
    URLSharingService,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Body {
    None,
    MultipartFormData,
    FormURLEncoded,
    JSON,
    XML,
    Binary,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpUploader {
    pub name: String,
    pub destination_type: DestinationType,
    pub request_method: String,
    pub request_url: String,
    pub parameters: Option<HashMap<String, String>>,
    pub headers: Option<HashMap<String, String>>,
    pub body: Body,
    pub arguments: Option<HashMap<String, String>>,
    pub file_form_name: Option<String>,

    pub url: String,
    pub thumbnail_url: Option<String>,
    pub deletion_url: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileUploader {
    pub name: String,      // Name of the uploader
    pub file_path: String, // Path the file will be saved to
    pub file_name: String, // Name of the file (without extension)
}

impl Uploader {
    pub fn from_sharex(c: sharex::Config) -> Self {
        Self::HTTP(HttpUploader {
            name: c.name,
            destination_type: c.destination_type,
            request_method: c.request_method,
            request_url: c.request_url,
            parameters: Some(c.parameters),
            headers: Some(c.headers),
            body: c.body,
            arguments: Some(c.arguments),
            file_form_name: Some(c.file_form_name),
            url: c.url,
            thumbnail_url: Some(c.thumbnail_url),
            deletion_url: Some(c.deletion_url),
            error_message: Some(c.error_message),
        })
    }
}

impl Config {
    pub fn from_file(path: String) -> Result<Self, Box<dyn std::error::Error>> {
        let s = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                // If the file doesn't exist, create it, if possible we want to ask the user before doing this though, so we check if stdin is a tty
                if e.kind() == std::io::ErrorKind::NotFound {
                    if atty::is(atty::Stream::Stdin) {
                        println!("Config file not found, would you like to create a new one at {}? [Y/n]", &path);

                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input)?;

                        input = input.trim().to_lowercase();
                        if input.len() == 0 {
                            input = "y".to_string();
                        }
                        if input != "y" {
                            println!("Ok, we won't create a new config file. We will use the default config instead.");
                            return Ok(Self::default());
                        }
                    }

                    let c = Self::default();
                    let s = serde_json::to_string_pretty(&c)?;

                    let path = std::path::Path::new(&path);
                    if let Some(parent) = path.parent() {
                        std::fs::create_dir_all(parent)?;
                    }

                    std::fs::write(&path, s)?;

                    return Ok(c);
                }

                return Ok(Self::default());
            }
        };

        let c: Config = serde_json::from_str(&s)?;

        Ok(c)
    }
}

impl Default for Config {
    fn default() -> Self {
        // Default config consists of uploading to imgur and saving to a a file located at ~/Screenshots

        let args: HashMap<String, String> = [("type", "file")]
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let headers: HashMap<String, String> = [("Authorization", "Client-ID 8c964e2b2514a95")]
            .iter()
            .map(|&(k, v)| (k.to_string(), v.to_string()))
            .collect();

        Self {
            uploaders: vec![
                Uploader::File(FileUploader {
                    name: "File".to_string(),
                    file_path: make_default_image_path(),
                    file_name: "%r12".to_string(),
                }),
                Uploader::HTTP(HttpUploader {
                    name: "imgur".to_string(),
                    destination_type: DestinationType::ImageUploader,
                    request_method: "POST".to_string(),
                    request_url: "https://api.imgur.com/3/image".to_string(),
                    parameters: None,
                    headers: Some(headers),
                    body: Body::MultipartFormData,
                    arguments: Some(args),
                    file_form_name: Some("image".to_string()),
                    url: "$json:data.link$".to_string(),
                    thumbnail_url: None,
                    deletion_url: Some(
                        "http://imgur.com/delete/$json:data.deletehash$".to_string(),
                    ),
                    error_message: None,
                }),
            ],
            screenshotter: None,
            last_index: 0,
            copy_to_clipboard: true,
            copy_url_to_clipboard: false,
            freeze_screen: true,

            #[cfg(target_os = "linux")]
            tessdata_path: Some("/usr/share/tessdata/".to_string()),

            #[cfg(target_os = "windows")]
            tessdata_path: Some("C:\\Program Files\\Tesseract-OCR\\tessdata".to_string()),
        }
    }
}

mod sharex {
    use serde_derive::{Deserialize, Serialize};
    use std::collections::HashMap;

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    pub struct Config {
        pub name: String,
        pub destination_type: crate::config::DestinationType,
        pub request_method: String,
        pub request_url: String,
        pub parameters: HashMap<String, String>,
        pub headers: HashMap<String, String>,
        pub body: crate::config::Body,
        pub arguments: HashMap<String, String>,
        pub file_form_name: String,
        pub url: String,
        pub thumbnail_url: String,
        pub deletion_url: String,
        pub error_message: String,
    }
}
