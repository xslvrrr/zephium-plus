<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { preferences } from "$domain/preferences";
  import Button from "$shared/ui/Button";
  import { mcp } from "$session/mcp.svelte";

  let names = $derived(mcp.clients());
</script>

{#if names.length > 0}
  <div class="mcp-indicator" role="status" aria-live="polite">
    <span class="dot" aria-hidden="true"></span>
    <span class="label">{m.mcp_connected({ name: names.join(", ") })}</span>
    <Button size="compact" onclick={() => void preferences.set("mcp.enabled", "false")}
      >{m.mcp_stop()}</Button
    >
  </div>
{/if}

<style>
  .mcp-indicator {
    display: flex;
    flex: none;
    align-items: center;
    gap: 8px;
    margin: 6px var(--sidebar-inset);
    padding: 6px 10px;
    border-radius: var(--radius-row);
    background: var(--row-active);
    box-shadow: var(--row-rim);
    font-size: var(--text-label);
    color: var(--color-text);
  }

  .dot {
    flex: none;
    inline-size: 6px;
    block-size: 6px;
    border-radius: var(--radius-capsule);
    background: var(--color-success);
  }

  @media (forced-colors: active) {
    .dot {
      background: CanvasText;
    }
  }

  .label {
    flex: 1;
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
