//! Casca Tauri: só traduz comandos da interface em chamadas ao núcleo
//! (`funkdown-core`) e repassa os eventos do núcleo para a janela.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use funkdown_core::platform::{detect, extract_urls, parse_url, playlist_hint, PlaylistHint};
use funkdown_core::{
    AddRequest, AddResult, Engine, EngineInfo, Event, History, HistoryEntry, Job, Manager,
    Platform, Settings, Tool,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager as _, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

const PUBKEY_PLACEHOLDER: &str = "SUBSTITUA";

struct App {
    manager: Manager,
    engine: Arc<Engine>,
    updater_enabled: bool,
}

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Bootstrap {
    settings: Settings,
    jobs: Vec<Job>,
    missing: Vec<Tool>,
    version: String,
    os: &'static str,
    updater_enabled: bool,
}

#[tauri::command]
fn bootstrap(app: AppHandle, state: State<App>) -> Bootstrap {
    Bootstrap {
        settings: state.manager.settings(),
        jobs: state.manager.jobs(),
        missing: state.engine.missing(),
        version: app.package_info().version.to_string(),
        os: std::env::consts::OS,
        updater_enabled: state.updater_enabled,
    }
}

/// Atualiza o yt-dlp se já passou o intervalo, libera a fila e avisa a
/// interface das versões instaladas.
async fn start_engine(app: AppHandle, engine: Arc<Engine>, manager: Manager) {
    if engine.update_due() && !manager.has_active() {
        // Sem internet a atualização só é pulada; o download em si avisa do problema.
        let _ = tokio::time::timeout(Duration::from_secs(30), engine.update_ytdlp()).await;
    }
    manager.set_tools(engine.tools());
    let _ = app.emit("engine", engine.info().await);
}

/// Primeira execução: baixa e verifica yt-dlp, FFmpeg e Deno.
#[tauri::command]
async fn setup_engine(app: AppHandle, state: State<'_, App>) -> CmdResult<()> {
    let emitter = app.clone();
    state
        .engine
        .ensure(&move |event| {
            let _ = emitter.emit("setup", event);
        })
        .await
        .map_err(err)?;
    start_engine(app, state.engine.clone(), state.manager.clone()).await;
    Ok(())
}

#[tauri::command]
async fn engine_info(state: State<'_, App>) -> CmdResult<EngineInfo> {
    Ok(state.engine.info().await)
}

#[tauri::command]
async fn update_engine(state: State<'_, App>) -> CmdResult<EngineInfo> {
    if state.manager.has_active() {
        return Err("busy".into());
    }
    state.engine.update_ytdlp().await.map_err(err)?;
    Ok(state.engine.info().await)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LinkInfo {
    url: String,
    platform: Platform,
    /// "none", "optional" (vídeo dentro de playlist) ou "only".
    playlist: &'static str,
}

#[tauri::command]
fn inspect_links(text: String) -> Vec<LinkInfo> {
    extract_urls(&text)
        .into_iter()
        .filter_map(|raw| {
            let url = parse_url(&raw).ok()?;
            Some(LinkInfo {
                platform: detect(&url),
                playlist: match playlist_hint(&url) {
                    PlaylistHint::None => "none",
                    PlaylistHint::Optional => "optional",
                    PlaylistHint::Only => "only",
                },
                url: raw,
            })
        })
        .collect()
}

#[tauri::command]
fn add_downloads(state: State<App>, request: AddRequest) -> CmdResult<AddResult> {
    state.manager.add(request).map_err(err)
}

#[tauri::command]
fn cancel_job(state: State<App>, id: u64) {
    state.manager.cancel(id);
}

#[tauri::command]
fn retry_job(state: State<App>, id: u64) {
    state.manager.retry(id);
}

#[tauri::command]
fn remove_job(state: State<App>, id: u64) {
    state.manager.remove(id);
}

#[tauri::command]
fn clear_finished(state: State<App>) {
    state.manager.clear_finished();
}

#[tauri::command]
fn save_settings(state: State<App>, settings: Settings) -> CmdResult<Settings> {
    state.manager.set_settings(settings).map_err(err)
}

#[tauri::command]
fn history_list(state: State<App>) -> Vec<HistoryEntry> {
    state.manager.history().list()
}

#[tauri::command]
fn history_remove(state: State<App>, id: u64) {
    state.manager.history().remove(id);
}

#[tauri::command]
fn history_clear(state: State<App>) {
    state.manager.history().clear();
}

/// Abre (ou mostra na pasta) um arquivo baixado. A interface só informa o
/// id: o caminho vem sempre do que o próprio app gravou, nunca da página.
#[tauri::command]
fn open_download(
    app: AppHandle,
    state: State<App>,
    source: String,
    id: u64,
    reveal: bool,
) -> CmdResult<()> {
    let file = match source.as_str() {
        "job" => state.manager.job(id).and_then(|j| j.files.last().cloned()),
        "history" => state.manager.history().get(id).map(|h| h.file),
        _ => None,
    }
    .ok_or("missing")?;
    let path = PathBuf::from(&file);
    if !path.exists() {
        return Err("missing".into());
    }
    if reveal {
        app.opener().reveal_item_in_dir(&path).map_err(err)
    } else {
        app.opener().open_path(file, None::<&str>).map_err(err)
    }
}

#[tauri::command]
fn open_download_dir(app: AppHandle, state: State<App>) -> CmdResult<()> {
    let dir = state.manager.settings().download_dir;
    std::fs::create_dir_all(&dir).map_err(err)?;
    app.opener()
        .open_path(dir.to_string_lossy(), None::<&str>)
        .map_err(err)
}

#[tauri::command]
async fn pick_folder(app: AppHandle) -> Option<String> {
    let picked = app.dialog().file().blocking_pick_folder()?;
    picked
        .into_path()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
async fn pick_cookies_file(app: AppHandle) -> Option<String> {
    let picked = app
        .dialog()
        .file()
        .add_filter("cookies.txt", &["txt"])
        .blocking_pick_file()?;
    picked
        .into_path()
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let paths = app.path();
            let data = paths.app_data_dir()?;
            let config = paths.app_config_dir()?;
            let cache = paths.app_cache_dir()?;
            let downloads = paths
                .download_dir()
                .or_else(|_| paths.home_dir())
                .unwrap_or_else(|_| data.clone())
                .join("FunkDown");
            std::fs::create_dir_all(data.join("bin"))?;

            // O atualizador só liga depois que a chave pública real for configurada.
            let updater_enabled = app
                .config()
                .plugins
                .0
                .get("updater")
                .and_then(|u| u.get("pubkey"))
                .and_then(|k| k.as_str())
                .is_some_and(|k| !k.is_empty() && !k.contains(PUBKEY_PLACEHOLDER));
            if updater_enabled {
                handle.plugin(tauri_plugin_updater::Builder::new().build())?;
            }

            let settings_path = config.join("settings.json");
            let emitter = handle.clone();
            let manager = Manager::new(
                Settings::load(&settings_path, &downloads),
                settings_path,
                History::load(data.join("history.json")),
                cache.join("jobs"),
                tauri::async_runtime::handle().inner().clone(),
                Arc::new(move |event| {
                    let _ = match event {
                        Event::Job(job) => emitter.emit("job", job),
                        Event::JobRemoved(id) => emitter.emit("job-removed", id),
                        Event::Setup(e) => emitter.emit("setup", e),
                        Event::Engine(e) => emitter.emit("engine", e),
                    };
                }),
            );
            let engine = Arc::new(Engine::new(data.join("bin")));
            if engine.missing().is_empty() {
                tauri::async_runtime::spawn(start_engine(handle, engine.clone(), manager.clone()));
            }
            app.manage(App {
                manager,
                engine,
                updater_enabled,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap,
            setup_engine,
            engine_info,
            update_engine,
            inspect_links,
            add_downloads,
            cancel_job,
            retry_job,
            remove_job,
            clear_finished,
            save_settings,
            history_list,
            history_remove,
            history_clear,
            open_download,
            open_download_dir,
            pick_folder,
            pick_cookies_file,
        ])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar o FunkDown");
}
