use serde::{Deserialize, Serialize};

use crate::error::ErrorCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Youtube,
    Instagram,
    Tiktok,
    Facebook,
    /// Qualquer outro site que o yt-dlp suporte (X, SoundCloud, Reddit...).
    Other,
}

impl Platform {
    /// Nome da subpasta de downloads.
    pub fn folder(self) -> &'static str {
        match self {
            Platform::Youtube => "YouTube",
            Platform::Instagram => "Instagram",
            Platform::Tiktok => "TikTok",
            Platform::Facebook => "Facebook",
            Platform::Other => "Outros",
        }
    }

    pub fn is_social(self) -> bool {
        matches!(
            self,
            Platform::Instagram | Platform::Tiktok | Platform::Facebook
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MediaKind {
    Audio,
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Starting,
    Downloading,
    Processing,
    Done,
    Failed,
    Canceled,
}

impl JobStatus {
    pub fn is_active(self) -> bool {
        matches!(
            self,
            JobStatus::Starting | JobStatus::Downloading | JobStatus::Processing
        )
    }
    pub fn is_finished(self) -> bool {
        matches!(
            self,
            JobStatus::Done | JobStatus::Failed | JobStatus::Canceled
        )
    }
}

/// Trecho a recortar, em `SS`, `MM:SS` ou `HH:MM:SS`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Trim {
    pub start: String,
    pub end: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobError {
    pub code: ErrorCode,
    /// Últimas linhas de erro do yt-dlp, para diagnóstico.
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: u64,
    pub url: String,
    pub platform: Platform,
    pub kind: MediaKind,
    pub playlist: bool,
    pub trim: Option<Trim>,
    pub status: JobStatus,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub thumbnail: Option<String>,
    pub duration: Option<f64>,
    /// 0–100 do arquivo atual.
    pub percent: f32,
    pub speed: Option<f64>,
    pub eta: Option<u64>,
    /// Posição dentro da playlist (quando for uma).
    pub item_index: Option<u32>,
    pub item_count: Option<u32>,
    /// Arquivos finais já gravados.
    pub files: Vec<String>,
    /// Itens de playlist que falharam e foram pulados.
    pub skipped: u32,
    pub error: Option<JobError>,
    pub created_at: u64,
}

pub(crate) fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
