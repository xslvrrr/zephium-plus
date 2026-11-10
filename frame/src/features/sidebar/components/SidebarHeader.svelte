<script lang="ts">
  import * as m from "$shared/i18n/messages";
  import { EllipsisIcon, SidebarLeftIcon } from "@hugeicons/core-free-icons";
  import { uiCommands as ui } from "$domain/ui-commands";
  import { untrack } from "svelte";
  import { IS_MAC, IS_WINDOWS } from "$shared/platform";
  import { fullscreen } from "$shared/lib/fullscreen.svelte";
  import { preferences } from "$domain/preferences";
  import IconButton from "$shared/ui/IconButton";
  import ModeTabs from "./ModeTabs.svelte";
  import NavControls from "./NavControls.svelte";
  import { openSidebarMenu, runMenuExtension } from "../lib/sidebar-menu";
  import SidebarSearch from "./SidebarSearch.svelte";
  import WindowControls from "$shared/ui/WindowControls";
  import { windowGroupDown } from "$session/window-group.svelte";

  let {
    compact,
    ontoggle,
    pageControls = true,
    navControls = true,
    launcher = false,
    search = true,
  }: {
    compact: boolean;
    ontoggle: () => void;
    /** Back, forward and reload act on a page; Work shows none. */
    pageControls?: boolean;
    /** rice fork: the notch carries back, forward and reload instead. */
    navControls?: boolean;
    /** Beside a tool panel, search stands where the shape toggle would. */
    launcher?: boolean;
    /** Tab search in the rail's head; a tool in full has no tabs to search. */
    search?: boolean;
  } = $props();

  // In fullscreen the lights are gone, and so is the room kept for them.
  let lights = $derived(IS_MAC && !fullscreen());
  // rice fork: with the window controls on hover, the window-controls group
  // draws the shape toggle in this place and slides it in with the lights.
  let grouped = $derived(lights && preferences.value("ui.window-controls") === "hover");
  // rice fork: grouped, a head with nothing of its own keeps no room for the
  // lights while they are away: the column starts at the top, and the head
  // opens (sliding the column down) as the group comes down into it.
  let folded = $derived(
    grouped && !launcher && !(pageControls && navControls) && !windowGroupDown(),
  );

  // At rail width the extensions' buttons live in this menu; a popup chosen
  // there hangs from the button that opened it.
  const menuOwner = Symbol("sidebar-header");
  function openMenu(event: MouseEvent) {
    const target = event.currentTarget;
    if (!(target instanceof HTMLButtonElement)) return;
    openSidebarMenu(menuOwner, target.getBoundingClientRect());
  }

  let handledCommand = 0;
  $effect(() => {
    const command = ui.uiCommand();
    if (command.seq === 0 || command.seq === handledCommand) return;
    handledCommand = command.seq;
    if (!command.id.startsWith("extension.action=")) return;
    const id = command.id.slice("extension.action=".length);
    untrack(() => runMenuExtension(menuOwner, id));
  });
</script>

<!--
  macOS keeps AppKit's traffic lights through the overlay title bar, so the
  leading 52px of the top row belongs to the system, except in fullscreen,
  where AppKit hides them. Windows reveals native
  controls at the top-right edge. On Linux the controls are ours, and at rail width they move into the collapsed menu
  because three buttons cannot fit beside anything else.
-->
{#if compact}
  <!--
    The rail's head: which environment you are in, then the two controls that
    act on the column. At 56px two 26px buttons are exactly what fits on one
    line, and the rule below them says where the head ends.
  -->
  <header class="flex shrink-0 flex-col items-center gap-[3px]" aria-label={m.ui_navigation()}>
    {#if lights}
      <div
        class:rice-fold={grouped}
        style:height={grouped && !windowGroupDown() ? "8px" : "var(--titlebar-height)"}
        aria-hidden="true"
      ></div>
    {:else if IS_MAC}
      <div style:height="8px" aria-hidden="true"></div>
    {/if}
    <ModeTabs compact />
    {#if pageControls || search}<div class="flex items-center gap-0.5">
        <!-- Its menu acts on the page and the column's shape; Work has neither. -->
        {#if pageControls}<IconButton
            icon={EllipsisIcon}
            label={m.ui_sidebar_options()}
            size={15}
            buttonSize={26}
            haspopup
            onclick={openMenu}
          />{/if}
        {#if search}<SidebarSearch small />{/if}
      </div>{/if}
    <span class="mt-[6px] mb-[7px] h-px w-[22px] rounded-full bg-border" aria-hidden="true"></span>
  </header>
{:else}
  <header
    class="flex shrink-0 items-center gap-px pe-1.5"
    class:rice-fold={grouped}
    style:height={folded ? "0px" : "var(--titlebar-height)"}
    style:padding-inline-start={lights
      ? "var(--traffic-light-inset)"
      : launcher && IS_WINDOWS
        ? "0"
        : "6px"}
    aria-label={m.ui_navigation()}
  >
    <!-- Beside a tool panel the panel's own close button already gives the
         column back, so a second way to the same shape would only crowd the
         lights; search stands there instead. -->
    {#if launcher && IS_WINDOWS}<span class="flex w-14 shrink-0 justify-center"
        ><ModeTabs compact /></span
      >{:else if launcher}<span class="ms-1.5 flex"><SidebarSearch /></span
      >{:else if !grouped}<IconButton
        icon={SidebarLeftIcon}
        label={m.ui_compact_mode()}
        onclick={ontoggle}
      />{/if}<span class="flex-1" aria-hidden="true"></span>

    {#if pageControls && navControls}<NavControls />{/if}

    {#if !IS_MAC && !IS_WINDOWS}
      <WindowControls />
    {/if}
  </header>
{/if}

<style>
  /* rice fork: the head opens and closes with the window-controls group, on
     the group's own curve. */
  .rice-fold {
    overflow: hidden;
    transition: height 180ms cubic-bezier(0.22, 1, 0.36, 1);
  }

  :global(:root[data-reduce-motion="true"]) .rice-fold {
    transition: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .rice-fold {
      transition: none;
    }
  }
</style>
