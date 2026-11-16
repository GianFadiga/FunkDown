//! O "motor": yt-dlp, FFmpeg e Deno. Em vez de irem dentro do instalador,
//! são baixados dos releases oficiais na primeira execução e só são usados
//! depois de conferir o SHA-256 publicado junto com cada release.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use futures_util::StreamExt;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::CoreError;

const UPDATE_INTERVAL: Duration = Duration::from_secs(12 * 60 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Ytdlp,
    Ffmpeg,
    Deno,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Downloading,
    Verifying,
    Installing,
    Done,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupEvent {
    pub tool: Tool,
    pub stage: Stage,
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Onde está cada ferramenta. `None` em `ffmpeg_dir` significa "use o do sistema".
#[derive(Debug, Clone)]
pub struct ToolPaths {
    pub ytdlp: PathBuf,
    pub ffmpeg_dir: Option<PathBuf>,
    pub deno: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineInfo {
    pub ytdlp: Option<String>,
    pub ffmpeg: Option<String>,
    pub deno: Option<String>,
    pub system_ffmpeg: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Archive {
    Raw,
    Zip,
    TarXz,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Asset {
    base: &'static str,
    name: String,
    sums: String,
    archive: Archive,
}

fn asset_for(tool: Tool, os: &str, arch: &str) -> Option<Asset> {
    const YTDLP: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest/download/";
    const FFMPEG: &str = "https://github.com/yt-dlp/FFmpeg-Builds/releases/latest/download/";
    const DENO: &str = "https://github.com/denoland/deno/releases/latest/download/";
    let arm = arch == "aarch64";
    let (base, name, archive) = match (tool, os) {
        (Tool::Ytdlp, "windows") => (YTDLP, "yt-dlp.exe".to_string(), Archive::Raw),
        (Tool::Ytdlp, "linux") => (
            YTDLP,
            if arm {
                "yt-dlp_linux_aarch64"
            } else {
                "yt-dlp_linux"
            }
            .to_string(),
            Archive::Raw,
        ),
        (Tool::Ffmpeg, "windows") => (
            FFMPEG,
            "ffmpeg-master-latest-win64-gpl.zip".to_string(),
            Archive::Zip,
        ),
        (Tool::Ffmpeg, "linux") => (
            FFMPEG,
            format!(
                "ffmpeg-master-latest-{}-gpl.tar.xz",
                if arm { "linuxarm64" } else { "linux64" }
            ),
            Archive::TarXz,
        ),
        (Tool::Deno, "windows") => (
            DENO,
            "deno-x86_64-pc-windows-msvc.zip".to_string(),
            Archive::Zip,
        ),
        (Tool::Deno, "linux") => (
            DENO,
            format!(
                "deno-{}-unknown-linux-gnu.zip",
                if arm { "aarch64" } else { "x86_64" }
            ),
            Archive::Zip,
        ),
        _ => return None,
    };
    let sums = match tool {
        Tool::Ytdlp => "SHA2-256SUMS".to_string(),
        Tool::Ffmpeg => "checksums.sha256".to_string(),
        Tool::Deno => format!("{name}.sha256sum"),
    };
    Some(Asset {
        base,
        name,
        sums,
        archive,
    })
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Acha o hash de `name` num arquivo de checksums. Entende o formato
/// `hash  nome` (sha256sum) e o do PowerShell (`Hash : ...`), usado pelo Deno no Windows.
pub fn parse_checksum(text: &str, name: &str) -> Option<String> {
    for line in text.lines() {
        let mut tokens = line.split_whitespace();
        if let (Some(hash), Some(file)) = (tokens.next(), tokens.next_back()) {
            if is_sha256(hash) && file.trim_start_matches('*') == name {
                return Some(hash.to_lowercase());
            }
        }
    }
    // Arquivo de um hash só, sem o nome ao lado.
    let mut hashes: Vec<String> = text
        .split(|c: char| !c.is_ascii_hexdigit())
        .filter(|t| is_sha256(t))
        .map(|t| t.to_lowercase())
        .collect();
    hashes.dedup();
    let named_lines = text
        .lines()
        .filter(|l| {
            l.split_whitespace().count() == 2
                && is_sha256(l.split_whitespace().next().unwrap_or(""))
        })
        .count();
    (hashes.len() == 1 && named_lines == 0).then(|| hashes.remove(0))
}

fn exe(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

fn find_in_path(name: &str) -> Option<PathBuf> {
    let file = exe(name);
    std::env::split_paths(&std::env::var_os("PATH")?)
        .map(|dir| dir.join(&file))
        .find(|p| p.is_file())
}

pub(crate) fn quiet_command(program: &Path) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(program);
    cmd.stdin(std::process::Stdio::null());
    #[cfg(windows)]
    cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: sem janela preta piscando
    cmd
}

async fn first_line(program: &Path, arg: &str) -> Option<String> {
    let out = quiet_command(program).arg(arg).output().await.ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    text.lines()
        .next()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
}

pub struct Engine {
    bin_dir: PathBuf,
    http: reqwest::Client,
}

impl Engine {
    pub fn new(bin_dir: PathBuf) -> Self {
        let http = reqwest::Client::builder()
            .https_only(true)
            .user_agent(concat!("FunkDown/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(Duration::from_secs(20))
            .build()
            .expect("cliente HTTP");
        Self { bin_dir, http }
    }

    fn local(&self, name: &str) -> PathBuf {
        self.bin_dir.join(exe(name))
    }

    fn has_local_ffmpeg(&self) -> bool {
        self.local("ffmpeg").is_file() && self.local("ffprobe").is_file()
    }

    fn system_ffmpeg(&self) -> bool {
        cfg!(unix) && find_in_path("ffmpeg").is_some() && find_in_path("ffprobe").is_some()
    }

    fn deno_path(&self) -> Option<PathBuf> {
        let local = self.local("deno");
        if local.is_file() {
            Some(local)
        } else {
            find_in_path("deno")
        }
    }

    pub fn tools(&self) -> ToolPaths {
        ToolPaths {
            ytdlp: self.local("yt-dlp"),
            ffmpeg_dir: self.has_local_ffmpeg().then(|| self.bin_dir.clone()),
            deno: self.deno_path(),
        }
    }

    pub fn missing(&self) -> Vec<Tool> {
        let mut out = Vec::new();
        if !self.local("yt-dlp").is_file() {
            out.push(Tool::Ytdlp);
        }
        if !self.has_local_ffmpeg() && !self.system_ffmpeg() {
            out.push(Tool::Ffmpeg);
        }
        if self.deno_path().is_none() {
            out.push(Tool::Deno);
        }
        out
    }

    /// Baixa, verifica e instala o que estiver faltando.
    pub async fn ensure(&self, on: &(dyn Fn(SetupEvent) + Send + Sync)) -> Result<(), CoreError> {
        for tool in self.missing() {
            self.install(tool, on).await?;
        }
        Ok(())
    }

    async fn install(
        &self,
        tool: Tool,
        on: &(dyn Fn(SetupEvent) + Send + Sync),
    ) -> Result<(), CoreError> {
        let asset = asset_for(tool, std::env::consts::OS, std::env::consts::ARCH)
            .ok_or_else(|| CoreError::Engine("sistema operacional não suportado".into()))?;
        let work = self.bin_dir.join(".download");
        tokio::fs::create_dir_all(&work).await?;
        let part = work.join(format!("{}.part", asset.name));

        let sums = self
            .http
            .get(format!("{}{}", asset.base, asset.sums))
            .send()
            .await?
            .error_for_status()?
            .text()
            .await?;
        let expected = parse_checksum(&sums, &asset.name).ok_or_else(|| {
            CoreError::Engine(format!("checksum de {} não encontrado", asset.name))
        })?;

        let resp = self
            .http
            .get(format!("{}{}", asset.base, asset.name))
            .send()
            .await?
            .error_for_status()?;
        let total = resp.content_length();
        let mut stream = resp.bytes_stream();
        let mut file = tokio::fs::File::create(&part).await?;
        let mut hasher = Sha256::new();
        let (mut downloaded, mut last) = (0u64, Instant::now());
        on(SetupEvent {
            tool,
            stage: Stage::Downloading,
            downloaded,
            total,
        });
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            hasher.update(&chunk);
            file.write_all(&chunk).await?;
            downloaded += chunk.len() as u64;
            if last.elapsed() > Duration::from_millis(120) {
                last = Instant::now();
                on(SetupEvent {
                    tool,
                    stage: Stage::Downloading,
                    downloaded,
                    total,
                });
            }
        }
        file.flush().await?;
        drop(file);

        on(SetupEvent {
            tool,
            stage: Stage::Verifying,
            downloaded,
            total,
        });
        let actual: String = hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        if actual != expected {
            let _ = tokio::fs::remove_file(&part).await;
            return Err(CoreError::Checksum(asset.name));
        }

        on(SetupEvent {
            tool,
            stage: Stage::Installing,
            downloaded,
            total,
        });
        let wanted: Vec<String> = match tool {
            Tool::Ytdlp => vec![exe("yt-dlp")],
            Tool::Ffmpeg => vec![exe("ffmpeg"), exe("ffprobe")],
            Tool::Deno => vec![exe("deno")],
        };
        let bin_dir = self.bin_dir.clone();
        let (archive, part2) = (asset.archive, part.clone());
        tokio::task::spawn_blocking(move || install_files(archive, &part2, &bin_dir, &wanted))
            .await
            .map_err(|e| CoreError::Engine(e.to_string()))??;
        let _ = tokio::fs::remove_file(&part).await;
        on(SetupEvent {
            tool,
            stage: Stage::Done,
            downloaded,
            total,
        });
        Ok(())
    }

    pub async fn info(&self) -> EngineInfo {
        let tools = self.tools();
        let ffmpeg = match &tools.ffmpeg_dir {
            Some(dir) => Some(dir.join(exe("ffmpeg"))),
            None => find_in_path("ffmpeg"),
        };
        let ffmpeg = match ffmpeg {
            // "ffmpeg version N-1234-gabc Copyright ..." -> "N-1234-gabc"
            Some(p) => first_line(&p, "-version")
                .await
                .and_then(|l| l.split_whitespace().nth(2).map(String::from)),
            None => None,
        };
        let deno = match &tools.deno {
            Some(p) => first_line(p, "--version")
                .await
                .and_then(|l| l.split_whitespace().nth(1).map(String::from)),
            None => None,
        };
        EngineInfo {
            ytdlp: first_line(&tools.ytdlp, "--version").await,
            ffmpeg,
            deno,
            system_ffmpeg: tools.ffmpeg_dir.is_none(),
        }
    }

    fn stamp(&self) -> PathBuf {
        self.bin_dir.join(".last-update")
    }

    /// Já faz mais de 12 h desde a última checagem de atualização do yt-dlp?
    pub fn update_due(&self) -> bool {
        std::fs::metadata(self.stamp())
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| SystemTime::now().duration_since(t).ok())
            .is_none_or(|age| age > UPDATE_INTERVAL)
    }

    /// Pede ao yt-dlp para se atualizar (ele mesmo confere o hash do release).
    /// Devolve a versão depois da atualização.
    pub async fn update_ytdlp(&self) -> Result<String, CoreError> {
        let ytdlp = self.local("yt-dlp");
        let out = quiet_command(&ytdlp)
            .args(["--ignore-config", "-U"])
            .output()
            .await
            .map_err(|e| CoreError::Engine(e.to_string()))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(CoreError::Engine(
                err.lines()
                    .last()
                    .unwrap_or("falha ao atualizar")
                    .to_string(),
            ));
        }
        let _ = std::fs::write(self.stamp(), b"");
        first_line(&ytdlp, "--version")
            .await
            .ok_or_else(|| CoreError::Engine("yt-dlp não respondeu".into()))
    }
}

fn install_files(
    archive: Archive,
    src: &Path,
    bin_dir: &Path,
    wanted: &[String],
) -> Result<(), CoreError> {
    let place = |tmp: &Path, name: &str| -> std::io::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(tmp, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(tmp, bin_dir.join(name))
    };
    match archive {
        Archive::Raw => {
            let tmp = bin_dir.join(format!("{}.new", wanted[0]));
            std::fs::copy(src, &tmp)?;
            place(&tmp, &wanted[0])?;
        }
        Archive::Zip => {
            let mut zip = zip::ZipArchive::new(std::fs::File::open(src)?)
                .map_err(|e| CoreError::Engine(format!("zip inválido: {e}")))?;
            let mut found = 0;
            for i in 0..zip.len() {
                let mut entry = zip
                    .by_index(i)
                    .map_err(|e| CoreError::Engine(e.to_string()))?;
                // Só o nome final importa: nada de caminhos vindos do zip.
                let Some(base) = entry.name().rsplit(['/', '\\']).next().map(String::from) else {
                    continue;
                };
                if entry.is_file() && wanted.contains(&base) {
                    let tmp = bin_dir.join(format!("{base}.new"));
                    std::io::copy(&mut entry, &mut std::fs::File::create(&tmp)?)?;
                    place(&tmp, &base)?;
                    found += 1;
                }
            }
            if found < wanted.len() {
                return Err(CoreError::Engine("arquivo esperado não está no zip".into()));
            }
        }
        Archive::TarXz => {
            let tmp_dir = bin_dir.join(".download").join("extract");
            let _ = std::fs::remove_dir_all(&tmp_dir);
            std::fs::create_dir_all(&tmp_dir)?;
            let ok = std::process::Command::new("tar")
                .arg("-xJf")
                .arg(src)
                .arg("-C")
                .arg(&tmp_dir)
                .args(["--strip-components=2", "--wildcards"])
                .args(wanted.iter().map(|w| format!("*/bin/{w}")))
                .status()?
                .success();
            if !ok {
                return Err(CoreError::Engine("falha ao extrair o FFmpeg (tar)".into()));
            }
            for name in wanted {
                place(&tmp_dir.join(name), name)?;
            }
            let _ = std::fs::remove_dir_all(&tmp_dir);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const H1: &str = "f5389d3698ecde4ef06f4e6b1ce0294fe9ac1396228c8990b8d3416d6d177a7d";
    const H2: &str = "767ba9dce7052aa196f8b12652f433d76de128ba168966fa84b6394d5e6c6795";

    #[test]
    fn finds_hash_by_exact_name() {
        let text = format!("{H2}  yt-dlp_linux\n{H1}  yt-dlp.exe\n{H2}  yt-dlp.exe.sig\n");
        assert_eq!(parse_checksum(&text, "yt-dlp.exe").as_deref(), Some(H1));
        assert_eq!(parse_checksum(&text, "yt-dlp_linux").as_deref(), Some(H2));
        // Nome ausente numa lista com vários hashes: nunca chuta.
        assert_eq!(parse_checksum(&text, "yt-dlp_macos"), None);
    }

    #[test]
    fn understands_powershell_format() {
        let text = format!(
            "\nAlgorithm : SHA256\nHash      : {}\nPath      : C:\\a\\deno\\deno-x86_64-pc-windows-msvc.zip\n",
            H1.to_uppercase()
        );
        assert_eq!(
            parse_checksum(&text, "deno-x86_64-pc-windows-msvc.zip").as_deref(),
            Some(H1)
        );
    }

    #[test]
    fn single_named_hash_for_other_file_is_rejected() {
        let text = format!("{H1}  outro-arquivo.zip\n");
        assert_eq!(parse_checksum(&text, "deno.zip"), None);
    }

    #[test]
    fn assets_per_platform() {
        let a = asset_for(Tool::Ytdlp, "windows", "x86_64").unwrap();
        assert_eq!(
            (a.name.as_str(), a.sums.as_str(), a.archive),
            ("yt-dlp.exe", "SHA2-256SUMS", Archive::Raw)
        );
        let a = asset_for(Tool::Ffmpeg, "linux", "aarch64").unwrap();
        assert_eq!(a.name, "ffmpeg-master-latest-linuxarm64-gpl.tar.xz");
        let a = asset_for(Tool::Deno, "linux", "x86_64").unwrap();
        assert_eq!(a.sums, "deno-x86_64-unknown-linux-gnu.zip.sha256sum");
        assert!(asset_for(Tool::Ytdlp, "macos", "aarch64").is_none());
    }
}
