import { invoke } from "@tauri-apps/api/core";
import type { AddResult, Bootstrap, EngineInfo, HistoryEntry, LinkInfo, MediaKind, Settings, Trim } from "./types";

/** Uma função por comando Rust (src-tauri/src/lib.rs). */
export const api = {
  bootstrap: () => invoke<Bootstrap>("bootstrap"),
  setupEngine: () => invoke<void>("setup_engine"),
  engineInfo: () => invoke<EngineInfo>("engine_info"),
  updateEngine: () => invoke<EngineInfo>("update_engine"),
  inspectLinks: (text: string) => invoke<LinkInfo[]>("inspect_links", { text }),
  addDownloads: (request: { urls: string[]; kind: MediaKind | null; playlist: boolean; trim: Trim | null }) =>
    invoke<AddResult>("add_downloads", { request }),
  cancelJob: (id: number) => invoke<void>("cancel_job", { id }),
  retryJob: (id: number) => invoke<void>("retry_job", { id }),
  removeJob: (id: number) => invoke<void>("remove_job", { id }),
  clearFinished: () => invoke<void>("clear_finished"),
  saveSettings: (settings: Settings) => invoke<Settings>("save_settings", { settings }),
  historyList: () => invoke<HistoryEntry[]>("history_list"),
  historyRemove: (id: number) => invoke<void>("history_remove", { id }),
  historyClear: () => invoke<void>("history_clear"),
  openDownload: (source: "job" | "history", id: number, reveal: boolean) =>
    invoke<void>("open_download", { source, id, reveal }),
  openDownloadDir: () => invoke<void>("open_download_dir"),
  pickFolder: () => invoke<string | null>("pick_folder"),
  pickCookiesFile: () => invoke<string | null>("pick_cookies_file"),
};
