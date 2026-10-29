import { SvelteSet } from "svelte/reactivity";
import { preferences } from "$domain/preferences";
import { surface } from "$domain/surface";
import { zoneInside } from "./reach-zones.svelte";

/** rice fork: the notch toolbar holds the one address field in top mode,
 *  except in Work, which keeps the sidebar's. */
export const notchMode = () =>
  preferences.value("ui.toolbar") === "top" && surface.currentPage() !== "work";

const holds = new SvelteSet<symbol>();
let pinned = $state(false);

/** Something inside the notch needs it down: focus in the field, an open menu. */
export function holdNotch(owner: symbol, held: boolean) {
  if (held) holds.add(owner);
  else holds.delete(owner);
}

/** The new tab and Zephium's own pages keep it down. */
export function pinNotch(value: boolean) {
  pinned = value;
}

export const notchDown = () => notchMode() && (pinned || holds.size > 0 || zoneInside("notch"));
