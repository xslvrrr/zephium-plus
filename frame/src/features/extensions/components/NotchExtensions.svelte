<script lang="ts">
  // rice fork: the extensions' buttons as a row of the notch toolbar. Each
  // button stays where it is, so its popup hangs from it; the tray's copy
  // keeps answering the extensions' keyboard shortcuts.
  import { extensions } from "$domain/extensions";
  import { tabs } from "$domain/tabs";
  import { webext } from "$domain/webext";
  import * as m from "$shared/i18n/messages";
  import ExtensionActionIcon from "./ExtensionActionIcon.svelte";
  import { notchHold } from "$session/notch.svelte";
  import { holdWhilePopupOpen } from "../lib/popup-hold";

  let profileId = $derived(tabs.profile()?.id ?? null);
  let tabId = $derived(tabs.activeId());
  let actions = $derived(extensions.activeActions(profileId, tabId));

  function invoke(event: MouseEvent, action: (typeof actions)[number]) {
    const target = event.currentTarget;
    if (profileId === null || tabId === null || !(target instanceof HTMLElement)) return;
    // The popup hangs from the notch, which stays down until it is gone.
    if (action.presents_popup) holdWhilePopupOpen(notchHold);
    void extensions.invoke(profileId, tabId, action, target.getBoundingClientRect());
  }

  function openMenu(event: MouseEvent, action: (typeof actions)[number]) {
    event.preventDefault();
    const target = event.currentTarget;
    if (profileId === null || !(target instanceof HTMLElement)) return;
    extensions.menu(profileId, action, target.getBoundingClientRect());
  }
</script>

{#if actions.length > 0}
  <div class="bar" role="group" aria-label={m.webext_page_title()}>
    {#each actions as action (action.runtime.install_id)}
      {@const name = action.label || webext.byInstall(action.runtime.install_id)?.name || ""}
      <button
        type="button"
        class="tile"
        title={name || m.webext_action()}
        aria-label={name || m.webext_action()}
        aria-haspopup={action.presents_popup ? "dialog" : undefined}
        disabled={!action.enabled || extensions.isInvoking(action.runtime.install_id)}
        onclick={(event) => invoke(event, action)}
        oncontextmenu={(event) => openMenu(event, action)}
      >
        <span class="glyph">
          <ExtensionActionIcon rgba={action.icon_rgba_base64} />
          {#if action.badge.length > 0}
            <span aria-hidden="true" class="badge" data-unread={action.unread_badge}>
              {action.badge}
            </span>
          {/if}
        </span>
      </button>
    {/each}
  </div>
{/if}

<style>
  .bar {
    display: flex;
    flex: none;
    align-items: center;
  }

  .tile {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    padding: 0;
    border: 0;
    border-radius: var(--radius-inset);
    background: none;
    cursor: default;
    outline: none;
    transition: background-color var(--motion-instant) var(--ease-smooth);
  }

  .tile:hover,
  .tile:focus-visible {
    background: var(--row-hover);
  }

  .tile:focus-visible {
    outline: 2px solid var(--color-ring);
    outline-offset: -2px;
  }

  .tile:active {
    background: var(--row-pressed);
  }

  .tile:disabled {
    opacity: 0.45;
  }

  .glyph {
    position: relative;
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
  }

  .glyph :global(canvas) {
    width: 18px;
    height: 18px;
  }

  .badge {
    position: absolute;
    top: -5px;
    right: -8px;
    max-width: 30px;
    overflow: hidden;
    padding-inline: 4px;
    border-radius: var(--radius-capsule);
    background: var(--color-accent);
    font-size: 9px;
    font-weight: 600;
    line-height: 14px;
    color: var(--color-on-accent);
    white-space: nowrap;
    text-overflow: ellipsis;
    pointer-events: none;
  }

  .badge[data-unread="true"] {
    background: var(--color-warning);
  }
</style>
