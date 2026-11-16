<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { formatBytes } from "../lib/format";
  import { app, t } from "../lib/state.svelte";
  import Equalizer from "./Equalizer.svelte";

  let error = $state("");
  let running = $state(false);
  // Fixa a lista no início: `app.missing` esvazia quando termina.
  const tools = [...app.missing];

  async function start() {
    error = "";
    running = true;
    try {
      await api.setupEngine();
      app.missing = [];
    } catch (e) {
      error = String(e);
    } finally {
      running = false;
    }
  }
  onMount(start);
</script>

<div class="overlay">
  <div class="panel">
    <Equalizer playing={running} size={44} />
    <h1>{t("setup.title")}</h1>
    <p class="muted">{t("setup.body")}</p>

    <ul>
      {#each tools as tool}
        {@const ev = app.setup[tool]}
        {@const pct = ev?.stage === "downloading" && ev.total ? (ev.downloaded / ev.total) * 100 : ev ? 100 : 0}
        <li>
          <div class="row">
            <span>{t(`setup.tool.${tool}`)}</span>
            <span class="muted stage">
              {t(`setup.stage.${ev?.stage ?? "waiting"}`)}
              {#if ev?.stage === "downloading"}· {formatBytes(ev.downloaded)}{ev.total ? ` / ${formatBytes(ev.total)}` : ""}{/if}
            </span>
          </div>
          <div class="bar"><span style:width="{pct}%" class:done={ev?.stage === "done"}></span></div>
        </li>
      {/each}
    </ul>

    {#if error}
      <p class="error">{t("setup.failed")} {error}</p>
      <button class="btn primary" onclick={start}>{t("setup.retry")}</button>
    {/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 20;
    display: grid;
    place-items: center;
    background: var(--bg);
  }
  .panel {
    width: min(520px, 90vw);
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  h1 {
    font-size: 26px;
    font-weight: 900;
    font-style: italic;
    letter-spacing: -0.02em;
  }
  ul {
    list-style: none;
    padding: 0;
    margin: 10px 0 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .row {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 6px;
    font-weight: 600;
  }
  .stage {
    font-weight: 400;
    font-size: 12px;
    font-variant-numeric: tabular-nums;
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
    transition: width 0.2s;
  }
  .bar span.done {
    background: var(--accent);
  }
  .error {
    color: var(--danger);
    user-select: text;
  }
  .btn {
    align-self: flex-start;
  }
</style>
