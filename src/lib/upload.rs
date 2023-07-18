use serde_derive::{Deserialize, Serialize};

use crate::{
    config::{self, Config, Uploader},
    util,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UploadResult {
    pub uploader_name: String,
    pub url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub deletion_url: Option<String>,
    pub error_message: Option<String>,

    pub file_path: Option<String>,
}

fn deserialize_to_x_www_form_urlencoded(data: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let parsed_data: Vec<(String, String)> = serde_urlencoded::from_bytes(data)?;

    let encoded_data: String = parsed_data
        .into_iter()
        .map(|(key, value)| format!("{}={}", key, value))
        .collect::<Vec<String>>()
        .join("&");

    Ok(encoded_data)
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
    let mut client: Option<reqwest::blocking::Client> = None;

    for uploader in &conf.uploaders {
        match uploader {
            Uploader::HTTP(ref u) => {
                if client.is_none() {
                    client = Some(reqwest::blocking::Client::new());
                }

                let client = client.as_ref().unwrap();

                let method = match u.request_method.to_uppercase().as_str() {
                    "GET" => reqwest::Method::GET,
                    "POST" => reqwest::Method::POST,
                    "PUT" => reqwest::Method::PUT,
                    "DELETE" => reqwest::Method::DELETE,
                    "HEAD" => reqwest::Method::HEAD,
                    "OPTIONS" => reqwest::Method::OPTIONS,
                    "CONNECT" => reqwest::Method::CONNECT,
                    "PATCH" => reqwest::Method::PATCH,
                    _ => reqwest::Method::POST,
                };

                let mut req = client.request(method, &u.request_url);

                if u.parameters.is_some() {
                    req = req.query(&u.parameters.as_ref().unwrap());
                }

                if u.headers.is_some() {
                    for (k, v) in u.headers.as_ref().unwrap() {
                        req = req.header(k, v);
                    }
                }

                let file_form_name = u.file_form_name.clone().unwrap_or("image".to_string());

                match &u.body {
                    config::Body::MultipartFormData => {
                        let mut form = reqwest::blocking::multipart::Form::new().part(
                            file_form_name,
                            reqwest::blocking::multipart::Part::bytes(data.to_vec()),
                        );

                        if u.arguments.is_some() {
                            for (k, v) in u.arguments.as_ref().unwrap() {
                                form = form.text(k.clone(), v.clone());
                            }
                        }

                        req = req.multipart(form);
                    }

                    config::Body::FormURLEncoded => {
                        let url_encoded = deserialize_to_x_www_form_urlencoded(data)?;

                        req = req
                            .header("Content-Type", "application/x-www-form-urlencoded")
                            .body(url_encoded);
                    }

                    config::Body::JSON => {
                        let json = serde_json::json!({
                            file_form_name: std::str::from_utf8(data)?
                        });

                        req = req.json(&json);
                    }

                    config::Body::XML => {
                        let xml = format!(
                            r#"<xml><name>{}</name><file>{}</file></xml>"#,
                            file_form_name,
                            std::str::from_utf8(data)?
                        );

                        req = req.header("Content-Type", "application/xml").body(xml);
                    }

                    config::Body::Binary => {
                        req = req
                            .header("Content-Type", "application/octet-stream")
                            .body(data.to_vec());
                    }

                    _ => {
                        req = req
                            .header("Content-Type", "application/octet-stream")
                            .body(data.to_vec());
                    }
                };

                let request = req.build()?;

                let res = client.execute(request)?;

                if !res.status().is_success() {
                    results.push(UploadResult {
                        uploader_name: u.name.clone(),
                        url: None,
                        thumbnail_url: None,
                        deletion_url: None,
                        error_message: Some(format!("{}: {}", res.status(), res.text()?)),
                        file_path: None,
                    });

                    continue;
                }

                let text = res.text()?;

                let (mut url, mut thumbnail_url, mut deletion_url): (String, String, String) =
                    (String::new(), String::new(), String::new());

                if !u.url.is_empty() {
                    url = util::parse_custom_syntax(&u.url, &text)?;
                }

                if u.thumbnail_url.is_some() {
                    thumbnail_url =
                        util::parse_custom_syntax(u.thumbnail_url.as_ref().unwrap(), &text)?;
                }

                if u.deletion_url.is_some() {
                    deletion_url =
                        util::parse_custom_syntax(u.deletion_url.as_ref().unwrap(), &text)?;
                }

                results.push(UploadResult {
                    uploader_name: u.name.clone(),
                    url: Some(url),
                    thumbnail_url: Some(thumbnail_url),
                    deletion_url: Some(deletion_url),
                    error_message: None,
                    file_path: None,
                });
            }

            Uploader::File(ref u) => {
                let filename = format!(
                    "{}/{}.{}",
                    u.file_path,
                    conf.make_filename(Some(&u.file_name)),
                    format
                );

                std::fs::create_dir_all(&u.file_path)?;

                std::fs::write(&filename, data)?;

                results.push(UploadResult {
                    uploader_name: u.name.clone(),
                    url: None,
                    thumbnail_url: None,
                    deletion_url: None,
                    error_message: None,
                    file_path: Some(filename),
                });
            }
        }
    }

    Ok(results)
}
