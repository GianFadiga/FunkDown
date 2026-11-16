<script lang="ts">
  import { api } from "../lib/api";
  import { fileName, formatClock, formatSpeed, PLATFORM_LABEL } from "../lib/format";
  import { app, t, toast } from "../lib/state.svelte";
  import type { Job } from "../lib/types";
  import Icon from "./Icon.svelte";

  let { job }: { job: Job } = $props();
  let thumbFailed = $state(false);

  const active = $derived(["starting", "downloading", "processing"].includes(job.status));
  const busy = $derived(job.status === "queued" || job.status === "starting" || job.status === "processing");
  const needsLogin = $derived(
    !!job.error && ["loginRequired", "cookiesBrowser", "ageRestricted", "private"].includes(job.error.code),
  );

  const detail = $derived.by(() => {
    switch (job.status) {
      case "downloading": {
        const parts = [`${Math.round(job.percent)}%`, formatSpeed(job.speed)];
        if (job.eta != null) parts.push(t("job.eta", { t: formatClock(job.eta) }));
        return parts.filter(Boolean).join(" · ");
      }
      case "done": {
        const name = job.files.length > 1 ? t("job.files", { n: job.files.length }) : fileName(job.files[0] ?? "");
        return job.skipped ? `${name} · ${t("job.skipped", { n: job.skipped })}` : name;
      }
      case "failed":
        return t(`error.${job.error?.code ?? "unknown"}`);
      default:
        return t(`status.${job.status}`);
    }
  });

  async function open(reveal: boolean) {
    try {
      await api.openDownload("job", job.id, reveal);
    } catch {
      toast(t("error.missingFile"), "error");
    }
  }
</script>

<article class="card {job.status}">
  <div class="thumb">
    {#if job.thumbnail && !thumbFailed}
      <img src={job.thumbnail} alt="" referrerpolicy="no-referrer" onerror={() => (thumbFailed = true)} />
    {:else}
      <Icon name={job.kind === "audio" ? "music" : "video"} />
    {/if}
  </div>

  <div class="body">
    <div class="title" title={job.title ?? job.url}>{job.title ?? job.url}</div>
    <div class="meta">
      <span class="chip {job.platform}">{PLATFORM_LABEL[job.platform]}</span>
      <span class="chip">{t(`kind.${job.kind}`)}</span>
      {#if job.playlist && job.itemIndex && job.itemCount}
        <span class="muted">{t("job.item", { i: job.itemIndex, n: job.itemCount })}</span>
      {/if}
      {#if job.trim}
        <span class="muted">{t("job.trim", { a: job.trim.start, b: job.trim.end })}</span>
      {/if}
      {#if job.uploader}
        <span class="muted uploader">{job.uploader}</span>
      {/if}
    </div>

    {#if !["done", "failed", "canceled"].includes(job.status)}
      <div class="bar" class:busy role="progressbar" aria-valuenow={Math.round(job.percent)} aria-valuemin="0" aria-valuemax="100">
        <span style:width="{busy ? 100 : job.percent}%"></span>
      </div>
    {/if}

    <div class="detail" class:error={job.status === "failed"}>
      {#if job.status === "done"}<span class="ok"><Icon name="check" /></span>{/if}
      <span>{detail}</span>
    </div>

    {#if job.status === "failed" && job.error}
      <div class="fail-actions">
        {#if needsLogin}
          <button class="btn" onclick={() => (app.view = "settings")}>{t("job.fixLogin")}</button>
        {/if}
        {#if job.error.detail}
          <details>
            <summary>{t("job.details")}</summary>
            <pre>{job.error.detail}</pre>
          </details>
        {/if}
      </div>
    {/if}
  </div>

  <div class="actions">
    {#if job.status === "done"}
      <button class="icon-btn" title={t("action.open")} aria-label={t("action.open")} onclick={() => open(false)}>
        <Icon name="play" />
      </button>
      <button class="icon-btn" title={t("action.reveal")} aria-label={t("action.reveal")} onclick={() => open(true)}>
        <Icon name="folder" />
      </button>
    {/if}
    {#if job.status === "failed" || job.status === "canceled"}
      <button class="icon-btn" title={t("action.retry")} aria-label={t("action.retry")} onclick={() => api.retryJob(job.id)}>
        <Icon name="retry" />
      </button>
    {/if}
    {#if active || job.status === "queued"}
      <button class="icon-btn" title={t("action.cancel")} aria-label={t("action.cancel")} onclick={() => api.cancelJob(job.id)}>
        <Icon name="close" />
      </button>
    {:else}
      <button class="icon-btn" title={t("action.remove")} aria-label={t("action.remove")} onclick={() => api.removeJob(job.id)}>
        <Icon name="close" />
      </button>
    {/if}
  </div>
</article>

<style>
  .card {
    display: grid;
    grid-template-columns: 96px 1fr auto;
    gap: 14px;
    padding: 12px;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .card.failed {
    border-color: color-mix(in srgb, var(--danger) 45%, var(--border));
  }
  .card.canceled {
    opacity: 0.65;
  }
  .thumb {
    width: 96px;
    height: 64px;
    border-radius: 9px;
    background: var(--surface-2);
    display: grid;
    place-items: center;
    overflow: hidden;
    color: var(--muted);
  }
  .thumb img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .thumb :global(svg) {
    width: 26px;
    height: 26px;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .body {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .title {
    font-weight: 700;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    user-select: text;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
    min-width: 0;
  }
  .uploader {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar {
    height: 6px;
    border-radius: 99px;
    background: var(--surface-2);
    overflow: hidden;
  }
  .bar span {
    display: block;
    height: 100%;
    border-radius: 99px;
    background: linear-gradient(90deg, var(--hot), var(--accent));
    transition: width 0.25s ease-out;
  }
  .bar.busy span {
    background: linear-gradient(90deg, transparent, var(--hot), transparent);
    background-size: 40% 100%;
    background-repeat: no-repeat;
    animation: sweep 1.2s linear infinite;
  }
  @keyframes sweep {
    from {
      background-position: -40% 0;
    }
    to {
      background-position: 140% 0;
    }
  }
  .detail {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
    min-width: 0;
  }
  .detail span:last-child {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .detail.error {
    color: var(--danger);
  }
  .ok {
    color: var(--ok);
    display: grid;
  }
  .ok :global(svg) {
    width: 14px;
    height: 14px;
    fill: none;
    stroke: currentColor;
    stroke-width: 3;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .fail-actions {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    font-size: 12px;
  }
  .fail-actions .btn {
    height: 28px;
    font-size: 12px;
  }
  details {
    color: var(--muted);
    min-width: 0;
  }
  summary {
    cursor: pointer;
    line-height: 28px;
  }
  pre {
    margin: 4px 0 0;
    padding: 8px;
    max-height: 120px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    background: var(--surface-2);
    border-radius: 8px;
    font: 11px/1.4 ui-monospace, Consolas, monospace;
    user-select: text;
  }
  .actions {
    display: flex;
    align-items: flex-start;
    gap: 2px;
  }
</style>
