<script lang="ts">
  /** Profilbild oder, falls keins da ist, Initialen. */
  let { name, image = null, size = 36 }: { name: string; image?: string | null; size?: number } =
    $props();

  /** Bild, das nicht geladen werden konnte (z. B. Gruppe ohne Bild). */
  let failed = $state<string | null>(null);
  const initials = $derived(
    name
      .split(/\s+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((p) => p[0]!.toUpperCase())
      .join(""),
  );
</script>

<span class="avatar" style="width:{size}px;height:{size}px;font-size:{size * 0.38}px" aria-hidden="true">
  {#if image && image !== failed}
    <img src={image} alt="" onerror={() => (failed = image)} />
  {:else}
    {initials}
  {/if}
</span>

<style>
  .avatar {
    flex: none;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
    background: var(--accent-soft);
    color: var(--accent-ink);
    font-weight: 600;
    overflow: hidden;
  }
  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
</style>
