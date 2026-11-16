export type Platform = "youtube" | "instagram" | "tiktok" | "facebook" | "other";
export type MediaKind = "audio" | "video";
export type JobStatus = "queued" | "starting" | "downloading" | "processing" | "done" | "failed" | "canceled";
export type Tool = "ytdlp" | "ffmpeg" | "deno";
export type ErrorCode =
  | "loginRequired"
  | "cookiesBrowser"
  | "ageRestricted"
  | "private"
  | "unavailable"
  | "unsupported"
  | "rateLimited"
  | "geoBlocked"
  | "network"
  | "engineMissing"
  | "diskFull"
  | "unknown";

export interface Trim {
  start: string;
  end: string;
}

export interface Job {
  id: number;
  url: string;
  platform: Platform;
  kind: MediaKind;
  playlist: boolean;
  trim: Trim | null;
  status: JobStatus;
  title: string | null;
  uploader: string | null;
  thumbnail: string | null;
  duration: number | null;
  percent: number;
  speed: number | null;
  eta: number | null;
  itemIndex: number | null;
  itemCount: number | null;
  files: string[];
  skipped: number;
  error: { code: ErrorCode; detail: string } | null;
  createdAt: number;
}

export interface Settings {
  downloadDir: string;
  subfolders: boolean;
  maxConcurrent: number;
  youtubeDefault: MediaKind;
  socialDefault: MediaKind;
  audioFormat: "mp3" | "m4a";
  audioQuality: number;
  videoQuality: "best" | "p1080" | "p720" | "p480";
  embedCover: boolean;
  squareCover: boolean;
  sponsorblock: boolean;
  playlistLimit: number;
  cookiesMode: "none" | "browser" | "file";
  cookiesBrowser: string;
  cookiesFile: string | null;
  saveHistory: boolean;
  notify: boolean;
  language: "auto" | "pt" | "en";
  theme: "system" | "dark" | "light";
}

export interface HistoryEntry {
  id: number;
  title: string;
  uploader: string | null;
  platform: Platform;
  kind: MediaKind;
  url: string;
  file: string;
  fileCount: number;
  finishedAt: number;
  exists: boolean;
}

export interface LinkInfo {
  url: string;
  platform: Platform;
  playlist: "none" | "optional" | "only";
}

export interface SetupEvent {
  tool: Tool;
  stage: "downloading" | "verifying" | "installing" | "done";
  downloaded: number;
  total: number | null;
}

export interface EngineInfo {
  ytdlp: string | null;
  ffmpeg: string | null;
  deno: string | null;
  systemFfmpeg: boolean;
}

export interface Bootstrap {
  settings: Settings;
  jobs: Job[];
  missing: Tool[];
  version: string;
  os: string;
  updaterEnabled: boolean;
}

export interface AddResult {
  added: Job[];
  invalid: string[];
  duplicates: string[];
}
