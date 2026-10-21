<script lang="ts">
  let { text }: { text: string } = $props();
</script>

<p>{text}</p>

<style>
  p {
    color: var(--text-muted);
  }
</style>
