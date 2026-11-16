use url::Url;

use crate::error::CoreError;
use crate::model::{MediaKind, Platform};

/// Valida e normaliza um link. Só aceita http/https com host.
pub fn parse_url(raw: &str) -> Result<Url, CoreError> {
    let raw = raw.trim();
    let url = Url::parse(raw).map_err(|_| CoreError::InvalidUrl(raw.to_string()))?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(CoreError::InvalidUrl(raw.to_string()));
    }
    Ok(url)
}

fn host_is(host: &str, domain: &str) -> bool {
    host == domain || host.strip_suffix(domain).is_some_and(|p| p.ends_with('.'))
}

pub fn detect(url: &Url) -> Platform {
    let host = url.host_str().unwrap_or("").to_lowercase();
    let any = |ds: &[&str]| ds.iter().any(|d| host_is(&host, d));
    if any(&["youtube.com", "youtu.be", "youtube-nocookie.com"]) {
        Platform::Youtube
    } else if any(&["instagram.com", "instagr.am"]) {
        Platform::Instagram
    } else if any(&["tiktok.com"]) {
        Platform::Tiktok
    } else if any(&["facebook.com", "fb.watch", "fb.com"]) {
        Platform::Facebook
    } else {
        Platform::Other
    }
}

/// Sites "outros" que são essencialmente de áudio.
pub fn is_audio_site(url: &Url) -> bool {
    let host = url.host_str().unwrap_or("").to_lowercase();
    [
        "soundcloud.com",
        "bandcamp.com",
        "mixcloud.com",
        "audiomack.com",
    ]
    .iter()
    .any(|d| host_is(&host, d))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistHint {
    /// Link de um item só.
    None,
    /// Vídeo que faz parte de uma playlist (`watch?v=...&list=...`).
    Optional,
    /// Link da playlist em si.
    Only,
}

pub fn playlist_hint(url: &Url) -> PlaylistHint {
    if detect(url) != Platform::Youtube {
        return PlaylistHint::None;
    }
    let has = |k: &str| url.query_pairs().any(|(key, v)| key == k && !v.is_empty());
    match (has("list"), has("v") || url.host_str() == Some("youtu.be")) {
        (true, true) => PlaylistHint::Optional,
        (true, false) => PlaylistHint::Only,
        _ => PlaylistHint::None,
    }
}

/// Tipo de mídia padrão quando o usuário deixa em "automático".
pub fn default_kind(url: &Url, youtube: MediaKind, social: MediaKind) -> MediaKind {
    match detect(url) {
        Platform::Youtube => youtube,
        Platform::Other if is_audio_site(url) => MediaKind::Audio,
        _ => social,
    }
}

/// Extrai todos os links http(s) de um texto colado, sem repetir.
pub fn extract_urls(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for token in text.split(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | '<' | '>' | '"'))
    {
        let token = token.trim_matches(|c: char| matches!(c, '(' | ')' | '[' | ']' | '\''));
        if (token.starts_with("http://") || token.starts_with("https://"))
            && parse_url(token).is_ok()
            && !out.iter().any(|u| u == token)
        {
            out.push(token.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(u: &str) -> Platform {
        detect(&parse_url(u).unwrap())
    }

    #[test]
    fn detects_platforms() {
        assert_eq!(p("https://www.youtube.com/watch?v=abc"), Platform::Youtube);
        assert_eq!(p("https://youtu.be/abc"), Platform::Youtube);
        assert_eq!(
            p("https://music.youtube.com/watch?v=abc"),
            Platform::Youtube
        );
        assert_eq!(
            p("https://www.instagram.com/reel/abc/"),
            Platform::Instagram
        );
        assert_eq!(p("https://vm.tiktok.com/abc/"), Platform::Tiktok);
        assert_eq!(p("https://fb.watch/abc/"), Platform::Facebook);
        assert_eq!(p("https://m.facebook.com/watch/?v=1"), Platform::Facebook);
        assert_eq!(p("https://soundcloud.com/a/b"), Platform::Other);
        // Domínios parecidos não podem enganar a detecção.
        assert_eq!(p("https://notyoutube.com/watch?v=abc"), Platform::Other);
        assert_eq!(p("https://youtube.com.evil.example/watch"), Platform::Other);
    }

    #[test]
    fn rejects_bad_urls() {
        for bad in [
            "",
            "abc",
            "ftp://x.com/a",
            "file:///etc/passwd",
            "javascript:alert(1)",
            "--exec=calc",
        ] {
            assert!(parse_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn playlist_hints() {
        let h = |u: &str| playlist_hint(&parse_url(u).unwrap());
        assert_eq!(h("https://www.youtube.com/watch?v=abc"), PlaylistHint::None);
        assert_eq!(
            h("https://www.youtube.com/watch?v=abc&list=PL1"),
            PlaylistHint::Optional
        );
        assert_eq!(h("https://youtu.be/abc?list=PL1"), PlaylistHint::Optional);
        assert_eq!(
            h("https://www.youtube.com/playlist?list=PL1"),
            PlaylistHint::Only
        );
        assert_eq!(
            h("https://www.instagram.com/reel/abc/?list=x"),
            PlaylistHint::None
        );
    }

    #[test]
    fn extracts_urls_from_pasted_text() {
        let text = "olha isso https://youtu.be/a, e esse (https://www.tiktok.com/@x/video/1)\nhttps://youtu.be/a nada.com";
        assert_eq!(
            extract_urls(text),
            vec!["https://youtu.be/a", "https://www.tiktok.com/@x/video/1"]
        );
    }

    #[test]
    fn default_kinds() {
        let k = |u: &str| default_kind(&parse_url(u).unwrap(), MediaKind::Audio, MediaKind::Video);
        assert_eq!(k("https://youtu.be/a"), MediaKind::Audio);
        assert_eq!(k("https://www.instagram.com/reel/a/"), MediaKind::Video);
        assert_eq!(k("https://soundcloud.com/a/b"), MediaKind::Audio);
        assert_eq!(k("https://x.com/a/status/1"), MediaKind::Video);
    }
}
