//! Núcleo do FunkDown.
//!
//! Tudo que não é interface mora aqui: detecção de plataforma, montagem dos
//! argumentos do yt-dlp, leitura do progresso, fila de downloads, histórico,
//! configurações e a instalação verificada (SHA-256) das ferramentas externas.
//! O crate não conhece o Tauri, então o mesmo núcleo serve para uma futura
//! versão web/servidor.

pub mod args;
pub mod engine;
pub mod error;
pub mod history;
pub mod model;
pub mod platform;
pub mod progress;
pub mod queue;
pub mod settings;

pub use engine::{Engine, EngineInfo, SetupEvent, Tool, ToolPaths};
pub use error::{CoreError, ErrorCode};
pub use history::{History, HistoryEntry};
pub use model::*;
pub use queue::{AddRequest, AddResult, Event, Manager};
pub use settings::Settings;
