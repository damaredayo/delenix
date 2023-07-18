use std::iter;

use chrono::{Datelike, Timelike, Utc};
use lazy_static::lazy_static;
use rand::Rng;
use regex::Regex;

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

pub fn handle_simple_upload(config: &config::Config, data: &[u8]) {
    tracing::info!("Uploading file");
    match upload::upload(config, data, "png") {
        Ok(results) => {
            for result in results {
                match result.uploader {
                    config::Uploader::HTTP(_) => {
                        tracing::info!("Uploaded to {}", result.location);
                    }
                    config::Uploader::File(_) => {
                        tracing::info!("Saved to {}", result.location);
                    }
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to upload: {}", e);
        }
    }
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
