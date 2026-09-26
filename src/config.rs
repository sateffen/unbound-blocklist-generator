use std::fs::read_to_string;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub target_filename: String,
    pub allowed_domains: Vec<String>,
    pub blocked_domains: Vec<String>,
    pub blocklist_urls: Vec<String>,
    pub max_parallel_downloads: Option<usize>,
}

impl Config {
    pub fn new(filename: &str) -> Result<Config, Box<dyn std::error::Error>> {
        let config_file_content = read_to_string(filename)?;
        let config: Config = toml::from_str(&config_file_content)?;

        Ok(config)
    }
}
