use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::model::MediaKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AudioFormat {
    Mp3,
    M4a,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VideoQuality {
    Best,
    P1080,
    P720,
    P480,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CookiesMode {
    None,
    Browser,
    File,
}

pub const BROWSERS: &[&str] = &[
    "firefox", "chrome", "edge", "brave", "opera", "vivaldi", "chromium",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub download_dir: PathBuf,
    pub subfolders: bool,
    pub max_concurrent: u8,
    pub youtube_default: MediaKind,
    pub social_default: MediaKind,
    pub audio_format: AudioFormat,
    /// kbps: 128, 192 ou 320.
    pub audio_quality: u16,
    pub video_quality: VideoQuality,
    pub embed_cover: bool,
    pub square_cover: bool,
    /// Remove trechos sem música (intro/outro falados) via SponsorBlock.
    /// Desligado por padrão: consulta um serviço de terceiros.
    pub sponsorblock: bool,
    pub playlist_limit: u32,
    pub cookies_mode: CookiesMode,
    pub cookies_browser: String,
    pub cookies_file: Option<PathBuf>,
    pub save_history: bool,
    pub notify: bool,
    /// "auto", "pt" ou "en".
    pub language: String,
    /// "system", "dark" ou "light".
    pub theme: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            download_dir: PathBuf::new(),
            subfolders: true,
            max_concurrent: 3,
            youtube_default: MediaKind::Audio,
            social_default: MediaKind::Video,
            audio_format: AudioFormat::Mp3,
            audio_quality: 192,
            video_quality: VideoQuality::P1080,
            embed_cover: true,
            square_cover: true,
            sponsorblock: false,
            playlist_limit: 100,
            cookies_mode: CookiesMode::None,
            cookies_browser: "firefox".into(),
            cookies_file: None,
            save_history: true,
            notify: true,
            language: "auto".into(),
            theme: "system".into(),
        }
    }
}

impl Settings {
    /// Corrige valores fora da faixa (arquivo editado à mão, versão antiga...).
    pub fn sanitized(mut self, default_dir: &Path) -> Self {
        if self.download_dir.as_os_str().is_empty() {
            self.download_dir = default_dir.to_path_buf();
        }
        self.max_concurrent = self.max_concurrent.clamp(1, 5);
        if ![128, 192, 320].contains(&self.audio_quality) {
            self.audio_quality = 192;
        }
        self.playlist_limit = self.playlist_limit.clamp(1, 1000);
        if !BROWSERS.contains(&self.cookies_browser.as_str()) {
            self.cookies_browser = "firefox".into();
        }
        if !["auto", "pt", "en"].contains(&self.language.as_str()) {
            self.language = "auto".into();
        }
        if !["system", "dark", "light"].contains(&self.theme.as_str()) {
            self.theme = "system".into();
        }
        self
    }

    pub fn load(path: &Path, default_dir: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<Settings>(&b).ok())
            .unwrap_or_default()
            .sanitized(default_dir)
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        write_atomic(path, &serde_json::to_vec_pretty(self).unwrap_or_default())
    }
}

/// Grava em arquivo temporário e renomeia, para nunca deixar um JSON pela metade.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_out_of_range_values() {
        let s = Settings {
            max_concurrent: 40,
            audio_quality: 999,
            playlist_limit: 0,
            cookies_browser: "netscape; rm -rf".into(),
            theme: "neon".into(),
            ..Default::default()
        }
        .sanitized(Path::new("/tmp/dl"));
        assert_eq!(s.max_concurrent, 5);
        assert_eq!(s.audio_quality, 192);
        assert_eq!(s.playlist_limit, 1);
        assert_eq!(s.cookies_browser, "firefox");
        assert_eq!(s.theme, "system");
        assert_eq!(s.download_dir, PathBuf::from("/tmp/dl"));
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        let s: Settings = serde_json::from_str(r#"{"maxConcurrent": 2}"#).unwrap();
        assert_eq!(s.max_concurrent, 2);
        assert!(s.subfolders);
        assert_eq!(s.youtube_default, MediaKind::Audio);
    }
}
