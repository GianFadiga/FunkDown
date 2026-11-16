<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { isPermissionGranted, requestPermission, sendNotification } from "@tauri-apps/plugin-notification";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { check, type Update } from "@tauri-apps/plugin-updater";
  import { api } from "./lib/api";
  import { app, applyTheme, t, toast, upsertJob, type View } from "./lib/state.svelte";
  import type { EngineInfo, Job, SetupEvent } from "./lib/types";
  import DownloadView from "./components/DownloadView.svelte";
  import Equalizer from "./components/Equalizer.svelte";
  import HistoryView from "./components/HistoryView.svelte";
  import Icon, { type IconName } from "./components/Icon.svelte";
  import SettingsView from "./components/SettingsView.svelte";
  import SetupOverlay from "./components/SetupOverlay.svelte";

  const NAV: { view: View; icon: IconName }[] = [
    { view: "download", icon: "download" },
    { view: "history", icon: "history" },
    { view: "settings", icon: "settings" },
  ];

  let update = $state<Update | null>(null);
  let installing = $state(false);
  const activeCount = $derived(app.jobs.filter((j) => !["done", "failed", "canceled"].includes(j.status)).length);

  async function notify(job: Job) {
    if (!app.settings?.notify || document.hasFocus()) return;
    if (!(await isPermissionGranted()) && (await requestPermission()) !== "granted") return;
    sendNotification({
      title: t(job.status === "done" ? "notify.done" : "notify.failed"),
      body: job.title ?? job.url,
    });
  }

  function onJob(job: Job) {
    const before = app.jobs.find((j) => j.id === job.id)?.status;
    upsertJob(job);
    if (before !== job.status && (job.status === "done" || job.status === "failed")) {
      notify(job).catch(() => {});
    }
  }

  /** `manual`: o usuário clicou em "procurar atualização" e merece uma resposta. */
  async function checkUpdate(manual = false) {
    if (!app.updaterEnabled) return;
    try {
      update = await check();
      if (!update && manual) toast(t("set.upToDate"));
    } catch {
      if (manual) toast(t("set.updateFailed"), "error");
    }
  }

  async function installUpdate() {
    if (!update) return;
    installing = true;
    try {
      await update.downloadAndInstall();
      await relaunch();
    } catch (e) {
      installing = false;
      toast(String(e), "error");
    }
  }

  onMount(() => {
    const stop: Array<() => void> = [];
    (async () => {
      const boot = await api.bootstrap();
      app.settings = boot.settings;
      app.jobs = boot.jobs;
      app.missing = boot.missing;
      app.version = boot.version;
      app.os = boot.os;
      app.updaterEnabled = boot.updaterEnabled;
      stop.push(
        await listen<Job>("job", (e) => onJob(e.payload)),
        await listen<number>("job-removed", (e) => (app.jobs = app.jobs.filter((j) => j.id !== e.payload))),
        await listen<SetupEvent>("setup", (e) => (app.setup[e.payload.tool] = e.payload)),
        await listen<EngineInfo>("engine", (e) => (app.engine = e.payload)),
      );
      app.ready = true;
      if (!boot.missing.length) app.engine = await api.engineInfo();
      checkUpdate();
    })();
    const media = matchMedia("(prefers-color-scheme: dark)");
    media.addEventListener("change", applyTheme);
    return () => {
      stop.forEach((off) => off());
      media.removeEventListener("change", applyTheme);
    };
  });

  $effect(() => {
    // Reaplica quando tema ou idioma mudam nos ajustes.
    void [app.settings?.theme, app.settings?.language];
    applyTheme();
  });
</script>

{#if app.ready && app.settings}
  <div class="shell">
    <nav>
      <div class="brand">
        <Equalizer playing={activeCount > 0} size={26} />
        <span>Funk<b>Down</b></span>
      </div>
      {#each NAV as item}
        <button class="nav" class:on={app.view === item.view} onclick={() => (app.view = item.view)}>
          <Icon name={item.icon} />
          <span>{t(`nav.${item.view}`)}</span>
          {#if item.view === "download" && activeCount}<em>{activeCount}</em>{/if}
        </button>
      {/each}
      <div class="spacer"></div>
      {#if update}
        <button class="update" onclick={installUpdate} disabled={installing}>
          <strong>{installing ? t("update.installing") : t("update.available", { v: update.version })}</strong>
          {#if !installing}<span>{t("update.install")}</span>{/if}
        </button>
      {/if}
      <small class="muted">v{app.version}</small>
    </nav>

    <main>
      {#if app.view === "download"}
        <DownloadView />
      {:else if app.view === "history"}
        <HistoryView />
      {:else}
        <SettingsView onCheckUpdate={() => checkUpdate(true)} />
      {/if}
    </main>
  </div>

  {#if app.missing.length}
    <SetupOverlay />
  {/if}

  <div class="toasts" aria-live="polite">
    {#each app.toasts as item (item.id)}
      <div class="toast {item.tone}">{item.text}</div>
    {/each}
  </div>
{/if}

<style>
  .shell {
    display: grid;
    grid-template-columns: 196px 1fr;
    height: 100%;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 18px 12px 14px;
    border-right: 1px solid var(--border);
    background: var(--surface);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px 18px;
    font-size: 19px;
    font-weight: 900;
    font-style: italic;
    letter-spacing: -0.02em;
  }
  .brand b {
    color: var(--accent);
  }
  .nav {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 38px;
    padding: 0 10px;
    border-radius: 10px;
    font-weight: 600;
    color: var(--muted);
    text-align: left;
  }
  .nav:hover {
    color: var(--text);
    background: var(--surface-2);
  }
  .nav.on {
    color: var(--text);
    background: var(--surface-2);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .nav :global(svg) {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .nav em {
    margin-left: auto;
    min-width: 20px;
    height: 20px;
    padding: 0 6px;
    border-radius: 99px;
    background: var(--hot);
    color: #fff;
    font-size: 11px;
    font-style: normal;
    font-weight: 800;
    display: grid;
    place-items: center;
  }
  .spacer {
    flex: 1;
  }
  .update {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px;
    margin-bottom: 8px;
    border-radius: 10px;
    background: var(--accent);
    color: var(--accent-ink);
    text-align: left;
    font-size: 12px;
  }
  nav small {
    padding: 0 8px;
  }
  main {
    min-width: 0;
    min-height: 0;
  }
  .toasts {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 30;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-width: 380px;
  }
  .toast {
    padding: 10px 14px;
    border-radius: 10px;
    background: var(--surface-2);
    border: 1px solid var(--border);
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.3);
  }
  .toast.error {
    border-color: var(--danger);
  }
</style>
