//! Imprime (um argumento por linha) o que o núcleo passaria ao yt-dlp. Útil para depurar.
use funkdown_core::args::{build_args, JobSpec};
use funkdown_core::{MediaKind, Platform, Settings, ToolPaths};
fn main() {
    let url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://youtu.be/exemplo".into());
    let spec = JobSpec {
        url: &url,
        platform: Platform::Youtube,
        kind: MediaKind::Audio,
        playlist: false,
        trim: None,
    };
    let tools = ToolPaths {
        ytdlp: "yt-dlp".into(),
        ffmpeg_dir: None,
        deno: None,
    };
    let out = std::path::Path::new("out");
    for a in build_args(&spec, &Settings::default(), &tools, out, &out.join("tmp")) {
        println!("{a}");
    }
}
