import { preferences } from "$domain/preferences";
import { surface } from "$domain/surface";
import { isHidden } from "./sidebar-mode.svelte";

/**
 * rice fork: whether the top band is drawn. It holds the one address field,
 * so it is the single answer to where that field lives. Work keeps the
 * sidebar's field, and a hidden sidebar keeps upstream's shape exactly; native
 * drops the band for a hidden sidebar the same way (`layout::sidebar`).
 */
export const toolbarBand = () =>
  preferences.value("ui.toolbar") === "top" && surface.currentPage() !== "work" && !isHidden();
