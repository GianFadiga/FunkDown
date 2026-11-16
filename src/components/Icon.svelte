<script lang="ts" module>
  // Ícones de traço desenhados para o app (viewBox 24x24).
  const paths = {
    download: "M12 4v11m0 0-4.5-4.5M12 15l4.5-4.5M5 20h14",
    history: "M4 12a8 8 0 1 0 2.6-5.9M4 5v4h4M12 8v4.5l3 1.8",
    settings: "M4 7h9m4 0h3M4 17h3m4 0h9M15 4.5v5M9 14.5v5",
    close: "M6 6l12 12M18 6 6 18",
    retry: "M20 12a8 8 0 1 1-2.6-5.9M20 5v4h-4",
    folder: "M3.5 7.5a2 2 0 0 1 2-2H9l2 2.5h7.5a2 2 0 0 1 2 2V17a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z",
    play: "M8 5.5v13l10.5-6.5z",
    trash: "M5 7h14M10 7V4.5h4V7m-7 0 .8 12h8.4L17 7",
    music: "M9 18V6l10-2v12M9 18a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0Zm10-2a2.5 2.5 0 1 1-5 0 2.5 2.5 0 0 1 5 0Z",
    video: "M4 7.5a2 2 0 0 1 2-2h8a2 2 0 0 1 2 2v9a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2zM16 10.5l4.5-2.5v8L16 13.5",
    check: "M5 12.5l4.5 4.5L19 7.5",
    alert: "M12 8v5m0 3.2v.1M10.3 4.3 2.9 17a2 2 0 0 0 1.7 3h14.8a2 2 0 0 0 1.7-3L13.7 4.3a2 2 0 0 0-3.4 0Z",
    scissors: "M6.5 9.5a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5Zm0 10a2.5 2.5 0 1 0 0-5 2.5 2.5 0 0 0 0 5ZM8.5 8.5 20 18M8.5 15.5 20 6",
    search: "M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14Zm5-2 4 4",
  };
  export type IconName = keyof typeof paths;
</script>

<script lang="ts">
  let { name }: { name: IconName } = $props();
</script>

<svg viewBox="0 0 24 24" aria-hidden="true"><path d={paths[name]} /></svg>
