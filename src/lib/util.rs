use std::iter;

use chrono::{Datelike, Timelike, Utc};
use jsonpath_lib::Selector;
use lazy_static::lazy_static;
use rand::Rng;
use regex::Regex;
use serde_json::Value;

use crate::{
    config::{self, Config},
    upload,
};

lazy_static! {
    static ref FILENAME_REGEX: Regex = Regex::new(r"%(y|mo|d|h|m|s|ts|t|r|i)").unwrap();
    static ref RAND_STRING_REGEX: Regex = Regex::new(r"%r(\d+)").unwrap();
}

impl Config {
    pub fn make_filename(&self, n: Option<&str>) -> String {
        let n = match n {
            Some(n) => n,
            None => return generate_random_string(12),
        };

        let now = Utc::now();

        let timestamp = format!(
            "{:04}-{:02}-{:02}_{:02}-{:02}-{:02}",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        ); // Equivalent to %ts

        let _date = format!("{:04}-{:02}-{:02}", now.year(), now.month(), now.day()); // Equivalent to %d

        let time = format!("{:02}-{:02}-{:02}", now.hour(), now.minute(), now.second()); // Equivalent to %t

        let mut result = n.to_string();

        result = FILENAME_REGEX
            .replace_all(&result, |caps: &regex::Captures| {
                match caps.get(1).unwrap().as_str() {
                    "y" => now.year().to_string(),
                    "mo" => now.month().to_string(),
                    "d" => now.day().to_string(),
                    "h" => now.hour().to_string(),
                    "m" => now.minute().to_string(),
                    "s" => now.second().to_string(),
                    "ts" => timestamp.to_string(),
                    "t" => time.to_string(),
                    "i" => self.last_index.to_string(),
                    _ => caps.get(0).unwrap().as_str().to_string(),
                }
            })
            .into_owned();

        result = RAND_STRING_REGEX
            .replace_all(&result, |caps: &regex::Captures| {
                let length: usize = caps
                    .get(1)
                    .map(|m| m.as_str().parse().unwrap_or(0))
                    .unwrap_or(0);
                generate_random_string(length)
            })
            .into_owned();

        result
    }
}

fn generate_random_string(l: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    let one_char = || CHARSET[rng.gen_range(0..CHARSET.len())] as char;
    iter::repeat_with(one_char).take(l).collect()
}

pub fn make_default_config_path() -> String {
    home::home_dir()
        .unwrap()
        .join(".config/delenix/config.json")
        .to_str()
        .unwrap()
        .to_string()
}

pub fn make_default_image_path() -> String {
    home::home_dir()
        .unwrap()
        .join("Screenshots")
        .to_str()
        .unwrap()
        .to_string()
}

pub fn handle_simple_upload(config: &config::Config, data: &[u8]) {
    tracing::info!("Uploading file");
    match upload::upload(config, data, "png") {
        Ok(results) => {
            for result in results {
                if result.error_message.is_some() {
                    tracing::error!("Failed to upload: {}", result.error_message.unwrap());
                    continue;
                }

                if result.url.is_some() {
                    tracing::info!("Uploaded URL: {}", result.url.unwrap());
                }

                if result.deletion_url.is_some() {
                    tracing::info!("Delete URL: {}", result.deletion_url.unwrap());
                }

                if result.file_path.is_some() {
                    tracing::info!("File path: {}", result.file_path.unwrap());
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to upload: {}", e);
        }
    }
}

lazy_static! {
    static ref JSON_PATH_REGEX: Regex = Regex::new(r"\$json:([^{}]+)\$").unwrap();
}

pub fn parse_custom_syntax(
    input: &str,
    json_response: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let parsed_response = JSON_PATH_REGEX.replace_all(input, |captures: &regex::Captures<'_>| {
        let json_path = captures.get(1).unwrap().as_str();
        match evaluate_json_path(json_response, json_path) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Error while parsing JSON path syntax: {}", e);
                String::new()
            }
        }
    });

    // Remove the square brackets and quotes from the parsed response
    let cleaned_response = parsed_response
        .as_ref()
        .replace("[\"", "")
        .replace("\"]", "");

    Ok(cleaned_response)
}

fn evaluate_json_path(json: &str, path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let value: Value = serde_json::from_str(json)?;
    Ok(Selector::new()
        .value(&value)
        .str_path(format!("$.{}", path).as_str())?
        .select_as_str()?)
}

#[macro_export]
macro_rules! handle_error {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Error: {}", e);
                return;
            }
        }
    };
}
