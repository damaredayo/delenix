use serde_derive::{Deserialize, Serialize};

use crate::config::{Config, Uploader};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadResult {
    pub uploader: Uploader,
    pub location: String,
}

// Upload the image to the uploaders specified in the config, data should be a processed image in its container.
// The format should be the format of the image, e.g. "png" or "jpg"
// Returns a vector of UploadResult, which contains the location of the image on the server or filesystem.
pub fn upload(
    conf: &Config,
    data: &[u8],
    format: &str,
) -> Result<Vec<UploadResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    for uploader in &conf.uploaders {
        match uploader {
            Uploader::HTTP(_u) => {
                results.push(UploadResult {
                    uploader: uploader.clone(),
                    location: "https://example.com".to_string(),
                });
            }
            Uploader::File(u) => {
                let filename = format!(
                    "{}/{}.{}",
                    u.file_path,
                    conf.make_filename(Some(&u.file_name)),
                    format
                );

                std::fs::write(&filename, data).unwrap();
                results.push(UploadResult {
                    uploader: uploader.clone(),
                    location: filename,
                });
            }
        }
    }

    Ok(results)
}
