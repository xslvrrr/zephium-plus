import { SvelteMap } from "svelte/reactivity";
import { commands } from "$shared/ipc/bindings";

type Spec = { activate: DOMRect[]; keep: DOMRect };
const specs = new SvelteMap<string, Spec>();
const inside = $state<Record<string, boolean>>({});
let queued = false;
let listening = false;

const plain = ({ x, y, width, height }: DOMRect) => ({ x, y, width, height });

/** rice fork: whether the pointer is in a named zone (native watches it, so
 *  it is known over the page as well as over the chrome). */
export const zoneInside = (name: string) => inside[name] === true;

export function setZone(name: string, spec: Spec | null) {
  if (!listening) {
    listening = true;
    window.addEventListener("zephium:reach-zone", (event) => {
      const { name: zone, inside: now } = (event as CustomEvent<{ name: string; inside: boolean }>)
        .detail;
      // Native repeats a held zone whenever the zones are sent again.
      if (inside[zone] !== now) inside[zone] = now;
    });
  }
  if (spec === null) specs.delete(name);
  else specs.set(name, spec);
  if (queued) return;
  queued = true;
  requestAnimationFrame(() => {
    queued = false;
    void commands.reachZones(
      [...specs].map(([zone, { activate, keep }]) => ({
        name: zone,
        activate: activate.map(plain),
        keep: plain(keep),
      })),
    );
  });
}
