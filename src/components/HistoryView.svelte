<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { formatDate, PLATFORM_LABEL } from "../lib/format";
  import { app, locale, t, toast } from "../lib/state.svelte";
  import type { HistoryEntry } from "../lib/types";
  import Icon from "./Icon.svelte";

  let entries = $state<HistoryEntry[]>([]);
  let query = $state("");
  let confirming = $state(false);

  const shown = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return q ? entries.filter((e) => `${e.title} ${e.uploader ?? ""}`.toLowerCase().includes(q)) : entries;
  });

  const load = async () => (entries = await api.historyList());
  onMount(load);

  async function open(entry: HistoryEntry, reveal: boolean) {
    try {
      await api.openDownload("history", entry.id, reveal);
    } catch {
      toast(t("error.missingFile"), "error");
      load();
    }
  }

  function redownload(entry: HistoryEntry) {
    app.prefill = entry.url;
    app.view = "download";
  }

  async function remove(entry: HistoryEntry) {
    await api.historyRemove(entry.id);
    load();
  }

  async function clear() {
    await api.historyClear();
    confirming = false;
    load();
  }
</script>

<section class="view">
  <h1>{t("history.title")}</h1>

  {#if entries.length}
    <div class="tools">
      <label class="search">
        <Icon name="search" />
        <input bind:value={query} placeholder={t("history.search")} spellcheck="false" />
      </label>
      {#if confirming}
        <span class="muted confirm">{t("history.clearConfirm")}</span>
        <button class="btn danger" onclick={clear}>{t("common.confirm")}</button>
        <button class="btn ghost" onclick={() => (confirming = false)}>{t("common.cancel")}</button>
      {:else}
        <button class="btn ghost" onclick={() => (confirming = true)}><Icon name="trash" /> {t("history.clear")}</button>
      {/if}
    </div>

    <ul>
      {#each shown as entry (entry.id)}
        <li class:gone={!entry.exists}>
          <span class="kind"><Icon name={entry.kind === "audio" ? "music" : "video"} /></span>
          <div class="info">
            <div class="title" title={entry.file}>{entry.title}</div>
            <div class="sub muted">
              <span class="chip {entry.platform}">{PLATFORM_LABEL[entry.platform]}</span>
              {#if entry.uploader}<span>{entry.uploader}</span>{/if}
              <span>{formatDate(entry.finishedAt, locale())}</span>
              {#if entry.fileCount > 1}<span>{t("job.files", { n: entry.fileCount })}</span>{/if}
              {#if !entry.exists}<span class="warn">{t("history.gone")}</span>{/if}
            </div>
          </div>
          <div class="actions">
            {#if entry.exists}
              <button class="icon-btn" title={t("action.open")} aria-label={t("action.open")} onclick={() => open(entry, false)}>
                <Icon name="play" />
              </button>
              <button class="icon-btn" title={t("action.reveal")} aria-label={t("action.reveal")} onclick={() => open(entry, true)}>
                <Icon name="folder" />
              </button>
            {/if}
            <button class="icon-btn" title={t("action.redownload")} aria-label={t("action.redownload")} onclick={() => redownload(entry)}>
              <Icon name="retry" />
            </button>
            <button class="icon-btn" title={t("action.remove")} aria-label={t("action.remove")} onclick={() => remove(entry)}>
              <Icon name="close" />
            </button>
          </div>
        </li>
      {:else}
        <li class="none muted">{t("history.noMatch")}</li>
      {/each}
    </ul>
  {:else}
    <p class="muted">{app.settings?.saveHistory ? t("history.empty") : t("history.disabled")}</p>
  {/if}
</section>

<style>
  .tools {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 14px;
  }
  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 8px;
    height: 36px;
    padding: 0 12px;
    border-radius: 10px;
    background: var(--surface);
    border: 1px solid var(--border);
    color: var(--muted);
  }
  .search:focus-within {
    border-color: var(--accent);
  }
  .search :global(svg) {
    width: 16px;
    height: 16px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .search input {
    flex: 1;
    border: 0;
    outline: 0;
    background: transparent;
    color: var(--text);
    user-select: text;
  }
  .confirm {
    font-size: 12px;
    max-width: 300px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
  }
  li + li {
    border-top: 1px solid var(--border);
  }
  li.gone .title {
    color: var(--muted);
    text-decoration: line-through;
  }
  .kind {
    width: 34px;
    height: 34px;
    border-radius: 9px;
    background: var(--surface-2);
    color: var(--muted);
    display: grid;
    place-items: center;
    flex: none;
  }
  .kind :global(svg) {
    width: 17px;
    height: 17px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .info {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-weight: 600;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .sub {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    margin-top: 3px;
    white-space: nowrap;
    overflow: hidden;
  }
  .warn {
    color: var(--danger);
  }
  .actions {
    display: flex;
    gap: 2px;
  }
  .none {
    justify-content: center;
    padding: 24px;
  }
</style>
