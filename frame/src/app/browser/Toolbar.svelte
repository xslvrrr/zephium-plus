<script lang="ts">
  import type { Snippet } from "svelte";
  import { NavControls } from "$features/sidebar";

  let { children }: { children: Snippet } = $props();
</script>

<!-- rice fork: Trance's top band. Its height matches the native layout's
     TOOLBAR_BAND, so the page's top edge meets the band's foot. -->
<header class="rice-toolbar" data-tauri-drag-region>
  <NavControls />
  <div class="rice-toolbar-address">{@render children()}</div>
</header>

<style>
  .rice-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    block-size: var(--rice-toolbar-height);
    padding-inline: 4px 0;
    flex-shrink: 0;
  }

  .rice-toolbar-address {
    flex: 1;
    min-inline-size: 0;
  }
</style>
