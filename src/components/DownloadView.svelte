<script lang="ts">
  import { untrack } from "svelte";
  import { api } from "../lib/api";
  import { PLATFORM_LABEL } from "../lib/format";
  import { app, t, toast } from "../lib/state.svelte";
  import type { LinkInfo, Trim } from "../lib/types";
  import Equalizer from "./Equalizer.svelte";
  import Icon from "./Icon.svelte";
  import JobCard from "./JobCard.svelte";

  type Mode = "auto" | "audio" | "video";
  const MODES: Mode[] = ["auto", "audio", "video"];

  let text = $state("");
  let links = $state<LinkInfo[]>([]);
  let mode = $state<Mode>("auto");
  let playlist = $state(false);
  let trimOn = $state(false);
  let trimStart = $state("");
  let trimEnd = $state("");
  let dragging = $state(false);
  let input: HTMLTextAreaElement;

  const jobs = $derived([...app.jobs].reverse());
  const hasFinished = $derived(app.jobs.some((j) => ["done", "failed", "canceled"].includes(j.status)));
  const anyActive = $derived(app.jobs.some((j) => ["starting", "downloading", "processing"].includes(j.status)));
  const playlistChoice = $derived(links.some((l) => l.playlist === "optional"));
  const canTrim = $derived(links.length === 1 && links[0].playlist !== "only" && !playlist);

  // A detecção de links é do Rust, para a interface e a fila nunca discordarem.
  $effect(() => {
    const value = text;
    const timer = setTimeout(async () => {
      links = value.trim() ? await api.inspectLinks(value) : [];
    }, 120);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    if (app.prefill) {
      append(app.prefill);
      app.prefill = "";
    }
  });

  function append(more: string) {
    const current = untrack(() => text).trim();
    text = current ? `${current}\n${more.trim()}` : more.trim();
    input?.focus();
  }

  const seconds = (stamp: string) => stamp.split(":").reduce((total, part) => total * 60 + Number(part), 0);
  const STAMP = /^\d{1,5}(:\d{2}){0,2}$/;

  async function submit() {
    const found = await api.inspectLinks(text);
    if (!found.length) {
      toast(t("add.none"), "error");
      return;
    }
    let trim: Trim | null = null;
    if (trimOn && canTrim) {
      const [start, end] = [trimStart.trim() || "0", trimEnd.trim()];
      if (!STAMP.test(start) || !STAMP.test(end) || seconds(end) <= seconds(start)) {
        toast(t("add.trim.invalid"), "error");
        return;
      }
      trim = { start, end };
    }
    try {
      const result = await api.addDownloads({
        urls: found.map((l) => l.url),
        kind: mode === "auto" ? null : mode,
        playlist,
        trim,
      });
      if (result.invalid.length) toast(t("add.invalid", { n: result.invalid.length }), "error");
      if (result.duplicates.length) toast(t("add.duplicates", { n: result.duplicates.length }));
      text = "";
      trimOn = false;
      playlist = false;
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  function onWindowPaste(e: ClipboardEvent) {
    const target = e.target as HTMLElement | null;
    if (target?.closest("input, textarea")) return;
    const pasted = e.clipboardData?.getData("text/plain");
    if (pasted) {
      e.preventDefault();
      append(pasted);
    }
  }

  function onDrop(e: DragEvent) {
    e.preventDefault();
    dragging = false;
    const dropped = e.dataTransfer?.getData("text/uri-list") || e.dataTransfer?.getData("text/plain");
    if (dropped) append(dropped);
  }
</script>

<svelte:window
  onpaste={onWindowPaste}
  ondragover={(e) => {
    e.preventDefault();
    dragging = true;
  }}
  ondragleave={() => (dragging = false)}
  ondrop={onDrop}
/>

<section class="view">
  <div class="add" class:dragging>
    <textarea
      bind:this={input}
      bind:value={text}
      onkeydown={onKeydown}
      rows={Math.min(4, Math.max(1, text.split("\n").length))}
      placeholder={t("add.placeholder")}
      spellcheck="false"
      autocomplete="off"
    ></textarea>

    {#if links.length}
      <div class="links">
        {#each links as link (link.url)}
          <span class="chip {link.platform}" title={link.url}>{PLATFORM_LABEL[link.platform]}</span>
        {/each}
      </div>
    {/if}

    <div class="controls">
      <div class="segmented" role="radiogroup" title={t("add.mode.autoHint")}>
        {#each MODES as m}
          <button role="radio" aria-checked={mode === m} class:on={mode === m} onclick={() => (mode = m)}>
            {t(`add.mode.${m}`)}
          </button>
        {/each}
      </div>

      {#if playlistChoice}
        <label class="check"><input type="checkbox" bind:checked={playlist} /> {t("add.playlist")}</label>
      {/if}
      {#if canTrim}
        <label class="check"><input type="checkbox" bind:checked={trimOn} /> <Icon name="scissors" /> {t("add.trim")}</label>
      {/if}
      {#if trimOn && canTrim}
        <input class="field stamp" bind:value={trimStart} placeholder="0:00" aria-label={t("add.trim.from")} />
        <span class="muted">–</span>
        <input class="field stamp" bind:value={trimEnd} placeholder="0:30" aria-label={t("add.trim.to")} />
      {/if}

      <button class="btn primary go" onclick={submit} disabled={!links.length}>
        <Icon name="download" />
        {t("add.button")}{links.length > 1 ? ` (${links.length})` : ""}
      </button>
    </div>
  </div>
  {#if !text}
    <p class="hint muted">{t("add.hint")}</p>
  {/if}

  {#if jobs.length}
    <header class="queue-head">
      <h2><Equalizer playing={anyActive} size={16} /> {t("queue.title")} <span class="muted">{jobs.length}</span></h2>
      <div>
        <button class="btn ghost" onclick={() => api.openDownloadDir()}><Icon name="folder" /> {t("queue.openFolder")}</button>
        {#if hasFinished}
          <button class="btn ghost" onclick={() => api.clearFinished()}>{t("queue.clear")}</button>
        {/if}
      </div>
    </header>
    <div class="queue">
      {#each jobs as job (job.id)}
        <JobCard {job} />
      {/each}
    </div>
  {:else}
    <div class="empty">
      <Equalizer size={72} />
      <h2>{t("queue.empty.title")}</h2>
      <p class="muted">{t("queue.empty.body")}</p>
      <button class="btn ghost" onclick={() => api.openDownloadDir()}><Icon name="folder" /> {t("queue.openFolder")}</button>
    </div>
  {/if}
</section>

<style>
  .add {
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 18px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    transition: border-color 0.15s, box-shadow 0.15s;
  }
  .add:focus-within,
  .add.dragging {
    border-color: var(--accent);
    box-shadow: 0 0 0 4px color-mix(in srgb, var(--accent) 14%, transparent);
  }
  textarea {
    width: 100%;
    resize: none;
    border: 0;
    outline: 0;
    background: transparent;
    font-size: 16px;
    line-height: 1.5;
    padding: 4px 6px;
    user-select: text;
  }
  textarea::placeholder {
    color: var(--muted);
  }
  .links {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    padding: 0 4px;
  }
  .controls {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 12px;
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
  .check {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
  }
  .check input {
    accent-color: var(--accent);
    width: 16px;
    height: 16px;
    margin: 0;
  }
  .check :global(svg) {
    width: 15px;
    height: 15px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
  }
  .stamp {
    width: 76px;
    height: 32px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  .go {
    margin-left: auto;
    height: 40px;
    padding: 0 20px;
    font-size: 15px;
  }
  .hint {
    font-size: 12px;
    margin: 10px 6px 0;
  }
  .queue-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin: 26px 2px 10px;
  }
  .queue-head h2 {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 15px;
    font-weight: 800;
  }
  .queue-head h2 .muted {
    font-weight: 600;
  }
  .queue {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    text-align: center;
    padding: 64px 20px 20px;
  }
  .empty h2 {
    font-size: 24px;
    font-weight: 900;
    font-style: italic;
    letter-spacing: -0.02em;
    margin-top: 8px;
  }
  .empty p {
    max-width: 380px;
  }
</style>
