<script lang="ts">
  import { untrack, type Snippet } from "svelte";
  import * as m from "$shared/i18n/messages";
  import { NavControls } from "$features/sidebar";
  import { surface } from "$domain/surface";
  import { tabs } from "$domain/tabs";
  import { reducedMotion } from "$shared/lib/motion";
  import { claimRegion } from "$session/chrome-regions.svelte";
  import { setZone } from "$session/reach-zones.svelte";
  import { holdNotch, notchDown } from "$session/notch.svelte";
  import { NOTCH, notchFigure } from "../lib/shape";

  let {
    content,
    newTab,
    address,
  }: {
    /** The page column, in window points. */
    content: { x: number; width: number };
    /** No web page stands beneath: the new tab, or a pane the chrome draws
     *  in the page's place. The notch then claims nothing over it. */
    newTab: boolean;
    /** The address field, which is the only one in the chrome while the
     *  notch holds it; its trailing tray carries the site menu and the
     *  extensions. */
    address: Snippet;
  } = $props();

  /** The activation strip along the window's top edge, and how far the
   *  keep-up area reaches beyond the notch's rect. */
  const STRIP = 4;
  const REACH = 32;
  /** Longer than the slide, for a transitionend that never comes. */
  const SETTLE_FALLBACK = 400;

  const owner = Symbol("notch");
  const focusOwner = Symbol("notch focus");
  const menuOwner = Symbol("notch menu");

  let nav = $state<HTMLElement>();
  let figure = $derived(notchFigure(content.x, content.width));
  let down = $derived(notchDown());
  // On screen at all: down, or still sliding up. The chrome stays raised and
  // the controls stay live until the slide has finished.
  let shown = $state(false);
  // Bumped whenever something opens, closes or resizes inside the notch, so
  // the claimed region follows its menus.
  let layoutVersion = $state(0);
  // Only a web page under the chrome needs it raised; over the new tab and
  // Zephium's own pages the stage is hidden, and claiming there would make
  // the chrome drawn in its place deaf to the pointer. A window still
  // waiting for its tabs has no page yet, which also keeps a reloaded chrome
  // from claiming before native has cleared its old regions.
  let overPage = $derived(
    !newTab && surface.currentPage() === null && Boolean(tabs.activeTab()?.url),
  );

  $effect(() => {
    if (down) {
      shown = true;
      return;
    }
    if (reducedMotion()) {
      shown = false;
      return;
    }
    const timer = setTimeout(() => (shown = false), SETTLE_FALLBACK);
    return () => clearTimeout(timer);
  });

  function settled(event: TransitionEvent) {
    if (event.target === nav && event.propertyName === "transform" && !down) shown = false;
    layoutVersion += 1;
  }

  const OPEN = '[popover]:popover-open, [role="dialog"], [data-open="true"]';

  /** The notch's rect, grown to take in whatever is open from it. */
  function region(): DOMRect | null {
    if (!figure) return null;
    let left = figure.x;
    let top = 0;
    let right = figure.x + figure.width;
    let bottom = figure.height;
    for (const open of nav?.querySelectorAll<HTMLElement>(OPEN) ?? []) {
      const rect = open.getBoundingClientRect();
      if (rect.width <= 0 || rect.height <= 0) continue;
      left = Math.min(left, rect.left);
      top = Math.min(top, rect.top);
      right = Math.max(right, rect.right);
      bottom = Math.max(bottom, rect.bottom);
    }
    return new DOMRect(left, top, right - left, bottom - top);
  }

  $effect(() => {
    void layoutVersion;
    claimRegion(owner, shown && overPage ? region() : null);
  });
  $effect(() => () => claimRegion(owner, null));

  $effect(() => {
    if (!figure) {
      setZone("notch", null);
      return;
    }
    const { x, width, height } = figure;
    setZone("notch", {
      activate: [new DOMRect(x, 0, width, STRIP)],
      keep: new DOMRect(x - REACH, 0, width + 2 * REACH, height + REACH),
    });
  });
  $effect(() => () => setZone("notch", null));

  // An open menu holds the notch down, and the region follows it as it opens,
  // closes and changes size. Menus in the field announce themselves through
  // `aria-expanded` on their trigger and `data-open` on their panel; a
  // popover through its toggle event, which reaches the notch only on the
  // way down.
  $effect(() => {
    const root = nav;
    if (!root) return;
    const sizes = new ResizeObserver(() => (layoutVersion += 1));
    const update = () => {
      holdNotch(menuOwner, root.querySelector('[aria-expanded="true"], :popover-open') !== null);
      sizes.disconnect();
      for (const open of root.querySelectorAll(OPEN)) sizes.observe(open);
      layoutVersion += 1;
    };
    const changes = new MutationObserver(update);
    changes.observe(root, {
      subtree: true,
      childList: true,
      attributes: true,
      attributeFilter: ["aria-expanded", "data-open"],
    });
    root.addEventListener("toggle", update, true);
    untrack(update);
    return () => {
      changes.disconnect();
      sizes.disconnect();
      root.removeEventListener("toggle", update, true);
      holdNotch(menuOwner, false);
    };
  });
  $effect(() => () => holdNotch(focusOwner, false));
</script>

{#if figure}
  <nav
    bind:this={nav}
    data-rice-notch
    data-chrome-region
    data-down={down || undefined}
    aria-label={m.ui_navigation()}
    aria-hidden={!shown || undefined}
    inert={!shown}
    style:left={`${figure.x}px`}
    style:width={`${figure.width}px`}
    style:height={`${figure.height}px`}
    style:--notch-flare={`${NOTCH.flare}px`}
    onfocusin={() => holdNotch(focusOwner, true)}
    onfocusout={(event) => {
      // WebKit does not focus a pressed button, so focus leaving for nothing
      // is focus leaving; an open menu holds the notch on its own.
      const next = event.relatedTarget;
      holdNotch(focusOwner, next instanceof Node && nav?.contains(next) === true);
    }}
    ontransitionend={settled}
    ontransitioncancel={settled}
  >
    <svg
      class="notch-ground"
      viewBox={`0 0 ${figure.width} ${figure.height}`}
      width={figure.width}
      height={figure.height}
      aria-hidden="true"
    >
      <path class="notch-fill" d={figure.fill} />
      <path class="notch-edge" d={figure.edge} />
    </svg>
    <div class="notch-row">
      <NavControls />
      <div class="notch-address">{@render address()}</div>
    </div>
  </nav>
{/if}

<style>
  /*
    Hangs from the window's top edge over the page column. Fixed, because its
    rect is given in window points: the same rect it claims and its zones
    name. Up, it rests just past the edge, out of view.
  */
  [data-rice-notch] {
    --notch-fill: var(--color-chrome);

    position: fixed;
    top: 0;
    z-index: 40;
    transform: translateY(calc(-100% - 2px));
    transition: transform 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  [data-rice-notch][data-down] {
    transform: translateY(0);
  }

  :global(:root[data-reduce-motion="true"]) [data-rice-notch] {
    transition: none;
  }

  @media (prefers-reduced-motion: reduce) {
    [data-rice-notch] {
      transition: none;
    }
  }

  /* The sidebar's own material. Over a native material the column is the
     window's ground, which a page beneath would cover, so the notch wears the
     wash the chrome's raised surfaces wear over it; Solid and the bootstrap
     paint keep the column's opaque chrome colour. */
  :global(:root:not([data-material="none"])[data-material]) [data-rice-notch] {
    --notch-fill: color-mix(in srgb, var(--color-chrome) var(--wash-raised), transparent);
  }

  .notch-ground {
    position: absolute;
    inset: 0;
    overflow: visible;
    pointer-events: none;
  }

  .notch-fill {
    fill: var(--notch-fill);
  }

  /* The hidden sidebar's hairline, so it reads against a page as light as
     it is. */
  .notch-edge {
    fill: none;
    stroke: var(--sidebar-card-edge);
    stroke-width: 1;
  }

  /* Inside the flares, level in the notch's depth. */
  .notch-row {
    position: relative;
    display: flex;
    gap: 6px;
    align-items: center;
    height: 100%;
    padding-inline: calc(var(--notch-flare) + 10px);
  }

  .notch-address {
    display: flex;
    flex: 1;
    min-width: 0;
  }

  /* Down, the pointer or the keyboard is already in the notch, so the
     field's tray glyph does not wait for it as it does in the sidebar. */
  [data-rice-notch] :global(.utilities-trigger) {
    opacity: 1;
  }
</style>
