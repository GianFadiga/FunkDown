//! Leitura das linhas que o yt-dlp imprime. Os templates abaixo fazem o
//! yt-dlp emitir linhas com prefixos nossos, fáceis de separar do resto.

use serde::Deserialize;

pub const PROGRESS_TEMPLATE: &str = "download:FDPROG|%(progress.status)s|%(progress.downloaded_bytes)s|%(progress.total_bytes)s|%(progress.total_bytes_estimate)s|%(progress.speed)s|%(progress.eta)s|%(info.playlist_index)s|%(info.playlist_count)s";
pub const POSTPROCESS_TEMPLATE: &str = "postprocess:FDPOST|%(progress.status)s";
pub const META_TEMPLATE: &str =
    "before_dl:FDMETA|%(.{id,title,uploader,channel,duration,thumbnail,playlist_title,playlist_index,playlist_count})j";
pub const FILE_TEMPLATE: &str = "after_move:FDFILE|%(filepath)s";

#[derive(Debug, Clone, PartialEq)]
pub struct Progress {
    pub finished: bool,
    pub percent: Option<f32>,
    pub speed: Option<f64>,
    pub eta: Option<u64>,
    pub item_index: Option<u32>,
    pub item_count: Option<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Meta {
    pub id: Option<String>,
    pub title: Option<String>,
    pub uploader: Option<String>,
    pub channel: Option<String>,
    pub duration: Option<f64>,
    pub thumbnail: Option<String>,
    pub playlist_title: Option<String>,
    pub playlist_index: Option<u32>,
    pub playlist_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    Progress(Progress),
    PostProcess,
    Meta(Box<Meta>),
    File(String),
    Other,
}

fn num(s: &str) -> Option<f64> {
    s.trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0)
}

pub fn parse_line(line: &str) -> Line {
    let line = line.trim_end_matches(['\r', '\n']);
    if let Some(rest) = line.strip_prefix("FDPROG|") {
        let f: Vec<&str> = rest.split('|').collect();
        if f.len() < 8 {
            return Line::Other;
        }
        let downloaded = num(f[1]);
        let total = num(f[2]).or_else(|| num(f[3])).filter(|t| *t > 0.0);
        let finished = f[0] == "finished";
        let percent = match (downloaded, total) {
            _ if finished => Some(100.0),
            (Some(d), Some(t)) => Some(((d / t) * 100.0).clamp(0.0, 100.0) as f32),
            _ => None,
        };
        Line::Progress(Progress {
            finished,
            percent,
            speed: num(f[4]),
            eta: num(f[5]).map(|v| v as u64),
            item_index: num(f[6]).map(|v| v as u32),
            item_count: num(f[7]).map(|v| v as u32),
        })
    } else if line.starts_with("FDPOST|") {
        Line::PostProcess
    } else if let Some(json) = line.strip_prefix("FDMETA|") {
        match serde_json::from_str::<Meta>(json) {
            Ok(m) => Line::Meta(Box::new(m)),
            Err(_) => Line::Other,
        }
    } else if let Some(path) = line.strip_prefix("FDFILE|") {
        Line::File(path.to_string())
    } else {
        Line::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_progress() {
        let Line::Progress(p) = parse_line("FDPROG|downloading|500|NA|2000.0|1024.5|12|NA|NA\r\n")
        else {
            panic!()
        };
        assert_eq!(p.percent, Some(25.0));
        assert_eq!(p.speed, Some(1024.5));
        assert_eq!(p.eta, Some(12));
        assert_eq!(p.item_index, None);
        assert!(!p.finished);
    }

    #[test]
    fn parses_playlist_and_finished() {
        let Line::Progress(p) = parse_line("FDPROG|finished|900|900|NA|NA|NA|3|25") else {
            panic!()
        };
        assert!(p.finished);
        assert_eq!(p.percent, Some(100.0));
        assert_eq!((p.item_index, p.item_count), (Some(3), Some(25)));
    }

    #[test]
    fn unknown_total_gives_no_percent() {
        let Line::Progress(p) = parse_line("FDPROG|downloading|500|NA|NA|NA|NA|NA|NA") else {
            panic!()
        };
        assert_eq!(p.percent, None);
    }

    #[test]
    fn parses_meta_and_file() {
        let m = parse_line(
            r#"FDMETA|{"id": "abc", "title": "MC Teste - Batidão", "duration": 187, "playlist_index": null}"#,
        );
        let Line::Meta(m) = m else { panic!() };
        assert_eq!(m.title.as_deref(), Some("MC Teste - Batidão"));
        assert_eq!(m.duration, Some(187.0));
        assert_eq!(
            parse_line("FDFILE|C:\\Músicas\\a | b.mp3"),
            Line::File("C:\\Músicas\\a | b.mp3".into())
        );
        assert_eq!(
            parse_line("[youtube] abc: Downloading webpage"),
            Line::Other
        );
        assert_eq!(parse_line("FDPROG|garbage"), Line::Other);
    }
}
