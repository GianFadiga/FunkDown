<script lang="ts">
  import { api } from "../lib/api";
  import { app, t, toast, updateSettings } from "../lib/state.svelte";
  import type { Settings } from "../lib/types";

  let { onCheckUpdate }: { onCheckUpdate: () => Promise<void> } = $props();
  let updatingEngine = $state(false);
  let checking = $state(false);

  const s = $derived(app.settings!);
  const BROWSERS = ["firefox", "chrome", "edge", "brave", "opera", "vivaldi", "chromium"];
  const blockedOnWindows = $derived(
    app.os === "windows" && s.cookiesMode === "browser" && ["chrome", "edge", "brave"].includes(s.cookiesBrowser),
  );

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) => updateSettings({ [key]: value });

  async function pickFolder() {
    const dir = await api.pickFolder();
    if (dir) set("downloadDir", dir);
  }

  async function pickCookies() {
    const file = await api.pickCookiesFile();
    if (file) set("cookiesFile", file);
  }

  async function updateEngine() {
    updatingEngine = true;
    try {
      app.engine = await api.updateEngine();
      toast(t("set.engineUpdated", { v: app.engine.ytdlp ?? "?" }));
    } catch (e) {
      toast(t(String(e) === "busy" ? "set.engineBusy" : "set.engineFailed"), "error");
    } finally {
      updatingEngine = false;
    }
  }

  async function checkUpdate() {
    checking = true;
    await onCheckUpdate();
    checking = false;
  }
</script>

{#snippet toggle(key: "subfolders" | "embedCover" | "squareCover" | "sponsorblock" | "saveHistory" | "notify", label: string, hint = "", disabled = false)}
  <label class="row">
    <span class="label">{label}{#if hint}<small class="muted">{hint}</small>{/if}</span>
    <input class="switch" type="checkbox" checked={s[key]} {disabled} onchange={(e) => set(key, e.currentTarget.checked)} />
  </label>
{/snippet}

<section class="view">
  <h1>{t("nav.settings")}</h1>

  <h2>{t("set.downloads")}</h2>
  <div class="group">
    <div class="row">
      <span class="label">{t("set.folder")}<small class="muted path">{s.downloadDir}</small></span>
      <span class="inline">
        <button class="btn" onclick={pickFolder}>{t("set.choose")}</button>
        <button class="btn ghost" onclick={() => api.openDownloadDir()}>{t("set.open")}</button>
      </span>
    </div>
    {@render toggle("subfolders", t("set.subfolders"))}
    <label class="row">
      <span class="label">{t("set.concurrent")}</span>
      <select class="field" value={s.maxConcurrent} onchange={(e) => set("maxConcurrent", Number(e.currentTarget.value))}>
        {#each [1, 2, 3, 4, 5] as n}<option value={n}>{n}</option>{/each}
      </select>
    </label>
  </div>

  <h2>{t("set.formats")}</h2>
  <div class="group">
    <label class="row">
      <span class="label">{t("set.youtubeDefault")}</span>
      <select class="field" value={s.youtubeDefault} onchange={(e) => set("youtubeDefault", e.currentTarget.value as Settings["youtubeDefault"])}>
        <option value="audio">{t("kind.audio")}</option>
        <option value="video">{t("kind.video")}</option>
      </select>
    </label>
    <label class="row">
      <span class="label">{t("set.socialDefault")}</span>
      <select class="field" value={s.socialDefault} onchange={(e) => set("socialDefault", e.currentTarget.value as Settings["socialDefault"])}>
        <option value="video">{t("kind.video")}</option>
        <option value="audio">{t("kind.audio")}</option>
      </select>
    </label>
    <label class="row">
      <span class="label">{t("set.audioFormat")}</span>
      <select class="field" value={s.audioFormat} onchange={(e) => set("audioFormat", e.currentTarget.value as Settings["audioFormat"])}>
        <option value="mp3">MP3</option>
        <option value="m4a">M4A (AAC)</option>
      </select>
    </label>
    <label class="row">
      <span class="label">{t("set.audioQuality")}</span>
      <select class="field" value={s.audioQuality} onchange={(e) => set("audioQuality", Number(e.currentTarget.value))}>
        {#each [128, 192, 320] as q}<option value={q}>{q} kbps</option>{/each}
      </select>
    </label>
    <label class="row">
      <span class="label">{t("set.videoQuality")}<small class="muted">{t("set.videoHint")}</small></span>
      <select class="field" value={s.videoQuality} onchange={(e) => set("videoQuality", e.currentTarget.value as Settings["videoQuality"])}>
        <option value="best">{t("set.videoBest")}</option>
        <option value="p1080">1080p</option>
        <option value="p720">720p</option>
        <option value="p480">480p</option>
      </select>
    </label>
    {@render toggle("embedCover", t("set.cover"))}
    {@render toggle("squareCover", t("set.square"), "", !s.embedCover)}
    {@render toggle("sponsorblock", t("set.sponsorblock"), t("set.sponsorblockHint"))}
    <label class="row">
      <span class="label">{t("set.playlistLimit")}</span>
      <input
        class="field num"
        type="number"
        min="1"
        max="1000"
        value={s.playlistLimit}
        onchange={(e) => set("playlistLimit", Math.min(1000, Math.max(1, Number(e.currentTarget.value) || 100)))}
      />
    </label>
  </div>

  <h2>{t("set.login")}</h2>
  <p class="note muted">{t("set.loginHint")}</p>
  <div class="group">
    <div class="row">
      <div class="segmented" role="radiogroup">
        {#each ["none", "browser", "file"] as const as m}
          <button role="radio" aria-checked={s.cookiesMode === m} class:on={s.cookiesMode === m} onclick={() => set("cookiesMode", m)}>
            {t(`set.cookies.${m}`)}
          </button>
        {/each}
      </div>
    </div>
    {#if s.cookiesMode === "browser"}
      <label class="row">
        <span class="label">{t("set.browser")}{#if blockedOnWindows}<small class="warn">{t("set.browserWarn")}</small>{/if}</span>
        <select class="field" value={s.cookiesBrowser} onchange={(e) => set("cookiesBrowser", e.currentTarget.value)}>
          {#each BROWSERS as b}<option value={b}>{b[0].toUpperCase() + b.slice(1)}</option>{/each}
        </select>
      </label>
    {:else if s.cookiesMode === "file"}
      <div class="row">
        <span class="label">{t("set.cookiesFile")}<small class="muted path">{s.cookiesFile ?? t("set.cookiesNone")}</small></span>
        <button class="btn" onclick={pickCookies}>{t("set.choose")}</button>
      </div>
    {/if}
  </div>

  <h2>{t("set.appearance")}</h2>
  <div class="group">
    <label class="row">
      <span class="label">{t("set.theme")}</span>
      <select class="field" value={s.theme} onchange={(e) => set("theme", e.currentTarget.value as Settings["theme"])}>
        <option value="system">{t("set.theme.system")}</option>
        <option value="dark">{t("set.theme.dark")}</option>
        <option value="light">{t("set.theme.light")}</option>
      </select>
    </label>
    <label class="row">
      <span class="label">{t("set.language")}</span>
      <select class="field" value={s.language} onchange={(e) => set("language", e.currentTarget.value as Settings["language"])}>
        <option value="auto">{t("set.language.auto")}</option>
        <option value="pt">Português (Brasil)</option>
        <option value="en">English</option>
      </select>
    </label>
  </div>

  <h2>{t("set.privacy")}</h2>
  <p class="note muted">{t("set.privacyNote")}</p>
  <div class="group">
    {@render toggle("saveHistory", t("set.history"))}
    {@render toggle("notify", t("set.notify"))}
  </div>

  <h2>{t("set.engine")}</h2>
  <p class="note muted">{t("set.engineNote")}</p>
  <div class="group">
    <div class="row">
      <span class="label versions">
        <span>yt-dlp <b>{app.engine?.ytdlp ?? "…"}</b></span>
        <span>FFmpeg <b>{app.engine?.ffmpeg ?? "…"}</b>{app.engine?.systemFfmpeg ? ` (${t("set.system")})` : ""}</span>
        <span>Deno <b>{app.engine?.deno ?? "…"}</b></span>
      </span>
      <button class="btn" onclick={updateEngine} disabled={updatingEngine}>
        {updatingEngine ? t("set.engineUpdating") : t("set.engineUpdate")}
      </button>
    </div>
  </div>

  <h2>{t("set.about")}</h2>
  <div class="group">
    <div class="row">
      <span class="label">{t("set.version", { v: app.version })}<small class="muted">{t("set.legal")}</small></span>
      {#if app.updaterEnabled}
        <button class="btn" onclick={checkUpdate} disabled={checking}>{t("set.checkUpdate")}</button>
      {/if}
    </div>
  </div>
</section>

<style>
  h2 {
    font-size: 12px;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--muted);
    margin: 26px 4px 8px;
  }
  .view > h1 + h2 {
    margin-top: 0;
  }
  .note {
    font-size: 12px;
    margin: 0 4px 10px;
    max-width: 640px;
  }
  .group {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 20px;
    min-height: 54px;
    padding: 9px 14px;
  }
  .row + .row {
    border-top: 1px solid var(--border);
  }
  .label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    font-weight: 600;
  }
  .label small {
    font-weight: 400;
    font-size: 12px;
  }
  .path {
    user-select: text;
    word-break: break-all;
  }
  .warn {
    color: var(--danger);
  }
  .inline {
    display: flex;
    gap: 6px;
  }
  .num {
    width: 90px;
  }
  select.field {
    min-width: 150px;
  }
  .versions {
    flex-direction: row;
    flex-wrap: wrap;
    gap: 4px 18px;
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }
  .switch {
    appearance: none;
    flex: none;
    width: 40px;
    height: 24px;
    margin: 0;
    border-radius: 99px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    position: relative;
    cursor: pointer;
    transition: background 0.15s;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--muted);
    transition: transform 0.15s, background 0.15s;
  }
  .switch:checked {
    background: var(--accent);
    border-color: transparent;
  }
  .switch:checked::after {
    transform: translateX(16px);
    background: var(--accent-ink);
  }
  .switch:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .segmented {
    display: inline-flex;
    padding: 3px;
    border-radius: 11px;
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .segmented button {
    height: 28px;
    padding: 0 12px;
    border-radius: 8px;
    font-weight: 600;
    color: var(--muted);
  }
  .segmented button.on {
    background: var(--bg);
    color: var(--text);
    box-shadow: 0 0 0 1px var(--border);
  }
</style>
