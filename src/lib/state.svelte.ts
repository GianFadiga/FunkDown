import { api } from "./api";
import { dictionaries, systemLang, type Key, type Lang } from "./i18n";
import type { EngineInfo, Job, Settings, SetupEvent, Tool } from "./types";

export type View = "download" | "history" | "settings";
export interface Toast {
  id: number;
  text: string;
  tone: "info" | "error";
}

/** Estado global da interface. O Rust é a fonte da verdade; isto é o espelho. */
export const app = $state({
  ready: false,
  view: "download" as View,
  settings: null as Settings | null,
  jobs: [] as Job[],
  missing: [] as Tool[],
  setup: {} as Partial<Record<Tool, SetupEvent>>,
  engine: null as EngineInfo | null,
  version: "",
  os: "",
  updaterEnabled: false,
  toasts: [] as Toast[],
  /** Links que outra tela pediu para colocar na caixa de download. */
  prefill: "",
});

export function lang(): Lang {
  const choice = app.settings?.language ?? "auto";
  return choice === "auto" ? systemLang() : choice;
}

export function locale(): string {
  return lang() === "pt" ? "pt-BR" : "en-US";
}

export function t(key: Key, params: Record<string, string | number> = {}): string {
  let text = dictionaries[lang()][key];
  for (const [name, value] of Object.entries(params)) {
    text = text.replace(`{${name}}`, String(value));
  }
  return text;
}

let toastId = 0;
export function toast(text: string, tone: Toast["tone"] = "info") {
  const id = ++toastId;
  app.toasts.push({ id, text, tone });
  setTimeout(() => {
    app.toasts = app.toasts.filter((x) => x.id !== id);
  }, 5000);
}

export function upsertJob(job: Job) {
  const i = app.jobs.findIndex((j) => j.id === job.id);
  if (i >= 0) app.jobs[i] = job;
  else app.jobs.push(job);
}

let saveTimer: ReturnType<typeof setTimeout> | undefined;
/** Aplica a mudança na hora e grava logo depois (agrupa cliques seguidos). */
export function updateSettings(patch: Partial<Settings>) {
  if (!app.settings) return;
  Object.assign(app.settings, patch);
  clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    if (!app.settings) return;
    try {
      app.settings = await api.saveSettings($state.snapshot(app.settings));
    } catch (e) {
      toast(String(e), "error");
    }
  }, 250);
}

export function applyTheme() {
  const choice = app.settings?.theme ?? "system";
  const dark = choice === "system" ? matchMedia("(prefers-color-scheme: dark)").matches : choice === "dark";
  document.documentElement.dataset.theme = dark ? "dark" : "light";
  document.documentElement.lang = locale();
}
