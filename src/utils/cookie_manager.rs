#![allow(dead_code)]

use std::path::{Path, PathBuf};

use anyhow::Context as _;
use tracing::info;

pub struct CookieManager;

impl CookieManager {
    pub fn get_cookie_path() -> PathBuf {
        std::env::var("YT_COOKIE_FILE")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("cookies.txt"))
    }

    pub fn is_valid_cookie_file(path: &Path) -> bool {
        path.exists() && path.is_file()
    }

    pub fn check_and_create() -> anyhow::Result<()> {
        let path = Self::get_cookie_path();
        if !path.exists() {
            std::fs::write(&path, "")
                .with_context(|| format!("Failed to create cookie file at {}", path.display()))?;
        }
        Ok(())
    }

    pub fn get_args() -> Vec<String> {
        let path = Self::get_cookie_path();
        if Self::is_valid_cookie_file(&path) {
            vec!["--cookies".to_string(), path.to_string_lossy().to_string()]
        } else {
            Vec::new()
        }
    }

    pub fn log_status() {
        let path = Self::get_cookie_path();
        if Self::is_valid_cookie_file(&path) {
            info!("Cookie file loaded from {}", path.display());
        } else {
            info!("Cookie file missing at {}", path.display());
        }
    }
}
