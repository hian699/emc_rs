use anyhow::{Context as _, anyhow};
use serde::Deserialize;
use tokio::process::Command;

#[derive(Debug, Clone, Deserialize)]
pub struct YtDlpVideoInfo {
    pub title: String,
    pub webpage_url: String,
    pub duration: Option<f64>,
}

pub struct YtDlpHelper;

impl YtDlpHelper {
    pub fn get_command() -> String {
        std::env::var("YT_DLP_BIN").unwrap_or_else(|_| "yt-dlp".to_string())
    }

    pub async fn execute(args: &[&str]) -> anyhow::Result<String> {
        let output = Command::new(Self::get_command())
            .args(args)
            .output()
            .await
            .context("Failed to execute yt-dlp")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(anyhow!("yt-dlp returned non-zero status: {stderr}"));
        }

        String::from_utf8(output.stdout).context("yt-dlp output was not UTF-8")
    }

    pub async fn get_video_info(url: &str) -> anyhow::Result<YtDlpVideoInfo> {
        let raw = Self::execute(&["--dump-single-json", "--no-warnings", url]).await?;
        serde_json::from_str(&raw).context("Failed to parse yt-dlp video info")
    }

    pub async fn search(query: &str) -> anyhow::Result<Vec<YtDlpVideoInfo>> {
        let target = format!("ytsearch5:{query}");
        let raw = Self::execute(&["--dump-single-json", "--no-warnings", &target]).await?;
        #[derive(Deserialize)]
        struct SearchOutput {
            entries: Option<Vec<YtDlpVideoInfo>>,
        }

        let parsed: SearchOutput =
            serde_json::from_str(&raw).context("Failed to parse yt-dlp search output")?;
        Ok(parsed.entries.unwrap_or_default())
    }
}
