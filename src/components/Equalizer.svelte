<script lang="ts">
  let { playing = false, size = 20 }: { playing?: boolean; size?: number } = $props();
</script>

<span class="eq" class:playing style:--size="{size}px" aria-hidden="true">
  <i></i><i></i><i></i><i></i><i></i>
</span>

<style>
  .eq {
    display: inline-flex;
    align-items: center;
    gap: calc(var(--size) / 10);
    height: var(--size);
  }
  i {
    width: calc(var(--size) / 7);
    border-radius: 99px;
    background: linear-gradient(var(--hot), var(--accent));
  }
  /* Parado, as barras desenham a seta para baixo do ícone do app. */
  i:nth-child(1),
  i:nth-child(5) {
    height: 30%;
  }
  i:nth-child(2),
  i:nth-child(4) {
    height: 62%;
  }
  i:nth-child(3) {
    height: 100%;
  }
  .playing i {
    animation: bounce 0.9s ease-in-out infinite;
  }
  .playing i:nth-child(2) {
    animation-delay: -0.35s;
  }
  .playing i:nth-child(3) {
    animation-delay: -0.6s;
  }
  .playing i:nth-child(4) {
    animation-delay: -0.15s;
  }
  .playing i:nth-child(5) {
    animation-delay: -0.75s;
  }
  @keyframes bounce {
    0%,
    100% {
      height: 25%;
    }
    50% {
      height: 100%;
    }
  }
</style>
