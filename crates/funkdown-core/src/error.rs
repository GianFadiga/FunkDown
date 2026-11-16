use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("link inválido: {0}")]
    InvalidUrl(String),
    #[error("trecho inválido: use SS, MM:SS ou HH:MM:SS")]
    InvalidTrim,
    #[error("erro de arquivo: {0}")]
    Io(#[from] std::io::Error),
    #[error("erro de rede: {0}")]
    Http(#[from] reqwest::Error),
    #[error("a verificação SHA-256 de {0} falhou; o arquivo baixado foi descartado")]
    Checksum(String),
    #[error("motor de download: {0}")]
    Engine(String),
    #[error("{0}")]
    Other(String),
}

impl Serialize for CoreError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

/// Causa provável de uma falha, para a interface mostrar uma mensagem útil
/// em vez do erro cru do yt-dlp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    LoginRequired,
    CookiesBrowser,
    AgeRestricted,
    Private,
    Unavailable,
    Unsupported,
    RateLimited,
    GeoBlocked,
    Network,
    EngineMissing,
    DiskFull,
    Unknown,
}

/// Classifica a saída de erro do yt-dlp.
pub fn classify(stderr: &str) -> ErrorCode {
    let s = stderr.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| s.contains(n));

    if has(&[
        "could not copy",
        "cookie database",
        "cookies database",
        "failed to decrypt",
        "dpapi",
        "could not find",
    ]) && s.contains("cookie")
    {
        ErrorCode::CookiesBrowser
    } else if has(&[
        "confirm your age",
        "age-restricted",
        "age restricted",
        "inappropriate for some users",
    ]) {
        ErrorCode::AgeRestricted
    } else if has(&["private video", "this video is private", "is private"]) {
        ErrorCode::Private
    } else if has(&[
        "sign in",
        "log in",
        "login required",
        "logged-in",
        "logged in",
        "use --cookies",
        "authentication",
    ]) {
        ErrorCode::LoginRequired
    } else if has(&[
        "http error 429",
        "too many requests",
        "rate-limit",
        "rate limit",
    ]) {
        ErrorCode::RateLimited
    } else if has(&[
        "not available in your country",
        "geo-restricted",
        "geo restricted",
        "geo-blocked",
    ]) {
        ErrorCode::GeoBlocked
    } else if has(&["unsupported url", "is not a valid url"]) {
        ErrorCode::Unsupported
    } else if has(&["no space left", "disk full", "not enough space"]) {
        ErrorCode::DiskFull
    } else if has(&[
        "ffmpeg not found",
        "ffprobe and ffmpeg not found",
        "ffmpeg is not installed",
    ]) {
        ErrorCode::EngineMissing
    } else if has(&[
        "video unavailable",
        "has been removed",
        "no longer available",
        "does not exist",
        "http error 404",
        "content isn't available",
        "content is not available",
    ]) {
        ErrorCode::Unavailable
    } else if has(&[
        "getaddrinfo",
        "name resolution",
        "timed out",
        "connection reset",
        "connection refused",
        "network is unreachable",
        "unable to connect",
        "unable to download webpage",
        "urlopen error",
        "proxy",
        "ssl",
    ]) {
        ErrorCode::Network
    } else {
        ErrorCode::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_common_failures() {
        let cases = [
            ("ERROR: [youtube] abc: Sign in to confirm you're not a bot. Use --cookies-from-browser", ErrorCode::LoginRequired),
            ("ERROR: [youtube] abc: Sign in to confirm your age. This video may be inappropriate for some users.", ErrorCode::AgeRestricted),
            ("ERROR: [youtube] abc: Private video. Sign in if you've been granted access to this video", ErrorCode::Private),
            ("ERROR: [Instagram] x: Requested content is not available, rate-limit reached or login required. Use --cookies", ErrorCode::LoginRequired),
            ("ERROR: Could not copy Chrome cookie database. See https://github.com/yt-dlp/yt-dlp/issues/7271", ErrorCode::CookiesBrowser),
            ("ERROR: Failed to decrypt with DPAPI. See cookies docs", ErrorCode::CookiesBrowser),
            ("ERROR: Unsupported URL: https://example.com/", ErrorCode::Unsupported),
            ("ERROR: unable to download video data: HTTP Error 429: Too Many Requests", ErrorCode::RateLimited),
            ("ERROR: [youtube] abc: Video unavailable", ErrorCode::Unavailable),
            ("ERROR: Unable to download webpage: <urlopen error [Errno 11001] getaddrinfo failed>", ErrorCode::Network),
            ("ERROR: [Instagram] x: Unable to download webpage: ('Unable to connect to proxy', OSError('Tunnel connection failed: 403 Forbidden'))", ErrorCode::Network),
            ("ERROR: [generic] Unable to download webpage: HTTP Error 404: Not Found", ErrorCode::Unavailable),
            ("ERROR: something nobody has seen before", ErrorCode::Unknown),
        ];
        for (msg, want) in cases {
            assert_eq!(classify(msg), want, "{msg}");
        }
    }
}
