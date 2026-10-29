<script lang="ts">
  import { Alert02Icon, Settings05Icon } from "@hugeicons/core-free-icons";
  import { untrack } from "svelte";
  import { uiCommands as ui } from "$domain/ui-commands";
  import IconButton from "$shared/ui/IconButton";
  import { openSidebarMenu, runMenuExtension } from "../lib/sidebar-menu";

  let {
    label,
    warning = null,
  }: {
    label: string;
    /** What is wrong with the page in front; the glyph then says so. */
    warning?: string | null;
  } = $props();

  // rice fork: the band sits over the page's column, where a DOM panel would
  // draw under the page view, so its utilities open the sidebar's native
  // menu. An extension popup chosen there hangs from this button.
  const menuOwner = Symbol("toolbar-utilities");
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

<span class="rice-utilities" data-warning={warning ? true : undefined}>
  <IconButton
    icon={warning ? Alert02Icon : Settings05Icon}
    {label}
    size={15}
    buttonSize={26}
    haspopup="menu"
    onclick={openMenu}
  />
</span>

<style>
  .rice-utilities {
    display: contents;
  }

  .rice-utilities[data-warning] :global(.icon-button) {
    color: var(--color-danger);
  }
</style>
