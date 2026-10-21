<script lang="ts">
  import Icon from "../components/Icon.svelte";

  let { icon, title, text }: { icon: string; title: string; text: string } = $props();
</script>

<div class="page">
  <h1>{title}</h1>
  <div class="empty card">
    <div class="tile"><Icon name={icon} size={28} /></div>
    <p class="muted">{text}</p>
  </div>
</div>

<style>
  .page {
    max-width: 1120px;
    margin: 0 auto;
    padding: 32px 40px;
    display: flex;
    flex-direction: column;
    gap: 24px;
  }
  h1 {
    font-size: 28px;
    font-weight: 700;
  }
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    padding: 56px 24px;
    text-align: center;
  }
  .tile {
    width: 56px;
    height: 56px;
    border-radius: 14px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--accent-soft);
    color: var(--accent-ink);
  }
  p {
    margin: 0;
  }
</style>
