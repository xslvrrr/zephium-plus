<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { SidebarLeftIcon } from "@hugeicons/core-free-icons";
  import IconButton from "$shared/ui/IconButton";
  import { commands } from "$shared/ipc/bindings";
  import { reducedMotion } from "$shared/lib/motion";
  import { surface } from "$domain/surface";
  import { tabs } from "$domain/tabs";
  import { claimRegion } from "$session/chrome-regions.svelte";
  import { setZone, zoneInside } from "$session/reach-zones.svelte";
  import { isHidden, toggleMode } from "$session/sidebar-mode.svelte";
  import { setWindowGroupDown } from "$session/window-group.svelte";

  /**
   * rice fork: with `ui.window-controls=hover`, the traffic lights and the
   * hide-sidebar button are one group at the window's top-left. Up, it waits
   * above the window's edge; the pointer reaching the top strip or the
   * lights brings it down, and native slides the real buttons with it.
   */
  let {
    toggle,
    newTab,
  }: {
    /** The sidebar's own toggle, drawn here wherever its head would draw
     *  it: not beside a tool panel or Settings, and not on the rail. */
    toggle: boolean;
    /** No web page stands beneath, as for the notch: nothing is claimed. */
    newTab: boolean;
  } = $props();

  /** The activation strip along the window's top edge, and how far the
   *  keep-up area reaches beyond the group's rect. */
  const STRIP = 4;
  const REACH = 24;
  /** The share of the group's height over the lights that also brings it
   *  down: only the top fifth, so reaching for Browse and Work just below
   *  does not. The keep-up area stays the whole group. */
  const LIGHTS_REACH = 0.2;
  /** Longer than the slide, for a transitionend that never comes. */
  const SETTLE_FALLBACK = 400;

  const owner = Symbol("window group");

  let group = $state<HTMLElement>();
  let button = $state<HTMLElement>();
  // Layout boxes, which the slide's transform leaves alone: the group's rect
  // down, in window points, since it stands at the window's corner.
  let width = $state(0);
  let height = $state(0);
  let focused = $state(false);
  let down = $derived(zoneInside("group") || focused);
  // On screen at all: down, or still sliding up.
  let shown = $state(false);
  let overPage = $derived(
    !newTab && surface.currentPage() === null && Boolean(tabs.activeTab()?.url),
  );
  // The lights run from the window's edge to the button; without one the
  // group is only them.
  let lightsEnd = $derived.by(() => {
    void width;
    return toggle && button ? button.offsetLeft : width;
  });

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

  // A cancelled slide is a reversed one; only a finished slide up settles.
  function settled(event: TransitionEvent) {
    if (event.target === group && event.propertyName === "transform" && !down) shown = false;
  }

  // Native moves the real buttons together with the slide; the sidebar's
  // head makes room for the group as it comes down.
  $effect(() => {
    void commands.windowGroup(down);
    setWindowGroupDown(down);
  });
  $effect(() => () => setWindowGroupDown(false));
  // Gone (fullscreen, or the preference changed), the group is put away;
  // native follows only in hover mode.
  $effect(() => () => void commands.windowGroup(false));

  // A pointer that goes straight to the corner meets the lights before the
  // strip, so their own rect reveals the group too.
  $effect(() => {
    if (width <= 0 || height <= 0) return;
    setZone("group", {
      activate: [
        new DOMRect(0, 0, width, STRIP),
        new DOMRect(0, 0, lightsEnd, Math.max(STRIP, height * LIGHTS_REACH)),
      ],
      keep: new DOMRect(0, 0, width + REACH, height + REACH),
    });
  });
  $effect(() => () => setZone("group", null));

  // Over a shown sidebar the group stands on chrome already; only a hidden
  // sidebar leaves a page beneath it, which would otherwise take its clicks.
  $effect(() => {
    const claim = shown && overPage && isHidden() && width > 0 && height > 0;
    claimRegion(owner, claim ? new DOMRect(0, 0, width, height) : null);
  });
  $effect(() => () => claimRegion(owner, null));
</script>

<div
  bind:this={group}
  bind:offsetWidth={width}
  bind:offsetHeight={height}
  class="window-group"
  data-tauri-drag-region
  data-chrome-region
  data-down={down || undefined}
  data-sidebar-away={isHidden() || undefined}
  aria-hidden={!shown || undefined}
  inert={!shown}
  onfocusin={() => (focused = true)}
  onfocusout={(event) => {
    const next = event.relatedTarget;
    focused = next instanceof Node && group?.contains(next) === true;
  }}
  ontransitionend={settled}
>
  {#if toggle}<span class="window-group-button" bind:this={button}
      ><IconButton icon={SidebarLeftIcon} label={m.ui_compact_mode()} onclick={toggleMode} /></span
    >{/if}
</div>

<style>
  /*
    Where the sidebar's head stands, from the window's corner: the shell's
    8 px gutter, then the head's own room for the lights and its height, so
    the button lands where the head would have drawn it. Up, it rests just
    past the window's top edge, as the notch does.
  */
  .window-group {
    position: fixed;
    inset-block-start: 0;
    inset-inline-start: 0;
    z-index: 40;
    display: flex;
    align-items: center;
    box-sizing: content-box;
    height: var(--titlebar-height);
    padding-block-start: 8px;
    padding-inline: calc(8px + var(--traffic-light-inset)) 6px;
    transform: translateY(calc(-100% - 2px));
    transition: transform 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  .window-group[data-down] {
    transform: translateY(0);
  }

  :global(:root[data-reduce-motion="true"]) .window-group {
    transition: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .window-group {
      transition: none;
    }
  }

  /* Clear of the lights, as the sidebar's head draws it. */
  .window-group-button {
    display: flex;
    margin-inline-start: var(--sidebar-toggle-gap);
    border-radius: var(--radius-control);
  }

  /* With the sidebar away the button stands over whatever is in its place
     (a page, the new tab's texture, a pane) and needs a ground of its own
     to read against it: the notch's, with the hidden sidebar's hairline. */
  .window-group[data-sidebar-away] .window-group-button {
    background: var(--color-chrome);
    box-shadow: 0 0 0 1px var(--sidebar-card-edge);
  }

  :global(:root:not([data-material="none"])[data-material])
    .window-group[data-sidebar-away]
    .window-group-button {
    background: color-mix(in srgb, var(--color-chrome) var(--wash-raised), transparent);
  }
</style>
