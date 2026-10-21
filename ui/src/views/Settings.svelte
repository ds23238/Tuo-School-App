<script lang="ts">
  import { getVersion } from "@tauri-apps/api/app";
  import { i18n, locales, type Locale } from "../lib/i18n/index.svelte";

  const version = getVersion().catch(() => "–");
</script>

<dl>
  <dt><label for="locale">{i18n.t.settings.language}</label></dt>
  <dd>
    <select
      id="locale"
      value={i18n.locale}
      onchange={(e) => (i18n.locale = e.currentTarget.value as Locale)}
    >
      {#each locales as l (l)}
        <option value={l}>{i18n.t.locale[l]}</option>
      {/each}
    </select>
  </dd>
  <dt>{i18n.t.settings.version}</dt>
  <dd>{#await version then v}{v}{/await}</dd>
</dl>

<style>
  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 12px 24px;
    align-items: center;
    margin: 0;
  }
  dd {
    margin: 0;
  }
</style>
