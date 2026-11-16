//! CLI mínima sobre o núcleo, para testar sem interface gráfica:
//!
//!   cargo run -p funkdown-core --example fdl -- [--audio|--video] [--trim 0:10-0:20] [--out PASTA] LINK...
//!
//! Usa o yt-dlp e o ffmpeg que estiverem no PATH (ou FUNKDOWN_YTDLP).

use std::path::PathBuf;
use std::sync::Arc;

use funkdown_core::{AddRequest, Event, History, Manager, MediaKind, Settings, ToolPaths, Trim};

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let mut urls = Vec::new();
    let (mut kind, mut trim, mut out) = (None, None, PathBuf::from("fdl-out"));
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--audio" => kind = Some(MediaKind::Audio),
            "--video" => kind = Some(MediaKind::Video),
            "--out" => out = PathBuf::from(args.next().expect("--out PASTA")),
            "--trim" => {
                let v = args.next().expect("--trim INICIO-FIM");
                let (start, end) = v.split_once('-').expect("--trim INICIO-FIM");
                trim = Some(Trim {
                    start: start.into(),
                    end: end.into(),
                });
            }
            _ => urls.push(arg),
        }
    }
    let state = std::env::temp_dir().join(format!("fdl-{}", std::process::id()));
    let settings = Settings {
        subfolders: false,
        ..Default::default()
    }
    .sanitized(&out);
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let manager = Manager::new(
        settings,
        state.join("settings.json"),
        History::load(state.join("history.json")),
        state.join("tmp"),
        tokio::runtime::Handle::current(),
        Arc::new(move |e| {
            let _ = tx.send(e);
        }),
    );
    manager.set_tools(ToolPaths {
        ytdlp: std::env::var_os("FUNKDOWN_YTDLP")
            .map(PathBuf::from)
            .unwrap_or_else(|| "yt-dlp".into()),
        ffmpeg_dir: None,
        deno: None,
    });
    let result = manager
        .add(AddRequest {
            urls,
            kind,
            playlist: false,
            trim,
        })
        .expect("pedido inválido");
    for bad in &result.invalid {
        eprintln!("link inválido: {bad}");
    }
    let mut pending = result.added.len();
    while pending > 0 {
        let Some(Event::Job(job)) = rx.recv().await else {
            continue;
        };
        println!("{}", serde_json::to_string(&job).unwrap());
        if job.status.is_finished() {
            pending -= 1;
        }
    }
    let _ = std::fs::remove_dir_all(state);
}
