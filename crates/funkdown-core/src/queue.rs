//! Fila de downloads: recebe links, respeita o limite de downloads
//! simultâneos, roda um yt-dlp por item e publica o estado a cada mudança.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::sync::Notify;

use crate::args::{build_args, validate_trim, JobSpec};
use crate::engine::{quiet_command, EngineInfo, SetupEvent, ToolPaths};
use crate::error::{classify, CoreError, ErrorCode};
use crate::history::{History, HistoryEntry};
use crate::model::*;
use crate::platform::{default_kind, detect, parse_url, playlist_hint, PlaylistHint};
use crate::progress::{parse_line, Line};
use crate::settings::Settings;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum Event {
    Job(Box<Job>),
    JobRemoved(u64),
    Setup(SetupEvent),
    Engine(EngineInfo),
}

pub type Emitter = Arc<dyn Fn(Event) + Send + Sync>;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddRequest {
    pub urls: Vec<String>,
    /// `None` = automático (áudio no YouTube, vídeo nas redes sociais).
    pub kind: Option<MediaKind>,
    #[serde(default)]
    pub playlist: bool,
    pub trim: Option<Trim>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AddResult {
    pub added: Vec<Job>,
    pub invalid: Vec<String>,
    pub duplicates: Vec<String>,
}

struct State {
    jobs: Vec<Job>,
    running: HashMap<u64, Arc<Notify>>,
}

struct Inner {
    state: Mutex<State>,
    settings: RwLock<Settings>,
    settings_path: PathBuf,
    tools: RwLock<Option<ToolPaths>>,
    temp_root: PathBuf,
    history: History,
    emit: Emitter,
    next_id: AtomicU64,
    runtime: tokio::runtime::Handle,
}

#[derive(Clone)]
pub struct Manager {
    inner: Arc<Inner>,
}

impl Manager {
    pub fn new(
        settings: Settings,
        settings_path: PathBuf,
        history: History,
        temp_root: PathBuf,
        runtime: tokio::runtime::Handle,
        emit: Emitter,
    ) -> Self {
        // Sobras de uma execução anterior que foi fechada no meio.
        let _ = std::fs::remove_dir_all(&temp_root);
        Self {
            inner: Arc::new(Inner {
                state: Mutex::new(State {
                    jobs: Vec::new(),
                    running: HashMap::new(),
                }),
                settings: RwLock::new(settings),
                settings_path,
                tools: RwLock::new(None),
                temp_root,
                history,
                emit,
                next_id: AtomicU64::new(1),
                runtime,
            }),
        }
    }

    pub fn settings(&self) -> Settings {
        self.inner.settings.read().unwrap().clone()
    }

    pub fn set_settings(&self, settings: Settings) -> Result<Settings, CoreError> {
        let default_dir = self.settings().download_dir;
        let settings = settings.sanitized(&default_dir);
        settings.save(&self.inner.settings_path)?;
        *self.inner.settings.write().unwrap() = settings.clone();
        self.pump();
        Ok(settings)
    }

    /// Informa onde estão as ferramentas; a fila só anda depois disso.
    pub fn set_tools(&self, tools: ToolPaths) {
        *self.inner.tools.write().unwrap() = Some(tools);
        self.pump();
    }

    pub fn history(&self) -> &History {
        &self.inner.history
    }

    pub fn jobs(&self) -> Vec<Job> {
        self.inner.state.lock().unwrap().jobs.clone()
    }

    pub fn job(&self, id: u64) -> Option<Job> {
        self.inner
            .state
            .lock()
            .unwrap()
            .jobs
            .iter()
            .find(|j| j.id == id)
            .cloned()
    }

    pub fn has_active(&self) -> bool {
        !self.inner.state.lock().unwrap().running.is_empty()
    }

    pub fn add(&self, req: AddRequest) -> Result<AddResult, CoreError> {
        if let Some(trim) = &req.trim {
            validate_trim(trim)?;
        }
        let settings = self.settings();
        let mut result = AddResult {
            added: vec![],
            invalid: vec![],
            duplicates: vec![],
        };
        {
            let mut state = self.inner.state.lock().unwrap();
            for raw in &req.urls {
                let Ok(url) = parse_url(raw) else {
                    result.invalid.push(raw.clone());
                    continue;
                };
                let url_str = url.to_string();
                let pending = |j: &Job| j.url == url_str && !j.status.is_finished();
                if state.jobs.iter().any(pending) {
                    result.duplicates.push(raw.clone());
                    continue;
                }
                let playlist = match playlist_hint(&url) {
                    PlaylistHint::Only => true,
                    PlaylistHint::Optional => req.playlist,
                    PlaylistHint::None => false,
                };
                let job = Job {
                    id: self.inner.next_id.fetch_add(1, Ordering::Relaxed),
                    platform: detect(&url),
                    kind: req.kind.unwrap_or_else(|| {
                        default_kind(&url, settings.youtube_default, settings.social_default)
                    }),
                    url: url_str,
                    playlist,
                    // Recorte não faz sentido para uma playlist inteira.
                    trim: if playlist { None } else { req.trim.clone() },
                    status: JobStatus::Queued,
                    title: None,
                    uploader: None,
                    thumbnail: None,
                    duration: None,
                    percent: 0.0,
                    speed: None,
                    eta: None,
                    item_index: None,
                    item_count: None,
                    files: vec![],
                    skipped: 0,
                    error: None,
                    created_at: now_secs(),
                };
                state.jobs.push(job.clone());
                result.added.push(job);
            }
        }
        for job in &result.added {
            (self.inner.emit)(Event::Job(Box::new(job.clone())));
        }
        self.pump();
        Ok(result)
    }

    pub fn cancel(&self, id: u64) {
        let queued = {
            let mut state = self.inner.state.lock().unwrap();
            if let Some(cancel) = state.running.get(&id) {
                cancel.notify_one();
                None
            } else {
                state
                    .jobs
                    .iter_mut()
                    .find(|j| j.id == id && j.status == JobStatus::Queued)
                    .map(|j| {
                        j.status = JobStatus::Canceled;
                        j.clone()
                    })
            }
        };
        if let Some(job) = queued {
            (self.inner.emit)(Event::Job(Box::new(job)));
        }
    }

    pub fn retry(&self, id: u64) {
        let job = self.inner.update(id, |j| {
            if matches!(j.status, JobStatus::Failed | JobStatus::Canceled) {
                j.status = JobStatus::Queued;
                j.error = None;
                j.percent = 0.0;
                j.speed = None;
                j.eta = None;
                j.files.clear();
                j.skipped = 0;
            }
        });
        if job.is_some() {
            self.pump();
        }
    }

    pub fn remove(&self, id: u64) {
        let removed = {
            let mut state = self.inner.state.lock().unwrap();
            let before = state.jobs.len();
            state.jobs.retain(|j| j.id != id || j.status.is_active());
            before != state.jobs.len()
        };
        if removed {
            (self.inner.emit)(Event::JobRemoved(id));
        }
    }

    pub fn clear_finished(&self) {
        let removed: Vec<u64> = {
            let mut state = self.inner.state.lock().unwrap();
            let ids = state
                .jobs
                .iter()
                .filter(|j| j.status.is_finished())
                .map(|j| j.id)
                .collect();
            state.jobs.retain(|j| !j.status.is_finished());
            ids
        };
        for id in removed {
            (self.inner.emit)(Event::JobRemoved(id));
        }
    }

    fn pump(&self) {
        Inner::pump(&self.inner);
    }
}

impl Inner {
    fn update(&self, id: u64, f: impl FnOnce(&mut Job)) -> Option<Job> {
        let job = {
            let mut state = self.state.lock().unwrap();
            let job = state.jobs.iter_mut().find(|j| j.id == id)?;
            f(job);
            job.clone()
        };
        (self.emit)(Event::Job(Box::new(job.clone())));
        Some(job)
    }

    /// Inicia itens da fila enquanto houver vaga.
    fn pump(inner: &Arc<Inner>) {
        if inner.tools.read().unwrap().is_none() {
            return;
        }
        let limit = inner.settings.read().unwrap().max_concurrent as usize;
        let mut started = Vec::new();
        {
            let mut state = inner.state.lock().unwrap();
            while state.running.len() < limit {
                let Some(job) = state
                    .jobs
                    .iter_mut()
                    .find(|j| j.status == JobStatus::Queued)
                else {
                    break;
                };
                job.status = JobStatus::Starting;
                let job = job.clone();
                let cancel = Arc::new(Notify::new());
                state.running.insert(job.id, cancel.clone());
                started.push((job, cancel));
            }
        }
        for (job, cancel) in started {
            (inner.emit)(Event::Job(Box::new(job.clone())));
            let inner2 = inner.clone();
            inner.runtime.spawn(async move {
                let id = job.id;
                let outcome = run_job(&inner2, job, cancel).await;
                inner2.finish(id, outcome);
                inner2.state.lock().unwrap().running.remove(&id);
                Inner::pump(&inner2);
            });
        }
    }

    fn finish(&self, id: u64, outcome: Outcome) {
        let job = self.update(id, |j| {
            j.speed = None;
            j.eta = None;
            match outcome {
                Outcome::Canceled => j.status = JobStatus::Canceled,
                // Numa playlist, alguns itens podem falhar e o resto valer.
                Outcome::Exited { ok, stderr } if ok || !j.files.is_empty() => {
                    j.status = JobStatus::Done;
                    j.percent = 100.0;
                    j.skipped = stderr.lines().filter(|l| l.starts_with("ERROR:")).count() as u32;
                }
                Outcome::Exited { stderr, .. } => {
                    j.status = JobStatus::Failed;
                    j.error = Some(JobError {
                        code: classify(&stderr),
                        detail: stderr,
                    });
                }
                Outcome::SpawnFailed(detail) => {
                    j.status = JobStatus::Failed;
                    j.error = Some(JobError {
                        code: ErrorCode::EngineMissing,
                        detail,
                    });
                }
            }
        });
        let Some(job) = job else { return };
        if job.status == JobStatus::Done && self.settings.read().unwrap().save_history {
            if let Some(file) = job.files.last() {
                self.history.add(HistoryEntry {
                    id: 0,
                    title: job.title.clone().unwrap_or_else(|| job.url.clone()),
                    uploader: job.uploader.clone(),
                    platform: job.platform,
                    kind: job.kind,
                    url: job.url.clone(),
                    file: file.clone(),
                    file_count: job.files.len() as u32,
                    finished_at: now_secs(),
                    exists: true,
                });
            }
        }
    }
}

enum Outcome {
    Exited { ok: bool, stderr: String },
    Canceled,
    SpawnFailed(String),
}

async fn read_lines<R: AsyncRead + Unpin>(reader: R, mut on_line: impl FnMut(String)) {
    let mut reader = BufReader::new(reader);
    let mut buf = Vec::new();
    // Bytes crus + conversão tolerante: uma linha fora de UTF-8 não derruba a leitura.
    while let Ok(n) = reader.read_until(b'\n', &mut buf).await {
        if n == 0 {
            break;
        }
        on_line(String::from_utf8_lossy(&buf).trim_end().to_string());
        buf.clear();
    }
}

fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .output();
    }
    #[cfg(unix)]
    {
        // O yt-dlp roda no próprio grupo de processos; isso leva o ffmpeg junto.
        let _ = std::process::Command::new("kill")
            .args(["-TERM", "--", &format!("-{pid}")])
            .output();
    }
}

async fn run_job(inner: &Arc<Inner>, job: Job, cancel: Arc<Notify>) -> Outcome {
    let Some(tools) = inner.tools.read().unwrap().clone() else {
        return Outcome::SpawnFailed("motor não instalado".into());
    };
    let settings = inner.settings.read().unwrap().clone();
    let mut out_dir = settings.download_dir.clone();
    if settings.subfolders {
        out_dir.push(job.platform.folder());
    }
    let temp_dir = inner.temp_root.join(format!("job-{}", job.id));
    if let Err(e) =
        std::fs::create_dir_all(&out_dir).and_then(|_| std::fs::create_dir_all(&temp_dir))
    {
        return Outcome::SpawnFailed(format!("não consegui criar a pasta de destino: {e}"));
    }

    let spec = JobSpec {
        url: &job.url,
        platform: job.platform,
        kind: job.kind,
        playlist: job.playlist,
        trim: job.trim.as_ref(),
    };
    let mut cmd = quiet_command(&tools.ytdlp);
    cmd.args(build_args(&spec, &settings, &tools, &out_dir, &temp_dir))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    cmd.process_group(0);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Outcome::SpawnFailed(format!("não consegui iniciar o yt-dlp: {e}")),
    };
    let pid = child.id();
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");

    let id = job.id;
    let inner_out = inner.clone();
    let out_task = tokio::spawn(async move {
        let mut last_emit = Instant::now() - Duration::from_secs(1);
        // Pós-processadores também rodam antes do download (metadados); só
        // conta como "convertendo" depois que o arquivo terminou de baixar.
        let mut downloaded = false;
        read_lines(stdout, |line| match parse_line(&line) {
            Line::Progress(p) => {
                downloaded = p.finished;
                // Progresso chega dezenas de vezes por segundo; a interface não precisa de tudo.
                if !p.finished && last_emit.elapsed() < Duration::from_millis(200) {
                    return;
                }
                last_emit = Instant::now();
                inner_out.update(id, |j| {
                    j.status = JobStatus::Downloading;
                    if let Some(pct) = p.percent {
                        j.percent = pct;
                    }
                    j.speed = p.speed;
                    j.eta = p.eta;
                    j.item_index = p.item_index.or(j.item_index);
                    j.item_count = p.item_count.or(j.item_count);
                });
            }
            Line::PostProcess if !downloaded => {}
            Line::PostProcess => {
                let processing = inner_out
                    .state
                    .lock()
                    .unwrap()
                    .jobs
                    .iter()
                    .any(|j| j.id == id && j.status == JobStatus::Processing);
                if !processing {
                    inner_out.update(id, |j| {
                        j.status = JobStatus::Processing;
                        j.speed = None;
                        j.eta = None;
                    });
                }
            }
            Line::Meta(m) => {
                downloaded = false;
                inner_out.update(id, |j| {
                    j.status = JobStatus::Downloading;
                    j.percent = 0.0;
                    // Numa playlist o cartão mostra o nome da playlist e a faixa atual vira subtítulo.
                    if j.playlist {
                        j.title = m.playlist_title.clone().or(m.title.clone());
                        j.uploader = m.title.clone();
                    } else {
                        j.title = m.title.clone();
                        j.uploader = m.uploader.clone().or(m.channel.clone());
                    }
                    j.thumbnail = m.thumbnail.clone().filter(|t| t.starts_with("https://"));
                    j.duration = m.duration;
                    j.item_index = m.playlist_index.or(j.item_index);
                    j.item_count = m.playlist_count.or(j.item_count);
                });
            }
            Line::File(path) => {
                inner_out.update(id, |j| j.files.push(path));
            }
            Line::Other => {}
        })
        .await;
    });
    let err_task = tokio::spawn(async move {
        let mut lines: Vec<String> = Vec::new();
        read_lines(stderr, |line| {
            if line.starts_with("ERROR") {
                lines.push(line);
                if lines.len() > 30 {
                    lines.remove(0);
                }
            }
        })
        .await;
        lines.join("\n")
    });

    let outcome = tokio::select! {
        status = child.wait() => {
            let _ = out_task.await;
            let stderr = err_task.await.unwrap_or_default();
            match status {
                Ok(s) => Outcome::Exited { ok: s.success(), stderr },
                Err(e) => Outcome::SpawnFailed(e.to_string()),
            }
        }
        _ = cancel.notified() => {
            if let Some(pid) = pid {
                kill_tree(pid);
            }
            let _ = child.kill().await;
            let _ = child.wait().await;
            Outcome::Canceled
        }
    };
    let _ = std::fs::remove_dir_all(&temp_dir);
    outcome
}
