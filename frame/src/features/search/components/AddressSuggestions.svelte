<script lang="ts">
  import { onMount, untrack } from "svelte";
  import * as m from "$shared/i18n/messages";
  import ResultList from "./ResultList.svelte";
  import { createSearchSurface } from "../lib/search-surface.svelte";

  /**
   * rice fork: the notch's address field suggests as New Tab's search does,
   * from the same native search: places, open tabs, history and the search
   * engine's suggestions, with the first host completed inline. The field
   * stays the address field; this is the sheet under it and the keys it
   * lends the list.
   */
  let {
    tabId,
    input,
    newTab,
    ondone,
  }: {
    /** The tab in front, which a row opens in unless `newTab`. */
    tabId: string;
    input: HTMLInputElement;
    /** Opened as a new tab over a page: an address opens in a new tab. */
    newTab: boolean;
    /** A row ran; the field's editing ends. */
    ondone: () => void;
  } = $props();

  const surface = createSearchSurface({
    tabId: untrack(() => tabId),
    destinations: [],
    onTool: () => {},
    newTab: () => newTab,
    onran: () => ondone(),
  });

  let list = $state<HTMLElement>();
  const LIST_ID = "address-suggestions";

  let visible = $derived(
    surface.open &&
      surface.query.trim().length > 0 &&
      (surface.rows.length > 0 || surface.empty || surface.failed || surface.error !== "none"),
  );

  $effect(() => surface.setList(list));
  // Tracked, as New Tab's is: the completion arrives with native's answer,
  // after the keystroke, and reading it here is what applies it then.
  $effect(() => {
    void surface.query;
    surface.applyCompletion(input);
  });
  // The field says which row its words stand for, for assistive technology.
  $effect(() => {
    const index = surface.selectedIndex;
    input.setAttribute("aria-expanded", String(visible));
    if (visible && index >= 0)
      input.setAttribute("aria-activedescendant", `${LIST_ID}-option-${index}`);
    else input.removeAttribute("aria-activedescendant");
  });

  onMount(() => {
    input.setAttribute("role", "combobox");
    input.setAttribute("aria-controls", LIST_ID);
    input.setAttribute("aria-autocomplete", "both");
    const stop = surface.mount();
    return () => {
      stop();
      for (const name of [
        "role",
        "aria-controls",
        "aria-autocomplete",
        "aria-expanded",
        "aria-activedescendant",
      ])
        input.removeAttribute(name);
    };
  });

  /** What was typed; the field's own draft. */
  export function changed(value: string) {
    surface.changed(value);
  }
  export function compositionStart() {
    surface.compositionStart();
  }
  export function compositionEnd(value: string) {
    surface.compositionEnd(value);
  }

  /** Keys the list takes from the field; true when one was taken. */
  export function keydown(event: KeyboardEvent): boolean {
    if (surface.composing || event.isComposing || event.keyCode === 229) return false;
    if ((event.key === "ArrowDown" || event.key === "ArrowUp") && visible) {
      event.preventDefault();
      void surface.move(event.key === "ArrowDown" ? 1 : -1);
      return true;
    }
    if (event.key === "Escape" && visible && surface.dismiss()) {
      event.preventDefault();
      return true;
    }
    return false;
  }

  /** Enter: the selected row runs, or the field's own way when the search
   *  has nothing to offer (it failed, or nothing was typed). */
  export function submit(): boolean {
    if (!surface.query.trim() || surface.failed || surface.error !== "none") return false;
    surface.submit();
    return true;
  }
</script>

{#if visible}
  <!-- data-open: the notch takes it into the region it raises over the page. -->
  <!-- Pressing a row keeps focus in the field, which would otherwise stop
       editing and take the list away before the press lands. -->
  <div
    class="address-sheet"
    data-open="true"
    role="presentation"
    onmousedown={(event) => event.preventDefault()}
  >
    {#if surface.failed || surface.error !== "none"}
      <div class="failure" role="alert">
        {surface.error === "too_long" ? m.panel_query_too_long() : m.panel_action_failed()}
        <button type="button" onclick={surface.retry}>{m.panel_retry()}</button>
      </div>
    {/if}
    {#if surface.empty}<p class="empty">{m.panel_no_results()}</p>{/if}
    <ResultList
      rows={surface.rows}
      selected={surface.selectedId}
      query={surface.answered}
      listId={LIST_ID}
      busy={surface.pending}
      variant="launcher"
      bind:ref={list}
      onhover={surface.hover}
      onrun={surface.activate}
    />
  </div>
{/if}

<style>
  /* Hangs from the field, as wide as it, over whatever is below. */
  .address-sheet {
    --row-radius: calc(var(--radius-panel) - 6px);

    position: absolute;
    inset-block-start: calc(100% + 6px);
    inset-inline: 0;
    z-index: 2;
    max-block-size: min(420px, calc(100vh - 120px));
    overflow-y: auto;
    overscroll-behavior: contain;
    padding: 6px;
    border-radius: var(--radius-panel);
    background: var(--color-raised);
    box-shadow:
      inset 0 0 0 0.5px var(--color-border),
      var(--shadow-float);
    text-align: start;
    animation: sheet-in 200ms var(--ease-emphasized) backwards;
  }

  @keyframes sheet-in {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
  }

  .empty {
    margin: 0;
    padding: 18px 12px;
    color: var(--color-faint);
    font-size: 13px;
    text-align: center;
  }

  .failure {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 8px 10px;
    color: var(--color-danger);
    font-size: 12px;
  }

  .failure button {
    padding: 5px 10px;
    border: 0;
    border-radius: var(--radius-inset);
    background: var(--color-fill);
    color: var(--color-text);
    cursor: default;
  }

  @media (prefers-reduced-motion: reduce) {
    .address-sheet {
      animation: none;
    }
  }

  :global(:root[data-reduce-motion="true"]) .address-sheet {
    animation: none;
  }

  @media (forced-colors: active) {
    .address-sheet {
      border: 1px solid CanvasText;
    }
  }
</style>
