//! Montagem da linha de comando do yt-dlp. Função pura, coberta por testes:
//! nada aqui toca em rede ou disco.

use std::path::Path;

use crate::engine::ToolPaths;
use crate::error::CoreError;
use crate::model::{MediaKind, Platform, Trim};
use crate::progress::{FILE_TEMPLATE, META_TEMPLATE, POSTPROCESS_TEMPLATE, PROGRESS_TEMPLATE};
use crate::settings::{AudioFormat, CookiesMode, Settings, VideoQuality, BROWSERS};

pub struct JobSpec<'a> {
    pub url: &'a str,
    pub platform: Platform,
    pub kind: MediaKind,
    pub playlist: bool,
    pub trim: Option<&'a Trim>,
}

/// Recorta a capa 16:9 do YouTube para um quadrado, como capa de álbum.
const SQUARE_COVER: &str =
    "ThumbnailsConvertor+ffmpeg_o:-c:v mjpeg -vf crop=\"'if(gt(ih,iw),iw,ih)':'if(gt(iw,ih),ih,iw)'\"";

/// Aceita `SS`, `MM:SS` ou `HH:MM:SS`.
pub fn valid_timestamp(s: &str) -> bool {
    let parts: Vec<&str> = s.split(':').collect();
    !s.is_empty()
        && parts.len() <= 3
        && parts.iter().enumerate().all(|(i, p)| {
            let len_ok = if i == 0 {
                (1..=5).contains(&p.len())
            } else {
                p.len() == 2
            };
            len_ok && p.chars().all(|c| c.is_ascii_digit())
        })
}

pub fn validate_trim(trim: &Trim) -> Result<(), CoreError> {
    if valid_timestamp(&trim.start) && valid_timestamp(&trim.end) {
        Ok(())
    } else {
        Err(CoreError::InvalidTrim)
    }
}

/// Template do nome do arquivo (sem a pasta).
fn name_template(platform: Platform) -> &'static str {
    match platform {
        // `fd_name` é montado pelos --parse-metadata em `build_args`.
        Platform::Youtube | Platform::Other => "%(fd_name)s",
        Platform::Instagram => {
            "@%(channel,uploader,uploader_id|instagram)s - %(title|video).60s (%(id)s)"
        }
        Platform::Tiktok => {
            "@%(uploader,channel,uploader_id|tiktok)s - %(title|video).60s (%(id)s)"
        }
        Platform::Facebook => "%(uploader,uploader_id|facebook)s - %(title|video).60s (%(id)s)",
    }
}

pub fn build_args(
    spec: &JobSpec,
    s: &Settings,
    tools: &ToolPaths,
    out_dir: &Path,
    temp_dir: &Path,
) -> Vec<String> {
    let mut a: Vec<String> = Vec::new();
    let mut push = |items: &[&str]| a.extend(items.iter().map(|i| i.to_string()));

    push(&[
        "--ignore-config",
        "--no-simulate",
        "--newline",
        "--progress",
        "--color",
        "never",
        "--encoding",
        "utf-8",
        "--progress-template",
        PROGRESS_TEMPLATE,
        "--progress-template",
        POSTPROCESS_TEMPLATE,
        "--print",
        META_TEMPLATE,
        "--print",
        FILE_TEMPLATE,
        "--windows-filenames",
        "--no-mtime",
        "--concurrent-fragments",
        "4",
        "--retries",
        "5",
        "--fragment-retries",
        "5",
    ]);

    if let Some(dir) = &tools.ffmpeg_dir {
        push(&["--ffmpeg-location", &dir.to_string_lossy()]);
    }
    if let Some(deno) = &tools.deno {
        push(&["--js-runtimes", &format!("deno:{}", deno.to_string_lossy())]);
    }

    match s.cookies_mode {
        CookiesMode::Browser if BROWSERS.contains(&s.cookies_browser.as_str()) => {
            push(&["--cookies-from-browser", &s.cookies_browser]);
        }
        CookiesMode::File => {
            if let Some(file) = &s.cookies_file {
                push(&["--cookies", &file.to_string_lossy()]);
            }
        }
        _ => {}
    }

    // Nome e tags: "Artista - Título" quando o título já vem assim; senão
    // artista/faixa do YouTube Music; senão o canal.
    let fd_names = matches!(spec.platform, Platform::Youtube | Platform::Other);
    if fd_names {
        push(&[
            "--replace-in-metadata",
            "uploader",
            " - Topic$",
            "",
            "--parse-metadata",
            r"title:^(?P<fd_artist>.+?) [-–—] (?P<fd_title>.+)$",
            "--parse-metadata",
            "%(fd_artist,artist,uploader|)s - %(fd_title,track,title)s:%(fd_name)s",
            "--replace-in-metadata",
            "fd_name",
            "^ - ",
            "",
        ]);
    }

    let mut name = name_template(spec.platform).to_string();
    if spec.trim.is_some() {
        name.push_str(" (%(section_start)d-%(section_end)d)");
    }
    name.push_str(".%(ext)s");
    let template = if spec.playlist {
        push(&[
            "--yes-playlist",
            "--ignore-errors",
            "--playlist-end",
            &s.playlist_limit.to_string(),
        ]);
        format!("%(playlist_title|Playlist)s/%(playlist_index)02d - {name}")
    } else {
        push(&["--no-playlist"]);
        name
    };
    push(&[
        "-P",
        &out_dir.to_string_lossy(),
        "-P",
        &format!("temp:{}", temp_dir.to_string_lossy()),
        "-o",
        &template,
    ]);

    match spec.kind {
        MediaKind::Audio => {
            let (format, codec) = match s.audio_format {
                AudioFormat::Mp3 => ("bestaudio/best", "mp3"),
                AudioFormat::M4a => ("bestaudio[ext=m4a]/bestaudio/best", "m4a"),
            };
            push(&[
                "-f",
                format,
                "-x",
                "--audio-format",
                codec,
                "--audio-quality",
                &format!("{}K", s.audio_quality),
                "--embed-metadata",
            ]);
            if fd_names {
                push(&[
                    "--parse-metadata",
                    "%(fd_artist,artist,uploader|)s:%(meta_artist)s",
                    "--parse-metadata",
                    "%(fd_title,track,title)s:%(meta_title)s",
                ]);
            }
            if s.embed_cover {
                push(&["--embed-thumbnail", "--convert-thumbnails", "jpg"]);
                if s.square_cover {
                    push(&["--ppa", SQUARE_COVER]);
                }
            }
            if s.sponsorblock && spec.platform == Platform::Youtube {
                push(&["--sponsorblock-remove", "music_offtopic"]);
            }
        }
        MediaKind::Video => {
            push(&["-f", "bv*+ba/b"]);
            let cap = match s.video_quality {
                VideoQuality::Best => None,
                VideoQuality::P1080 => Some(1080),
                VideoQuality::P720 => Some(720),
                VideoQuality::P480 => Some(480),
            };
            if let Some(cap) = cap {
                // Prefere H.264 + AAC: é o que WhatsApp, Instagram e TVs abrem sem reclamar.
                push(&["-S", &format!("res:{cap},vcodec:h264,acodec:aac")]);
            }
            push(&[
                "--merge-output-format",
                "mp4",
                "--remux-video",
                "mp4",
                "--embed-metadata",
            ]);
        }
    }

    if let Some(trim) = spec.trim {
        push(&[
            "--download-sections",
            &format!("*{}-{}", trim.start, trim.end),
        ]);
        if spec.kind == MediaKind::Video {
            push(&["--force-keyframes-at-cuts"]);
        }
    }

    // `--` garante que o link nunca seja lido como opção.
    push(&["--", spec.url]);
    a
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tools() -> ToolPaths {
        ToolPaths {
            ytdlp: "yt-dlp".into(),
            ffmpeg_dir: Some("/bin/ff".into()),
            deno: Some("/bin/deno".into()),
        }
    }

    fn args(spec: &JobSpec, s: &Settings) -> Vec<String> {
        build_args(spec, s, &tools(), Path::new("/dl"), Path::new("/tmp/job"))
    }

    fn has_pair(a: &[String], k: &str, v: &str) -> bool {
        a.windows(2).any(|w| w[0] == k && w[1] == v)
    }

    fn spec(url: &str, platform: Platform, kind: MediaKind) -> JobSpec<'_> {
        JobSpec {
            url,
            platform,
            kind,
            playlist: false,
            trim: None,
        }
    }

    #[test]
    fn youtube_audio_defaults() {
        let a = args(
            &spec("https://youtu.be/a", Platform::Youtube, MediaKind::Audio),
            &Settings::default(),
        );
        assert!(has_pair(&a, "--audio-format", "mp3"));
        assert!(has_pair(&a, "--audio-quality", "192K"));
        assert!(has_pair(&a, "-o", "%(fd_name)s.%(ext)s"));
        assert!(has_pair(&a, "--ffmpeg-location", "/bin/ff"));
        assert!(has_pair(&a, "--js-runtimes", "deno:/bin/deno"));
        assert!(a.contains(&"--embed-thumbnail".to_string()));
        assert!(a.contains(&"--no-playlist".to_string()));
        assert!(!a.contains(&"--sponsorblock-remove".to_string()));
        assert!(!a.iter().any(|x| x.starts_with("--cookies")));
        assert_eq!(&a[a.len() - 2..], ["--", "https://youtu.be/a"]);
    }

    #[test]
    fn social_video_uses_handle_name_and_quality_cap() {
        let s = Settings {
            video_quality: VideoQuality::P720,
            ..Default::default()
        };
        let a = args(
            &spec(
                "https://www.tiktok.com/@x/video/1",
                Platform::Tiktok,
                MediaKind::Video,
            ),
            &s,
        );
        assert!(has_pair(&a, "-S", "res:720,vcodec:h264,acodec:aac"));
        assert!(has_pair(&a, "--merge-output-format", "mp4"));
        assert!(a.iter().any(|x| x.starts_with("@%(uploader,channel")));
        assert!(!a.contains(&"-x".to_string()));
        assert!(!a.iter().any(|x| x.contains("fd_name")));
    }

    #[test]
    fn best_quality_has_no_sort() {
        let s = Settings {
            video_quality: VideoQuality::Best,
            ..Default::default()
        };
        let a = args(
            &spec("https://youtu.be/a", Platform::Youtube, MediaKind::Video),
            &s,
        );
        assert!(!a.contains(&"-S".to_string()));
    }

    #[test]
    fn playlist_goes_to_subfolder_with_limit() {
        let s = Settings {
            playlist_limit: 30,
            ..Default::default()
        };
        let mut sp = spec(
            "https://www.youtube.com/playlist?list=PL1",
            Platform::Youtube,
            MediaKind::Audio,
        );
        sp.playlist = true;
        let a = args(&sp, &s);
        assert!(has_pair(&a, "--playlist-end", "30"));
        assert!(has_pair(
            &a,
            "-o",
            "%(playlist_title|Playlist)s/%(playlist_index)02d - %(fd_name)s.%(ext)s"
        ));
        assert!(!a.contains(&"--no-playlist".to_string()));
    }

    #[test]
    fn cookies_modes() {
        let s = Settings {
            cookies_mode: CookiesMode::Browser,
            cookies_browser: "firefox".into(),
            ..Default::default()
        };
        let sp = spec(
            "https://www.instagram.com/reel/a/",
            Platform::Instagram,
            MediaKind::Video,
        );
        assert!(has_pair(
            &args(&sp, &s),
            "--cookies-from-browser",
            "firefox"
        ));

        let s = Settings {
            cookies_mode: CookiesMode::File,
            cookies_file: Some(PathBuf::from("/home/u/cookies.txt")),
            ..Default::default()
        };
        assert!(has_pair(&args(&sp, &s), "--cookies", "/home/u/cookies.txt"));

        // Navegador fora da lista nunca chega à linha de comando.
        let s = Settings {
            cookies_mode: CookiesMode::Browser,
            cookies_browser: "x --exec calc".into(),
            ..Default::default()
        };
        assert!(!args(&sp, &s).iter().any(|x| x.starts_with("--cookies")));
    }

    #[test]
    fn trim_and_options() {
        let trim = Trim {
            start: "0:30".into(),
            end: "1:15".into(),
        };
        let mut sp = spec("https://youtu.be/a", Platform::Youtube, MediaKind::Video);
        sp.trim = Some(&trim);
        let a = args(&sp, &Settings::default());
        assert!(has_pair(&a, "--download-sections", "*0:30-1:15"));
        assert!(a.contains(&"--force-keyframes-at-cuts".to_string()));

        let s = Settings {
            sponsorblock: true,
            embed_cover: false,
            audio_format: AudioFormat::M4a,
            audio_quality: 320,
            ..Default::default()
        };
        let a = args(
            &spec("https://youtu.be/a", Platform::Youtube, MediaKind::Audio),
            &s,
        );
        assert!(has_pair(&a, "--sponsorblock-remove", "music_offtopic"));
        assert!(has_pair(&a, "--audio-format", "m4a"));
        assert!(has_pair(&a, "--audio-quality", "320K"));
        assert!(!a.contains(&"--embed-thumbnail".to_string()));
    }

    #[test]
    fn timestamps() {
        for ok in ["5", "90", "1:05", "01:02:03"] {
            assert!(valid_timestamp(ok), "{ok}");
        }
        for bad in ["", "1:5", "a", "1:02:03:04", "-5", "1:60x", "*0-10", "1 "] {
            assert!(!valid_timestamp(bad), "{bad}");
        }
    }
}
