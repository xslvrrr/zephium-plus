<script lang="ts">
  import { untrack, type Snippet } from "svelte";
  import * as m from "$shared/i18n/messages";
  import { EllipsisIcon } from "@hugeicons/core-free-icons";
  import { NavControls, openSidebarMenu, runMenuExtension } from "$features/sidebar";
  import { surface } from "$domain/surface";
  import { uiCommands as ui } from "$domain/ui-commands";
  import IconButton from "$shared/ui/IconButton";
  import { tabs } from "$domain/tabs";
  import { commands } from "$shared/ipc/bindings";
  import { reducedMotion } from "$shared/lib/motion";
  import { claimRegion } from "$session/chrome-regions.svelte";
  import { setZone } from "$session/reach-zones.svelte";
  import { holdNotch, notchDown, notchHold } from "$session/notch.svelte";
  import { holdWhilePopupOpen } from "$features/extensions";
  import { NOTCH, notchFigure } from "../lib/shape";

  let {
    content,
    newTab,
    address,
    tools,
  }: {
    /** The page column, in window points; `top` is the page's top edge. */
    content: { x: number; width: number; top?: number };
    /** No web page stands beneath: the new tab, or a pane the chrome draws
     *  in the page's place. The notch then claims nothing over it. */
    newTab: boolean;
    /** The address field, which is the only one in the chrome while the
     *  notch holds it; its trailing tray carries the site menu and the
     *  extensions. */
    address: Snippet;
    /** The extensions' buttons, beside the menu: each opens its popup from
     *  the notch, as on Zen's toolbar. */
    tools?: Snippet;
  } = $props();

  /** The activation strip along the window's top edge, and how far the
   *  keep-up area reaches beyond the notch's rect. */
  const STRIP = 4;
  const REACH = 32;
  /** Longer than the slide, for a transitionend that never comes (a slide
   *  reversed midway, Reduce Motion). */
  const SETTLE_FALLBACK = 400;
  /** The field's row stands this far down the notch, so the 34 px field is
   *  level in its depth; whatever the field shows under itself (a page's
   *  request, the store's install button) flows down from there, and the
   *  notch deepens to hold it with this much below. */
  const ROW_TOP = 8;
  const HANG_PAD = 6;

  const owner = Symbol("notch");
  const focusOwner = Symbol("notch focus");
  const menuOwner = Symbol("notch menu");

  let nav = $state<HTMLElement>();
  let addressBox = $state<HTMLElement>();
  let addressHeight = $state(0);
  // Past the notch's own depth only when something hangs under the field.
  let depth = $derived.by(() => {
    const bottom = ROW_TOP + addressHeight;
    return bottom > NOTCH.height + 0.5 ? Math.ceil(bottom) + HANG_PAD : NOTCH.height;
  });
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
  // Over a web page the notch is cut into it: the page's edge turns down
  // round the notch, whose ground is the window's own. Native cuts the page
  // (`page_notch`); here only the edge is drawn, its flares on the page's
  // top edge.
  let figure = $derived(
    notchFigure(content.x, content.width, depth, overPage ? (content.top ?? 0) : 0),
  );
  $effect(() => {
    const cut =
      figure && overPage
        ? {
            left: figure.x + NOTCH.flare,
            width: figure.width - 2 * NOTCH.flare,
            depth: figure.height,
            radius: NOTCH.radius,
            flare: NOTCH.flare,
            down,
          }
        : null;
    void commands.pageNotch(cut, !untrack(reducedMotion));
  });
  $effect(() => () => void commands.pageNotch(null, false));

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

  // A cancelled slide is a reversed one; only a finished slide up settles,
  // and the fallback covers one that never finishes.
  function settled(event: TransitionEvent) {
    if (event.target === nav && event.propertyName === "transform" && !down) shown = false;
    layoutVersion += 1;
  }

  // A page's request (an alert dialog in the field) holds the notch down
  // until it is answered, as an open menu does.
  const HOLDS = '[aria-expanded="true"], :popover-open, [role="alertdialog"]';
  const OPEN = '[popover]:popover-open, [role="dialog"], [role="alertdialog"], [data-open="true"]';

  /** The notch's rect, grown to take in the field with whatever hangs under
   *  it and whatever is open from it. Measured boxes are shifted by the
   *  slide, so the rect is the one the notch has down. */
  function region(): DOMRect | null {
    if (!figure) return null;
    let left = figure.x;
    let top = 0;
    let right = figure.x + figure.width;
    let bottom = figure.height;
    const slide = nav ? nav.getBoundingClientRect().top : 0;
    const boxes = [addressBox, ...(nav?.querySelectorAll<HTMLElement>(OPEN) ?? [])];
    for (const box of boxes) {
      if (!box) continue;
      const measured = box.getBoundingClientRect();
      const rect = new DOMRect(measured.x, measured.y - slide, measured.width, measured.height);
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
      holdNotch(menuOwner, root.querySelector(HOLDS) !== null);
      sizes.disconnect();
      for (const open of root.querySelectorAll(OPEN)) sizes.observe(open);
      layoutVersion += 1;
    };
    // The field's own box: the notch deepens to hold what hangs under it.
    const field = new ResizeObserver(([entry]) => {
      if (entry) addressHeight = entry.borderBoxSize[0]?.blockSize ?? entry.contentRect.height;
      layoutVersion += 1;
    });
    if (addressBox) field.observe(addressBox);
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
      field.disconnect();
      root.removeEventListener("toggle", update, true);
      holdNotch(menuOwner, false);
      // The notch can go (no room for it) with focus still inside.
      holdNotch(focusOwner, false);
    };
  });

  // The sidebar's "…" menu, opened from the notch; an extension chosen in it
  // hangs its popup from this button. The notch stays down while the native
  // menu is open, which the pointer can reach only by leaving the notch.
  const sidebarMenu = Symbol("notch menu button");
  function openMenu(event: MouseEvent) {
    const target = event.currentTarget;
    if (!(target instanceof HTMLButtonElement)) return;
    const release = notchHold();
    void openSidebarMenu(sidebarMenu, target.getBoundingClientRect()).finally(release);
  }
  let handledCommand = 0;
  $effect(() => {
    const command = ui.uiCommand();
    if (command.seq === 0 || command.seq === handledCommand) return;
    handledCommand = command.seq;
    if (!command.id.startsWith("extension.action=")) return;
    const id = command.id.slice("extension.action=".length);
    untrack(() => {
      if (runMenuExtension(sidebarMenu, id)) holdWhilePopupOpen(notchHold);
    });
  });
</script>

{#if figure}
  <nav
    bind:this={nav}
    data-rice-notch
    data-chrome-region
    data-down={down || undefined}
    data-cut={overPage || undefined}
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
    <div class="notch-row" style:--notch-row-top={`${ROW_TOP}px`}>
      <div class="notch-side"><NavControls /></div>
      <div class="notch-address" bind:this={addressBox}>{@render address()}</div>
      <div class="notch-side">
        {@render tools?.()}
        <IconButton
          icon={EllipsisIcon}
          label={m.ui_sidebar_options()}
          size={15}
          buttonSize={28}
          haspopup
          onclick={openMenu}
        />
      </div>
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

  /* Cut into the page, the notch stands on the window's ground. */
  [data-rice-notch][data-cut] .notch-fill {
    fill: none;
  }

  /* The hidden sidebar's hairline, so it reads against a page as light as
     it is. */
  .notch-edge {
    fill: none;
    stroke: var(--sidebar-card-edge);
    stroke-width: 1;
  }

  /* Cut into the page, the page's own edge is the notch's outline: native
     draws the page's hairline along the cut (`page_notch.rs`). */
  [data-rice-notch][data-cut] .notch-edge {
    display: none;
  }

  /* Inside the flares, the field level in the notch's own depth; what the
     field shows under itself flows down, and the notch deepens round it. */
  .notch-row {
    position: relative;
    display: flex;
    gap: 6px;
    align-items: flex-start;
    padding-block-start: var(--notch-row-top);
    padding-inline: calc(var(--notch-flare) + 10px);
  }

  /* Level with the 34 px field. */
  .notch-side {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    height: 34px;
  }

  .notch-address {
    flex: 1;
    min-width: 0;
  }

  /* Down, the pointer or the keyboard is already in the notch, so the
     field's tray glyph does not wait for it as it does in the sidebar. */
  [data-rice-notch] :global(.utilities-trigger) {
    opacity: 1;
  }
</style>
